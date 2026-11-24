//! Native interaction with the Gallery's local approval and policy demos.

use std::time::Duration;

use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ScriptViewHandle, UiValue};

use super::conversation::{gallery, node, ready};
use crate::support::{bounds, center, click_center, snapshot};

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let geometry = bounds(visual, view, role, name);
    // Native document focus can shift the scrolled parent after a branch
    // replacement. Act on the geometry presented after moving the pointer.
    visual.simulate_mouse_move(center(geometry), None, Modifiers::none());
    ready(visual, view);
    let target = bounds(visual, view, role, name);
    click_center(visual, target);
    ready(visual, view);
}

#[gpui::test]
fn gallery_approval_gates_preserves_edited_evidence_on_host_rejection(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "approval_gates", "modern-light");
    visual.simulate_resize(size(px(1160.), px(900.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Full context");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name.contains("Compare the two draft summaries"))
    );
    click(&mut visual, &view, "button", "Less context");
    node(&mut visual, &view, "button", "Full context");
    click(&mut visual, &view, "button", "Edit");
    node(&mut visual, &view, "text_field", "Edit request");
    let cancel = bounds(&mut visual, &view, "button", "Cancel");
    click(&mut visual, &view, "button", "Cancel");
    let after = snapshot(&mut visual, &view);
    assert!(
        after
            .find_by_role_and_name("button", "Edit")
            .next()
            .is_some(),
        "Cancelled at {cancel:?}, available buttons: {:?}",
        after
            .nodes()
            .filter(|n| n.role == "button")
            .map(|n| n.name.as_str())
            .collect::<Vec<_>>()
    );
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "text_field", "Edit request");
    let revised = "{\"audience\":\"team\",\"length\":\"one sentence\"}";
    click(&mut visual, &view, "text_field", "Edit request");
    visual.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string(revised.into())));
    visual.simulate_keystrokes(&crate::support::primary_keys("secondary-a secondary-v"));
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Edit request").value,
        Some(UiValue::String(revised.into()))
    );
    click(&mut visual, &view, "button", "Decline next proposal");
    click(&mut visual, &view, "button", "Approve with changes");
    visual.executor().advance_clock(Duration::from_millis(660));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined this proposal.",
    );
    assert_eq!(
        node(&mut visual, &view, "text_field", "Edit request").value,
        Some(UiValue::String(revised.into()))
    );
    click(&mut visual, &view, "button", "Approve with changes");
    visual.executor().advance_clock(Duration::from_millis(660));
    ready(&mut visual, &view);
    node(&mut visual, &view, "heading", "Update the release draft");
    assert_eq!(
        node(&mut visual, &view, "button", "Prepare a release summary").description,
        "Approved with changes"
    );
    click(&mut visual, &view, "button", "Prepare a release summary");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == revised)
    );
}

#[gpui::test]
fn gallery_approval_gates_file_feedback_does_not_approve_a_file(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "approval_gates", "clean-dark");
    click(&mut visual, &view, "button", "Next gate");
    node(&mut visual, &view, "heading", "Changes");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "release-draft.txt")
    );
    click(&mut visual, &view, "button", "Add sample gate");
    node(&mut visual, &view, "heading", "Update the release draft");
    click(&mut visual, &view, "button", "Request changes");
    assert!(node(&mut visual, &view, "button", "Request changes").disabled);
    click(&mut visual, &view, "text_field", "Requested changes");
    visual.simulate_input("Keep Thursday.");
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Request changes");
    visual.executor().advance_clock(Duration::from_millis(680));
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "button", "Update the release draft").description,
        "Changes requested"
    );
    click(&mut visual, &view, "button", "Reject");
    visual.executor().advance_clock(Duration::from_millis(680));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Stopped");
}

#[gpui::test]
fn gallery_autonomy_dial_respects_accepted_policy_and_mandatory_pins(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "autonomy_dial", "clean-light");
    let yes = Some(UiValue::Bool(true));
    assert_eq!(node(&mut visual, &view, "radio", "If risky").checked, yes);
    click(&mut visual, &view, "button", "Decline next proposal");
    click(&mut visual, &view, "radio", "Full auto");
    assert_eq!(node(&mut visual, &view, "radio", "If risky").checked, yes);
    visual.executor().advance_clock(Duration::from_millis(680));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined this policy change.",
    );
    assert_eq!(
        node(&mut visual, &view, "listitem", "Update notes").description,
        "Asks you first"
    );
    click(&mut visual, &view, "radio", "Full auto");
    visual.executor().advance_clock(Duration::from_millis(680));
    ready(&mut visual, &view);
    assert_eq!(node(&mut visual, &view, "radio", "Full auto").checked, yes);
    assert_eq!(
        node(&mut visual, &view, "listitem", "Update notes").description,
        "Runs on its own"
    );
    assert_eq!(
        node(&mut visual, &view, "listitem", "Replace archive").description,
        "Asks you first"
    );
    assert!(
        node(
            &mut visual,
            &view,
            "button",
            "Confirmation required for Replace archive"
        )
        .disabled
    );
    let before = bounds(&mut visual, &view, "listitem", "Update notes").y;
    click(
        &mut visual,
        &view,
        "button",
        "Always ask before Update notes",
    );
    visual.executor().advance_clock(Duration::from_millis(680));
    ready(&mut visual, &view);
    assert!(bounds(&mut visual, &view, "listitem", "Update notes").y > before);
    assert_eq!(
        node(&mut visual, &view, "button", "Unlock Update notes").pressed,
        Some(true)
    );
    // The keyed button remains the same control when the accepted policy moves the row.
    visual.simulate_keystrokes("space");
    visual.executor().advance_clock(Duration::from_millis(680));
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "listitem", "Update notes").description,
        "Runs on its own"
    );
}
