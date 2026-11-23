//! Bounded, side-effect-free task placement and its Rhai adapter.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub limit: u32,
    pub booked: u32,
    pub cards: u16,
    pub card_limit: u16,
    pub past: bool,
}

impl Window {
    fn available(self) -> u32 {
        if self.past || self.cards >= self.card_limit {
            0
        } else {
            self.limit.saturating_sub(self.booked)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub window: usize,
    pub units: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    /// Chronological first fit. An ordinary task is never split just to fill gaps.
    Auto,
    /// Honor the requested first window. Split when needed, wrapping to the earliest
    /// remaining legal window once the chronological tail has been visited.
    Drop(usize),
}

pub fn place(
    windows: &[Window],
    units: u32,
    minimum: u32,
    policy: Placement,
) -> Option<Vec<Piece>> {
    if windows.is_empty() || windows.len() > 56 || units == 0 || minimum == 0 {
        return None;
    }
    let mut order = Vec::with_capacity(windows.len());
    match policy {
        Placement::Auto => {
            let fits_one = windows
                .iter()
                .filter(|window| !window.past)
                .any(|window| units <= window.limit);
            if fits_one {
                let window = windows
                    .iter()
                    .position(|window| window.available() >= units)?;
                return Some(vec![Piece { window, units }]);
            }
            order.extend(0..windows.len());
        }
        Placement::Drop(first) => {
            let free = windows.get(first)?.available();
            if free >= units {
                return Some(vec![Piece {
                    window: first,
                    units,
                }]);
            }
            if free < minimum || units < minimum {
                return None;
            }
            order.extend(first..windows.len());
            order.extend(0..first);
        }
    }

    let eligible: Vec<_> = order
        .into_iter()
        .filter_map(|index| {
            let free = windows[index].available();
            (free >= minimum).then_some((index, free))
        })
        .collect();
    if eligible.is_empty() {
        return None;
    }

    // For exactly K pieces drawn from each suffix, the achievable sum is a
    // contiguous interval [K * minimum, sum of the K largest free windows].
    // Keeping upper bounds means no search table proportional to usage units,
    // and no greedy short final piece or avoidable partial allocation.
    let width = eligible.len() + 1;
    let mut upper = vec![vec![0_u64; width]; width];
    for i in (0..eligible.len()).rev() {
        for k in 1..width - i {
            let include = upper[i + 1][k - 1] + u64::from(eligible[i].1);
            upper[i][k] = if k < eligible.len() - i {
                include.max(upper[i + 1][k])
            } else {
                include
            };
        }
    }

    let mut left = u64::from(units);
    let min = u64::from(minimum);
    let mut pieces = Vec::new();
    for (i, (window, free)) in eligible.iter().copied().enumerate() {
        let free = u64::from(free);
        let rest = if left >= min && left <= free {
            Some(0)
        } else if left >= min {
            let earliest = left.saturating_sub(free);
            (1..eligible.len() - i)
                .filter_map(|k| {
                    let low = earliest.max((k as u64) * min);
                    let high = (left - min).min(upper[i + 1][k]);
                    (low <= high).then_some(low)
                })
                .min()
        } else {
            None
        };
        if let Some(rest) = rest {
            pieces.push(Piece {
                window,
                units: u32::try_from(left - rest).ok()?,
            });
            left = rest;
            if left == 0 {
                return Some(pieces);
            }
        } else if i == 0 && matches!(policy, Placement::Drop(_)) {
            return None;
        }
    }
    None
}

#[derive(Debug, PartialEq, Eq)]
pub struct WeekProposal {
    pub placements: Vec<(usize, Vec<Piece>)>,
    pub remaining: Vec<usize>,
}

pub fn auto_week(windows: &[Window], demand: &[u32], minimum: u32) -> Option<WeekProposal> {
    if windows.is_empty() || windows.len() > 56 || demand.len() > 64 || minimum == 0 {
        return None;
    }
    let mut used = windows.to_vec();
    let mut result = WeekProposal {
        placements: Vec::new(),
        remaining: Vec::new(),
    };
    for (task, units) in demand.iter().copied().enumerate() {
        if let Some(pieces) = place(&used, units, minimum, Placement::Auto) {
            for piece in &pieces {
                let slot = &mut used[piece.window];
                slot.booked += piece.units;
                slot.cards += 1;
            }
            result.placements.push((task, pieces));
        } else {
            result.remaining.push(task);
        }
    }
    Some(result)
}

use std::collections::BTreeSet;

use rhai::{Array, Dynamic, Engine, EvalAltResult, Map};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    windows: Vec<WireWindow>,
    tasks: Vec<WireTask>,
    minimum: u32,
    #[serde(default)]
    target: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireWindow {
    id: String,
    limit: u32,
    booked: u32,
    cards: u16,
    card_limit: u16,
    past: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireTask {
    id: String,
    units: u32,
}

#[derive(Serialize)]
struct Reply<'a> {
    placements: Vec<WirePlacement<'a>>,
    remaining: Vec<&'a str>,
}

#[derive(Serialize)]
struct WirePlacement<'a> {
    task: &'a str,
    pieces: Vec<WirePiece<'a>>,
}

#[derive(Serialize)]
struct WirePiece<'a> {
    window: &'a str,
    units: u32,
}

