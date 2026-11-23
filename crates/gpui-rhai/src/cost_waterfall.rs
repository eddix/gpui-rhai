//! One projection for the visible tree, exact rollups and signed waterfall lanes.

use std::collections::{BTreeMap, BTreeSet};

use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Position};

const LIMIT: usize = 192;
const MAX_SPEND: i64 = 1_000_000_000_000;
const MAX_TOKENS: i64 = 1_000_000_000_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Amounts {
    micros: i64,
    input: i64,
    output: i64,
    priced: bool,
}

impl Amounts {
    fn merge(&mut self, rhs: Self) {
        self.micros += rhs.micros;
        self.input += rhs.input;
        self.output += rhs.output;
        self.priced |= rhs.priced;
    }
}

#[derive(Clone, Debug)]
struct Step {
    id: String,
    parent: Option<String>,
    agent: bool,
    amounts: Amounts,
}

#[derive(Clone, Copy)]
enum Mode {
    Cost,
    Tokens,
}

impl Mode {
    fn value(self, counts: Amounts) -> i64 {
        match self {
            Self::Cost => counts.micros,
            Self::Tokens => counts.input + counts.output,
        }
    }

    fn minimum(self) -> i64 {
        match self {
            Self::Cost => 100_000,
            Self::Tokens => 10_000,
        }
    }
}

#[derive(Debug)]
struct Row {
    index: usize,
    depth: usize,
    start: i64,
    end: i64,
    folded: bool,
    hidden: bool,
    children: bool,
    sum: Amounts,
}

#[derive(Default)]
struct Projection {
    rows: Vec<Row>,
    rollup: Amounts,
    pos: i64,
    min: i64,
    max: i64,
    top: Option<usize>,
}

struct Tree<'a> {
    steps: &'a [Step],
    children: Vec<Vec<usize>>,
    roots: Vec<usize>,
    collapsed: &'a BTreeSet<String>,
    mode: Mode,
}

impl<'a> Tree<'a> {
    fn new(steps: &'a [Step], collapsed: &'a BTreeSet<String>, mode: Mode) -> Option<Self> {
        if steps.len() > LIMIT || collapsed.len() > LIMIT {
            return None;
        }
        let ids: BTreeMap<_, _> = steps
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.as_str(), i))
            .collect();
        if ids.len() != steps.len() {
            return None;
        }
        let mut roots = Vec::new();
        let mut children = vec![Vec::new(); steps.len()];
        for (i, s) in steps.iter().enumerate() {
            if let Some(parent) = &s.parent {
                let position = *ids.get(parent.as_str())?;
                children[position].push(i);
            } else {
                roots.push(i);
            }
        }
        Some(Self {
            steps,
            children,
            roots,
            collapsed,
            mode,
        })
    }

    fn visit(&self, i: usize, depth: usize, hidden: bool, p: &mut Projection) -> Option<Amounts> {
        if depth > 32 {
            return None;
        }
        let s = &self.steps[i];
        let children = !self.children[i].is_empty();
        let folded = children && self.collapsed.contains(&s.id);
        let row = p.rows.len();
        p.rows.push(Row {
            index: i,
            depth,
            start: p.pos,
            end: p.pos,
            folded,
            hidden,
            children,
            sum: s.amounts,
        });
        p.pos += self.mode.value(s.amounts);
        p.min = p.min.min(p.pos);
        p.max = p.max.max(p.pos);
        if !s.agent
            && s.amounts.micros > 0
            && p.top
                .is_none_or(|top| self.steps[top].amounts.micros < s.amounts.micros)
        {
            p.top = Some(i);
        }
        let mut subtotal = s.amounts;
        for &child in &self.children[i] {
            subtotal.merge(self.visit(child, depth + 1, hidden || folded, p)?);
        }
        p.rows[row].end = p.pos;
        p.rows[row].sum = subtotal;
        Some(subtotal)
    }

    fn project(&self) -> Option<Projection> {
        let mut p = Projection::default();
        let mut total = Amounts::default();
        for &root in &self.roots {
            total.merge(self.visit(root, 0, false, &mut p)?);
        }
        // A cycle cannot have a root in this single-parent forest. Never
        // accept a partial view which silently omits such an island.
        if p.rows.len() != self.steps.len() {
            return None;
        }
        p.rollup = total;
        Some(p)
    }
}

