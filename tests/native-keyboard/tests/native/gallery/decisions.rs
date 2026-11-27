//! The Gallery's quick and advanced decision workflows in real native input.

use std::time::Duration;

use gpui::{TestAppContext, px, size};
use gpui_rhai::UiValue;

use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, snapshot};

#[gpui::test]
fn gallery_decision_card_number_keys_are_plain_only_and_scoped(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "decision_card", "modern-light");
    click(&mut visual, &view, "button", "Later");
    node(&mut visual, &view, "button", "Show decisions");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "option")
    );
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "option", "Review first");
    visual.simulate_keystrokes("cmd-2 ctrl-2 cmd-enter");
    visual.executor().advance_clock(Duration::from_millis(600));
    ready(&mut visual, &view);
    assert!(!node(&mut visual, &view, "option", "Accept summary").disabled);
    visual.simulate_keystrokes("2");
    ready(&mut visual, &view);
    visual.executor().advance_clock(Duration::from_millis(600));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "heading",
        "Which layout should the overview use?",
    );
    node(&mut visual, &view, "status", "Recorded: Accept summary");
    click(&mut visual, &view, "option", "Split view");
    visual.executor().advance_clock(Duration::from_millis(600));
    ready(&mut visual, &view);
    click(&mut visual, &view, "option", "Approve");
    visual.executor().advance_clock(Duration::from_millis(600));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Nothing else waits");
    assert!(bounds(&mut visual, &view, "region", "Decision card").height < 1.);
}

#[gpui::test]
fn gallery_decision_card_rejection_and_paging_keep_only_the_same_revision_draft(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "decision_card", "clean-dark");
    click(&mut visual, &view, "text_field", "Your answer");
    visual.simulate_input("Wait until Thursday.");
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Next decision");
    assert_eq!(
        node(&mut visual, &view, "text_field", "Your answer").value,
        Some(UiValue::String(String::new()))
    );
    // The pager should still own focus, not a row in the freshly shown card.
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "heading", "Record the final note?");
    click(&mut visual, &view, "button", "Next decision");
    assert_eq!(
        node(&mut visual, &view, "text_field", "Your answer").value,
        Some(UiValue::String("Wait until Thursday.".into()))
    );
    click(&mut visual, &view, "button", "Decline next answer");
    click(&mut visual, &view, "text_field", "Your answer");
    visual.simulate_keystrokes("enter");
    visual.executor().advance_clock(Duration::from_millis(640));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host rejected this answer.",
    );
    assert_eq!(
        node(&mut visual, &view, "text_field", "Your answer").value,
        Some(UiValue::String("Wait until Thursday.".into()))
    );
    click(&mut visual, &view, "button", "Revise first decision");
    node(
        &mut visual,
        &view,
        "heading",
        "How should we review the revised rollout?",
    );
    assert_eq!(
        node(&mut visual, &view, "text_field", "Your answer").value,
        Some(UiValue::String(String::new()))
    );
}

#[gpui::test]
fn gallery_decision_card_pick_layout_reflows_and_approval_returns_specific_receipt(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "decision_card", "clean-light");
    click(&mut visual, &view, "button", "Next decision");
    let a = bounds(&mut visual, &view, "option", "Split view");
    let b = bounds(&mut visual, &view, "option", "Stacked view");
    assert!(b.x > a.x + 20. && (b.y - a.y).abs() < 3., "{a:?}, {b:?}");
    visual.simulate_resize(size(px(620.), px(780.)));
    ready(&mut visual, &view);
    let a = bounds(&mut visual, &view, "option", "Split view");
    let b = bounds(&mut visual, &view, "option", "Stacked view");
    assert!(b.y > a.y + 20. && (b.x - a.x).abs() < 1., "{a:?}, {b:?}");
    click(&mut visual, &view, "button", "Next decision");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Team only")
    );
    click(&mut visual, &view, "option", "Approve");
    visual.executor().advance_clock(Duration::from_millis(640));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Recorded: Approved");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Record the final note?")
    );
}

#[gpui::test]
fn gallery_decision_inbox_count_and_disclosure_follow_accepted_receipts(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "decision_inbox", "modern-dark");
    node(&mut visual, &view, "status", "3 need your input");
    node(
        &mut visual,
        &view,
        "button",
        "Which layout should the overview use?",
    );
    click(&mut visual, &view, "button", "Decline next answer");
    click(&mut visual, &view, "button", "Review first");
    assert!(node(&mut visual, &view, "button", "Processing…").disabled);
    node(&mut visual, &view, "status", "3 need your input");
    visual.executor().advance_clock(Duration::from_millis(600));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host rejected this answer.",
    );
    node(
        &mut visual,
        &view,
        "button",
        "Which layout should the overview use?",
    );
    click(&mut visual, &view, "button", "Review first");
    visual.executor().advance_clock(Duration::from_millis(600));
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "2 need your input");
    node(&mut visual, &view, "status", "Review first");
    node(&mut visual, &view, "button", "Split view");
    visual.executor().advance_clock(Duration::from_secs(2));
    ready(&mut visual, &view);
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "How should we review the rollout?")
    );
}
