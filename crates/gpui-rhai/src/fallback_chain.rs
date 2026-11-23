//! Bounded snapshot totals and paint-only route segments.
use rhai::{Array, Dynamic, Engine, EvalAltResult, INT, Map, Position};
use std::collections::BTreeSet;

use crate::canvas::{
    CanvasCommand, CanvasFill, CanvasPathSegment, CanvasScene, CanvasStrokeStyle, CanvasTransform,
    flatten_path,
};

const MAX: INT = 1_000_000_000_000;
const MAX_ATTEMPTS: usize = 24;

fn invalid(msg: &str) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(msg.into(), Position::NONE)
}

fn integer(map: &Map, key: &str, min: INT, max: INT) -> Result<Option<INT>, EvalAltResult> {
    map.get(key)
        .filter(|v| !v.is_unit())
        .map(|v| {
            v.as_int()
                .ok()
                .filter(|n| (min..=max).contains(n))
                .ok_or_else(|| invalid("fallback time or cost is out of range"))
        })
        .transpose()
}

fn value(fields: impl IntoIterator<Item = (&'static str, Dynamic)>) -> Dynamic {
    Dynamic::from_map(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

struct Attempt {
    id: rhai::ImmutableString,
    status: rhai::ImmutableString,
    limit: INT,
    started: Option<INT>,
    duration: Option<INT>,
    charged: INT,
}

impl Attempt {
    fn parse(data: Dynamic, now: INT) -> Result<Self, EvalAltResult> {
        let m = data
            .try_cast::<Map>()
            .ok_or_else(|| invalid("attempts must be maps"))?;
        let id = m
            .get("id")
            .and_then(|value| value.clone().try_cast::<rhai::ImmutableString>())
            .filter(|id| !id.is_empty() && id.len() <= 96 && !id.chars().any(char::is_control))
            .ok_or_else(|| invalid("attempt needs a bounded stable ID"))?;
        let status = m
            .get("status")
            .filter(|value| !value.is_unit())
            .map(|value| {
                value
                    .clone()
                    .try_cast::<rhai::ImmutableString>()
                    .ok_or_else(|| invalid("invalid fallback status"))
            })
            .transpose()?
            .unwrap_or_else(|| "waiting".into());
        Ok(Self {
            id,
            status,
            limit: integer(&m, "timeout_ms", 1, MAX)?.unwrap_or(10_000),
            started: integer(&m, "started_ms", 0, now)?,
            duration: integer(&m, "duration_ms", 0, MAX)?,
            charged: integer(&m, "cost_micros", -MAX, MAX)?.unwrap_or(0),
        })
    }
}

fn summary(attempts: Array, now: INT) -> Result<Dynamic, Box<EvalAltResult>> {
    if attempts.len() > MAX_ATTEMPTS || !(0..=MAX).contains(&now) {
        return Err(invalid(
            "fallback chain accepts up to 24 attempts and a bounded elapsed clock",
        )
        .into());
    }
    let mut seen = BTreeSet::new();
    let mut rows = Vec::with_capacity(attempts.len());
    let mut failed = 0;
    let mut skipped = 0;
    let mut tried = 0;
    let mut trying = None;
    let mut winner = None;
    let mut cost = 0;
    let mut time = 0;
    for (index, data) in attempts.into_iter().enumerate() {
        let Attempt {
            id,
            status,
            limit,
            started,
            duration,
            charged,
        } = Attempt::parse(data, now)?;
        if !seen.insert(id) {
            return Err(invalid("duplicate fallback attempt").into());
        }
        let i = INT::try_from(index).expect("bounded attempt index");
        let elapsed = match status.as_str() {
            "waiting" => None,
            "skipped" => {
                skipped += 1;
                None
            }
            "failed" => {
                failed += 1;
                Some(duration.unwrap_or(0))
            }
            "trying" => {
                if trying.replace(i).is_some() {
                    return Err(invalid("a fallback chain cannot try two models at once").into());
                }
                Some(started.map_or(0, |at| now - at))
            }
            "answered" => {
                if winner.replace(i).is_some() {
                    return Err(invalid("a fallback chain has at most one answer").into());
                }
                Some(duration.unwrap_or(0))
            }
            _ => return Err(invalid("invalid fallback status").into()),
        };
        if let Some(elapsed) = elapsed {
            tried += 1;
            time += elapsed;
            cost += charged;
        }
        rows.push(value([
            ("status", Dynamic::from(status)),
            (
                "elapsed_ms",
                elapsed.map_or(Dynamic::UNIT, Dynamic::from_int),
            ),
            ("timeout_ms", Dynamic::from_int(limit)),
            (
                "cost_micros",
                elapsed.map_or(Dynamic::UNIT, |_| Dynamic::from_int(charged)),
            ),
        ]));
    }
    if trying.is_some() && winner.is_some() {
        return Err(invalid("an answered chain cannot still be trying a model").into());
    }
    let state = if winner.is_some() {
        "answered"
    } else if trying.is_some() {
        "trying"
    } else if failed > 0
        && failed + skipped == i64::try_from(rows.len()).expect("bounded attempt count")
    {
        "failed"
    } else {
        "waiting"
    };
    Ok(value([
        ("state", Dynamic::from(state)),
        ("trying", trying.map_or(Dynamic::UNIT, Dynamic::from_int)),
        ("winner", winner.map_or(Dynamic::UNIT, Dynamic::from_int)),
        ("tried", Dynamic::from_int(tried)),
        ("failed", Dynamic::from_int(failed)),
        ("time", Dynamic::from_int(time)),
        ("cost", Dynamic::from_int(cost)),
        ("rows", Dynamic::from_array(rows)),
    ]))
}

fn dash_route(input: Array, on: f64, off: f64) -> Result<Array, Box<EvalAltResult>> {
    if !(2..=12).contains(&input.len())
        || !(0.005..=0.9).contains(&on)
        || !(0.005..=0.9).contains(&off)
    {
        return Err(
            invalid("a dashed branch needs bounded segments and nonzero dash fractions").into(),
        );
    }
    let segments = input
        .into_iter()
        .map(|part| {
            part.try_cast::<CanvasPathSegment>()
                .ok_or_else(|| invalid("branch dashes require Canvas path segments"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let paths = flatten_path(&segments, CanvasTransform::default());
    let mut out = Vec::new();
    for points in paths {
        let length: f64 = points
            .windows(2)
            .map(|pair| (pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1))
            .sum();
        if !(0.001..=10_000.0).contains(&length) {
            continue;
        }
        let mut draw = true;
        let mut remaining = length * on;
        out.push(Dynamic::from(CanvasPathSegment::Move {
            x: points[0].0,
            y: points[0].1,
        }));
        for points in points.windows(2) {
            let [(ax, ay), (bx, by)] = points else {
                unreachable!()
            };
            let segment = (bx - ax).hypot(by - ay);
            if segment <= 1e-9 {
                continue;
            }
            let mut cursor = 0.0;
            while cursor < segment - 1e-9 {
                let end = (cursor + remaining).min(segment);
                let t = end / segment;
                let (x, y) = ((bx - ax).mul_add(t, *ax), (by - ay).mul_add(t, *ay));
                if draw {
                    out.push(Dynamic::from(CanvasPathSegment::Line { x, y }));
                }
                remaining -= end - cursor;
                cursor = end;
                if remaining < 1e-9 {
                    draw = !draw;
                    remaining = length * if draw { on } else { off };
                    if draw {
                        out.push(Dynamic::from(CanvasPathSegment::Move { x, y }));
                    }
                }
            }
        }
    }
    if out.len() < 2 {
        return Ok(segments.into_iter().map(Dynamic::from).collect());
    }
    Ok(out)
}

fn append_dot(dst: &mut Vec<CanvasPathSegment>, x: f64, y: f64) {
    const BEND: f64 = 0.552_284_749_831;
    dst.push(CanvasPathSegment::Move { x: x + 1.0, y });
    for (end_x, end_y, ax, ay, bx, by) in [
        (0., 1., 1., BEND, BEND, 1.),
        (-1., 0., -BEND, 1., -1., BEND),
        (0., -1., -1., -BEND, -BEND, -1.),
        (1., 0., BEND, -1., 1., -BEND),
    ] {
        dst.push(CanvasPathSegment::Cubic {
            x: x + end_x,
            y: y + end_y,
            control_a_x: x + ax,
            control_a_y: y + ay,
            control_b_x: x + bx,
            control_b_y: y + by,
        });
    }
    dst.push(CanvasPathSegment::Close);
}

fn dot_grid(w: f64, h: f64, color: crate::ColorValue) -> Result<CanvasScene, Box<EvalAltResult>> {
    if !(1.0..=800.0).contains(&w) || !(1.0..=2200.0).contains(&h) {
        return Err(invalid("fallback dot grid exceeds its bounded drawable").into());
    }
    let fill = CanvasFill::Solid(color.into());
    let mut commands = Vec::new();
    let mut y = 6.0;
    // Batch each row as one solid path. A sprite tint cannot guarantee the
    // same coverage and alpha at these one-pixel edges.
    while y + 1.0 <= h {
        let mut segments = Vec::new();
        let mut x = 6.0;
        while x + 1.0 <= w {
            append_dot(&mut segments, x, y);
            x += 12.0;
        }
        if !segments.is_empty() {
            commands.push(CanvasCommand::Path {
                key: format!("grid-row:{}", commands.len()),
                segments,
                fill: Some(fill.clone()),
                stroke: None,
                stroke_style: CanvasStrokeStyle::default(),
                transform: CanvasTransform::default(),
                clip: None,
            });
        }
        y += 12.0;
    }
    CanvasScene::new(commands)
        .and_then(|scene| scene.with_view_box(0., 0., w, h))
        .map_err(|e| invalid(&e.to_string()).into())
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("fallback_chain_summary", summary);
    engine.register_fn("fallback_dash_route", dash_route);
    engine.register_fn("fallback_dot_grid", dot_grid);
}

#[cfg(test)]
mod tests {
    use super::register;
    use rhai::{Array, Engine, Map};

    #[test]
    fn fallback_chain_totals_use_accepted_states_not_timeout_inference() {
        let mut engine = Engine::new();
        register(&mut engine);
        let result = engine
            .eval::<Map>(
                r#"fallback_chain_summary([
            #{id:"a",status:"failed",duration_ms:890,cost_micros:1000},
            #{id:"b/界",status:"trying",started_ms:4000,timeout_ms:2000,cost_micros:5300},
            #{id:"c",status:"waiting",cost_micros:999999}],8000)"#,
            )
            .unwrap();
        assert_eq!(result["state"].clone().into_string().unwrap(), "trying");
        assert_eq!(result["time"].as_int().unwrap(), 4890);
        assert_eq!(result["cost"].as_int().unwrap(), 6300);
        assert_eq!(result["trying"].as_int().unwrap(), 1);
        assert!(engine.eval::<Map>(r#"fallback_chain_summary([#{id:"a",status:"trying"},#{id:"b",status:"trying"}],0)"#).is_err());
        let failure=engine.eval::<Map>(r#"fallback_chain_summary([#{id:"a",status:"failed"},#{id:"s",status:"skipped"}],0)"#).unwrap();
        assert_eq!(failure["state"].clone().into_string().unwrap(), "failed");
    }

    #[test]
    fn fallback_chain_dashes_make_gaps_along_a_measured_route() {
        use crate::canvas::CanvasPathSegment::{Line, Move};
        let route = vec![
            rhai::Dynamic::from(Move { x: 0., y: 0. }),
            rhai::Dynamic::from(Line { x: 100., y: 0. }),
        ];
        let out: Array = super::dash_route(route, 0.1, 0.1).unwrap();
        assert!(
            matches!(out[1].clone().cast::<crate::canvas::CanvasPathSegment>(),Line{x,y} if (x-10.).abs()<1e-9 && y.abs()<1e-9)
        );
        assert!(
            matches!(out[2].clone().cast::<crate::canvas::CanvasPathSegment>(),Move{x,..} if (x-20.).abs()<1e-9)
        );
    }
}
