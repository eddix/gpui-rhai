//! A Table group header shows the group value as the rows carry it. The value
//! comes from a data column (`group_by`) and is often a case-sensitive
//! identifier (a cluster, a pod, a path), so the header must not change it.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;

use crate::support::{Mount, Step, last_error, settle, snapshot};

const GROUPS: [&str; 2] = [
    "default · 583444 · Singapore-Central · TikTok/default",
    "canary · 583445 · US-East · TikTok/canary",
];

fn mount(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
    let source = format!(
        r#"import "components/table" as table;
fn view(ctx) {{
    column([table::Table(#{{ key: "instances", label: "Instances", row_key: "id", height: 240,
        group_by: "cluster",
        rows: [#{{ id: "i0", pod: "dp-09aa6aec4c-kbzgl", cluster: "{a}" }},
               #{{ id: "i1", pod: "dp-09aa6aec4c-0lO1I", cluster: "{b}" }}],
        columns: [#{{ key: "pod", title: "Pod", width: #{{ kind: "fixed", value: 240 }} }},
                  #{{ key: "cluster", title: "Cluster", width: #{{ kind: "fixed", value: 360 }} }}] }})])
        .with_style(style().width(px(640)))
}}"#,
        a = GROUPS[0],
        b = GROUPS[1],
    );
    let (mut visual, view) = Mount::components(source).id("groups").mount(cx);
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

/// Every node of the rendered tree, realized virtual rows included.
fn walk<'a>(node: &'a UiNode, out: &mut Vec<&'a UiNode>) {
    out.push(node);
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for child in children {
                walk(child, out);
            }
        }
        UiNodeKind::VirtualCollection { spec } => {
            for child in spec.realized.values() {
                walk(child, out);
            }
        }
        _ => {}
    }
}

#[gpui::test]
fn a_group_header_shows_the_group_value_unchanged(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx);
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut nodes = Vec::new();
    walk(&root, &mut nodes);
    // The visible group label text (key `<header key>-label`).
    let shown = nodes
        .iter()
        .filter(|node| {
            node.key()
                .is_some_and(|key| key.as_str().ends_with("-label"))
        })
        .filter_map(|node| match node.kind() {
            UiNodeKind::Text { text } => Some(text.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    // What assistive technology reads for the same header row.
    let spoken = snapshot(&mut visual, &view)
        .nodes()
        .filter(|node| node.role == "rowheader")
        .map(|node| node.name.clone())
        .collect::<Vec<_>>();
    println!("shown {shown:?}");
    println!("spoken {spoken:?}");
    assert_eq!(
        shown, GROUPS,
        "the group header shows the value as the rows carry it"
    );
}
