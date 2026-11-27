//! The activity timeline uses real task bounds for input and its handoff scene.

use std::time::Duration;

use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{
    AutomationCommand, CanvasCommand, CanvasPathSegment, CanvasScene, PrimitiveValue,
    ScriptViewHandle, UiNode, UiNodeKind, UiValue,
};

use super::conversation::{gallery, node, ready};
use crate::support::{bounds, center, click_center, snapshot};

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) {
    let rect = bounds(visual, view, "button", name);
    visual.simulate_mouse_move(center(rect), None, Modifiers::none());
    ready(visual, view);
    let rect = bounds(visual, view, "button", name);
    click_center(visual, rect);
    ready(visual, view);
}

fn later(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(610));
    ready(visual, view);
}

fn connectors(node: &UiNode) -> Option<&CanvasScene> {
    match node.kind() {
        UiNodeKind::Canvas { scene }
            if node.key().is_some_and(|key| key.as_str() == "handoffs") =>
        {
            Some(scene)
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(connectors)
        }
        UiNodeKind::Custom { primitive } => {
            primitive.props.iter().find_map(|(_, prop)| match prop {
                PrimitiveValue::Node(node) => connectors(node),
                PrimitiveValue::Nodes(nodes) => nodes.iter().find_map(connectors),
                _ => None,
            })
        }
        _ => None,
    }
}

fn anchors(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> Option<((f64, f64), (f64, f64))> {
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let scene = connectors(&root)?;
    let CanvasCommand::Path { segments, .. } =
        scene.commands().iter().find(|c| c.key() == "handoff-1")?
    else {
        return None;
    };
    let [
        CanvasPathSegment::Move { x, y },
        CanvasPathSegment::Cubic {
            x: to_x, y: to_y, ..
        },
    ] = segments.as_slice()
    else {
        panic!("expected actual curve endpoints")
    };
    Some(((*x, *y), (*to_x, *to_y)))
}

fn check_endpoints(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    let stage = bounds(visual, view, "group", "Activity timeline");
    let source = bounds(visual, view, "button", "Outline rollout");
    let destination = bounds(visual, view, "button", "Build preview");
    let (from, to) =
        anchors(visual, view).expect("handoff remains between the two presented tasks");
    assert!((stage.x + from.0 - source.x - source.width).abs() < 2.0);
    assert!((stage.y + from.1 - source.y - source.height * 0.5).abs() < 2.0);
    assert!((stage.x + to.0 - destination.x).abs() < 2.0);
    assert!((stage.y + to.1 - destination.y - destination.height * 0.5).abs() < 2.0);
}

#[gpui::test]
fn gallery_agent_lanes_reading_scale_preserves_track_hit_separation(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_lanes", "clean-light");
    visual.simulate_resize(size(px(1270.), px(1060.)));
    later(&mut visual, &view);
    let original = bounds(&mut visual, &view, "button", "Build preview");
    for scale in ["125", "150"] {
        visual.update(|window, cx| {
            view.automate(
                AutomationCommand::Action {
                    id: "gallery.set_scale".into(),
                    payload: Some(UiValue::String(scale.into())),
                },
                window,
                cx,
            )
            .unwrap();
        });
        later(&mut visual, &view);
        let first = bounds(&mut visual, &view, "button", "Build preview");
        let second = bounds(&mut visual, &view, "button", "Read changelog");
        assert!(first.height > original.height);
        assert!(
            second.x < first.x + first.width && second.y >= first.y + first.height,
            "overlapping ranges need separate native hit rows: {first:?}, {second:?}"
        );
        check_endpoints(&mut visual, &view);
        click(&mut visual, &view, "Read changelog");
        node(&mut visual, &view, "heading", "Read changelog");
        click(&mut visual, &view, "Close details");
    }
}

#[gpui::test]
fn gallery_agent_lanes_real_curve_endpoints_follow_resize_and_row_reorder(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_lanes", "modern-light");
    visual.simulate_resize(size(px(1300.), px(1030.)));
    ready(&mut visual, &view);
    check_endpoints(&mut visual, &view);
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "7.5 min ago"),
        "the axis must not label a 7:30 grid line as 7 minutes"
    );
    let first = bounds(&mut visual, &view, "button", "Build preview");
    let second = bounds(&mut visual, &view, "button", "Read changelog");
    assert!(second.x < first.x + first.width && second.y >= first.y + first.height);
    click(&mut visual, &view, "Read changelog");
    node(&mut visual, &view, "heading", "Read changelog");
    click(&mut visual, &view, "Close details");

    visual.simulate_resize(size(px(650.), px(1120.)));
    ready(&mut visual, &view);
    check_endpoints(&mut visual, &view);
    let (old_y, new_y) = anchors(&mut visual, &view)
        .map(|(from, to)| (from.1, to.1))
        .unwrap();
    assert!(old_y < new_y);
    click(&mut visual, &view, "Reverse lane order");
    check_endpoints(&mut visual, &view);
    let (from, to) = anchors(&mut visual, &view).unwrap();
    assert!(
        from.1 > to.1,
        "the connector must not stay on the old lanes"
    );
}

