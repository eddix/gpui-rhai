use std::time::Duration;

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{
    CanvasCommand, CanvasPathSegment, CanvasScene, PrimitiveValue, ScriptViewHandle, UiNode,
    UiNodeKind,
};

use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, snapshot};

fn later(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(560));
    ready(visual, view);
}

fn missing(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    assert!(
        snapshot(visual, view)
            .find_by_role_and_name(role, name)
            .next()
            .is_none(),
        "expected {role} {name} to be absent"
    );
}

#[gpui::test]
fn gallery_agent_trail_clock_and_disclosure_keep_all_substeps_reachable(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_trail", "modern-light");
    v.simulate_resize(size(px(1260.), px(1020.)));
    later(&mut v, &view);
    let active = node(&mut v, &view, "listitem", "Prepare preview");
    assert!(active.description.contains("20 s"));
    let done = node(&mut v, &view, "listitem", "Collect context");
    assert!(done.description.contains("6 s"));
    assert_eq!(
        node(&mut v, &view, "listitem", "Capture rendered output")
            .current
            .as_deref(),
        Some("step")
    );
    missing(&mut v, &view, "listitem", "Read inputs");
    missing(&mut v, &view, "listitem", "Clean temporary render files");
    click(&mut v, &view, "button", "+2 earlier");
    node(&mut v, &view, "listitem", "Read inputs");
    node(&mut v, &view, "listitem", "Clean temporary render files");
    // The Show fewer control retains focus across the change in its label.
    v.simulate_keystrokes("enter");
    later(&mut v, &view);
    missing(&mut v, &view, "listitem", "Read inputs");
    node(&mut v, &view, "button", "+3 more");
    click(&mut v, &view, "button", "+3 more");
    node(&mut v, &view, "listitem", "Clean temporary render files");
    v.simulate_keystrokes("enter");
    later(&mut v, &view);
    missing(&mut v, &view, "listitem", "Clean temporary render files");
    click(&mut v, &view, "button", "Advance preview clock");
    assert!(
        node(&mut v, &view, "listitem", "Prepare preview")
            .description
            .contains("27 s")
    );
    assert_eq!(
        node(&mut v, &view, "listitem", "Collect context").description,
        done.description
    );
    click(&mut v, &view, "button", "Finish active substep");
    assert_eq!(
        node(&mut v, &view, "listitem", "Capture rendered output").current,
        None
    );
    assert_eq!(
        node(&mut v, &view, "listitem", "Compare baselines")
            .current
            .as_deref(),
        Some("step")
    );
    click(&mut v, &view, "button", "Switch disclosure owner");
    missing(&mut v, &view, "listitem", "Capture rendered output");
    click(&mut v, &view, "button", "Show substeps of Prepare preview");
    node(&mut v, &view, "status", "Accepted disclosure for build");
    node(&mut v, &view, "listitem", "Capture rendered output");
    v.simulate_keystrokes("enter");
    later(&mut v, &view);
    missing(&mut v, &view, "listitem", "Capture rendered output");
}

#[gpui::test]
fn gallery_agent_trail_waits_for_host_and_preserves_evidence_on_refusal(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_trail", "clean-dark");
    v.simulate_resize(size(px(1260.), px(1070.)));
    later(&mut v, &view);
    click(&mut v, &view, "button", "Refuse next decision");
    click(&mut v, &view, "button", "Review Draft handoff");
    node(&mut v, &view, "region", "Approval for Draft handoff");
    click(&mut v, &view, "button", "Approve");
    node(&mut v, &view, "status", "Waiting for the demo Host…");
    assert!(node(&mut v, &view, "button", "Approve").disabled);
    later(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "The demo Host refused the decision.",
    );
    node(&mut v, &view, "region", "Approval for Draft handoff");
    assert!(!node(&mut v, &view, "button", "Approve").disabled);
    click(&mut v, &view, "button", "Approve");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Accepted approve for review/确认");
    missing(&mut v, &view, "region", "Approval for Draft handoff");
    assert_eq!(
        node(&mut v, &view, "listitem", "Record the decision")
            .current
            .as_deref(),
        Some("step")
    );
    click(&mut v, &view, "button", "Reset trail");
    click(&mut v, &view, "button", "Review Draft handoff");
    click(&mut v, &view, "button", "Decline");
    click(&mut v, &view, "button", "Reset trail");
    later(&mut v, &view);
    missing(&mut v, &view, "status", "Accepted decline for review/确认");
    node(&mut v, &view, "button", "Review Draft handoff");
}

fn rail(node: &UiNode) -> Option<&CanvasScene> {
    match node.kind() {
        UiNodeKind::Canvas { scene } if node.key().is_some_and(|key| key.as_str() == "rail") => {
            Some(scene)
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(rail)
        }
        UiNodeKind::Custom { primitive } => {
            primitive.props.iter().find_map(|(_, prop)| match prop {
                PrimitiveValue::Node(node) => rail(node),
                PrimitiveValue::Nodes(nodes) => nodes.iter().find_map(rail),
                _ => None,
            })
        }
        _ => None,
    }
}

fn assert_branch(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> f64 {
    let stage = bounds(visual, view, "group", "Execution trail");
    let node = bounds(visual, view, "listitem", "Capture rendered output");
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let scene = rail(&root).expect("measured standing rail");
    let CanvasCommand::Path { segments, .. } = scene
        .commands()
        .iter()
        .find(|c| c.key() == "edge-1")
        .unwrap()
    else {
        panic!("expected the running step's branch");
    };
    assert!(
        segments
            .iter()
            .any(|s| matches!(s, CanvasPathSegment::Cubic { .. })),
        "the branch must leave the parent column"
    );
    assert!(
        segments.iter().any(|s| match s {
            CanvasPathSegment::Cubic { y, .. } | CanvasPathSegment::Line { y, .. } =>
                (stage.y + y - node.y - node.height / 2.0).abs() < 2.0,
            _ => false,
        }),
        "the branch must touch the rendered running substep"
    );
    stage.width
}

#[gpui::test]
fn gallery_agent_trail_branch_follows_native_rows_after_resize(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_trail", "clean-light");
    v.simulate_resize(size(px(1250.), px(1020.)));
    later(&mut v, &view);
    let wide = assert_branch(&mut v, &view);
    v.simulate_resize(size(px(650.), px(1110.)));
    later(&mut v, &view);
    assert!(assert_branch(&mut v, &view) < wide);
    let fold = bounds(&mut v, &view, "button", "Hide substeps of Prepare preview");
    let details = bounds(&mut v, &view, "text", "Checking output");
    assert!(
        fold.y < details.y,
        "narrow details should sit below the disclosure control, not displace it: {fold:?}, {details:?}"
    );
}
