//! Quotation arithmetic, not invoicing or payment execution.
use rhai::{Array, Dynamic, Engine, EvalAltResult, INT, Map, Position};
use std::collections::BTreeSet;

const MAX_LINES: usize = 64;
const MAX_MINOR: INT = 1_000_000_000;
const BASIS: INT = 10_000;

fn invalid(message: &str) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(message.into(), Position::NONE)
}

fn string(line: &Map, key: &str) -> Result<rhai::ImmutableString, EvalAltResult> {
    line.get(key)
        .and_then(|v| v.clone().try_cast())
        .ok_or_else(|| invalid("billing line is missing a string field"))
}

fn number(line: &Map, key: &str, max: INT) -> Result<INT, EvalAltResult> {
    line.get(key)
        .and_then(|v| v.as_int().ok())
        .filter(|v| (0..=max).contains(v))
        .ok_or_else(|| invalid("billing line has an invalid integer amount"))
}

fn months(duration: &str) -> Result<i32, EvalAltResult> {
    match duration {
        "milestone" => Ok(0),
        "1m" => Ok(1),
        "3m" => Ok(3),
        "6m" => Ok(6),
        "12m" => Ok(12),
        _ => Err(invalid("unknown billing phase duration")),
    }
}

fn occurrences(repeat: &str, months: i32) -> Result<INT, EvalAltResult> {
    match repeat {
        "once" | "yearly" => Ok(1),
        "monthly" => Ok(INT::from(months.max(1))),
        "quarterly" => Ok(INT::from((months.max(1) + 2) / 3)),
        _ => Err(invalid("unknown repeat interval")),
    }
}

fn projection(start: &str, duration: &str, lines: Array) -> Result<Dynamic, Box<EvalAltResult>> {
    if lines.len() > MAX_LINES {
        return Err(invalid("billing schedule is limited to 64 lines").into());
    }
    let start = crate::GregorianDate::parse_iso(start)
        .map_err(|_| invalid("billing schedule needs a valid Gregorian start date"))?;
    let months = months(duration)?;
    let end = (months > 0)
        .then(|| {
            start
                .checked_add_months(months)
                .and_then(|v| v.checked_add_days(-1))
                .ok()
        })
        .flatten();
    let mut seen = BTreeSet::new();
    let mut first = 0;
    let mut total = 0;
    let mut discount = None;
    let mut minimum = None;
    for value in lines {
        let line = value
            .try_cast::<Map>()
            .ok_or_else(|| invalid("billing schedule lines must be maps"))?;
        let id = string(&line, "id")?;
        let name = string(&line, "name")?;
        if id.is_empty()
            || id.len() > 96
            || id.chars().any(char::is_control)
            || name.trim().is_empty()
            || name.len() > 512
            || !seen.insert(id)
        {
            return Err(
                invalid("billing lines need distinct bounded IDs and nonempty names").into(),
            );
        }
        match string(&line, "kind")?.as_str() {
            "product" => {
                let amount = number(&line, "price_minor", MAX_MINOR)?;
                let repeat = string(&line, "repeat")?;
                total += amount * occurrences(repeat.as_str(), months)?;
                match string(&line, "timing")?.as_str() {
                    "advance" => first += amount,
                    "arrears" => {}
                    _ => return Err(invalid("unknown billing timing").into()),
                }
            }
            "discount" if discount.is_none() => {
                discount = Some(number(&line, "discount_bps", BASIS)?);
            }
            "minimum" if minimum.is_none() => {
                minimum = Some(number(&line, "price_minor", MAX_MINOR)?);
            }
            "discount" | "minimum" => {
                return Err(invalid("only one discount and one minimum per phase").into());
            }
            _ => return Err(invalid("unknown billing line kind").into()),
        }
    }
    let factor = BASIS - discount.unwrap_or(0);
    // MAX_LINES × 12 × MAX_MINOR × BASIS fits in i64. Apply discount once
    // to each subtotal, rounding a half minor unit upward.
    let adjusted = |amount| (amount * factor + BASIS / 2) / BASIS;
    let before_minimum = adjusted(total);
    let minimum = minimum.unwrap_or(0);
    Ok(Dynamic::from_map(
        [
            ("first_minor", Dynamic::from_int(adjusted(first))),
            (
                "total_minor",
                Dynamic::from_int(before_minimum.max(minimum)),
            ),
            ("before_minimum_minor", Dynamic::from_int(before_minimum)),
            (
                "minimum_applies",
                Dynamic::from_bool(minimum > before_minimum),
            ),
            (
                "end_date",
                end.map_or(Dynamic::UNIT, |d| Dynamic::from(d.to_iso())),
            ),
        ]
        .into_iter()
        .map(|(key, v)| (key.into(), v))
        .collect(),
    ))
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("billing_schedule_projection", projection);
}

#[cfg(test)]
mod tests {
    use super::register;
    use rhai::{Engine, Map};

    #[test]
    fn billing_schedule_applies_discount_once_and_floor_only_to_phase() {
        let mut engine = Engine::new();
        register(&mut engine);
        let m = engine.eval::<Map>(r#"
            billing_schedule_projection("2024-01-31","3m",[
                #{id:"a",kind:"product",name:"A",price_minor:10001,repeat:"monthly",timing:"advance"},
                #{id:"b",kind:"product",name:"B",price_minor:200,repeat:"once",timing:"arrears"},
                #{id:"d",kind:"discount",name:"Discount",discount_bps:1500},
                #{id:"f",kind:"minimum",name:"Floor",price_minor:30000}])
        "#).unwrap();
        assert_eq!(m["first_minor"].as_int().unwrap(), 8501);
        assert_eq!(m["before_minimum_minor"].as_int().unwrap(), 25673);
        assert_eq!(m["total_minor"].as_int().unwrap(), 30000);
        assert!(m["minimum_applies"].as_bool().unwrap());
        assert_eq!(m["end_date"].clone().into_string().unwrap(), "2024-04-29");
    }

    #[test]
    fn billing_schedule_has_explicit_milestones_leap_years_and_failure_boundaries() {
        let mut engine = Engine::new();
        register(&mut engine);
        let m = engine
            .eval::<Map>(r#"billing_schedule_projection("2024-01-31","1m",[])"#)
            .unwrap();
        assert_eq!(m["end_date"].clone().into_string().unwrap(), "2024-02-28");
        for (start, duration) in [("2026-02-03", "milestone"), ("9999-12-01", "1m")] {
            let m = engine
                .eval::<Map>(&format!(
                    "billing_schedule_projection(\"{start}\",\"{duration}\",[])"
                ))
                .unwrap();
            assert!(m["end_date"].is_unit());
        }
        assert!(
            engine
                .eval::<Map>(r#"billing_schedule_projection("2026-02-30","1m",[])"#)
                .is_err()
        );
        assert!(
            engine
                .eval::<Map>(
                    r#"
            billing_schedule_projection("2026-01-01","1m",[
                #{id:"a",kind:"discount",name:"A",discount_bps:0},
                #{id:"b",kind:"discount",name:"B",discount_bps:1000}])
        "#
                )
                .is_err()
        );
    }
}