fn error() -> EvalAltResult {
    EvalAltResult::ErrorRuntime(
        "cost_waterfall_projection expects a bounded, acyclic ledger with valid counts and IDs"
            .into(),
        Position::NONE,
    )
}

fn integer(data: &Map, name: &str) -> Result<Option<i64>, EvalAltResult> {
    data.get(name)
        .filter(|v| !v.is_unit())
        .map(|v| v.as_int().map_err(|_| error()))
        .transpose()
}

fn string(data: &Map, name: &str) -> Result<Option<String>, EvalAltResult> {
    data.get(name)
        .filter(|v| !v.is_unit())
        .map(|v| {
            let s = v.clone().into_string().map_err(|_| error())?;
            if s.len() > 96 { Err(error()) } else { Ok(s) }
        })
        .transpose()
}

fn position(index: usize) -> Dynamic {
    Dynamic::from(i64::try_from(index).expect("bounded waterfall index"))
}

fn value(fields: impl IntoIterator<Item = (&'static str, Dynamic)>) -> Dynamic {
    Dynamic::from_map(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

fn array(it: impl IntoIterator<Item = Dynamic>) -> Dynamic {
    Dynamic::from_array(it.into_iter().collect())
}

fn counts(sum: Amounts) -> Dynamic {
    value([
        ("micros", sum.micros.into()),
        ("input", sum.input.into()),
        ("output", sum.output.into()),
        ("priced", sum.priced.into()),
    ])
}

#[allow(clippy::cast_precision_loss)] // The input bound keeps exact ledger integers below 2^53.
fn encode(p: Projection, mode: Mode) -> Dynamic {
    let upper = (p.max as f64 * 1.08).max(mode.minimum() as f64);
    let lower = (-p.min as f64 * 1.08).max(0.0);
    let target = (upper + lower) / 5.0;
    let base = 10_f64.powf(target.log10().floor());
    let step = [1.0, 2.0, 2.5, 5.0, 10.0]
        .into_iter()
        .map(|n| (n * base).ceil())
        .find(|n| *n >= target)
        .unwrap_or(10.0 * base);
    let lo = -(lower / step).ceil() * step;
    let hi = (upper / step).ceil() * step;
    let range = hi - lo;
    let x = |v: f64| ((v - lo) / range).clamp(0.0, 1.0);
    let mut ticks = Vec::new();
    let mut tick = lo;
    while tick <= hi && ticks.len() < 12 {
        ticks.push(value([("value", tick.into()), ("at", x(tick).into())]));
        tick += step;
    }
    let rows = p.rows.into_iter().map(|r| {
        let a = x(r.start as f64);
        let b = x(r.end as f64);
        let all = r.sum.input + r.sum.output;
        let split = match mode {
            Mode::Tokens if all > 0 => r.sum.output as f64 / all as f64,
            _ => 0.0,
        };
        value([
            ("index", position(r.index)),
            ("depth", position(r.depth)),
            ("folded", r.folded.into()),
            ("hidden", r.hidden.into()),
            ("children", r.children.into()),
            ("total", counts(r.sum)),
            ("left", a.min(b).into()),
            ("size", (b - a).abs().into()),
            ("split", split.into()),
            ("credit", (r.end < r.start).into()),
            ("from", r.start.into()),
            ("to", r.end.into()),
        ])
    });
    value([
        ("rows", array(rows)),
        ("total", counts(p.rollup)),
        ("ticks", Dynamic::from_array(ticks)),
        ("min", lo.into()),
        ("max", hi.into()),
        ("top", p.top.map_or(Dynamic::UNIT, position)),
    ])
}

fn projection(input: Array, mode: &str, folds: Array) -> Result<Dynamic, Box<EvalAltResult>> {
    let mode = match mode {
        "cost" => Mode::Cost,
        "tokens" => Mode::Tokens,
        _ => return Err(error().into()),
    };
    if input.len() > LIMIT || folds.len() > LIMIT {
        return Err(error().into());
    }
    let mut steps = Vec::with_capacity(input.len());
    for data in input {
        let m = data.try_cast::<Map>().ok_or_else(error)?;
        let id = string(&m, "id")?
            .filter(|id| !id.is_empty())
            .ok_or_else(error)?;
        let parent = string(&m, "parent")?.filter(|id| !id.is_empty());
        let agent = string(&m, "kind")?.as_deref() == Some("agent");
        let micros = integer(&m, "cost_micros")?;
        let input = integer(&m, "tokens_in")?.unwrap_or_default();
        let output = integer(&m, "tokens_out")?.unwrap_or_default();
        if micros.is_some_and(|v| !(-MAX_SPEND..=MAX_SPEND).contains(&v))
            || ![input, output]
                .into_iter()
                .all(|v| (0..=MAX_TOKENS).contains(&v))
        {
            return Err(error().into());
        }
        steps.push(Step {
            id,
            parent,
            agent,
            amounts: Amounts {
                micros: micros.unwrap_or_default(),
                input,
                output,
                priced: micros.is_some(),
            },
        });
    }
    let mut folded = BTreeSet::new();
    for id in folds {
        let id = id.into_string().map_err(|_| error())?;
        if id.len() > 96 || id.is_empty() {
            return Err(error().into());
        }
        folded.insert(id);
    }
    let p = Tree::new(&steps, &folded, mode)
        .and_then(|tree| tree.project())
        .ok_or_else(error)?;
    Ok(encode(p, mode))
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("cost_waterfall_projection", projection);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(id: &str, parent: Option<&str>, micros: i64, input: i64, output: i64) -> Step {
        Step {
            id: id.into(),
            parent: parent.map(str::to_owned),
            agent: id == "group",
            amounts: Amounts {
                micros,
                input,
                output,
                priced: true,
            },
        }
    }

    #[test]
    fn folded_parent_spans_exact_children_without_recounting_them() {
        let steps = [
            s("child", Some("group"), 20_000, 70, 30),
            s("group", None, 1_000, 20, 5),
            s("refund", None, -31_000, 0, 0),
            s("tail", None, 5_000, 0, 10),
        ];
        let collapsed = BTreeSet::from(["group".into()]);
        let p = Tree::new(&steps, &collapsed, Mode::Cost)
            .unwrap()
            .project()
            .unwrap();
        assert_eq!(
            p.rollup,
            Amounts {
                micros: -5_000,
                input: 90,
                output: 45,
                priced: true
            }
        );
        assert_eq!(p.top, Some(0));
        assert_eq!((p.min, p.max), (-10_000, 21_000));
        assert_eq!((p.rows[0].start, p.rows[0].end), (0, 21_000));
        assert!(p.rows[1].hidden);
        assert_eq!((p.rows[2].start, p.rows[2].end), (21_000, -10_000));
        let tokens = Tree::new(&steps, &collapsed, Mode::Tokens)
            .unwrap()
            .project()
            .unwrap();
        assert_eq!((tokens.rows[0].start, tokens.rows[0].end), (0, 125));
        assert_eq!((tokens.rows[2].start, tokens.rows[2].end), (125, 125));
    }

    #[test]
    fn cycles_missing_parents_and_duplicate_ids_are_not_partial_ledgers() {
        let folded = BTreeSet::new();
        let bad = [
            vec![
                s("a", None, 1, 0, 0),
                s("b", Some("c"), 1, 0, 0),
                s("c", Some("b"), 1, 0, 0),
            ],
            vec![s("a", Some("missing"), 1, 0, 0)],
            vec![s("a", None, 1, 0, 0), s("a", None, 1, 0, 0)],
        ];
        for steps in bad {
            assert!(
                Tree::new(&steps, &folded, Mode::Cost)
                    .and_then(|t| t.project())
                    .is_none()
            );
        }
    }

    #[test]
    fn rhai_bridge_keeps_signed_receipts_and_bounded_fractions() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(engine.eval::<bool>(r#"
            let input=[#{id:"credit",cost_micros:-61000},#{id:"charge",cost_micros:16000,tokens_out:135}];
            let m=cost_waterfall_projection(input,"cost",[]);
            let t=cost_waterfall_projection(input,"tokens",[]);
            m.total.micros == -45000 && m.min < -61000.0 && m.rows[0].credit
                && m.rows[0].left+m.rows[0].size<=1.00000001 && t.rows[1].split == 1.0
        "#).unwrap());
    }
}