fn error(message: &'static str) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(message.into(), rhai::Position::NONE)
}

fn validate_ids<'a>(ids: impl IntoIterator<Item = &'a str>) -> Result<(), EvalAltResult> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.is_empty() || id.len() > 256 || id.chars().any(char::is_control) || !seen.insert(id) {
            return Err(error(
                "schedule_capacity requires distinct, bounded identities",
            ));
        }
    }
    Ok(())
}

fn schedule_capacity(input: Map) -> Result<Dynamic, Box<EvalAltResult>> {
    let acceptable_array = |field: &str, max: usize| {
        input
            .get(field)
            .and_then(Dynamic::read_lock::<Array>)
            .is_some_and(|items| items.len() <= max)
    };
    if !acceptable_array("windows", 56) || !acceptable_array("tasks", 64) {
        return Err(
            error("schedule_capacity window or task array exceeds the planning bounds").into(),
        );
    }
    let request: Request = rhai::serde::from_dynamic(&Dynamic::from_map(input)).map_err(|_| {
        error("schedule_capacity expects bounded integer windows, tasks, minimum and an optional target")
    })?;
    if request.windows.is_empty()
        || request.windows.len() > 56
        || request.tasks.len() > 64
        || request.minimum == 0
        || request
            .windows
            .iter()
            .any(|window| window.limit == 0 || window.card_limit == 0)
        || request.tasks.iter().any(|task| task.units == 0)
    {
        return Err(error(
            "schedule_capacity requires 1–56 nonzero windows, up to 64 nonzero tasks, and a positive minimum",
        ).into());
    }
    validate_ids(request.windows.iter().map(|window| window.id.as_str()))?;
    validate_ids(request.tasks.iter().map(|task| task.id.as_str()))?;
    let windows = request
        .windows
        .iter()
        .map(|w| Window {
            limit: w.limit,
            booked: w.booked,
            cards: w.cards,
            card_limit: w.card_limit,
            past: w.past,
        })
        .collect::<Vec<_>>();
    let plan = if let Some(target) = request.target.as_deref() {
        let first = request
            .windows
            .iter()
            .position(|window| window.id == target)
            .ok_or_else(|| error("schedule_capacity target does not name a window"))?;
        let [task] = request.tasks.as_slice() else {
            return Err(error(
                "schedule_capacity accepts exactly one task for a targeted placement",
            )
            .into());
        };
        let Some(pieces) = place(
            &windows,
            task.units,
            request.minimum,
            Placement::Drop(first),
        ) else {
            return Ok(Dynamic::UNIT);
        };
        WeekProposal {
            placements: vec![(0, pieces)],
            remaining: Vec::new(),
        }
    } else {
        let demand = request
            .tasks
            .iter()
            .map(|task| task.units)
            .collect::<Vec<_>>();
        auto_week(&windows, &demand, request.minimum)
            .ok_or_else(|| error("schedule_capacity could not construct a proposal"))?
    };
    let result = Reply {
        placements: plan
            .placements
            .iter()
            .map(|(i, parts)| WirePlacement {
                task: &request.tasks[*i].id,
                pieces: parts
                    .iter()
                    .map(|piece| WirePiece {
                        window: &request.windows[piece.window].id,
                        units: piece.units,
                    })
                    .collect(),
            })
            .collect(),
        remaining: plan
            .remaining
            .iter()
            .map(|i| request.tasks[*i].id.as_str())
            .collect(),
    };
    rhai::serde::to_dynamic(result)
        .map_err(|_| error("could not encode schedule_capacity result").into())
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("schedule_capacity", schedule_capacity);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rhai_adapter_uses_stable_ids_and_rejects_malformed_requests() {
        let mut engine = Engine::new();
        register(&mut engine);
        let input = r#"
          let a = #{id:"tue/am",limit:100,booked:30,cards:1,card_limit:2,past:false};
          let b = #{id:"wed/am",limit:100,booked:0,cards:0,card_limit:2,past:false};
          schedule_capacity(#{windows:[a,b],tasks:[#{id:"run/a",units:130}],minimum:15})
        "#;
        let actual: serde_json::Value =
            rhai::serde::from_dynamic(&engine.eval::<Dynamic>(input).unwrap()).unwrap();
        assert_eq!(
            actual,
            serde_json::json!({
                "placements":[{"task":"run/a","pieces":[{"window":"tue/am","units":70},{"window":"wed/am","units":60}]}],
                "remaining":[]
            })
        );
        assert!(
            engine
                .eval::<Dynamic>("schedule_capacity(#{windows:[],tasks:[],minimum:0})")
                .is_err()
        );
        assert!(
            engine
                .eval::<Dynamic>(&input.replace("units:130", "units:-1"))
                .is_err()
        );
        assert!(
            engine
                .eval::<Dynamic>(&input.replace("tue/am", "wed/am"))
                .is_err()
        );
    }

    fn slot(limit: u32, booked: u32) -> Window {
        Window {
            limit,
            booked,
            cards: 0,
            card_limit: 2,
            past: false,
        }
    }

    #[test]
    fn a_small_task_does_not_get_partitioned_in_auto() {
        assert_eq!(
            place(
                &[slot(10, 6), slot(10, 4), slot(10, 3)],
                7,
                2,
                Placement::Auto
            ),
            Some(vec![Piece {
                window: 2,
                units: 7
            }])
        );
        assert_eq!(
            place(&[slot(10, 6), slot(10, 4)], 7, 2, Placement::Auto),
            None
        );
    }

    #[test]
    fn avoid_tiny_tail_without_discarding_earlier_window() {
        assert_eq!(
            place(&[slot(100, 0), slot(100, 0)], 107, 15, Placement::Auto),
            Some(vec![
                Piece {
                    window: 0,
                    units: 92
                },
                Piece {
                    window: 1,
                    units: 15
                }
            ])
        );
    }

    #[test]
    fn past_capacity_does_not_define_a_whole_task_for_the_future() {
        let windows = [
            Window {
                past: true,
                ..slot(500, 0)
            },
            slot(100, 0),
            slot(100, 0),
        ];
        let placed = place(&windows, 150, 15, Placement::Auto).unwrap();
        assert_eq!(
            placed
                .iter()
                .map(|p| (p.window, p.units))
                .collect::<Vec<_>>(),
            [(1, 100), (2, 50)]
        );
    }

    #[test]
    fn manual_wrap_honors_requested_window_and_skips_past() {
        let mut a = slot(100, 0);
        a.past = true;
        let windows = [a, slot(100, 0), slot(100, 65)];
        assert_eq!(
            place(&windows, 65, 15, Placement::Drop(2)),
            Some(vec![
                Piece {
                    window: 2,
                    units: 35
                },
                Piece {
                    window: 1,
                    units: 30
                }
            ])
        );
        assert_eq!(place(&windows, 65, 15, Placement::Drop(0)), None);
        assert_eq!(place(&windows, 150, 15, Placement::Drop(2)), None);
    }

    #[test]
    fn refusal_does_not_consume_capacity_and_slot_count_is_capped() {
        let mut first = slot(100, 30);
        first.cards = 1;
        let windows = [first, slot(100, 0)];
        let proposal = auto_week(&windows, &[190, 55, 12, 18, 12], 15).unwrap();
        assert_eq!(proposal.remaining, [0, 4]);
        assert_eq!(
            proposal.placements,
            [
                (
                    1,
                    vec![Piece {
                        window: 0,
                        units: 55
                    }]
                ),
                (
                    2,
                    vec![Piece {
                        window: 1,
                        units: 12
                    }]
                ),
                (
                    3,
                    vec![Piece {
                        window: 1,
                        units: 18
                    }]
                )
            ]
        );
        assert_eq!(windows[0].booked, 30);
    }

    fn brute(caps: &[u32], goal: u32, min: u32, must_use_first: bool) -> bool {
        if goal == 0 {
            return true;
        }
        let Some((&head, tail)) = caps.split_first() else {
            return false;
        };
        if !must_use_first && brute(tail, goal, min, false) {
            return true;
        }
        (min..=head.min(goal)).any(|n| brute(tail, goal - n, min, false))
    }

    #[test]
    fn exhaustive_small_domains_match_a_separate_feasibility_oracle() {
        let mut cases = 0;
        for x in 0..=6 {
            for y in 0..=6 {
                for z in 0..=6 {
                    let slots = [slot(6, 6 - x), slot(6, 6 - y), slot(6, 6 - z)];
                    for minimum in 1..=4 {
                        for units in 1..=18 {
                            for first in 0..3 {
                                let caps = [x, y, z];
                                let order =
                                    [caps[first], caps[(first + 1) % 3], caps[(first + 2) % 3]];
                                let want =
                                    caps[first] >= units || brute(&order, units, minimum, true);
                                let result = place(&slots, units, minimum, Placement::Drop(first));
                                assert_eq!(
                                    result.is_some(),
                                    want,
                                    "{caps:?}, {units}, {minimum}, {first}"
                                );
                                if let Some(pieces) = result {
                                    assert_eq!(pieces[0].window, first);
                                    assert_eq!(
                                        pieces.iter().map(|piece| piece.units).sum::<u32>(),
                                        units
                                    );
                                    for p in &pieces {
                                        assert!(
                                            p.units <= caps[p.window]
                                                && (pieces.len() == 1 || p.units >= minimum)
                                        );
                                    }
                                }
                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(cases, 74_088);
    }
}
