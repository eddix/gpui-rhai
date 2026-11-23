use std::time::Duration;

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{
    CanvasCommand, CanvasPathSegment, CanvasScene, PrimitiveValue, ScriptViewHandle, UiNode,
    UiNodeKind,
};

use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, snapshot};

fn later(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(650));
    ready(visual, view);
}

fn absent(v: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    assert!(
        snapshot(v, view)
            .find_by_role_and_name(role, name)
            .next()
            .is_none(),
        "unexpected {role} {name}"
    );
}

#[gpui::test]
fn gallery_agent_fleet_controlled_lead_is_not_replaced_by_a_rejected_proposal(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "agent_fleet", "modern-light");
    v.simulate_resize(size(px(1300.), px(1100.)));
    later(&mut v, &view);
    node(&mut v, &view, "tree", "Sub-agents of Delivery");
    node(&mut v, &view, "status", "Run total: 0.35 USD");
    click(&mut v, &view, "button", "Refuse next disclosure");
    click(&mut v, &view, "button", "Research");
    node(&mut v, &view, "status", "The Host kept the old branch.");
    node(&mut v, &view, "tree", "Sub-agents of Delivery");
    v.simulate_keystrokes("enter");
    later(&mut v, &view);
    node(&mut v, &view, "tree", "Sub-agents of Research");
    absent(&mut v, &view, "button", "Palette review");
    assert!(
        node(&mut v, &view, "button", "Copy review")
            .description
            .contains("-<0.01 USD")
    );
    node(&mut v, &view, "status", "Run total: 0.35 USD");
    v.simulate_keystrokes("up enter");
    later(&mut v, &view);
    node(&mut v, &view, "tree", "Sub-agents of Delivery");
}

#[gpui::test]
fn gallery_agent_fleet_shared_review_preserves_evidence_after_refusal(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_fleet", "clean-dark");
    v.simulate_resize(size(px(1300.), px(1100.)));
    later(&mut v, &view);
    click(&mut v, &view, "button", "Refuse next resolution");
    click(&mut v, &view, "button", "Palette review");
    node(&mut v, &view, "dialog", "Tasks for Palette review");
    node(&mut v, &view, "status", "Inspecting visual/配色");
    node(&mut v, &view, "listitem", "Capture light palette");
    click(&mut v, &view, "button", "Approve Palette review");
    node(&mut v, &view, "status", "Waiting for the demo Host…");
    assert!(node(&mut v, &view, "button", "Approve Palette review").disabled);
    later(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "The demo Host refused the resolution.",
    );
    node(
        &mut v,
        &view,
        "text",
        "The light palette has been checked. Keep the new preview?",
    );
    click(&mut v, &view, "button", "Approve Palette review");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Accepted approve for visual/配色");
    absent(&mut v, &view, "button", "Approve Palette review");
    node(&mut v, &view, "listitem", "Capture light palette");
    v.simulate_keystrokes("escape");
    later(&mut v, &view);
    absent(&mut v, &view, "dialog", "Tasks for Palette review");
    click(&mut v, &view, "button", "Artifact scan");
    node(
        &mut v,
        &view,
        "text",
        "The archive is missing its manifest.",
    );
    click(&mut v, &view, "button", "Retry Artifact scan");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Accepted retry for artifact");
}

fn canvas(n: &UiNode) -> Option<&CanvasScene> {
    match n.kind() {
        UiNodeKind::Canvas { scene } if n.key().is_some_and(|k| k.as_str() == "fleet-wires") => {
            Some(scene)
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(canvas)
        }
        UiNodeKind::Custom { primitive } => primitive.props.iter().find_map(|(_, p)| match p {
            PrimitiveValue::Node(n) => canvas(n),
            PrimitiveValue::Nodes(nodes) => nodes.iter().find_map(canvas),
            _ => None,
        }),
        _ => None,
    }
}

fn check_geometry(v: &mut VisualTestContext, view: &ScriptViewHandle, wide: bool) {
    let stage = bounds(v, view, "group", "Fleet roster");
    let top = bounds(v, view, "button", "Delivery");
    let first = bounds(v, view, "button", "Build runner");
    let remained = bounds(v, view, "button", "Research");
    let root = v.update(|_, cx| view.root(cx).unwrap().unwrap());
    let wires = canvas(&root).expect("native anchors have emitted fleet wires");
    let CanvasCommand::Path { segments, .. } = wires
        .commands()
        .iter()
        .find(|c| c.key() == "edge-3")
        .unwrap()
    else {
        panic!("expected fleet path")
    };
    if wide {
        assert!(first.x >= top.x + top.width);
        let [
            CanvasPathSegment::Move { x, y },
            CanvasPathSegment::Cubic { x: tx, y: ty, .. },
        ] = segments.as_slice()
        else {
            panic!("expected wide path: {segments:?}")
        };
        assert!((stage.x + x - top.x - top.width).abs() < 2.0);
        assert!((stage.y + y - top.y - top.height * 0.5).abs() < 2.0);
        assert!((stage.x + tx - first.x).abs() < 2.0);
        assert!((stage.y + ty - first.y - first.height * 0.5).abs() < 2.0);
    } else {
        assert!(top.y + top.height <= first.y && first.y + first.height <= remained.y);
        assert!(
            first.x > top.x,
            "compact children must indent from their lead"
        );
        let [
            CanvasPathSegment::Move { x, .. },
            CanvasPathSegment::Line { .. },
            CanvasPathSegment::Line { .. },
            CanvasPathSegment::Line { x: tx, y: ty },
        ] = segments.as_slice()
        else {
            panic!("expected compact connector: {segments:?}")
        };
        assert!((stage.x + x - top.x).abs() < 2.0);
        assert!((stage.x + tx - first.x).abs() < 2.0);
        assert!((stage.y + ty - first.y - first.height * 0.5).abs() < 2.0);
    }
}

#[gpui::test]
fn gallery_agent_fleet_narrow_branch_reuses_lead_identity_and_projects_the_wires(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "agent_fleet", "clean-light");
    v.simulate_resize(size(px(1300.), px(1170.)));
    later(&mut v, &view);
    check_geometry(&mut v, &view, true);
    let lead = node(&mut v, &view, "button", "Delivery").id;
    click(&mut v, &view, "button", "Delivery");
    v.simulate_keystrokes("enter");
    later(&mut v, &view);
    v.simulate_resize(size(px(645.), px(1190.)));
    later(&mut v, &view);
    check_geometry(&mut v, &view, false);
    assert_eq!(node(&mut v, &view, "button", "Delivery").id, lead);
    v.simulate_keystrokes("down enter");
    later(&mut v, &view);
    node(&mut v, &view, "tree", "Sub-agents of Research");
}

#[gpui::test]
fn gallery_agent_fleet_highlight_reveals_its_root_in_local_mode(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_fleet", "modern-dark");
    v.simulate_resize(size(px(1270.), px(1150.)));
    later(&mut v, &view);
    click(&mut v, &view, "button", "Toggle copy highlight");
    node(&mut v, &view, "tree", "Sub-agents of Research");
    click(&mut v, &view, "button", "Copy review");
    node(&mut v, &view, "dialog", "Tasks for Copy review");
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Delivery");
    node(&mut v, &view, "tree", "Sub-agents of Delivery");
    absent(&mut v, &view, "dialog", "Tasks for Copy review");
}