#[gpui::test]
fn gallery_agent_lanes_window_clips_time_and_preserves_minimum_hit_target(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_lanes", "clean-dark");
    visual.simulate_resize(size(px(1280.), px(990.)));
    ready(&mut visual, &view);
    let absent = |visual: &mut VisualTestContext, name| {
        snapshot(visual, &view)
            .find_by_role_and_name("button", name)
            .next()
            .is_none()
    };
    assert!(absent(&mut visual, "Archived prep") && absent(&mut visual, "Future work"));
    let axis = bounds(&mut visual, &view, "group", "Time axis");
    let at_now = bounds(&mut visual, &view, "button", "Recorded checkpoint");
    assert!(at_now.width >= 20.0 && (at_now.x + at_now.width - axis.x - axis.width).abs() < 2.0);
    click(&mut visual, &view, "Recorded checkpoint");
    node(&mut visual, &view, "heading", "Recorded checkpoint");
    click(&mut visual, &view, "Advance demo clock");
    assert!(absent(&mut visual, "Outline rollout") && anchors(&mut visual, &view).is_none());
    let clipped = bounds(&mut visual, &view, "button", "Build preview");
    let axis = bounds(&mut visual, &view, "group", "Time axis");
    assert!((clipped.x - axis.x).abs() < 2.0);
    node(&mut visual, &view, "button", "Future work");
}

#[gpui::test]
fn gallery_agent_lanes_keyboard_inspection_and_host_decisions_keep_identity(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "agent_lanes", "clean-light");
    visual.simulate_resize(size(px(1270.), px(1070.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "Outline rollout");
    visual.simulate_keystrokes("down enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "heading", "Build preview");
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "heading", "Build preview");
    click(&mut visual, &view, "Open task");
    node(
        &mut visual,
        &view,
        "status",
        "Host received open: Build preview",
    );
    click(&mut visual, &view, "Confirm release copy");
    click(&mut visual, &view, "Decline next decision");
    click(&mut visual, &view, "Approve");
    node(&mut visual, &view, "status", "Waiting for the demo Host…");
    assert!(
        node(&mut visual, &view, "button", "Confirm release copy")
            .description
            .starts_with("Needs review")
    );
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined the decision.",
    );
    click(&mut visual, &view, "Approve");
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Accepted approval: Confirm release copy",
    );
    assert!(
        node(&mut visual, &view, "button", "Confirm release copy")
            .description
            .starts_with("Running")
    );
    click(&mut visual, &view, "Reset timeline demo");
    click(&mut visual, &view, "Confirm release copy");
    click(&mut visual, &view, "Reject");
    later(&mut visual, &view);
    assert!(
        node(&mut visual, &view, "button", "Confirm release copy")
            .description
            .starts_with("Failed")
    );
}
