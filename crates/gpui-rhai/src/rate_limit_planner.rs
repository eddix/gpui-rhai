//! Minute-resolution forecasting and bounded, deterministic proposal search.
use std::collections::BTreeSet;

use rhai::{Array, Dynamic, Engine, EvalAltResult, ImmutableString, Map, Position};

const DAY: usize = 1440;
const CAP: usize = 64;
const MAX_LOAD: i32 = 1_000_000_000;
const ROUND: f64 = 28.;
const TAIL: f64 = 0.12;

type Error = Box<EvalAltResult>;

fn err(message: &str) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(message.to_owned().into(), Position::NONE)
}

fn as_map(value: Dynamic) -> Result<Map, Error> {
    value
        .try_cast()
        .ok_or_else(|| err("rate planner expected a map").into())
}

fn opt<'a>(m: &'a Map, k: &str) -> Option<&'a Dynamic> {
    m.get(k).filter(|v| !v.is_unit())
}

fn text(m: &Map, k: &str) -> Option<ImmutableString> {
    opt(m, k)?.clone().try_cast()
}

fn integer(m: &Map, k: &str, min: i32, max: i32) -> Result<Option<i32>, Error> {
    opt(m, k)
        .map(|v| {
            v.as_int()
                .ok()
                .and_then(|value| i32::try_from(value).ok())
                .filter(|n| (min..=max).contains(n))
                .ok_or_else(|| err("rate planner integer outside its bound").into())
        })
        .transpose()
}

fn number(v: &Dynamic) -> Option<f64> {
    v.as_float()
        .ok()
        .or_else(|| i32::try_from(v.as_int().ok()?).ok().map(f64::from))
        .filter(|v| v.is_finite())
}

fn minute(m: &Map, k: &str) -> Result<Option<usize>, Error> {
    let end = i32::try_from(DAY - 1).expect("minutes in one day fit i32");
    integer(m, k, 0, end)
        .map(|v| v.map(|n| usize::try_from(n).expect("validated minute in one day")))
}

