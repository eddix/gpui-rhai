//! Bounded fault tree with a conservative, Host-confirmed proof.
use std::collections::BTreeMap;

use rhai::{Array, Dynamic, Engine, EvalAltResult, ImmutableString, Map, Position};
use unicode_segmentation::UnicodeSegmentation as _;

const CAP: usize = 96;
const MIN_CARD: f64 = 150.0;
const GAP: f64 = 12.0;
const CARD_HEIGHT: f64 = 84.0;
const LEVEL: f64 = 44.0;

type Error = Box<EvalAltResult>;

fn error(message: &str) -> EvalAltResult {
    EvalAltResult::ErrorRuntime(message.to_owned().into(), Position::NONE)
}

fn text(node: &Map, key: &str) -> Option<ImmutableString> {
    node.get(key)?.clone().try_cast()
}

fn valid_id(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= 96 && !id.chars().any(char::is_control)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Status {
    Open,
    Checking,
    RuledOut,
    Confirmed,
}

struct Cause {
    id: ImmutableString,
    parent: Option<ImmutableString>,
    title_width: f64,
    all: bool,
    status: Status,
}

impl Cause {
    fn parse(value: Dynamic) -> Result<Self, Error> {
        let m = value
            .try_cast::<Map>()
            .ok_or_else(|| error("fault tree requires cause objects"))?;
        let id = text(&m, "id")
            .filter(|id| valid_id(id))
            .ok_or_else(|| error("invalid cause ID"))?;
        let parent = m
            .get("parent")
            .filter(|v| !v.is_unit())
            .map(|_| {
                text(&m, "parent")
                    .filter(|id| valid_id(id))
                    .ok_or_else(|| error("invalid parent ID"))
            })
            .transpose()?;
        let title = text(&m, "title")
            .filter(|s| !s.trim().is_empty() && s.len() <= 512)
            .ok_or_else(|| error("cause title is missing or too long"))?;
        let all = match text(&m, "gate").as_deref() {
            None | Some("or") => false,
            Some("and") => true,
            _ => return Err(error("unknown cause gate").into()),
        };
        let status = match text(&m, "status").as_deref() {
            None | Some("open") => Status::Open,
            Some("checking") => Status::Checking,
            Some("ruled-out") => Status::RuledOut,
            Some("confirmed") => Status::Confirmed,
            _ => return Err(error("unknown cause status").into()),
        };
        // This estimates preferred, never minimum, fit. Real native text is
        // clamped to one line and remains fully available in the evidence view.
        let chrome = if m.get("icon").is_some_and(|v| !v.is_unit()) {
            45.
        } else {
            25.
        };
        let title_width = chrome
            + title
                .graphemes(true)
                .map(|g| if g.is_ascii() { 7.2 } else { 12.0 })
                .sum::<f64>();
        Ok(Self {
            id,
            parent,
            title_width,
            all,
            status,
        })
    }
}

fn map(entries: impl IntoIterator<Item = (&'static str, Dynamic)>) -> Dynamic {
    Dynamic::from_map(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
}
fn ix(i: usize) -> Dynamic {
    Dynamic::from_int(i64::try_from(i).expect("bounded cause index"))
}
fn opt(i: Option<usize>) -> Dynamic {
    i.map_or(Dynamic::UNIT, ix)
}

fn count_f64(count: usize) -> f64 {
    f64::from(u32::try_from(count).expect("fault trees contain up to 96 nodes"))
}

fn children_span(kids: &[usize], span: &[f64]) -> f64 {
    kids.iter().map(|i| span[*i]).sum::<f64>() + kids.iter().skip(1).map(|_| GAP).sum::<f64>()
}

fn widths(order: &[usize], root: usize, kids: &[Vec<usize>], root_w: f64, card_w: f64) -> Vec<f64> {
    let mut span = vec![0.; order.len()];
    for &i in order.iter().rev() {
        let own = if i == root { root_w } else { card_w };
        span[i] = own.max(children_span(&kids[i], &span));
    }
    span
}

struct Tree {
    root: usize,
    kids: Vec<Vec<usize>>,
    parents: Vec<Option<usize>>,
    set_index: Vec<usize>,
    depth: Vec<usize>,
    order: Vec<usize>,
    levels: Vec<Vec<usize>>,
}

fn index(causes: &[Cause]) -> Result<Tree, Error> {
    let count = causes.len();
    let mut ids = BTreeMap::new();
    for (i, c) in causes.iter().enumerate() {
        if ids.insert(c.id.as_str(), i).is_some() {
            return Err(error("duplicate cause ID").into());
        }
    }
    let mut maybe_root = None;
    let mut kids = vec![vec![]; count];
    let mut parents = vec![None; count];
    let mut set_index = vec![0; count];
    for (i, c) in causes.iter().enumerate() {
        if let Some(id) = &c.parent {
            let p = *ids
                .get(id.as_str())
                .ok_or_else(|| error("cause has unknown parent"))?;
            parents[i] = Some(p);
            set_index[i] = kids[p].len();
            kids[p].push(i);
        } else if maybe_root.replace(i).is_some() {
            return Err(error("fault tree requires one failure root").into());
        }
    }
    let root = maybe_root.ok_or_else(|| error("fault tree requires a failure root"))?;
    let mut depth = vec![0; count];
    let mut order = vec![];
    let mut levels: Vec<Vec<usize>> = vec![];
    let mut stack = vec![(root, 0)];
    while let Some((i, d)) = stack.pop() {
        order.push(i);
        depth[i] = d;
        if levels.len() == d {
            levels.push(vec![]);
        }
        levels[d].push(i);
        stack.extend(kids[i].iter().rev().map(|c| (*c, d + 1)));
    }
    if order.len() != count {
        return Err(error("fault tree contains a disconnected cycle").into());
    }
    Ok(Tree {
        root,
        kids,
        parents,
        set_index,
        depth,
        order,
        levels,
    })
}

fn prove(causes: &[Cause], tree: &Tree) -> (Vec<bool>, Vec<bool>) {
    let mut proved = vec![false; causes.len()];
    for &i in tree.order.iter().rev() {
        let children = &tree.kids[i];
        let group = if children.is_empty() {
            i != tree.root
        } else if causes[i].all {
            children.iter().all(|k| proved[*k])
        } else {
            children.iter().any(|k| proved[*k])
        };
        proved[i] = group && (i == tree.root || causes[i].status == Status::Confirmed);
    }
    let mut path = vec![false; causes.len()];
    path[tree.root] = proved[tree.root];
    for &i in tree.order.iter().skip(1) {
        path[i] = tree.parents[i].is_some_and(|p| path[p]) && proved[i];
    }
    (proved, path)
}

struct Layout {
    narrow: bool,
    root_width: f64,
    card_width: f64,
    centers: Vec<f64>,
}

fn layout(causes: &[Cause], tree: &Tree, width: f64) -> Layout {
    let desired_root = causes[tree.root].title_width.clamp(204., 300.);
    let root_w = desired_root.min(width.max(204.));
    let leaf_count = count_f64(tree.kids.iter().filter(|k| k.is_empty()).count());
    let desired_card = causes
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != tree.root)
        .map(|(_, c)| c.title_width)
        .fold(184.0, f64::max)
        .min(232.);
    let card = ((width - (leaf_count - 1.) * GAP) / leaf_count)
        .floor()
        .clamp(MIN_CARD, desired_card);
    let span = widths(&tree.order, tree.root, &tree.kids, root_w, card);
    let narrow = span[tree.root] > width;
    let mut centers = vec![0.; causes.len()];
    let mut pending = vec![(tree.root, ((width - span[tree.root]) / 2.).max(0.))];
    while let Some((i, left)) = pending.pop() {
        centers[i] = left + span[i] / 2.;
        let mut child_x = left + (span[i] - children_span(&tree.kids[i], &span)) / 2.;
        for &k in &tree.kids[i] {
            pending.push((k, child_x));
            child_x += span[k] + GAP;
        }
    }
    Layout {
        narrow,
        root_width: root_w,
        card_width: card,
        centers,
    }
}

fn projection(source: Array, width: f64) -> Result<Dynamic, Error> {
    if source.is_empty()
        || source.len() > CAP
        || !width.is_finite()
        || !(1.0..=20_000.).contains(&width)
    {
        return Err(error("fault tree count or width outside supported bounds").into());
    }
    let causes = source
        .into_iter()
        .map(Cause::parse)
        .collect::<Result<Vec<_>, _>>()?;
    let tree = index(&causes)?;
    let (proved, path) = prove(&causes, &tree);
    let layout = layout(&causes, &tree, width);
    let Tree {
        root,
        kids,
        parents,
        set_index,
        depth,
        order,
        levels,
    } = tree;
    let checking = order
        .iter()
        .copied()
        .skip(1)
        .find(|i| causes[*i].status == Status::Checking);
    let first = order
        .iter()
        .copied()
        .find(|i| *i != root && path[*i] && kids[*i].is_empty());
    let total_ruled = order
        .iter()
        .skip(1)
        .filter(|i| causes[**i].status == Status::RuledOut)
        .count();
    let mut peer = vec![(None, None); causes.len()];
    for row in &levels {
        for (i, k) in row.iter().enumerate() {
            peer[*k] = (i.checked_sub(1).map(|p| row[p]), row.get(i + 1).copied());
        }
    }
    let rows = (0..causes.len())
        .map(|i| {
            let card = if i == root {
                layout.root_width
            } else {
                layout.card_width
            };
            let count = parents[i].map_or(1, |p| kids[p].len());
            map([
                ("depth", ix(depth[i])),
                ("parent", opt(parents[i])),
                ("child", opt(kids[i].first().copied())),
                ("before", opt(peer[i].0)),
                ("after", opt(peer[i].1)),
                ("count", ix(count)),
                ("position", ix(set_index[i] + 1)),
                ("sub_count", ix(kids[i].len())),
                ("all", causes[i].all.into()),
                ("proved", proved[i].into()),
                ("path", path[i].into()),
                (
                    "actionable",
                    (i != root && path[i] && kids[i].is_empty()).into(),
                ),
                ("cx", layout.centers[i].into()),
                ("y", (count_f64(depth[i]) * (CARD_HEIGHT + LEVEL)).into()),
                ("w", card.into()),
            ])
        })
        .collect();
    Ok(map([
        ("root", ix(root)),
        ("rows", Dynamic::from_array(rows)),
        (
            "order",
            Dynamic::from_array(order.into_iter().map(ix).collect()),
        ),
        ("narrow", layout.narrow.into()),
        (
            "height",
            (count_f64(levels.len()) * (CARD_HEIGHT + LEVEL) - LEVEL).into(),
        ),
        ("card_height", CARD_HEIGHT.into()),
        ("level", LEVEL.into()),
        ("depth", ix(levels.len() - 1)),
        ("solved", proved[root].into()),
        ("ruled", ix(total_ruled)),
        ("checking", opt(checking)),
        ("cause", opt(first)),
    ]))
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_fn("root_cause_projection", projection);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_cause_requires_every_confirmed_and_input_and_omits_ruled_out_paths() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(
            engine
                .eval::<bool>(
                    r#"
           let rows=[#{id:"second",parent:"group",title:"Second",status:"checking"},
            #{id:"failure",title:"Failure"},
            #{id:"group",parent:"failure",title:"All checks",gate:"and",status:"confirmed"},
            #{id:"first",parent:"group",title:"First",status:"confirmed"},
            #{id:"excluded",parent:"failure",title:"Excluded",status:"ruled-out"}];
           let a=root_cause_projection(rows,820.0);
           rows[0].status="confirmed";
           let b=root_cause_projection(rows,820.0);
           let c=root_cause_projection(rows,300.0);
           !a.solved && !a.rows[3].actionable && a.checking==0 && b.solved &&
             b.rows[0].actionable && b.rows[3].actionable && !b.rows[4].path &&
             b.order==[1,2,0,3,4] && b.rows[2].cx==(b.rows[0].cx+b.rows[3].cx)/2.0 &&
             !b.narrow && c.narrow && b.rows[3].position==2 && b.rows[3].count==2
        "#
                )
                .unwrap()
        );
    }

    #[test]
    fn root_cause_top_and_is_not_a_shortcut_and_bad_graphs_are_rejected() {
        let mut engine = Engine::new();
        register(&mut engine);
        assert!(
            engine
                .eval::<bool>(
                    r#"
           let r=[#{id:"r",title:"Failure",gate:"and"},
             #{id:"a",parent:"r",title:"A",status:"confirmed"},
             #{id:"b",parent:"r",title:"B",status:"open"}];
           let x=root_cause_projection(r,500.0);
           r[0].gate="or";
           let y=root_cause_projection(r,500.0);
           !x.solved && y.solved && y.rows[1].path && !y.rows[2].path
        "#
                )
                .unwrap()
        );
        for s in [
            r#"[#{id:"r",title:"Failure"},#{id:"a",parent:"a",title:"Cycle"}]"#,
            r#"[#{id:"r",title:"Failure"},#{id:"r",parent:"r",title:"Duplicate"}]"#,
            r#"[#{id:"r",title:"Failure"},#{id:"a",parent:"missing",title:"Orphan"}]"#,
        ] {
            assert!(
                engine
                    .eval::<Dynamic>(&format!("root_cause_projection({s},500.0)"))
                    .is_err()
            );
        }
    }
}
