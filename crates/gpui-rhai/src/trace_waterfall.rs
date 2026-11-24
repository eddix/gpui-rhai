//! Bounded tree, exact cost/token roll-ups and one canonical time domain for `TraceWaterfall`.
use std::collections::{BTreeMap, BTreeSet};

use rhai::{Array, Dynamic, Engine, EvalAltResult, ImmutableString, Map, Position};

const CAP: usize = 128;
const MAX_METRIC: i64 = 1_000_000_000_000;
const MAX_MS: i64 = 31_536_000_000; // One year of elapsed milliseconds.
const HOUR: i64 = 3_600_000;
const RANGES: &[i64] = &[
    5, 10, 15, 20, 30, 45, 60, 90, 120, 180, 300, 450, 600, 900, 1200, 1800, 2700, 3600, 5400, 7200,
];
const STEPS: &[i64] = &[
    1, 2, 3, 5, 10, 15, 20, 30, 60, 120, 300, 600, 900, 1800, 3600,
];

type Error = EvalAltResult;
fn bad(s: &str) -> Error {
    EvalAltResult::ErrorRuntime(s.to_owned().into(), Position::NONE)
}

fn string(m: &Map, key: &str) -> Option<ImmutableString> {
    m.get(key)?.clone().try_cast::<ImmutableString>()
}

fn number(m: &Map, key: &str, max: i64) -> Result<Option<i64>, Error> {
    match m.get(key).filter(|item| !item.is_unit()) {
        Some(x) => x
            .as_int()
            .ok()
            .filter(|v| (0..=max).contains(v))
            .map(Some)
            .ok_or_else(|| bad("trace time or metric is outside its bound")),
        None => Ok(None),
    }
}

fn identifier(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= 96 && !id.chars().any(char::is_control)
}

#[derive(Clone, Copy, Debug, Default)]
struct Totals {
    cost: i64,
    input: i64,
    output: i64,
    llms: i64,
    tools: i64,
    errors: i64,
}

impl Totals {
    fn add(&mut self, child: Self) {
        self.cost += child.cost;
        self.input += child.input;
        self.output += child.output;
        self.llms += child.llms;
        self.tools += child.tools;
        self.errors += child.errors;
    }
}

struct Span {
    id: ImmutableString,
    parent: Option<ImmutableString>,
    start: i64,
    end: Option<i64>,
    running: bool,
    is_call: bool,
    totals: Totals,
}

fn parse(entry: Dynamic) -> Result<Span, Error> {
    let m = entry
        .try_cast::<Map>()
        .ok_or_else(|| bad("trace spans must be maps"))?;
    let id = string(&m, "id")
        .filter(|s| identifier(s))
        .ok_or_else(|| bad("invalid trace span ID"))?;
    let parent = m
        .get("parent")
        .filter(|v| !v.is_unit())
        .map(|_| {
            string(&m, "parent")
                .filter(|s| identifier(s))
                .ok_or_else(|| bad("invalid trace parent ID"))
        })
        .transpose()?;
    let start =
        number(&m, "start_ms", MAX_MS)?.ok_or_else(|| bad("trace span needs a start time"))?;
    let end = number(&m, "end_ms", MAX_MS)?;
    if end.is_some_and(|v| v < start) {
        return Err(bad("trace end precedes start"));
    }
    let (running, failed) = match string(&m, "status").as_deref() {
        Some("running") => (true, false),
        Some("done") => (false, false),
        Some("failed") => (false, true),
        _ => return Err(bad("invalid trace span status")),
    };
    let (llms, tools) = match string(&m, "kind").as_deref() {
        Some("llm") => (1, 0),
        Some("tool") => (0, 1),
        Some("agent" | "subagent") => (0, 0),
        _ => return Err(bad("invalid trace span kind")),
    };
    Ok(Span {
        id,
        parent,
        start,
        end,
        running,
        is_call: llms + tools > 0,
        totals: Totals {
            cost: number(&m, "cost_micros", MAX_METRIC)?.unwrap_or(0),
            input: number(&m, "tokens_in", MAX_METRIC)?.unwrap_or(0),
            output: number(&m, "tokens_out", MAX_METRIC)?.unwrap_or(0),
            llms,
            tools,
            errors: i64::from(failed),
        },
    })
}

