//! Components inside a virtual item are named by the item (#110): their path is
//! `VirtualCollection[key]/Item[<item key>]/...`, so a keyless component in a
//! newly realized row cannot take the path of a retained row's component.

use gpui::{
    Modifiers, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px,
};
use gpui_rhai::*;

use crate::support::{Mount, Step, last_error, settle, snapshot};

/// A keyless formal component (stateless, like an adornment Badge), once per row.
const MARK: &str = r#"
define_component(#{
    metadata: #{ id: "components/mark", "export": "Mark", version: "0.0.1",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{}, tokens: [], environment: [] },
    schema: #{
        props: #{ text: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{} },
        events: #{}, slots: #{}, parts: ["root"],
    },
    render: Fn("render_Mark")
});
fn Mark(props) { render_component("components/mark", props) }
fn render_Mark(ctx, props) { text(props.text) }
"#;

const MAIN: &str = r#"
import "components/mark" as mark;
fn row_view(ctx, payload) {
    row([text(payload.item.label), mark::Mark(#{ text: "mark" })])
        .accessibility_role("group").test_id(payload.key)
        .with_style(style().height(px(40)))
}
fn view(ctx) {
    let items = [];
    for index in 0..60 { items.push(#{ key: `r${index}`, label: `Row ${index}` }); }
    column([virtual_collection(#{ key: "rows", label: "Rows", data: items,
        estimated_height: 40.0, height: 200.0 }, Fn("row_view"))])
        .with_style(style().width(px(400)))
}
"#;

fn realized(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
    snapshot(visual, view)
        .nodes()
        .filter_map(|node| node.test_id.clone())
        .filter(|id| id.starts_with('r'))
        .collect()
}

/// Component instance paths of the rendered Marks.
fn mark_paths(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
    fn walk(node: &UiNode, out: &mut Vec<String>) {
        if let Some(path) = node.component_root() {
            out.push(path.to_string());
        }
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
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut paths = Vec::new();
    walk(&root, &mut paths);
    paths.retain(|path| path.contains("/Mark["));
    paths
}

#[gpui::test]
fn keyless_components_in_new_rows_do_not_take_retained_paths(cx: &mut TestAppContext) {
    // A theme of its own, no token base, no assets; the motion preference from the
    // environment.
    let (mut visual, view) = Mount::new(MAIN)
        .module("components/mark", MARK)
        .theme(r#"fn theme(){#{family:"Paths",name:"Dark",mode:"dark",tokens:#{}}}"#)
        .no_token_base()
        .id("paths")
        .mount(cx);
    settle(&mut visual, Step::Frame, 1);

    let before = realized(&mut visual, &view);
    let paths = mark_paths(&mut visual, &view);
    println!("before {before:?}\n{paths:#?}");
    assert!(
        paths
            .iter()
            .any(|path| path.contains("/VirtualCollection[rows]/Item[r0]/Mark[")),
        "a row's component is named by its row: {paths:?}"
    );

    let position = point(px(200.0), px(100.0));
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(0.0), px(-120.0))),
        ..Default::default()
    });
    visual.run_until_parked();
    settle(&mut visual, Step::Frame, 1);
    let after = realized(&mut visual, &view);
    println!("after {after:?}");
    assert!(
        after.iter().any(|id| before.contains(id)) && after.iter().any(|id| !before.contains(id)),
        "the scroll keeps some rows and realizes new ones: {before:?} -> {after:?}"
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let paths = mark_paths(&mut visual, &view);
    let mut unique = paths.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), paths.len(), "{paths:#?}");
    for id in &after {
        let segment = format!("/Item[{id}]/Mark[");
        assert!(
            paths.iter().filter(|path| path.contains(&segment)).count() == 1,
            "{id} has its own Mark: {paths:#?}"
        );
    }
}
