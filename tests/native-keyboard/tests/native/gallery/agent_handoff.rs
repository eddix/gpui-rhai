//! Drive the relay through the native controls, not direct script callbacks.

use std::time::Duration;

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::{
    CanvasCommand, CanvasPathSegment, CanvasScene, GeometryBounds, PrimitiveValue,
    ScriptViewHandle, UiNode, UiNodeKind,
};

use super::conversation::{gallery, node, ready};
use crate::support::{bounds, center, click_center, snapshot};

fn press(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) {
    let target = bounds(visual, view, "button", name);
    press_at(visual, view, target);
}

fn press_at(visual: &mut VisualTestContext, view: &ScriptViewHandle, target: GeometryBounds) {
    visual.simulate_mouse_move(center(target), None, Modifiers::none());
    ready(visual, view);
    click_center(visual, target);
    ready(visual, view);
}

fn later(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(680));
    ready(visual, view);
}

const PACKET: &str = "Inspect handoff: Release draft";

fn packet(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    summary: &str,
) -> GeometryBounds {
    snapshot(visual, view)
        .find_by_role_and_name("button", PACKET)
        .find(|n| n.description == summary)
        .unwrap_or_else(|| panic!("missing packet {summary}"))
        .geometry
        .unwrap()
        .visual
}

fn open_latest(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    let target = packet(visual, view, "2 delivered · 1 omitted");
    press_at(visual, view, target);
}

#[gpui::test]
fn gallery_agent_handoff_restoration_targets_the_exact_packet_and_material(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "agent_handoff", "modern-light");
    visual.simulate_resize(size(px(1320.), px(1110.)));
    ready(&mut visual, &view);
    let main = packet(&mut visual, &view, "2 delivered · 1 omitted");
    let old = packet(&mut visual, &view, "1 delivered · 1 omitted");
    assert!(main.y < old.y);
    open_latest(&mut visual, &view);
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name.contains("recipient did not get the rollback"))
    );
    press(&mut visual, &view, "Decline next proposal");
    press(&mut visual, &view, "Restore Rollback checklist");
    node(&mut visual, &view, "status", "Waiting for the demo Host…");
    packet(&mut visual, &view, "2 delivered · 1 omitted");
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined the proposal.",
    );
    press(&mut visual, &view, "Restore Rollback checklist");
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Restored context in pass/current",
    );
    packet(&mut visual, &view, "3 delivered · 0 omitted");
    packet(&mut visual, &view, "1 delivered · 1 omitted");
    press(&mut visual, &view, "Close details");
    press(&mut visual, &view, "Deliver new transfer");
    let latest = packet(&mut visual, &view, "2 delivered · 0 omitted");
    let previous = packet(&mut visual, &view, "3 delivered · 0 omitted");
    assert!(latest.y < previous.y);
}

#[gpui::test]
fn gallery_agent_handoff_keyboard_assignment_is_not_confused_by_equal_titles(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "agent_handoff", "clean-dark");
    visual.simulate_resize(size(px(1310.), px(1080.)));
    ready(&mut visual, &view);
    open_latest(&mut visual, &view);
    press(&mut visual, &view, "Choose destination");
    node(&mut visual, &view, "button", "Assign to Planner");
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("button", "Assign to Planner")
            .next()
            .is_none()
    );
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "button", "Assign to Builder");
    visual.simulate_keystrokes("right enter");
    ready(&mut visual, &view);
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Assigned draft/current to build",
    );
    let sig = snapshot(&mut visual, &view);
    assert!(
        sig.find_by_role_and_name("button", "Release draft")
            .any(|n| n.description == "Done")
    );
    assert!(
        sig.find_by_role_and_name("button", "Release draft")
            .any(|n| n.description == "Working")
    );
}

fn drag_to(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str, agent: &str) {
    let origin = center(bounds(visual, view, "button", name));
    let dest = center(bounds(visual, view, "group", agent));
    visual.simulate_mouse_down(origin, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_move(
        point(origin.x + px(12.), origin.y - px(4.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    visual.simulate_mouse_move(dest, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(dest, MouseButton::Left, Modifiers::none());
    ready(visual, view);
}

#[gpui::test]
fn gallery_agent_handoff_native_drag_respects_unavailable_destinations(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_handoff", "clean-light");
    visual.simulate_resize(size(px(685.), px(1370.)));
    ready(&mut visual, &view);
    drag_to(
        &mut visual,
        &view,
        "Move Release draft",
        "Drop task on Reviewer",
    );
    later(&mut visual, &view);
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .all(|n| !n.name.starts_with("Assigned draft/current")),
        "the disabled lane must not receive a move"
    );
    drag_to(
        &mut visual,
        &view,
        "Move Release draft",
        "Drop task on Builder",
    );
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Assigned draft/current to build",
    );
}

fn scene(root: &UiNode) -> Option<&CanvasScene> {
    match root.kind() {
        UiNodeKind::Canvas { scene } if root.key().is_some_and(|key| key.as_str() == "wires") => {
            Some(scene)
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(scene)
        }
        UiNodeKind::Custom { primitive } => primitive.props.iter().find_map(|(_, p)| match p {
            PrimitiveValue::Node(n) => scene(n),
            PrimitiveValue::Nodes(nodes) => nodes.iter().find_map(scene),
            _ => None,
        }),
        _ => None,
    }
}

fn check_curve(visual: &mut VisualTestContext, view: &ScriptViewHandle, compact: bool) {
    let stage = bounds(visual, view, "group", "Relay map");
    let agent = bounds(visual, view, "button", "Inspect Planner");
    let pair = bounds(visual, view, "group", "Planner → Builder");
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let scene = scene(&root).expect("measured connector scene");
    let CanvasCommand::Path { segments, .. } = scene
        .commands()
        .iter()
        .find(|c| c.key() == "into-0")
        .unwrap()
    else {
        panic!("missing curve")
    };
    let [
        CanvasPathSegment::Move { x, y },
        CanvasPathSegment::Cubic { x: x2, y: y2, .. },
    ] = segments.as_slice()
    else {
        panic!("missing curve endpoints")
    };
    assert!((stage.x + x - agent.x - agent.width * 0.5).abs() < 2.0);
    assert!((stage.y + y - agent.y - agent.height).abs() < 2.0);
    let entry = pair.x + pair.width * 0.5 - if compact { 0.0 } else { 9.0 };
    assert!((stage.x + x2 - entry).abs() < 2.0);
    assert!((stage.y + y2 - pair.y).abs() < 2.0);
}

#[gpui::test]
fn gallery_agent_handoff_wires_follow_wide_and_compact_card_positions(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_handoff", "modern-dark");
    visual.simulate_resize(size(px(1340.), px(1070.)));
    ready(&mut visual, &view);
    check_curve(&mut visual, &view, false);
    visual.simulate_resize(size(px(665.), px(1320.)));
    ready(&mut visual, &view);
    check_curve(&mut visual, &view, true);
    press(&mut visual, &view, "Inspect Builder");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Render preview")
    );
}
