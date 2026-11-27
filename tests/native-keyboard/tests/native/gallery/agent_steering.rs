use std::time::Duration;

use gpui::{ClipboardItem, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{AutomationCommand, PrimitiveValue, ScriptViewHandle, UiNode, UiNodeKind, UiValue};

use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, snapshot};

const INITIAL: &str = "Keep the dark palette while measuring.";
const MOD_ENTER: &str = if cfg!(target_os = "macos") {
    "cmd-enter"
} else {
    "ctrl-enter"
};

fn later(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(630));
    ready(visual, view);
}
fn replace(v: &mut VisualTestContext, view: &ScriptViewHandle, text: &str) {
    click(v, view, "text_field", "Steer the agent");
    v.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string(text.into())));
    v.simulate_keystrokes(&crate::support::primary_keys("secondary-a secondary-v"));
    ready(v, view);
}
fn draft(v: &mut VisualTestContext, view: &ScriptViewHandle, expected: &str) {
    assert_eq!(
        node(v, view, "text_field", "Steer the agent").value,
        Some(UiValue::String(expected.into()))
    );
}
fn distinct_node(v: &mut VisualTestContext, view: &ScriptViewHandle, title: &str) {
    assert_eq!(
        snapshot(v, view)
            .find_by_role_and_name("listitem", title)
            .count(),
        1
    );
}

#[gpui::test]
fn gallery_agent_steering_applied_note_uses_the_step_rail_and_its_own_icon_track(
    cx: &mut TestAppContext,
) {
    for (theme, inset) in [("modern-light", 26.), ("clean-dark", 24.)] {
        let (mut v, view) = gallery(cx, "agent_steering", theme);
        v.simulate_resize(size(px(1260.), px(1080.)));
        later(&mut v, &view);
        let step = bounds(&mut v, &view, "text", "Read the preview inputs");
        let message = "Preserve the original color names.";
        let bubble = bounds(&mut v, &view, "listitem", message);
        let words = bounds(&mut v, &view, "text", message);
        assert!(
            (step.x - bubble.x).abs() <= 0.5,
            "{theme}: bubble starts on the step title rail: {step:?} {bubble:?}"
        );
        assert!(
            (words.x - bubble.x - inset).abs() <= 0.5,
            "{theme}: the wrapped text starts after the padded 12px icon: {words:?} {bubble:?}"
        );
        let status = bounds(&mut v, &view, "text", "Picked up");
        assert!(
            status.x > words.x && status.x + status.width <= bubble.x + bubble.width,
            "{theme}: picked status stays in the note: {status:?} {bubble:?}"
        );
        distinct_node(&mut v, &view, message);

        for (action, value) in [
            ("gallery.set_density", "compact"),
            ("gallery.set_scale", "150"),
        ] {
            v.update(|window, cx| {
                view.automate(
                    AutomationCommand::Action {
                        id: action.into(),
                        payload: Some(UiValue::String(value.into())),
                    },
                    window,
                    cx,
                )
            })
            .unwrap();
        }
        later(&mut v, &view);
        let step = bounds(&mut v, &view, "text", "Read the preview inputs");
        let bubble = bounds(&mut v, &view, "listitem", message);
        let words = bounds(&mut v, &view, "text", message);
        assert!(
            (step.x - bubble.x).abs() <= 0.5,
            "{theme}: same rail in compact density at 150%: {step:?} {bubble:?}"
        );
        assert!(
            words.x > bubble.x + 24.,
            "{theme}: icon track scales: {words:?}"
        );
    }
}

