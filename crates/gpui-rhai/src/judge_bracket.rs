//! Bounded tournament projection: an unplayed upstream match is not a bye.
use std::collections::BTreeSet;

use rhai::{Array, Dynamic, Engine, EvalAltResult, ImmutableString, Map, Position};

const MAX_ENTRANTS: usize = 16;
const SLOT_HEIGHT: f64 = 40.0;
const PAIR_GAP: f64 = 4.0;
const MATCH_GAP: f64 = 16.0;

type Error = Box<EvalAltResult>;

fn invalid(text: &str) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(text.to_owned().into(), Position::NONE)
}

fn string(map: &Map, key: &str) -> Option<ImmutableString> {
    map.get(key)?.clone().try_cast()
}

fn identifier(label: &str) -> bool {
    !label.trim().is_empty() && label.len() <= 96 && !label.chars().any(char::is_control)
}

fn contender_ids(source: Array) -> Result<Vec<ImmutableString>, Error> {
    if source.len() > MAX_ENTRANTS {
        return Err(invalid("at most 16 contenders can be bracketed").into());
    }
    let mut seen = BTreeSet::new();
    source
        .into_iter()
        .map(|item| {
            let contender = item
                .try_cast::<Map>()
                .ok_or_else(|| invalid("expected contender records"))?;
            let id = string(&contender, "id")
                .filter(|id| identifier(id))
                .ok_or_else(|| invalid("invalid contender ID"))?;
            if !seen.insert(id.clone()) {
                return Err(invalid("duplicate contender ID").into());
            }
            string(&contender, "name")
                .filter(|name| !name.trim().is_empty() && name.len() <= 512)
                .ok_or_else(|| invalid("contender display name is missing or too long"))?;
            Ok(id)
        })
        .collect()
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Slot {
    Vacant,
    Waiting,
    Contender(usize),
}

impl Slot {
    fn index(self) -> Option<usize> {
        if let Self::Contender(index) = self {
            Some(index)
        } else {
            None
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Vacant => "vacant",
            Self::Waiting => "waiting",
            Self::Contender(_) => "ready",
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum State {
    Empty,
    Waiting,
    Open,
    Bye,
    Decided,
}

impl State {
    fn name(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Waiting => "waiting",
            Self::Open => "open",
            Self::Bye => "bye",
            Self::Decided => "decided",
        }
    }
}

#[derive(Clone, Copy)]
struct Seed {
    slot: Slot,
    y: f64,
}

struct Match {
    key: String,
    round: usize,
    index: usize,
    inputs: [Seed; 2],
    state: State,
    winner: Option<usize>,
    stale: bool,
}

impl Match {
    fn from_inputs(
        round: usize,
        index: usize,
        inputs: [Seed; 2],
        contender_ids: &[ImmutableString],
        verdicts: &Map,
    ) -> Self {
        let key = format!("{round}:{index}");
        let verdict = verdicts.get(key.as_str());
        let (state, winner) = match (inputs[0].slot, inputs[1].slot) {
            (Slot::Vacant, Slot::Vacant) => (State::Empty, None),
            (Slot::Waiting, _) | (_, Slot::Waiting) => (State::Waiting, None),
            (Slot::Contender(id), Slot::Vacant) | (Slot::Vacant, Slot::Contender(id)) => {
                (State::Bye, Some(id))
            }
            (Slot::Contender(left), Slot::Contender(right)) => {
                let accepted = verdict
                    .and_then(|v| v.clone().try_cast::<Map>())
                    .filter(|v| {
                        string(v, "a").as_ref() == contender_ids.get(left)
                            && string(v, "b").as_ref() == contender_ids.get(right)
                    })
                    .and_then(|v| string(&v, "winner"));
                if accepted.as_ref() == contender_ids.get(left) {
                    (State::Decided, Some(left))
                } else if accepted.as_ref() == contender_ids.get(right) {
                    (State::Decided, Some(right))
                } else {
                    (State::Open, None)
                }
            }
        };
        Self {
            key,
            round,
            index,
            inputs,
            state,
            winner,
            stale: verdict.is_some() && state != State::Decided,
        }
    }

    fn next(&self) -> Seed {
        Seed {
            y: self.inputs[0].y.midpoint(self.inputs[1].y),
            slot: self.winner.map_or(
                if self.state == State::Empty {
                    Slot::Vacant
                } else {
                    Slot::Waiting
                },
                Slot::Contender,
            ),
        }
    }

    fn into_script(self, champion: Option<usize>) -> Dynamic {
        let advanced = self.winner.is_some() && self.winner == champion;
        map([
            ("key", self.key.into()),
            ("round", integer(self.round)),
            ("index", integer(self.index)),
            ("a", optional(self.inputs[0].slot.index())),
            ("b", optional(self.inputs[1].slot.index())),
            ("slot_a", self.inputs[0].slot.name().into()),
            ("slot_b", self.inputs[1].slot.name().into()),
            ("ya", self.inputs[0].y.into()),
            ("yb", self.inputs[1].y.into()),
            ("state", self.state.name().into()),
            ("winner", optional(self.winner)),
            ("stale", self.stale.into()),
            ("champion_path", advanced.into()),
        ])
    }
}

fn map(entries: impl IntoIterator<Item = (&'static str, Dynamic)>) -> Dynamic {
    Dynamic::from_map(
        entries
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}

fn integer(value: usize) -> Dynamic {
    Dynamic::from_int(i64::try_from(value).expect("small bracket count"))
}

fn numeric(value: usize) -> f64 {
    f64::from(u32::try_from(value).expect("small bracket count"))
}

fn optional(value: Option<usize>) -> Dynamic {
    value.map_or(Dynamic::UNIT, integer)
}

fn group_height() -> f64 {
    2.0 * SLOT_HEIGHT + PAIR_GAP
}

fn field_size(count: usize, capacity: i64) -> Result<usize, Error> {
    let size = if capacity == 0 {
        count.max(2).next_power_of_two()
    } else {
        usize::try_from(capacity).map_err(|_| invalid("invalid bracket capacity"))?
    };
    if ![2, 4, 8, 16].contains(&size) || size < count {
        return Err(
            invalid("bracket capacity must fit its contenders and be 2, 4, 8 or 16").into(),
        );
    }
    Ok(size)
}

fn rounds(ids: &[ImmutableString], verdicts: &Map, size: usize) -> Vec<Vec<Match>> {
    let mut entrants: Vec<_> = (0..size)
        .map(|index| Seed {
            slot: if index < ids.len() {
                Slot::Contender(index)
            } else {
                Slot::Vacant
            },
            y: numeric(index / 2) * (group_height() + MATCH_GAP)
                + numeric(index % 2) * (SLOT_HEIGHT + PAIR_GAP)
                + SLOT_HEIGHT / 2.0,
        })
        .collect();
    let mut rounds = Vec::new();
    while entrants.len() >= 2 {
        let round = entrants
            .as_chunks::<2>()
            .0
            .iter()
            .enumerate()
            .map(|(index, pair)| Match::from_inputs(rounds.len(), index, *pair, ids, verdicts))
            .collect::<Vec<_>>();
        entrants = round.iter().map(Match::next).collect();
        rounds.push(round);
    }
    rounds
}

fn project(source: Array, verdicts: &Map, capacity: i64) -> Result<Dynamic, Error> {
    if verdicts.len() > MAX_ENTRANTS - 1 {
        return Err(invalid("at most 15 bracket verdicts are supported").into());
    }
    let ids = contender_ids(source)?;
    let field = field_size(ids.len(), capacity)?;
    let schedule = rounds(&ids, verdicts, field);
    let size = schedule.len();
    let champion = (ids.len() >= 2)
        .then(|| schedule[size - 1][0].winner)
        .flatten();
    let current = schedule.iter().position(|round| {
        round
            .iter()
            .any(|m| matches!(m.state, State::Open | State::Waiting))
    });
    let played = schedule
        .iter()
        .flatten()
        .filter(|m| m.state == State::Decided)
        .count();
    let ready = schedule.iter().flatten().find(|m| m.state == State::Open);
    let match_to_play = ready.map_or(Dynamic::UNIT, |m| m.key.clone().into());
    let rows = schedule
        .into_iter()
        .map(|round| {
            Dynamic::from_array(round.into_iter().map(|m| m.into_script(champion)).collect())
        })
        .collect();
    let count = field / 2;
    Ok(map([
        ("rounds", Dynamic::from_array(rows)),
        ("size", integer(size)),
        ("champion", optional(champion)),
        ("current", optional(current)),
        ("played", integer(played)),
        ("total", integer(ids.len().saturating_sub(1))),
        ("next", match_to_play),
        (
            "height",
            (numeric(count) * group_height() + numeric(count - 1) * MATCH_GAP).into(),
        ),
        ("slot_height", SLOT_HEIGHT.into()),
        ("pair_gap", PAIR_GAP.into()),
    ]))
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn(
        "judge_bracket_project",
        |source: Array, verdicts: Map, capacity: i64| project(source, &verdicts, capacity),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> bool {
        let mut engine = Engine::new();
        register(&mut engine);
        engine.eval(source).unwrap()
    }

    #[test]
    fn judge_bracket_true_byes_propagate_but_waiting_matches_do_not() {
        assert!(check(
            r#"
            let entries=[#{id:"a",name:"A"},#{id:"b",name:"B"},#{id:"c",name:"C"}];
            let first=judge_bracket_project(entries,#{},8);
            let decided=judge_bracket_project(entries,#{"0:0":#{a:"a",b:"b",winner:"b"}},8);
            let done=judge_bracket_project(entries,#{"0:0":#{a:"a",b:"b",winner:"b"},
                "1:0":#{a:"b",b:"c",winner:"c"}},8);
            first.rounds[0][1].state=="bye" &&
            first.rounds[1][0].state=="waiting" && first.rounds[1][0].b==2 &&
            first.rounds[2][0].state=="waiting" && first.champion==() &&
            decided.rounds[1][0].state=="open" && decided.rounds[1][0].a==1 &&
            done.rounds[2][0].state=="bye" && done.champion==2 &&
            done.played==2 && done.total==2 && done.current==()
        "#
        ));
    }

    #[test]
    fn judge_bracket_replaced_pairing_cannot_reuse_a_verdict_with_the_same_winner() {
        assert!(check(
            r#"
            let c=[#{id:"a",name:"A"},#{id:"b",name:"B"},#{id:"c",name:"C"},#{id:"d",name:"D"}];
            let stale=judge_bracket_project(c,#{"0:0":#{a:"a",b:"b",winner:"b"},
                "0:1":#{a:"c",b:"d",winner:"c"},"1:0":#{a:"a",b:"c",winner:"c",scores:[1,2]}},0);
            stale.rounds[1][0].stale && stale.rounds[1][0].state=="open" && stale.champion==() &&
            stale.next=="1:0" && stale.rounds[0][0].champion_path==false
        "#
        ));
    }

    #[test]
    fn judge_bracket_keeps_five_seeds_and_does_not_promote_a_single_entry() {
        assert!(check(
            r#"
            let c=[#{id:"a",name:"A"},#{id:"b",name:"B"},#{id:"c",name:"C"},#{id:"d",name:"D"},#{id:"e",name:"E"}];
            let z=judge_bracket_project(c,#{},0);
            let one=judge_bracket_project([c[0]],#{},2);
            z.size==3 && z.rounds[0][2].a==4 && z.rounds[1][1].state=="bye" &&
            z.rounds[2][0].b==4 && z.champion==() && one.champion==()
        "#
        ));
    }

    #[test]
    fn judge_bracket_eliminates_exactly_one_entrant_per_decision_at_every_capacity() {
        for capacity in [2, 4, 8, 16] {
            for count in 2..=capacity {
                let ids: Vec<ImmutableString> = (0..count)
                    .map(|index| format!("seed-{index}").into())
                    .collect();
                let mut verdicts = Map::new();
                for _ in 1..count {
                    let schedule = rounds(&ids, &verdicts, capacity);
                    assert!(
                        schedule.last().unwrap()[0].winner.is_none(),
                        "{count}/{capacity} contenders have unresolved contests"
                    );
                    let next = schedule
                        .iter()
                        .flatten()
                        .find(|m| m.state == State::Open)
                        .unwrap();
                    let left = next.inputs[0].slot.index().unwrap();
                    let right = next.inputs[1].slot.index().unwrap();
                    verdicts.insert(
                        next.key.as_str().into(),
                        map([
                            ("a", ids[left].clone().into()),
                            ("b", ids[right].clone().into()),
                            ("winner", ids[right].clone().into()),
                        ]),
                    );
                }
                let schedule = rounds(&ids, &verdicts, capacity);
                assert_eq!(schedule.last().unwrap()[0].winner, Some(count - 1));
                assert_eq!(
                    schedule
                        .iter()
                        .flatten()
                        .filter(|m| m.state == State::Decided)
                        .count(),
                    count - 1
                );
            }
        }
    }
}
