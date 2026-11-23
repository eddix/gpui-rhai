//! Pure bounded data and geometry for a dense task field. The script owns
//! selection, and the frame clock is exclusively the SDK's `MotionRuntime`.
use rhai::{Array, Dynamic, Engine, EvalAltResult, INT, Map, Position};

const MAX_TASKS: usize = 1024;
const MAX_VALUE: INT = 1_000_000_000_000;
const STATUS: [&str; 5] = ["queued", "running", "retrying", "done", "failed"];

fn error(message: &str) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(message.into(), Position::NONE)
}

fn value(fields: impl IntoIterator<Item = (&'static str, Dynamic)>) -> Dynamic {
    Dynamic::from_map(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

fn integer(task: &Map, key: &str) -> Result<Option<INT>, Box<EvalAltResult>> {
    task.get(key)
        .filter(|v| !v.is_unit())
        .map(|v| {
            v.as_int()
                .map_err(|_| error("fan-out time and cost fields require signed integers").into())
        })
        .transpose()
}

fn task_id(task: &Map) -> Result<rhai::ImmutableString, EvalAltResult> {
    task.get("id")
        .and_then(|v| v.clone().try_cast::<rhai::ImmutableString>())
        .filter(|id| !id.is_empty() && id.len() <= 96 && !id.chars().any(char::is_control))
        .ok_or_else(|| {
            error("fan-out task IDs must be nonempty, control-free and at most 96 bytes")
        })
}

fn summary(tasks: Array, now: INT, workers: INT) -> Result<Dynamic, Box<EvalAltResult>> {
    if tasks.len() > MAX_TASKS
        || !(0..=MAX_VALUE).contains(&now)
        || workers != -1 && !(1..=1024).contains(&workers)
    {
        return Err(error("fan-out snapshot exceeds its task, time or worker bounds").into());
    }
    let mut ids = Map::new();
    let mut failed = Array::new();
    let mut counts = [0; STATUS.len()];
    let mut total_cost = 0;
    let mut end_max = None;
    let mut duration_sum = 0;
    let mut samples = 0;
    for (index, item) in tasks.into_iter().enumerate() {
        let task = item
            .try_cast::<Map>()
            .ok_or_else(|| error("fan-out tasks must be maps"))?;
        let id = task_id(&task)?;
        if ids
            .insert(
                id.as_str().into(),
                Dynamic::from_int(INT::try_from(index).expect("bounded by MAX_TASKS")),
            )
            .is_some()
        {
            return Err(error("fan-out task IDs must be distinct").into());
        }
        let status = task
            .get("status")
            .and_then(|v| v.clone().try_cast::<rhai::ImmutableString>())
            .and_then(|status| STATUS.iter().position(|name| *name == status.as_str()))
            .ok_or_else(|| error("fan-out task status is not recognized"))?;
        counts[status] += 1;
        let start = integer(&task, "start_ms")?;
        let end = integer(&task, "end_ms")?;
        if start
            .into_iter()
            .chain(end)
            .any(|t| !(0..=MAX_VALUE).contains(&t))
            || start.zip(end).is_some_and(|(start, end)| end < start)
        {
            return Err(error("fan-out time fields must be bounded and ordered").into());
        }
        let cost = integer(&task, "cost_micros")?.unwrap_or(0);
        if !(-MAX_VALUE..=MAX_VALUE).contains(&cost) {
            return Err(error("fan-out task cost exceeds its integer bound").into());
        }
        total_cost += cost;
        if let Some(end) = end {
            end_max = Some(end_max.map_or(end, |previous: INT| previous.max(end)));
        }
        match status {
            3 => {
                if let Some((start, end)) = start.zip(end) {
                    duration_sum += end - start;
                    samples += 1;
                }
            }
            4 => failed.push(Dynamic::from(id)),
            _ => {}
        }
    }
    let total = INT::try_from(ids.len()).expect("bounded by MAX_TASKS");
    let left = total - counts[3] - counts[4];
    let lanes = if workers == -1 {
        (counts[1] + counts[2]).max(1)
    } else {
        workers
    };
    let finished = total > 0 && left == 0;
    // All products are bounded by MAX_TASKS² * MAX_VALUE, below i64::MAX.
    let eta = if left > 0 && samples >= 3 {
        let denominator = samples * lanes;
        Dynamic::from_int((duration_sum * left + denominator / 2) / denominator)
    } else {
        Dynamic::UNIT
    };
    Ok(value([
        (
            "counts",
            value(STATUS.into_iter().zip(counts.map(Dynamic::from_int))),
        ),
        ("ids", Dynamic::from_map(ids)),
        ("failed", Dynamic::from_array(failed)),
        ("total_cost", Dynamic::from_int(total_cost)),
        ("finished", Dynamic::from_bool(finished)),
        (
            "elapsed_ms",
            Dynamic::from_int(if finished {
                end_max.unwrap_or(now)
            } else {
                now
            }),
        ),
        ("eta_ms", eta),
    ]))
}

#[allow(clippy::cast_precision_loss)] // Validated integers never exceed MAX_TASKS.
fn layout(width: f64, cell: INT, gap: INT, count: INT) -> Result<Dynamic, Box<EvalAltResult>> {
    if !width.is_finite()
        || !(0.0..=32768.0).contains(&width)
        || !(1..=128).contains(&cell)
        || !(0..=128).contains(&gap)
        || !(0..=1024).contains(&count)
    {
        return Err(error("fan-out grid dimensions are outside their bounds").into());
    }
    let cell = (cell as f64).min(width);
    let gap = gap as f64;
    let columns = if cell == 0.0 {
        1
    } else {
        ((width + gap) / (cell + gap)).floor().clamp(1.0, 1024.0) as INT
    };
    let pitch = if columns > 1 {
        (width - cell) / (columns - 1) as f64
    } else {
        cell
    };
    let rows = (count + columns - 1) / columns;
    let height = if rows == 0 {
        0.0
    } else {
        cell + (rows - 1) as f64 * (cell + gap)
    };
    Ok(value([
        ("cell", Dynamic::from_float(cell)),
        ("columns", Dynamic::from_int(columns)),
        ("pitch", Dynamic::from_float(pitch)),
        ("stride", Dynamic::from_float(cell + gap)),
        ("height", Dynamic::from_float(height)),
    ]))
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("fan_out_summary", summary);
    engine.register_fn("fan_out_layout", layout);
}

#[cfg(test)]
mod tests {
    use super::register;
    use rhai::{Engine, Map};

    #[test]
    fn fan_out_estimate_uses_success_samples_and_worker_lanes() {
        let mut engine = Engine::new();
        register(&mut engine);
        let m = engine
            .eval::<Map>(
                r#"
            fan_out_summary([
                #{id:"one",status:"done",start_ms:0,end_ms:92,cost_micros:12000},
                #{id:"two",status:"done",start_ms:5,end_ms:108,cost_micros:-18000},
                #{id:"three",status:"done",start_ms:0,end_ms:105},
                #{id:"a/界",status:"failed",start_ms:1,end_ms:701},
                #{id:"next",status:"running"},#{id:"last",status:"retrying"}
            ],500,2)
        "#,
            )
            .unwrap();
        assert_eq!(m["eta_ms"].as_int().unwrap(), 100);
        assert_eq!(m["elapsed_ms"].as_int().unwrap(), 500);
        assert_eq!(m["total_cost"].as_int().unwrap(), -6000);
        assert!(!m["finished"].as_bool().unwrap());
        assert_eq!(
            m["failed"].clone().into_array().unwrap()[0]
                .clone()
                .into_string()
                .unwrap(),
            "a/界"
        );
        assert!(
            engine
                .eval::<Map>(
                    r#"fan_out_summary([#{id:"a",status:"done",start_ms:2,end_ms:1}],0,-1)"#
                )
                .is_err()
        );
        assert!(
            engine
                .eval::<Map>(
                    r#"fan_out_summary([#{id:"a",status:"done"},#{id:"a",status:"queued"}],0,-1)"#
                )
                .is_err()
        );
    }

    #[test]
    fn fan_out_layout_preserves_tracks_on_partial_rows_and_clamps_narrow_points() {
        let mut engine = Engine::new();
        register(&mut engine);
        let m = engine.eval::<Map>("fan_out_layout(158.0,12,4,12)").unwrap();
        assert_eq!(m["columns"].as_int().unwrap(), 10);
        let pitch = m["pitch"].as_float().unwrap();
        assert!((pitch * 9.0 + 12.0 - 158.0).abs() < 1e-12);
        assert!((m["height"].as_float().unwrap() - 28.0).abs() < 1e-12);
        let tiny = engine.eval::<Map>("fan_out_layout(7.0,10,3,1)").unwrap();
        assert_eq!(tiny["columns"].as_int().unwrap(), 1);
        assert!((tiny["cell"].as_float().unwrap() - 7.0).abs() < 1e-12);
        assert!(
            engine
                .eval::<Map>("fan_out_layout(100.0,12,4,1025)")
                .is_err()
        );
    }
}