fn map(entries: impl IntoIterator<Item = (&'static str, Dynamic)>) -> Dynamic {
    Dynamic::from_map(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

fn ix(v: usize) -> Dynamic {
    Dynamic::from_int(i64::try_from(v).expect("bounded plan index"))
}
fn idx(v: Option<usize>) -> Dynamic {
    v.map_or(Dynamic::UNIT, ix)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Day {
    Weekdays,
    Weekend,
}

struct Config {
    start: usize,
    now: Option<usize>,
    checkpoint: Option<usize>,
    day: Day,
    limit: f64,
    buffer: f64,
    selected: Option<ImmutableString>,
}

impl Config {
    fn parse(mut m: Map) -> Result<Self, Error> {
        let start = minute(&m, "start")?.unwrap_or(360);
        let day = match text(&m, "day").as_deref() {
            None | Some("weekdays") => Day::Weekdays,
            Some("weekend") => Day::Weekend,
            _ => return Err(err("unknown planning day").into()),
        };
        let limit = f64::from(
            integer(&m, "limit", 1, MAX_LOAD)?
                .ok_or_else(|| err("a positive limit is required"))?,
        );
        let buffer = opt(&m, "buffer")
            .map_or(Some(0.8), number)
            .filter(|x| (0.05..=1.).contains(x))
            .ok_or_else(|| err("safety buffer must be within 0.05 and 1"))?;
        Ok(Self {
            start,
            now: minute(&m, "now")?.map(|m| relative(m, start)),
            checkpoint: minute(&m, "checkpoint")?.map(|m| relative(m, start)),
            day,
            limit,
            buffer,
            selected: m.remove("selected").and_then(Dynamic::try_cast),
        })
    }

    fn locked(&self, at: usize) -> bool {
        self.now.is_some_and(|now| at < now)
    }
    fn absolute(&self, offset: usize) -> usize {
        (offset + self.start) % DAY
    }
}

fn relative(at: usize, start: usize) -> usize {
    (at + DAY - start) % DAY
}

struct Run {
    id: ImmutableString,
    index: usize,
    start: usize,
    /// Reusable tail; tiny values are dropped with the same rule for every query.
    shape: Vec<f64>,
    busy: usize,
}

fn shape(peak: f64, rise: i32, half: i32) -> (Vec<f64>, usize) {
    let rise_time = f64::from(rise);
    let half = f64::from(half);
    let values = (0..DAY)
        .map(|m| {
            let t = day_sample(m);
            let share = if t < rise_time {
                let f = t / rise_time;
                f * f * (3. - 2. * f)
            } else {
                let tail = t - rise_time;
                0.5_f64.powf((tail.hypot(ROUND) - ROUND) / half)
            };
            if share < 0.002 && m > 60 && t > rise_time {
                0.
            } else {
                share
            }
        })
        .collect::<Vec<_>>();
    let busy = (usize::try_from(rise).expect("validated rise within one day")..DAY)
        .find(|i| values[*i] <= TAIL)
        .unwrap_or(DAY);
    (values.into_iter().map(|x| x * peak).collect(), busy)
}

fn day_sample(offset: usize) -> f64 {
    f64::from(u16::try_from(offset).expect("sample offset within one day"))
}

fn active(source: Array, cfg: &Config) -> Result<Vec<Run>, Error> {
    if source.len() > CAP {
        return Err(err("at most 64 runs can be planned").into());
    }
    let mut ids = BTreeSet::new();
    let mut runs = Vec::new();
    for (index, value) in source.into_iter().enumerate() {
        let rec = as_map(value)?;
        let id = text(&rec, "id")
            .filter(|s| !s.trim().is_empty() && s.len() <= 96 && !s.chars().any(char::is_control))
            .ok_or_else(|| err("invalid run ID"))?;
        if !ids.insert(id.clone()) {
            return Err(err("duplicate run ID").into());
        }
        let start = relative(
            minute(&rec, "at")?.ok_or_else(|| err("run needs a start time"))?,
            cfg.start,
        );
        let peak = f64::from(
            integer(&rec, "peak", 0, MAX_LOAD)?.ok_or_else(|| err("run needs a peak forecast"))?,
        );
        let rise = integer(&rec, "rise", 1, 720)?.unwrap_or(30);
        let half = integer(&rec, "half_life", 1, 1440)?.unwrap_or(45);
        let shows = match text(&rec, "days").as_deref() {
            None | Some("daily") => true,
            Some("weekdays") => cfg.day == Day::Weekdays,
            Some("weekend") => cfg.day == Day::Weekend,
            _ => return Err(err("unknown run recurrence").into()),
        };
        if shows {
            let (shape, busy) = shape(peak, rise, half);
            runs.push(Run {
                id,
                index,
                start,
                shape,
                busy,
            });
        }
    }
    Ok(runs)
}

fn update(total: &mut [f64], shape: &[f64], start: usize, sign: f64) {
    for (dest, value) in total[start..].iter_mut().zip(shape) {
        *dest += sign * value;
    }
}

fn peak(values: &[f64]) -> f64 {
    values.iter().copied().fold(0., f64::max)
}

/// The earliest minute at the actual maximum; a close lower sample is not a
/// tie. Inputs are bounded finite forecasts constructed by this module.
fn first_peak(values: &[f64]) -> (usize, f64) {
    values
        .iter()
        .copied()
        .enumerate()
        .fold(
            (0, 0.),
            |best, current| {
                if current.1 > best.1 { current } else { best }
            },
        )
}

fn combined_peak(base: &[f64], floor: f64, shape: &[f64], at: usize, stop_above: f64) -> f64 {
    let mut max = floor;
    for (&base, &v) in base[at..].iter().zip(shape) {
        max = max.max(base + v);
        if max > stop_above {
            break;
        }
    }
    max
}

fn bounds(cfg: &Config, busy: usize) -> Option<(usize, usize)> {
    // End before the busy tail leaves the window; do not rank truncated candidates.
    let lo = cfg.now.unwrap_or(0);
    let hi = DAY.saturating_sub(busy).min(DAY - 1);
    (lo <= hi).then_some((lo, hi))
}

fn fit(run: &Run, cfg: &Config, others: &[f64]) -> Option<usize> {
    let (lo, hi) = bounds(cfg, run.busy)?;
    let floor = peak(others);
    if floor > cfg.limit || cfg.locked(run.start) {
        return None;
    }
    for delta in (5..DAY).step_by(5) {
        for s in [run.start.checked_sub(delta), Some(run.start + delta)]
            .into_iter()
            .flatten()
        {
            if (lo..=hi).contains(&s)
                && combined_peak(others, floor, &run.shape, s, cfg.limit) <= cfg.limit
            {
                return Some(s);
            }
        }
    }
    None
}

fn over_intervals(plan: &[f64], limit: f64) -> Array {
    let mut out = vec![];
    let mut first = None;
    for (i, value) in plan.iter().copied().chain([limit]).enumerate() {
        if value > limit {
            first.get_or_insert(i);
        } else if let Some(start) = first.take() {
            out.push(map([("from", ix(start)), ("to", ix(i))]));
        }
    }
    out
}

fn chart_samples(series: &[f64], lo: usize, hi: usize) -> Array {
    (lo..hi)
        .step_by(6)
        .map(|i| map([("at", ix(i)), ("v", series[i].max(0.).into())]))
        .chain((lo < hi).then(|| map([("at", ix(hi)), ("v", series[hi - 1].max(0.).into())])))
        .collect()
}

fn plot_extent(value: &Dynamic) -> Result<f64, Error> {
    number(value)
        .filter(|n| *n > 0. && *n <= 1.0e7)
        .ok_or_else(|| err("plot extent must be finite and within the native range").into())
}

fn plot_vertices(
    samples: Array,
    width: f64,
    project: impl Fn(f64) -> f64,
) -> Result<Vec<[f64; 2]>, Error> {
    if !(2..=1024).contains(&samples.len()) {
        return Err(err("plot path requires 2–1024 samples").into());
    }
    let mut previous = -1.;
    samples
        .into_iter()
        .map(|sample| {
            let sample = as_map(sample)?;
            let at = opt(&sample, "at")
                .and_then(number)
                .filter(|at| (0. ..=day_sample(DAY)).contains(at) && *at > previous)
                .ok_or_else(|| err("plot sample times must increase within the day"))?;
            let value = opt(&sample, "v")
                .and_then(number)
                .filter(|value| *value >= 0.)
                .ok_or_else(|| err("plot load must be finite and non-negative"))?;
            previous = at;
            let y = project(value);
            if !y.is_finite() || y.abs() > 1.0e7 {
                return Err(err("plot ordinate must fit the native range").into());
            }
            Ok([width * at / day_sample(DAY), y])
        })
        .collect()
}

fn plot_curve(
    samples: Array,
    width: &Dynamic,
    height: &Dynamic,
    ceiling: &Dynamic,
) -> Result<Array, Error> {
    let height = plot_extent(height)?;
    let ceiling = number(ceiling)
        .filter(|n| *n > 0.)
        .ok_or_else(|| err("plot ceiling must be finite and positive"))?;
    let vertices = plot_vertices(samples, plot_extent(width)?, |value| {
        height * (1. - value.min(ceiling) / ceiling)
    })?;
    crate::canvas::catmull_rom_segments(&vertices)
        .map(|path| path.into_iter().map(Dynamic::from).collect())
}

fn plot_load_path(samples: Array, width: &Dynamic, unit: &Dynamic) -> Result<Array, Error> {
    use crate::CanvasPathSegment::{Line, Move};
    let unit = number(unit)
        .filter(|n| *n > 0.)
        .ok_or_else(|| err("plot load unit must be finite and positive"))?;
    let vertices = plot_vertices(samples, plot_extent(width)?, |value| -value / unit)?;
    Ok(vertices
        .into_iter()
        .enumerate()
        .map(|(i, [x, y])| Dynamic::from(if i == 0 { Move { x, y } } else { Line { x, y } }))
        .collect())
}

#[derive(Clone, Copy)]
enum ChartGrid {
    Legacy,
    Responsive { steps: u16 },
}

impl ChartGrid {
    fn from_settings(settings: &Map) -> Result<Self, Error> {
        let Some(width) = opt(settings, "plot_width") else {
            return Ok(Self::Legacy);
        };
        let width = number(width)
            .filter(|width| *width > 0.)
            .ok_or_else(|| err("plot_width must be a finite positive number"))?;
        // At most 1024 vertices, including the two endpoints, so the
        // shared peak and hatch routes can always consume the grid.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = (width / 4.).round().clamp(1., 1023.) as u16;
        Ok(Self::Responsive { steps })
    }

    fn size(self) -> usize {
        match self {
            Self::Legacy => (DAY - 1) / 6 + 2,
            Self::Responsive { steps } => usize::from(steps) + 1,
        }
    }

    fn sample(self, series: &[f64], lo: usize, hi: usize) -> Array {
        let Self::Responsive { steps } = self else {
            return chart_samples(series, lo, hi);
        };
        if lo == hi {
            return vec![];
        }
        let begin = day_sample(lo);
        let end = day_sample(hi);
        let point =
            |at: f64, index: usize| map([("at", at.into()), ("v", series[index].max(0.).into())]);
        let denominator = usize::from(steps);
        let interior = (1..steps).filter_map(|i| {
            let at = day_sample(DAY) * f64::from(i) / f64::from(steps);
            if !(begin < at && at < end) {
                return None;
            }
            // X is evenly spaced; round only the data lookup to a minute.
            let minute = (usize::from(i) * (DAY - 1) + denominator / 2) / denominator;
            Some(point(at, minute.clamp(lo, hi - 1)))
        });
        std::iter::once(point(begin, lo))
            .chain(interior)
            .chain(std::iter::once(point(end, hi - 1)))
            .collect()
    }
}

struct Observation {
    samples: Array,
    pace: Dynamic,
    last: Option<(usize, f64)>,
    hover: Option<f64>,
}

fn observe(
    measured: Array,
    cfg: &Config,
    plan: &[f64],
    hover: Option<usize>,
    grid: ChartGrid,
) -> Result<Observation, Error> {
    if measured.len() > DAY {
        return Err(err("actual load takes at most one day").into());
    }
    let empty = Observation {
        samples: vec![],
        pace: Dynamic::UNIT,
        last: None,
        hover: None,
    };
    let Some(now) = cfg.now.filter(|now| *now > 0 && !measured.is_empty()) else {
        return Ok(empty);
    };
    let end = now.min(measured.len());
    let mut last = 0.;
    let sample = measured
        .into_iter()
        .take(end)
        .map(|value| {
            if let Some(value) = number(&value).filter(|x| (0.0..=f64::from(MAX_LOAD)).contains(x))
            {
                last = value;
            }
            last
        })
        .collect::<Vec<_>>();
    let beg = now.saturating_sub(60);
    let pace = if end == now && now - beg >= 30 {
        let expected = plan[beg..now].iter().sum::<f64>();
        if expected >= 1. {
            let ratio = sample[beg..now].iter().sum::<f64>() / expected;
            let (at, future) = first_peak(&plan[now..]);
            map([
                ("ratio", ratio.into()),
                ("peak", (future * ratio).into()),
                ("at", ix(now + at)),
            ])
        } else {
            Dynamic::UNIT
        }
    } else {
        Dynamic::UNIT
    };
    Ok(Observation {
        samples: grid.sample(&sample, 0, end),
        pace,
        last: Some((end, last)),
        hover: hover.and_then(|minute| sample.get(minute).copied()),
    })
}

fn project_plan(plan: &[f64], observed: Option<(usize, f64)>, grid: ChartGrid) -> Array {
    let Some((restart, end)) = observed else {
        return grid.sample(plan, 0, DAY);
    };
    let delta = end - plan[restart];
    let mut projected = plan.to_vec();
    for (offset, value) in projected[restart..].iter_mut().enumerate() {
        *value = (*value + delta * (-day_sample(offset) / 40.).exp()).max(0.);
    }
    projected[restart] = end;
    grid.sample(&projected, restart, DAY)
}

fn read(source: Array, settings: Map, measured: Array) -> Result<Dynamic, Error> {
    let hover = minute(&settings, "hover")?;
    let grid = ChartGrid::from_settings(&settings)?;
    let cfg = Config::parse(settings)?;
    let runs = active(source, &cfg)?;
    let sel = runs
        .iter()
        .position(|r| Some(&r.id) == cfg.selected.as_ref())
        .or_else(|| runs.iter().position(|r| !cfg.locked(r.start)))
        .or_else(|| (!runs.is_empty()).then_some(0));
    let mut plan = vec![0.; DAY];
    let mut selected = vec![0.; DAY];
    for (i, r) in runs.iter().enumerate() {
        update(&mut plan, &r.shape, r.start, 1.);
        if Some(i) == sel {
            update(&mut selected, &r.shape, r.start, 1.);
        }
    }
    let (at, high) = first_peak(&plan);
    let total = plan.iter().sum::<f64>();
    let over = over_intervals(&plan, cfg.limit);
    let forecast = observe(measured, &cfg, &plan, hover, grid)?;
    // The tooltip describes accepted data, not the downsampled drawing
    // or the continuity adjustment where measured and forecast paths meet.
    let hover = hover.map_or(Dynamic::UNIT, |at| {
        map([
            ("value", forecast.hover.unwrap_or(plan[at]).into()),
            ("measured", forecast.hover.is_some().into()),
        ])
    });
    let mut proposal = None;
    let mut still_over = false;
    if !over.is_empty()
        && let Some(i) = sel
    {
        let run = &runs[i];
        let mut rest = plan.clone();
        update(&mut rest, &run.shape, run.start, -1.);
        still_over = peak(&rest) > cfg.limit;
        if !still_over {
            proposal = fit(run, &cfg, &rest);
        }
    }
    let rows = runs
        .iter()
        .map(|r| {
            map([
                ("index", ix(r.index)),
                ("offset", ix(r.start)),
                ("busy", ix(r.busy.min(DAY - r.start))),
                ("locked", cfg.locked(r.start).into()),
            ])
        })
        .collect();
    Ok(map([
        ("selected", idx(sel)),
        ("now", idx(cfg.now)),
        ("fit", idx(proposal.map(|p| cfg.absolute(p)))),
        ("over_without", still_over.into()),
        ("rows", Dynamic::from_array(rows)),
        ("peak", high.into()),
        ("at", ix(at)),
        ("amount", total.into()),
        ("over", Dynamic::from_array(over)),
        ("chart_grid_size", ix(grid.size())),
        (
            "plan",
            Dynamic::from_array(project_plan(&plan, forecast.last, grid)),
        ),
        (
            "unjoined_plan",
            forecast.last.map_or(Dynamic::UNIT, |_| {
                Dynamic::from_array(grid.sample(&plan, 0, DAY))
            }),
        ),
        (
            "selected_line",
            Dynamic::from_array(grid.sample(&selected, 0, DAY)),
        ),
        ("actual", Dynamic::from_array(forecast.samples)),
        ("hover", hover),
        ("pace", forecast.pace),
        (
            "checkpoint",
            cfg.checkpoint.map_or(Dynamic::UNIT, |p| {
                map([("at", ix(p)), ("load", plan[p].into())])
            }),
        ),
    ]))
}

fn spread(source: Array, settings: Map) -> Result<Array, Error> {
    let cfg = Config::parse(settings)?;
    let runs = active(source, &cfg)?;
    let mut starts = runs.iter().map(|r| r.start).collect::<Vec<_>>();
    let mut total = vec![0.; DAY];
    for r in &runs {
        update(&mut total, &r.shape, r.start, 1.);
    }
    for _ in 0..3 {
        for (i, r) in runs.iter().enumerate() {
            let old = starts[i];
            if cfg.locked(r.start) {
                continue;
            }
            let Some((lo, hi)) = bounds(&cfg, r.busy) else {
                continue;
            };
            let lo = lo.max(r.start.saturating_sub(180)).div_ceil(15) * 15;
            let hi = hi.min(r.start + 180);
            let current = peak(&total);
            update(&mut total, &r.shape, old, -1.);
            let base = peak(&total);
            let mut best = old;
            let mut score = current;
            for start in (lo..=hi).step_by(15) {
                let candidate = combined_peak(&total, base, &r.shape, start, score);
                if candidate < score - 1e-7
                    || (candidate - score).abs() <= 1e-7
                        && start.abs_diff(r.start) < best.abs_diff(r.start)
                {
                    best = start;
                    score = candidate;
                }
            }
            starts[i] = best;
            update(&mut total, &r.shape, best, 1.);
        }
    }
    Ok(runs
        .iter()
        .zip(starts)
        .filter(|(r, s)| r.start != *s)
        .map(|(r, s)| map([("index", ix(r.index)), ("at", ix(cfg.absolute(s)))]))
        .collect())
}

fn add_slot(source: Array, settings: Map, peak_value: i64) -> Result<Dynamic, Error> {
    let cfg = Config::parse(settings)?;
    let runs = active(source, &cfg)?;
    let peak_value = i32::try_from(peak_value)
        .ok()
        .filter(|value| (0..=MAX_LOAD).contains(value))
        .ok_or_else(|| err("invalid peak forecast"))?;
    let (shape, busy) = shape(f64::from(peak_value), 40, 70);
    let Some((lo, hi)) = bounds(&cfg, busy) else {
        return Ok(Dynamic::UNIT);
    };
    let mut base = vec![0.; DAY];
    for r in &runs {
        update(&mut base, &r.shape, r.start, 1.);
    }
    let floor = peak(&base);
    let mut best = None;
    let mut score = f64::INFINITY;
    for at in (lo.div_ceil(15) * 15..=hi).step_by(15) {
        let value = combined_peak(&base, floor, &shape, at, score);
        if value <= cfg.limit * cfg.buffer {
            return Ok(ix(cfg.absolute(at)));
        }
        if value < score {
            best = Some(at);
            score = value;
        }
    }
    Ok(idx(best.map(|s| cfg.absolute(s))))
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("rate_plan_read", read);
    engine.register_fn(
        "rate_plan_curve",
        |samples: Array, width: Dynamic, height: Dynamic, ceiling: Dynamic| {
            plot_curve(samples, &width, &height, &ceiling)
        },
    );
    engine.register_fn(
        "rate_plan_load_path",
        |samples: Array, width: Dynamic, unit: Dynamic| plot_load_path(samples, &width, &unit),
    );
    engine.register_fn("rate_plan_spread", spread);
    engine.register_fn("rate_plan_add_slot", add_slot);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plot_paths_project_samples_without_changing_the_underlying_load_order() {
        use crate::CanvasPathSegment::{Cubic, Line, Move};
        let mut engine = Engine::new();
        register(&mut engine);
        let values = "[#{at:0,v:0},#{at:720,v:13},#{at:1440,v:17}]";
        let curve = engine
            .eval::<Array>(&format!("rate_plan_curve({values},720,132,13)"))
            .unwrap();
        assert_eq!(
            curve[0].clone_cast::<crate::CanvasPathSegment>(),
            Move { x: 0., y: 132. }
        );
        assert!(matches!(
            curve[2].clone_cast::<crate::CanvasPathSegment>(),
            Cubic { x: 720., y: 0., .. }
        ));
        let loads = engine
            .eval::<Array>(&format!("rate_plan_load_path({values},720,10)"))
            .unwrap();
        assert_eq!(
            loads[2].clone_cast::<crate::CanvasPathSegment>(),
            Line { x: 720., y: -1.7 }
        );
        assert!(
            engine
                .eval::<Array>("rate_plan_curve([#{at:30,v:0},#{at:30,v:1}],720,132,13)")
                .is_err()
        );
        assert!(
            engine
                .eval::<Array>("rate_plan_load_path([#{at:0,v:0},#{at:1,v:-1}],720,10)")
                .is_err()
        );
    }

    #[test]
    fn rate_plan_offsets_recurring_runs_and_keeps_lock_and_exact_intervals() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(
            engine
                .eval::<bool>(
                    r#"
            let runs=[
              #{id:"locked",at:370,peak:5800,rise:1,half_life:30},
              #{id:"future",at:390,peak:7400,rise:1,half_life:45},
              #{id:"weekend",at:390,peak:100000,days:"weekend"}];
            let c=#{limit:9000,start:360,now:380,day:"weekdays",selected:"future",checkpoint:432};
            let a=rate_plan_read(runs,c,[]);
            let shift=rate_plan_spread(runs,c);
            for move in shift { runs[move.index].at=move.at; }
            let b=rate_plan_read(runs,c,[]);
            a.rows.len==2 && a.rows[0].locked && !a.rows[1].locked && a.peak>9000.0 &&
            a.over[0].from>=30 && a.over[0].to>a.over[0].from && a.fit!=() &&
            b.peak<a.peak && runs[0].at==370 && runs[2].at==390 && a.checkpoint.at==72
        "#
                )
                .unwrap()
        );
    }

    #[test]
    fn rate_plan_measured_gaps_stop_at_available_history() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(
            engine
                .eval::<bool>(
                    r#"
            let cfg=#{limit:10000,start:360,now:410};
            let m=rate_plan_read([#{id:"r",at:360,peak:4500}],cfg,[3200,(),-5,4700]);
            m.actual.len==2 && m.actual[1].at==4 && m.actual[1].v==4700.0 &&
            m.plan[0].at==4 && m.plan[0].v==4700.0 && m.pace==()
        "#
                )
                .unwrap()
        );
    }

    #[test]
    fn chart_grid_tracks_plot_width_but_preserves_the_exact_measurement_join() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(
            engine
                .eval::<bool>(
                    r#"
            let cfg=#{limit:10000,start:360,now:410,hover:3,plot_width:640};
            let runs=[#{id:"r",at:360,peak:4500}];
            let observations=[3200,(),-5,4700];
            let wide=rate_plan_read(runs,cfg,observations);
            cfg.plot_width=360.0;
            let narrow=rate_plan_read(runs,cfg,observations);
            wide.unjoined_plan.len==161 && narrow.unjoined_plan.len==91 &&
                wide.selected_line.len==161 && narrow.selected_line.len==91 &&
                wide.plan[1].at==9.0 && narrow.plan[1].at==16.0 &&
                wide.actual[1]==wide.plan[0] && narrow.actual[1]==narrow.plan[0] &&
                wide.plan[0].at==4.0 && wide.plan[0].v==4700.0 &&
                wide.peak==narrow.peak && wide.at==narrow.at && wide.hover==narrow.hover &&
                wide.over==narrow.over && wide.plan[wide.plan.len-1].at==1440.0
        "#,
                )
                .unwrap()
        );
    }

    #[test]
    fn hover_uses_exact_accepted_samples_and_never_calls_missing_history_measured() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(
            engine
                .eval::<bool>(
                    r#"
            let runs = [#{id:"run",at:360,peak:3600,rise:6}];
            let actual = [1000,900,()];
            let p = #{limit:10000,start:360,now:390,hover:2};
            let gap = rate_plan_read(runs,p,actual).hover;
            p.hover=3;
            let short_history = rate_plan_read(runs,p,actual).hover;
            p.now=362;
            let future = rate_plan_read(runs,p,actual).hover;
            p.hover=1439;
            let last = rate_plan_read(runs,p,[]).hover;
            p.hover=();
            let absent = rate_plan_read(runs,p,actual).hover;
            gap.measured && gap.value==900.0 && !short_history.measured &&
                short_history.value==1800.0 && future==short_history &&
                !last.measured && absent==()
        "#
                )
                .unwrap()
        );
        assert!(
            engine
                .eval::<Dynamic>("rate_plan_read([],#{limit:1000,hover:1440},[])")
                .is_err()
        );
    }

    #[test]
    fn rate_plan_never_suggests_a_truncated_add() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(
            engine
                .eval::<bool>(
                    r"
            rate_plan_add_slot([],#{limit:100,start:360,now:350},15)==() &&
            rate_plan_add_slot([],#{limit:100,start:360,now:420},15)==420
        "
                )
                .unwrap()
        );
    }

    #[test]
    fn declared_load_and_time_limits_survive_the_script_boundary() {
        let mut engine = Engine::new();
        register(&mut engine);
        let full = engine
            .eval::<bool>(
                r#"
            let result=rate_plan_read([#{id:"max",at:1439,peak:1000000000,rise:1,half_life:1440}],
                #{limit:1000000000,start:1438,now:2,checkpoint:1439},
                [1000000000,1000000001,9223372036854775807,7.25]);
            result.rows[0].offset==1 && result.peak==1000000000.0 &&
                result.actual[1].at==4 && result.actual[1].v==7.25 &&
                result.plan[0].at==4 && result.plan[0].v==7.25 && result.checkpoint.at==1
        "#,
            )
            .unwrap();
        assert!(full);
        for invalid in [
            "rate_plan_read([],#{limit:1000000001},[])",
            "rate_plan_read([],#{limit:9223372036854775807},[])",
            "rate_plan_read([],#{limit:20,start:1440},[])",
            "rate_plan_read([],#{limit:20,start:-1},[])",
            "rate_plan_read([#{id:\"r\",at:1,peak:0,rise:721}],#{limit:20},[])",
            "rate_plan_add_slot([],#{limit:20},9223372036854775807)",
        ] {
            assert!(engine.eval::<Dynamic>(invalid).is_err(), "{invalid}");
        }
        assert_eq!(first_peak(&[0., 4., 4., 3.999_999_999]), (1, 4.));
        assert_eq!(first_peak(&[4., 4.000_000_001, 4.]), (1, 4.000_000_001));
    }
}
