//! Exact, bounded scheduling projection for the plan comparison surface.
use rhai::{Array, Dynamic, Engine, EvalAltResult, INT, Map, Position};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MAX_STEPS: usize = 96;
const MAX_DEPS: usize = 32;
const MAX_TIME: INT = 31_536_000_000;
const MAX_COST: INT = 1_000_000_000_000;
const TOLERANCE: INT = 1_500;

#[derive(Debug)]
struct Step {
    plan: INT,
    start: Option<INT>,
    end: Option<INT>,
    cost: Option<INT>,
    budget: Option<INT>,
    deps: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Default)]
struct Span {
    start: INT,
    end: INT,
}

impl Span {
    fn len(self) -> INT {
        self.end - self.start
    }
    fn at(start: INT, length: INT) -> Self {
        Self {
            start,
            end: start + length,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Placed {
    plan: Span,
    expected: Span,
    actual: Option<Span>,
}

fn malformed(msg: &str) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(msg.into(), Position::NONE)
}

fn int(data: &Map, key: &str, low: INT, high: INT) -> Result<Option<INT>, EvalAltResult> {
    data.get(key)
        .filter(|v| !v.is_unit())
        .map(|v| {
            v.as_int()
                .ok()
                .filter(|n| (low..=high).contains(n))
                .ok_or_else(|| malformed("plan step integer is outside its declared bound"))
        })
        .transpose()
}

fn id(value: &Dynamic) -> Result<rhai::ImmutableString, EvalAltResult> {
    value
        .clone()
        .try_cast::<rhai::ImmutableString>()
        .filter(|name| !name.is_empty() && name.len() <= 96 && !name.chars().any(char::is_control))
        .ok_or_else(|| {
            malformed("plan step IDs must be nonempty, control free and at most 96 bytes")
        })
}

fn parse_step(
    data: &Map,
    ids: &BTreeMap<rhai::ImmutableString, usize>,
    now: INT,
) -> Result<Step, EvalAltResult> {
    let plan = int(data, "planned_ms", 0, MAX_TIME)?
        .ok_or_else(|| malformed("the step is missing planned_ms"))?;
    let start = int(data, "start_ms", 0, now)?;
    let end = int(data, "end_ms", 0, now)?;
    if start.zip(end).is_some_and(|(a, b)| a > b) {
        return Err(malformed("actual end precedes the actual start"));
    }
    let mut deps = Vec::new();
    if let Some(raw) = data.get("depends_on").filter(|v| !v.is_unit()) {
        let names = raw
            .read_lock::<Array>()
            .filter(|names| names.len() <= MAX_DEPS)
            .ok_or_else(|| malformed("a step may require at most 32 dependencies"))?;
        let mut seen = BTreeSet::new();
        for name in names.iter() {
            let name = id(name)?;
            let index = ids
                .get(&name)
                .copied()
                .ok_or_else(|| malformed("plan references a missing prerequisite"))?;
            if !seen.insert(index) {
                return Err(malformed("duplicate prerequisite in plan step"));
            }
            deps.push(index);
        }
    }
    Ok(Step {
        plan,
        start,
        end,
        deps,
        cost: int(data, "spent_micros", -MAX_COST, MAX_COST)?,
        budget: int(data, "planned_micros", 0, MAX_COST)?,
    })
}

fn place(steps: &[Step], now: INT) -> Result<(Vec<Placed>, Vec<Vec<usize>>), EvalAltResult> {
    let mut after = vec![Vec::new(); steps.len()];
    for (i, step) in steps.iter().enumerate() {
        for dep in &step.deps {
            after[*dep].push(i);
        }
    }
    let mut left: Vec<_> = steps.iter().map(|step| step.deps.len()).collect();
    let mut ready: VecDeque<_> = left
        .iter()
        .enumerate()
        .filter_map(|(i, n)| (*n == 0).then_some(i))
        .collect();
    let mut out = vec![Placed::default(); steps.len()];
    let mut ordered = 0;
    while let Some(i) = ready.pop_front() {
        let step = &steps[i];
        let plan_start = step
            .deps
            .iter()
            .map(|dep| out[*dep].plan.end)
            .max()
            .unwrap_or(0);
        let plan = Span::at(plan_start, step.plan);
        let (expected, actual) = match (step.start, step.end) {
            (start, Some(end)) => {
                let span = Span {
                    start: start.unwrap_or_else(|| (end - step.plan).max(0)),
                    end,
                };
                (span, Some(span))
            }
            (Some(start), None) => (
                Span {
                    start,
                    end: now.max(start + step.plan),
                },
                Some(Span { start, end: now }),
            ),
            (None, None) => {
                let start = step
                    .deps
                    .iter()
                    .map(|dep| out[*dep].expected.end)
                    .max()
                    .unwrap_or(now)
                    .max(now);
                (Span::at(start, step.plan), None)
            }
        };
        out[i] = Placed {
            plan,
            expected,
            actual,
        };
        ordered += 1;
        for &j in &after[i] {
            left[j] -= 1;
            if left[j] == 0 {
                ready.push_back(j);
            }
        }
    }
    if ordered != steps.len() {
        return Err(malformed("plan contains a cyclic dependency"));
    }
    Ok((out, after))
}

fn value(fields: impl IntoIterator<Item = (&'static str, Dynamic)>) -> Dynamic {
    Dynamic::from_map(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

fn indices(items: &[usize]) -> Dynamic {
    Dynamic::from_array(
        items
            .iter()
            .map(|&i| Dynamic::from_int(INT::try_from(i).expect("bounded step index")))
            .collect(),
    )
}

fn row(step: &Step, geometry: Placed, dependents: &[usize]) -> Dynamic {
    let status = if step.end.is_some() {
        "done"
    } else if step.start.is_some() {
        "running"
    } else {
        "waiting"
    };
    let actual_duration = geometry.actual.map_or(0, Span::len);
    let delta = if status == "waiting" {
        0
    } else {
        actual_duration - step.plan
    };
    value([
        ("status", Dynamic::from(status)),
        ("plan_start", Dynamic::from_int(geometry.plan.start)),
        ("plan_end", Dynamic::from_int(geometry.plan.end)),
        ("start", Dynamic::from_int(geometry.expected.start)),
        ("end", Dynamic::from_int(geometry.expected.end)),
        (
            "observed_end",
            geometry
                .actual
                .map_or(Dynamic::UNIT, |r| Dynamic::from_int(r.end)),
        ),
        ("elapsed", Dynamic::from_int(actual_duration)),
        ("delta", Dynamic::from_int(delta)),
        ("over", Dynamic::from_bool(delta >= TOLERANCE)),
        ("deps", indices(&step.deps)),
        ("next", indices(dependents)),
    ])
}

fn axis(later: INT) -> (INT, Vec<Dynamic>) {
    let extent = (later * 122 + 99) / 100;
    let mut tick = if later > 240_000 { 60_000 } else { 30_000 };
    for candidate in [
        120_000,
        300_000,
        600_000,
        900_000,
        1_800_000,
        3_600_000,
        7_200_000,
        10_800_000,
        21_600_000,
        43_200_000,
        86_400_000,
        172_800_000,
        604_800_000,
    ] {
        if extent <= 10 * tick {
            break;
        }
        tick = candidate;
    }
    while extent > 10 * tick {
        tick *= 2;
    }
    let cells = ((extent + tick - 1) / tick).max(1);
    (
        cells * tick,
        (1..cells).map(|i| Dynamic::from_int(i * tick)).collect(),
    )
}

fn optional_sum(amounts: impl Iterator<Item = Option<INT>>) -> Dynamic {
    amounts
        .flatten()
        .fold(None, |sum, x| Some(sum.unwrap_or(0) + x))
        .map_or(Dynamic::UNIT, Dynamic::from_int)
}

fn projection(input: Array, now: INT) -> Result<Dynamic, Box<EvalAltResult>> {
    if input.len() > MAX_STEPS || !(0..=MAX_TIME).contains(&now) {
        return Err(
            malformed("plan comparison accepts up to 96 steps and bounded elapsed time").into(),
        );
    }
    let mut ids = BTreeMap::new();
    let mut data = Vec::new();
    for item in input {
        let step = item
            .try_cast::<Map>()
            .ok_or_else(|| malformed("plan steps must be maps"))?;
        let key = id(step
            .get("id")
            .ok_or_else(|| malformed("plan step is missing its ID"))?)?;
        if ids.insert(key, data.len()).is_some() {
            return Err(malformed("duplicate plan step ID").into());
        }
        data.push(step);
    }
    let steps: Vec<_> = data
        .iter()
        .map(|step| parse_step(step, &ids, now))
        .collect::<Result<_, _>>()?;
    let (placed, after) = place(&steps, now)?;
    let baseline = placed.iter().map(|r| r.plan.end).max().unwrap_or(0);
    let eta = placed.iter().map(|r| r.expected.end).max().unwrap_or(0);
    let finished = !steps.is_empty() && steps.iter().all(|s| s.end.is_some());
    let (range, ticks) = axis(baseline.max(eta).max(if finished { 0 } else { now }));
    let rows = steps
        .iter()
        .zip(placed)
        .zip(after)
        .map(|((s, p), after)| row(s, p, &after))
        .collect();
    Ok(value([
        ("rows", Dynamic::from_array(rows)),
        ("range", Dynamic::from_int(range)),
        ("ticks", Dynamic::from_array(ticks)),
        ("planned_end", Dynamic::from_int(baseline)),
        ("eta", Dynamic::from_int(eta)),
        ("late_by", Dynamic::from_int(eta - baseline)),
        ("finished", Dynamic::from_bool(finished)),
        ("spent", optional_sum(steps.iter().map(|s| s.cost))),
        ("budget", optional_sum(steps.iter().map(|s| s.budget))),
    ]))
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("plan_vs_actual_projection", projection);
}

#[cfg(test)]
mod tests {
    use super::register;
    use rhai::{Array, Engine, Map};

    #[test]
    fn plan_projection_keeps_parallel_branches_and_reversed_input_order() {
        let mut engine = Engine::new();
        register(&mut engine);
        let v = engine
            .eval::<Map>(
                r#"
            plan_vs_actual_projection([
                #{id:"ship",planned_ms:4000,depends_on:["a","b"],planned_micros:0},
                #{id:"b",planned_ms:9000,start_ms:2000,end_ms:8500,spent_micros:-2600},
                #{id:"a",planned_ms:10000,start_ms:1000,spent_micros:7700}
            ],18000)
        "#,
            )
            .unwrap();
        assert_eq!(v["planned_end"].as_int().unwrap(), 14000);
        assert_eq!(v["eta"].as_int().unwrap(), 22000);
        assert_eq!(v["late_by"].as_int().unwrap(), 8000);
        assert_eq!(v["spent"].as_int().unwrap(), 5100);
        assert_eq!(v["budget"].as_int().unwrap(), 0);
        let rows = v["rows"].clone().cast::<Array>();
        let waiting = rows[0].clone().cast::<Map>();
        assert_eq!(waiting["start"].as_int().unwrap(), 18000);
        assert!(waiting["observed_end"].is_unit());
        let running = rows[2].clone().cast::<Map>();
        assert_eq!(running["elapsed"].as_int().unwrap(), 17000);
        assert!(running["over"].as_bool().unwrap());
    }

    #[test]
    fn projection_keeps_observed_time_and_rejects_unfounded_graphs() {
        let mut engine = Engine::new();
        register(&mut engine);
        for case in [
            r#"[#{id:"a",planned_ms:1,depends_on:["a"]}]"#,
            r#"[#{id:"a",planned_ms:1,depends_on:["missing"]}]"#,
            r#"[#{id:"a",planned_ms:1,start_ms:40,end_ms:20}]"#,
            r#"[#{id:"a",planned_ms:1,start_ms:501}]"#,
        ] {
            assert!(
                engine
                    .eval::<Map>(&format!("plan_vs_actual_projection({case},500)"))
                    .is_err()
            );
        }
        let m = engine
            .eval::<Map>(
                r#"plan_vs_actual_projection([#{id:"known",planned_ms:500,end_ms:70}],120000)"#,
            )
            .unwrap();
        assert!(m["finished"].as_bool().unwrap());
        assert_eq!(m["range"].as_int().unwrap(), 30000);
        assert!(m["spent"].is_unit());
        assert!(m["budget"].is_unit());
        assert_eq!(
            m["rows"].clone().cast::<Array>()[0].clone().cast::<Map>()["start"]
                .as_int()
                .unwrap(),
            0
        );
    }
}
