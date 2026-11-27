//! Native plan editing, reordering, execution, and recovery in the Gallery.

use std::time::Duration;

use gpui::{
    ClipboardItem, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px, size,
};
use gpui_rhai::{CanvasScene, PrimitiveValue, ScriptViewHandle, UiNode, UiNodeKind, UiValue};

use super::conversation::{gallery, node, ready};
use crate::support::{bounds, center, click_center, snapshot};

fn press(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, label: &str) {
    let at = bounds(visual, view, role, label);
    visual.simulate_mouse_move(center(at), None, Modifiers::none());
    ready(visual, view);
    let at = bounds(visual, view, role, label);
    click_center(visual, at);
    ready(visual, view);
}
fn button(visual: &mut VisualTestContext, view: &ScriptViewHandle, label: &str) {
    press(visual, view, "button", label);
}
fn later(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(630));
    ready(visual, view);
}
fn replace_text(visual: &mut VisualTestContext, view: &ScriptViewHandle, content: &str) {
    visual.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string(content.into())));
    visual.simulate_keystrokes(&crate::support::primary_keys("secondary-a secondary-v"));
    ready(visual, view);
}
const FIRST: &str = "Collect evidence";
const SECOND: &str = "Build preview";
const THIRD: &str = "Confirm rollback";
fn comes_first(visual: &mut VisualTestContext, view: &ScriptViewHandle, a: &str, b: &str) -> bool {
    let a = bounds(visual, view, "listitem", a);
    let b = bounds(visual, view, "listitem", b);
    a.y < b.y
}

#[gpui::test]
fn gallery_agent_plan_native_reorder_uses_business_id_and_waits_for_host(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_plan", "modern-light");
    visual.simulate_resize(size(px(1260.), px(1000.)));
    ready(&mut visual, &view);
    button(&mut visual, &view, "Decline next request");
    button(&mut visual, &view, "Reorder Collect evidence");
    visual.simulate_keystrokes("alt-down");
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Waiting for the demo Host…");
    assert!(comes_first(&mut visual, &view, FIRST, SECOND));
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined the request.",
    );
    assert!(comes_first(&mut visual, &view, FIRST, SECOND));
    button(&mut visual, &view, "Reorder Collect evidence");
    visual.simulate_keystrokes("alt-down");
    ready(&mut visual, &view);
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Accepted reorder: collect/evidence",
    );
    assert!(comes_first(&mut visual, &view, SECOND, FIRST));

    let start = center(bounds(
        &mut visual,
        &view,
        "button",
        "Reorder Build preview",
    ));
    let mut end = bounds(&mut visual, &view, "listitem", THIRD);
    end.y += end.height * 0.5;
    end.height *= 0.5;
    let end = center(end);
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_move(
        point(start.x, start.y + px(10.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Waiting for the demo Host…");
    assert!(comes_first(&mut visual, &view, SECOND, THIRD));
    later(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted reorder: preview");
    assert!(comes_first(&mut visual, &view, THIRD, SECOND));
    button(&mut visual, &view, "Run plan");
    later(&mut visual, &view);
    node(&mut visual, &view, "status", "Started collect/evidence");
}

#[gpui::test]
fn gallery_agent_plan_edits_stay_open_on_refusal_and_add_remove_wait_for_host(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "agent_plan", "clean-dark");
    visual.simulate_resize(size(px(1280.), px(1140.)));
    ready(&mut visual, &view);
    button(&mut visual, &view, "Edit Collect evidence");
    node(&mut visual, &view, "text_field", "Step title");
    replace_text(&mut visual, &view, "Collect revised evidence");
    assert!(node(&mut visual, &view, "button", "Run plan").disabled);
    button(&mut visual, &view, "Decline next request");
    button(&mut visual, &view, "Save step");
    node(&mut visual, &view, "listitem", FIRST);
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined the request.",
    );
    assert_eq!(
        node(&mut visual, &view, "text_field", "Step title").value,
        Some(UiValue::String("Collect revised evidence".into()))
    );
    button(&mut visual, &view, "Save step");
    later(&mut visual, &view);
    node(&mut visual, &view, "status", "Saved collect/evidence");
    node(&mut visual, &view, "listitem", "Collect revised evidence");
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("text_field", "Step title")
            .next()
            .is_none()
    );

    button(&mut visual, &view, "Add step");
    replace_text(&mut visual, &view, "Review local artifact");
    button(&mut visual, &view, "Save step");
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("listitem", "Review local artifact")
            .next()
            .is_none()
    );
    later(&mut visual, &view);
    node(&mut visual, &view, "listitem", "Review local artifact");
    button(&mut visual, &view, "Remove Review local artifact");
    later(&mut visual, &view);
    node(&mut visual, &view, "status", "Removed added/1");
    button(&mut visual, &view, "Add step");
    replace_text(&mut visual, &view, "Discarded title");
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("text_field", "Step title")
            .next()
            .is_none()
    );
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Step title").value,
        Some(UiValue::String(String::new()))
    );
}

