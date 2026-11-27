//! One ordered projection for token capacity, a quantized field, and an exact readout.
use rhai::{Array, Dynamic, Engine, EvalAltResult, INT, Map, Position};

const MAX_PARTS: usize = 32;
const MAX_CELLS: INT = 384;
pub(crate) const MAX_COUNT: INT = 9_007_199_254_740_991;

#[derive(Debug, PartialEq, Eq)]
struct Capacity {
    used: INT,
    remaining: INT,
    overflow: INT,
    owners: Vec<INT>,
}

fn project(weights: &[INT], limit: INT, cells: INT) -> Option<Capacity> {
    if weights.len() > MAX_PARTS
        || !(0..=MAX_CELLS).contains(&cells)
        || !(0..=MAX_COUNT).contains(&limit)
        || weights.iter().any(|n| !(0..=MAX_COUNT).contains(n))
    {
        return None;
    }
    let used = weights.iter().copied().try_fold(0, INT::checked_add)?;
    let remaining = (limit - used).max(0);
    let overflow = if limit > 0 { (used - limit).max(0) } else { 0 };
    let mut owners = vec![-1; usize::try_from(cells).ok()?];
    if limit > 0 {
        let capacity = i128::from(limit);
        let n = i128::from(cells);
        let mut prefix = 0_i128;
        let mut start = 0;
        for (index, weight) in weights.iter().copied().enumerate() {
            prefix += i128::from(weight);
            // Round cumulative boundaries, not each part independently;
            // otherwise many tiny, unrelated parts can invent occupied cells.
            let end =
                usize::try_from(((2 * prefix * n + capacity) / (2 * capacity)).min(n)).ok()?;
            owners[start..end].fill(INT::try_from(index).ok()?);
            start = end;
        }
    }
    Some(Capacity {
        used,
        remaining,
        overflow,
        owners,
    })
}

fn bound_error() -> EvalAltResult {
    EvalAltResult::ErrorRuntime(
        Dynamic::from(
            "context_capacity expects up to 32 bounded integer token counts, a nonnegative limit and up to 384 cells",
        ),
        Position::NONE,
    )
}

fn projection(weights: Array, limit: INT, cells: INT) -> Result<Map, Box<EvalAltResult>> {
    if weights.len() > MAX_PARTS {
        return Err(bound_error().into());
    }
    let counts: Vec<INT> = weights
        .into_iter()
        .map(|value| value.as_int().map_err(|_| bound_error()))
        .collect::<Result<_, _>>()?;
    let result = project(&counts, limit, cells).ok_or_else(bound_error)?;
    Ok(Map::from_iter([
        ("used".into(), Dynamic::from(result.used)),
        ("remaining".into(), Dynamic::from(result.remaining)),
        ("overflow".into(), Dynamic::from(result.overflow)),
        (
            "owners".into(),
            Dynamic::from_array(result.owners.into_iter().map(Dynamic::from).collect()),
        ),
    ]))
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("context_capacity", projection);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cumulative_rounding_does_not_allocate_a_cell_to_every_tiny_part() {
        let a = project(&[1; 32], 10_000, 216).unwrap();
        assert_eq!(a.used, 32);
        assert_eq!(a.owners.iter().filter(|owner| **owner >= 0).count(), 1);
        let a = project(&[4, 0, 8, 9], 10, 10).unwrap();
        assert_eq!(a.owners, [0, 0, 0, 0, 2, 2, 2, 2, 2, 2]);
        assert_eq!(a.overflow, 11);
        assert_eq!(a.remaining, 0);
    }

    #[test]
    fn totals_and_weighted_spans_match_the_exact_capacity_for_small_domains() {
        let mut checked = 0;
        for cap in 1_i64..=13 {
            for a in 0_i64..=7 {
                for b in 0_i64..=7 {
                    for c in 0_i64..=7 {
                        for n in 1_i64..=12 {
                            let r = project(&[a, b, c], cap, n).unwrap();
                            let mut remaining = cap;
                            let mut expected = Vec::new();
                            // Independent cell-centre oracle: a tie belongs to
                            // the earlier occupied region; free space comes last.
                            for j in 0..n {
                                let tick = cap * (2 * j + 1);
                                let owner = if tick <= 2 * n * a {
                                    0
                                } else if tick <= 2 * n * (a + b) {
                                    1
                                } else if tick <= 2 * n * (a + b + c) {
                                    2
                                } else {
                                    -1
                                };
                                expected.push(owner);
                            }
                            for cost in [a, b, c] {
                                remaining = remaining.saturating_sub(cost).max(0);
                            }
                            assert_eq!(r.owners, expected);
                            assert_eq!(r.used, a + b + c);
                            assert_eq!(r.remaining, remaining);
                            assert_eq!(r.overflow, (a + b + c - cap).max(0));
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(checked, 79_872);
    }

    #[test]
    fn runtime_projection_rejects_invalid_inputs_and_marks_an_unknown_limit() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(
            engine
                .eval::<bool>("context_capacity([10,20],0,2).owners == [-1,-1]")
                .unwrap()
        );
        assert!(
            engine
                .eval::<Dynamic>("context_capacity([-1],100,12)")
                .is_err()
        );
        assert!(
            engine
                .eval::<Dynamic>("context_capacity([1.5],100,12)")
                .is_err()
        );
        assert!(
            engine
                .eval::<Dynamic>("context_capacity([1],100,385)")
                .is_err()
        );
        let r = project(&[MAX_COUNT; MAX_PARTS], MAX_COUNT, MAX_CELLS).unwrap();
        assert_eq!(r.used, MAX_COUNT * INT::try_from(MAX_PARTS).unwrap());
        assert!(r.owners.iter().all(|owner| *owner == 0));
    }
}