fn map(entries: impl IntoIterator<Item = (&'static str, Dynamic)>) -> Dynamic {
    Dynamic::from_map(
        entries
            .into_iter()
            .map(|(key, v)| (key.into(), v))
            .collect(),
    )
}
fn dy(n: usize) -> Dynamic {
    Dynamic::from_int(i64::try_from(n).expect("bounded trace index"))
}

fn ticks(range: i64, fit: i64) -> Array {
    // Beyond the authored short ticks, choose whole hours rather than
    // introducing inconsistent fractional labels at the end of a long run.
    let step = STEPS
        .iter()
        .map(|v| v * 1000)
        .find(|v| range <= v * fit)
        .unwrap_or_else(|| ((range + HOUR * fit - 1) / (HOUR * fit)) * HOUR);
    (0..fit)
        .map(|i| i * step)
        .take_while(|x| *x < range)
        .map(Dynamic::from_int)
        .collect()
}

struct TraceOrder {
    kids: Vec<Vec<usize>>,
    parents: Vec<Option<usize>>,
    root: Option<usize>,
    order: Vec<usize>,
    visible: Vec<usize>,
    depths: Vec<usize>,
    anchor: Vec<Option<usize>>,
}

fn topology(nodes: &[Span], collapsed: &BTreeSet<ImmutableString>) -> Result<TraceOrder, Error> {
    let mut ids = BTreeMap::new();
    let mut kids = vec![Vec::new(); nodes.len()];
    let mut parents = vec![None; nodes.len()];
    let mut root = None;
    for (i, n) in nodes.iter().enumerate() {
        if ids.insert(n.id.as_str(), i).is_some() {
            return Err(bad("duplicate trace span ID"));
        }
    }
    for (i, n) in nodes.iter().enumerate() {
        if let Some(parent) = &n.parent {
            let p = *ids
                .get(parent.as_str())
                .ok_or_else(|| bad("trace span has unknown parent"))?;
            parents[i] = Some(p);
            kids[p].push(i);
        } else if root.replace(i).is_some() {
            return Err(bad("trace requires a single root"));
        }
    }
    let mut order = vec![];
    let mut visible = vec![];
    let mut depths = vec![0; nodes.len()];
    let mut anchor = vec![None; nodes.len()];
    if let Some(root) = root {
        let mut stack = vec![(root, 0, None)];
        while let Some((i, depth, hidden_by)) = stack.pop() {
            depths[i] = depth;
            order.push(i);
            anchor[i] = hidden_by;
            if hidden_by.is_none() {
                visible.push(i);
            }
            let child_anchor = hidden_by.or_else(|| collapsed.contains(&nodes[i].id).then_some(i));
            stack.extend(kids[i].iter().rev().map(|c| (*c, depth + 1, child_anchor)));
        }
    }
    if order.len() != nodes.len() {
        return Err(bad("trace contains a parent cycle"));
    }
    Ok(TraceOrder {
        kids,
        parents,
        root,
        order,
        visible,
        depths,
        anchor,
    })
}

fn projection(
    source: Array,
    collapsed: Array,
    now: i64,
    min_range: i64,
    fit: i64,
) -> Result<Dynamic, Box<Error>> {
    if source.len() > CAP
        || collapsed.len() > CAP
        || !(0..=MAX_MS).contains(&now)
        || !(1000..=MAX_MS).contains(&min_range)
        || !(3..=8).contains(&fit)
    {
        return Err(bad("trace projection exceeds its bounds").into());
    }
    let nodes: Vec<_> = source.into_iter().map(parse).collect::<Result<_, _>>()?;
    let collapsed: BTreeSet<_> = collapsed
        .into_iter()
        .map(|v| {
            v.try_cast::<ImmutableString>()
                .filter(|s| identifier(s))
                .ok_or_else(|| bad("invalid folded span ID"))
        })
        .collect::<Result<_, _>>()?;
    let topology = topology(&nodes, &collapsed)?;
    Ok(encode_projection(
        &nodes, &collapsed, topology, now, min_range, fit,
    ))
}

fn encode_projection(
    nodes: &[Span],
    collapsed: &BTreeSet<ImmutableString>,
    TraceOrder {
        kids,
        parents,
        root,
        order,
        visible,
        depths,
        anchor,
    }: TraceOrder,
    now: i64,
    min_range: i64,
    fit: i64,
) -> Dynamic {
    // A folded branch changes visibility only: totals, slowest call and the
    // time domain are all projected from the complete accepted snapshot.
    let mut sum: Vec<_> = nodes.iter().map(|n| n.totals).collect();
    let mut descendants = vec![0; nodes.len()];
    for &i in order.iter().rev() {
        if let Some(p) = parents[i] {
            let row = sum[i];
            sum[p].add(row);
            descendants[p] += descendants[i] + 1;
        }
    }
    let ends: Vec<_> = nodes
        .iter()
        .map(|n| {
            n.end
                .unwrap_or(if n.running { now.max(n.start) } else { n.start })
        })
        .collect();
    let live = nodes.iter().any(|n| n.running);
    let latest = ends
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .max(if live { now } else { 0 });
    let wanted = min_range.max((latest * 6 + 4) / 5);
    let range = RANGES
        .iter()
        .map(|x| x * 1000)
        .find(|x| *x >= wanted)
        .unwrap_or_else(|| ((wanted + HOUR - 1) / HOUR) * HOUR);
    let slowest = order
        .iter()
        .rev()
        .copied()
        .filter(|i| {
            let n = &nodes[*i];
            n.is_call && !n.running && n.end.is_some()
        })
        .max_by_key(|i| ends[*i] - nodes[*i].start);
    let mut fragments: Vec<Vec<usize>> = vec![vec![]; nodes.len()];
    if let Some(root) = root {
        for &i in &order {
            if nodes[i].is_call && i != root {
                let target = anchor[i].unwrap_or(root);
                if target != i {
                    fragments[target].push(i);
                }
            }
        }
    }
    let rows = (0..nodes.len())
        .map(|i| {
            let n = &nodes[i];
            let stats = sum[i];
            let marks = fragments[i].iter().copied().map(dy).collect();
            map([
                ("index", dy(i)),
                ("depth", dy(depths[i])),
                ("parent", parents[i].map_or(Dynamic::UNIT, dy)),
                ("hidden_by", anchor[i].map_or(Dynamic::UNIT, dy)),
                (
                    "first_child",
                    kids[i].first().copied().map_or(Dynamic::UNIT, dy),
                ),
                ("descendants", dy(descendants[i])),
                (
                    "folded",
                    Dynamic::from_bool(!kids[i].is_empty() && collapsed.contains(&n.id)),
                ),
                ("start", n.start.into()),
                ("end", ends[i].into()),
                ("duration", (ends[i] - n.start).into()),
                ("cost", stats.cost.into()),
                ("tokens_in", stats.input.into()),
                ("tokens_out", stats.output.into()),
                ("marks", Dynamic::from_array(marks)),
            ])
        })
        .collect();
    let s = root.map_or_else(Default::default, |i| sum[i]);
    map([
        ("range", range.into()),
        ("latest", latest.into()),
        ("now", now.into()),
        ("live", live.into()),
        ("root", root.map_or(Dynamic::UNIT, dy)),
        ("rows", Dynamic::from_array(rows)),
        (
            "visible",
            Dynamic::from_array(visible.into_iter().map(dy).collect()),
        ),
        ("ticks", Dynamic::from_array(ticks(range, fit))),
        ("slowest", slowest.map_or(Dynamic::UNIT, dy)),
        ("cost", s.cost.into()),
        ("tokens_in", s.input.into()),
        ("tokens_out", s.output.into()),
        ("llms", s.llms.into()),
        ("tools", s.tools.into()),
        ("errors", s.errors.into()),
    ])
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("trace_waterfall_projection", projection);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_waterfall_fold_does_not_change_domain_totals_or_slowest_identity() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(engine.eval::<bool>(r#"
          let spans = [
              #{id:"tool",parent:"child",kind:"tool",status:"failed",start_ms:20,end_ms:6400,cost_micros:65},
              #{id:"root",kind:"agent",status:"running",start_ms:0,cost_micros:40},
              #{id:"child",parent:"root",kind:"subagent",status:"done",start_ms:10,end_ms:7000},
              #{id:"llm",parent:"child",kind:"llm",status:"done",start_ms:3000,end_ms:6400,tokens_in:234,tokens_out:20}
          ];
          let a=trace_waterfall_projection(spans,[],8500,10000,5);
          let b=trace_waterfall_projection(spans,["child"],8500,10000,5);
          a.visible == [1,2,0,3] && b.visible == [1,2] && b.rows[2].marks == [0,3] &&
          a.range==15000 && a.range==b.range && b.slowest==0 && b.cost==105 && b.errors==1 &&
          b.rows[3].hidden_by==2 && b.rows[2].descendants==2 && b.tokens_in==234 && b.tokens_out==20
        "#).unwrap());
    }

    #[test]
    fn trace_waterfall_rejects_cycles_missing_parents_and_backward_spans() {
        let mut engine = Engine::new();
        register(&mut engine);
        for source in [
            r#"[#{id:"a",parent:"a",kind:"agent",status:"done",start_ms:0}]"#,
            r#"[#{id:"a",parent:"b",kind:"agent",status:"done",start_ms:0}]"#,
            r#"[#{id:"a",kind:"agent",status:"done",start_ms:100,end_ms:20}]"#,
            r#"[#{id:"a",kind:"agent",status:"done",start_ms:0},#{id:"a",kind:"llm",status:"done",start_ms:5}]"#,
        ] {
            assert!(
                engine
                    .eval::<Dynamic>(&format!(
                        "trace_waterfall_projection({source},[],400,10000,5)"
                    ))
                    .is_err()
            );
        }
    }
}