fn find_rail(root: &UiNode) -> Option<&CanvasScene> {
    match root.kind() {
        UiNodeKind::Canvas { scene } if root.key().is_some_and(|k| k.as_str() == "rail") => {
            Some(scene)
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(find_rail)
        }
        UiNodeKind::Custom { primitive } => primitive.props.iter().find_map(|(_, p)| match p {
            PrimitiveValue::Node(node) => find_rail(node),
            PrimitiveValue::Nodes(nodes) => nodes.iter().find_map(find_rail),
            _ => None,
        }),
        _ => None,
    }
}
fn has_segment(visual: &mut VisualTestContext, view: &ScriptViewHandle, key: &str) -> bool {
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    find_rail(&root).is_some_and(|scene| scene.commands().iter().any(|c| c.key() == key))
}

#[gpui::test]
fn gallery_agent_plan_execution_live_clock_failure_retry_and_cancel_are_real(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "agent_plan", "clean-light");
    visual.simulate_resize(size(px(680.), px(1190.)));
    ready(&mut visual, &view);
    button(&mut visual, &view, "Run plan");
    node(&mut visual, &view, "status", "Draft");
    later(&mut visual, &view);
    node(&mut visual, &view, "status", "Running");
    assert!(has_segment(&mut visual, &view, "remaining-0"));
    button(&mut visual, &view, "Advance demo clock");
    assert_eq!(
        node(&mut visual, &view, "listitem", FIRST).description,
        "Running · 6 s"
    );
    button(&mut visual, &view, "Finish active step");
    assert_eq!(
        node(&mut visual, &view, "listitem", FIRST).description,
        "Done · 6 s"
    );
    assert!(has_segment(&mut visual, &view, "complete-0"));
    button(&mut visual, &view, "Advance demo clock");
    button(&mut visual, &view, "Fail active step");
    assert_eq!(
        node(&mut visual, &view, "listitem", SECOND).description,
        "Failed · 6 s"
    );
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "The local fixture rejected this preview.")
    );
    button(&mut visual, &view, "Retry from here");
    later(&mut visual, &view);
    node(&mut visual, &view, "status", "Retrying preview");
    assert_eq!(
        node(&mut visual, &view, "listitem", FIRST).description,
        "Done · 6 s"
    );
    assert_eq!(
        node(&mut visual, &view, "listitem", SECOND).description,
        "Running · 0 s"
    );
    button(&mut visual, &view, "Cancel plan");
    later(&mut visual, &view);
    node(&mut visual, &view, "status", "Plan cancelled.");
    assert_eq!(
        node(&mut visual, &view, "listitem", FIRST).description,
        "Done · 6 s"
    );
    assert_eq!(
        node(&mut visual, &view, "listitem", SECOND).description,
        "Pending"
    );
}