#[gpui::test]
fn gallery_agent_steering_keyboard_uses_two_routes_and_refusal_keeps_draft(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "agent_steering", "modern-light");
    v.simulate_resize(size(px(1260.), px(1100.)));
    later(&mut v, &view);
    click(&mut v, &view, "button", "Refuse next proposal");
    replace(&mut v, &view, "Keep labels aligned");
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Waiting for the demo Host…");
    draft(&mut v, &view, "Keep labels aligned");
    later(&mut v, &view);
    node(&mut v, &view, "status", "The Host refused the proposal.");
    draft(&mut v, &view, "Keep labels aligned");
    v.simulate_keystrokes("enter");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Queued Keep labels aligned");
    let queue = node(&mut v, &view, "list", "Next step");
    assert_eq!(
        node(&mut v, &view, "listitem", "Keep labels aligned").parent,
        Some(queue.id)
    );
    draft(&mut v, &view, "");
    replace(&mut v, &view, "Compare the two previews");
    v.simulate_keystrokes(MOD_ENTER);
    later(&mut v, &view);
    node(&mut v, &view, "status", "Applied draft/2 to build");
    node(&mut v, &view, "text", "Interrupted");
    assert_ne!(
        node(&mut v, &view, "listitem", "Compare the two previews").parent,
        Some(queue.id)
    );
    draft(&mut v, &view, "");
    click(&mut v, &view, "button", "Switch draft owner");
    replace(&mut v, &view, "The Host keeps this text");
    v.simulate_keystrokes("enter");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Queued The Host keeps this text");
    draft(&mut v, &view, "The Host keeps this text");
}

#[gpui::test]
fn gallery_agent_steering_accepted_receipt_does_not_clear_newer_edits(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_steering", "clean-dark");
    v.simulate_resize(size(px(1220.), px(1080.)));
    later(&mut v, &view);
    replace(&mut v, &view, "Earlier note");
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    replace(&mut v, &view, "New draft");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Queued Earlier note");
    draft(&mut v, &view, "New draft");
    v.simulate_keystrokes("shift-enter");
    ready(&mut v, &view);
    draft(&mut v, &view, "New draft\n");
    v.simulate_keystrokes("enter");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Queued New draft");
    draft(&mut v, &view, "");
    replace(&mut v, &view, "Receipt must expire");
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Reset steering");
    later(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("listitem", "Receipt must expire")
            .next()
            .is_none()
    );
    draft(&mut v, &view, "");
}

fn shared_note(node: &UiNode) -> Option<(UiValue, UiValue)> {
    if node
        .key()
        .is_some_and(|key| key.as_str() == "note:note/保留")
    {
        let attrs = node.attributes();
        return Some((
            attrs["shared_layout_group"].clone(),
            attrs["shared_layout_id"].clone(),
        ));
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(shared_note)
        }
        UiNodeKind::Custom { primitive } => primitive.props.iter().find_map(|(_, p)| match p {
            PrimitiveValue::Node(child) => shared_note(child),
            PrimitiveValue::Nodes(children) => children.iter().find_map(shared_note),
            _ => None,
        }),
        _ => None,
    }
}

#[gpui::test]
fn gallery_agent_steering_accepted_note_moves_to_step_with_same_motion_identity(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "agent_steering", "clean-light");
    v.simulate_resize(size(px(1240.), px(1090.)));
    later(&mut v, &view);
    let before_rect = bounds(&mut v, &view, "listitem", INITIAL);
    let before = v.update(|_, cx| shared_note(&view.root(cx).unwrap().unwrap()).unwrap());
    click(&mut v, &view, "button", "Begin next steering step");
    later(&mut v, &view);
    let after = v.update(|_, cx| shared_note(&view.root(cx).unwrap().unwrap()).unwrap());
    assert_eq!(before, after);
    assert!(bounds(&mut v, &view, "listitem", INITIAL).y < before_rect.y);
    assert_eq!(
        node(&mut v, &view, "listitem", "Review the preview")
            .current
            .as_deref(),
        Some("step")
    );
    distinct_node(&mut v, &view, INITIAL);
    click(&mut v, &view, "button", "Reset steering");
    click(&mut v, &view, "button", &format!("Apply now: {INITIAL}"));
    later(&mut v, &view);
    node(&mut v, &view, "status", "Applied note/保留 to build");
    distinct_node(&mut v, &view, INITIAL);
    node(&mut v, &view, "text", "Interrupted");
}
