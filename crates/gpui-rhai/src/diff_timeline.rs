//! One bounded, ordered edit projection for a timeline's plot and file lanes.

use std::collections::BTreeMap;

use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Position};

const LIMIT: usize = 192;
const MAX_TIME: f64 = 604_800.0;
const MAX_CHANGE: i64 = 10_000_000;

#[derive(Clone, Debug)]
struct Edit {
    index: usize,
    at: f64,
    file: String,
    added: i64,
    removed: i64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Count {
    a: i64,
    d: i64,
}

impl Count {
    fn push(&mut self, change: &Edit) {
        self.a += change.added;
        self.d += change.removed;
    }
}

#[derive(Default)]
struct FileLane {
    file: String,
    first: usize,
    whole: Count,
    shown: Count,
    edits: Vec<usize>,
}

struct Projection {
    all: Vec<Edit>,
    seen: usize,
    end: f64,
    head: f64,
    floor: f64,
    ceiling: f64,
    step: f64,
    total: Count,
    lanes: Vec<FileLane>,
    max_file: i64,
    samples: Vec<[f64; 3]>,
    guides: Vec<[f64; 3]>,
}

fn nice_step(raw: f64) -> f64 {
    let base = 10_f64.powf(raw.max(1.0).log10().floor());
    [1.0, 2.0, 2.5, 5.0, 10.0]
        .into_iter()
        .map(|n| (n * base).ceil())
        .find(|&n| n >= raw)
        .unwrap_or(10.0 * base)
}

#[allow(clippy::cast_precision_loss)] // The bounded ledger's integral sums fit exactly in f64.
fn smooth_at(edits: &[Edit], prefix: &[Count], time: f64, end: f64, ramp: f64) -> [f64; 3] {
    let finish = |e: &Edit| (e.at + ramp).min(end);
    let complete = edits.partition_point(|e| finish(e) <= time);
    let base = prefix[complete];
    let mut a = base.a as f64;
    let mut d = base.d as f64;
    for e in edits[complete..].iter().take_while(|e| e.at <= time) {
        let fraction = ((time - e.at) / (finish(e) - e.at)).clamp(0.0, 1.0);
        let weight = fraction * fraction * (3.0 - 2.0 * fraction);
        a += e.added as f64 * weight;
        d += e.removed as f64 * weight;
    }
    [time, a, d]
}

#[allow(clippy::cast_precision_loss)] // Integral counts are capped before entering the projection.
fn project(mut all: Vec<Edit>, elapsed: Option<f64>, duration: Option<f64>) -> Projection {
    all.sort_by(|a, b| a.at.total_cmp(&b.at).then(a.index.cmp(&b.index)));
    let last = all.last().map_or(0.0, |e| e.at);
    let end = duration
        .unwrap_or_else(|| (((last * 1.04 + 1.0) / 60.0).ceil() * 60.0).max(60.0))
        .max(last)
        .max(1.0);
    let ramp = end * 0.03;
    // Without an elapsed cursor, show every edit including its settled tail.
    let head = elapsed.unwrap_or((last + ramp).min(end)).min(end);
    let seen = all.partition_point(|e| e.at <= head);
    let mut prefix = vec![Count::default()];
    let mut low_net = 0_i64;
    let mut total = Count::default();
    let mut lanes: Vec<FileLane> = Vec::new();
    let mut keyed = BTreeMap::new();
    for (i, e) in all.iter().enumerate() {
        total.push(e);
        prefix.push(total);
        low_net = low_net.min(total.a - total.d);
        let slot = *keyed.entry(e.file.as_str()).or_insert_with(|| {
            let slot = lanes.len();
            lanes.push(FileLane {
                file: e.file.clone(),
                first: e.index,
                ..FileLane::default()
            });
            slot
        });
        let file = &mut lanes[slot];
        file.whole.push(e);
        if i < seen {
            file.shown.push(e);
            file.edits.push(e.index);
        }
    }
    let peak = (total.a.max(total.d) as f64 * 1.04).max(30.0);
    let step = nice_step(peak / 4.0);
    let ceiling = (peak / step).ceil().max(3.0) * step;
    let floor = (low_net as f64 / step).floor() * step;
    let max_file = lanes
        .iter()
        .map(|f| f.whole.a + f.whole.d)
        .max()
        .unwrap_or(0)
        .max(1);
    let draw_at = |t| smooth_at(&all[..seen], &prefix, t, end, ramp);
    let guides: Vec<_> = all[..seen]
        .iter()
        .map(|e| draw_at((e.at + ramp).min(head)))
        .collect();
    // Include start, endpoint, and every edit's visible start/finish in the
    // line itself. Guide dots are therefore on the submitted polyline.
    let n = ((head / end) * 220.0).ceil().max(1.0);
    let mut times = Vec::with_capacity(606);
    let mut i = 0.0;
    while i < n {
        times.push(head * i / n);
        i += 1.0;
    }
    times.push(head);
    for e in &all[..seen] {
        times.push(e.at);
        times.push((e.at + ramp).min(head));
    }
    times.sort_by(f64::total_cmp);
    // Exact authored times share a point; unlike an epsilon this preserves
    // genuinely distinct short edits. Normalize signed zero for the key.
    times.dedup_by_key(|time| (*time + 0.).to_bits());
    let samples = times.into_iter().map(draw_at).collect();
    Projection {
        all,
        seen,
        end,
        head,
        floor,
        ceiling,
        step,
        total: prefix[seen],
        lanes,
        max_file,
        samples,
        guides,
    }
}

fn invalid() -> EvalAltResult {
    EvalAltResult::ErrorRuntime(
        "diff_timeline_projection requires ≤192 edits with a bounded path, time and nonnegative line counts".into(),
        Position::NONE,
    )
}

fn seconds(value: &Dynamic) -> Result<f64, EvalAltResult> {
    let result = value
        .as_float()
        .ok()
        .or_else(|| i32::try_from(value.as_int().ok()?).ok().map(f64::from))
        .ok_or_else(invalid)?;
    if !(0.0..=MAX_TIME).contains(&result) {
        return Err(invalid());
    }
    Ok(result.max(0.0))
}

fn optional_seconds(value: Dynamic) -> Result<Option<f64>, EvalAltResult> {
    // Consume the Rhai argument on the typed fast path. Integer inputs are
    // still accepted by the same bounded decoder as inline edit times.
    match value.try_cast_result::<rhai::FLOAT>() {
        Ok(seconds) if (0.0..=MAX_TIME).contains(&seconds) => Ok(Some(seconds.max(0.))),
        Ok(_) => Err(invalid()),
        Err(value) if value.is_unit() => Ok(None),
        Err(value) => seconds(&value).map(Some),
    }
}

fn value(fields: impl IntoIterator<Item = (&'static str, Dynamic)>) -> Dynamic {
    Dynamic::from_map(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

fn array(items: impl IntoIterator<Item = Dynamic>) -> Dynamic {
    Dynamic::from_array(items.into_iter().collect())
}

fn position(index: usize) -> Dynamic {
    Dynamic::from(i64::try_from(index).expect("bounded timeline indices"))
}

fn point([t, a, d]: [f64; 3]) -> Dynamic {
    value([("t", t.into()), ("a", a.into()), ("d", d.into())])
}

fn projection(
    input: Array,
    elapsed: Dynamic,
    duration: Dynamic,
) -> Result<Dynamic, Box<EvalAltResult>> {
    if input.len() > LIMIT {
        return Err(invalid().into());
    }
    let mut edits = Vec::with_capacity(input.len());
    for (index, row) in input.into_iter().enumerate() {
        let row = row.try_cast::<Map>().ok_or_else(invalid)?;
        let field = |name: &str| row.get(name).ok_or_else(invalid);
        let file = field("file")?
            .clone()
            .into_string()
            .map_err(|_| invalid())?;
        let at = seconds(field("at")?)?;
        let added = field("added")?.as_int().map_err(|_| invalid())?;
        let removed = field("removed")?.as_int().map_err(|_| invalid())?;
        if file.is_empty()
            || file.len() > 512
            || !(0..=MAX_CHANGE).contains(&added)
            || !(0..=MAX_CHANGE).contains(&removed)
        {
            return Err(invalid().into());
        }
        edits.push(Edit {
            index,
            at,
            file,
            added,
            removed,
        });
    }
    let p = project(
        edits,
        optional_seconds(elapsed)?,
        optional_seconds(duration)?,
    );
    let shown = p.all[..p.seen]
        .iter()
        .zip(p.guides)
        .map(|(e, guide)| value([("index", position(e.index)), ("guide", point(guide))]));
    let files = p.lanes.into_iter().map(|f| {
        value([
            ("path", f.file.into()),
            ("first", position(f.first)),
            ("add", f.shown.a.into()),
            ("del", f.shown.d.into()),
            ("final_add", f.whole.a.into()),
            ("final_del", f.whole.d.into()),
            ("edits", array(f.edits.into_iter().map(position))),
        ])
    });
    Ok(value([
        ("end", p.end.into()),
        ("head", p.head.into()),
        ("floor", p.floor.into()),
        ("ceiling", p.ceiling.into()),
        ("step", p.step.into()),
        ("shown", array(shown)),
        ("files", array(files)),
        ("max_file", p.max_file.into()),
        ("add", p.total.a.into()),
        ("del", p.total.d.into()),
        ("samples", array(p.samples.into_iter().map(point))),
    ]))
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("diff_timeline_projection", projection);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(index: usize, at: f64, file: &str, added: i64, removed: i64) -> Edit {
        Edit {
            index,
            at,
            file: file.into(),
            added,
            removed,
        }
    }

    #[test]
    fn future_edits_hold_the_scale_and_reserve_rows_without_showing_totals() {
        let input = vec![
            e(0, 40., "two", 200, 4),
            e(1, 3., "one", 2, 90),
            e(2, 5., "one", 4, 0),
        ];
        let live = project(input.clone(), Some(4.0), None);
        let settled = project(input, None, None);
        assert_eq!(
            [live.end, live.floor, live.ceiling, live.step],
            [settled.end, settled.floor, settled.ceiling, settled.step]
        );
        assert_eq!(live.total, Count { a: 2, d: 90 });
        assert_eq!(live.lanes[0].file, "one");
        assert_eq!(live.lanes[1].shown, Count::default());
        assert!(live.floor <= -88.0);
        assert_eq!(
            settled.samples.last().copied(),
            Some([settled.head, 206., 94.])
        );
    }

    #[test]
    fn overlapping_and_coincident_edits_have_exact_typed_guides_and_one_path_endpoint() {
        let p = project(
            vec![
                e(0, 10., "first", 0, 20),
                e(1, 10., "second", 10, 0),
                e(2, 11., "first", 30, 0),
            ],
            Some(12.0),
            Some(12.0),
        );
        assert_eq!(p.all.iter().map(|e| e.index).collect::<Vec<_>>(), [0, 1, 2]);
        assert_eq!(p.samples.last(), Some(&[12., 40., 20.]));
        assert!(p.guides.iter().all(|point| p.samples.contains(point)));
        assert!(p.samples.windows(2).all(|x| x[0][0] < x[1][0]));
        let edge = project(vec![e(0, 12., "end", 5, 0)], Some(12.), Some(12.));
        assert_eq!(edge.samples.last(), Some(&[12., 5., 0.]));
    }

    #[test]
    fn bridge_rejects_invalid_input_and_keeps_unordered_edits_available_by_source_index() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(engine.eval::<bool>(r#"let p=diff_timeline_projection([
            #{at:20,file:"later",added:7,removed:0},#{at:2,file:"资料/first",added:0,removed:7}],3,60);
            p.shown[0].index==1 && p.files[0].path=="资料/first" && p.files[1].edits.is_empty()
                && p.del==7 && p.add==0 && p.floor<0"#).unwrap());
        assert!(
            engine
                .eval::<Dynamic>(
                    r#"diff_timeline_projection([#{at:-2,file:"x",added:0,removed:7}],(),())"#
                )
                .is_err()
        );
    }
}
