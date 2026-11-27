//! Native Gallery checks for the controlled review queue.

use std::time::Duration;

use gpui::{ClipboardItem, Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ScriptViewHandle, UiValue};

use super::conversation::{gallery, node, ready};
use crate::support::{Mount, bounds, center, click_center, snapshot};

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let hit = bounds(visual, view, role, name);
    visual.simulate_mouse_move(center(hit), None, Modifiers::none());
    ready(visual, view);
    let hit = bounds(visual, view, role, name);
    click_center(visual, hit);
    ready(visual, view);
}

fn later(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(650));
    ready(visual, view);
}

fn paste(visual: &mut VisualTestContext, view: &ScriptViewHandle, value: &str) {
    visual.update(|_, cx| cx.write_to_clipboard(ClipboardItem::new_string(value.into())));
    visual.simulate_keystrokes(&crate::support::primary_keys("secondary-a secondary-v"));
    ready(visual, view);
}

fn shown(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) -> bool {
    snapshot(visual, view)
        .find_by_role_and_name(role, name)
        .next()
        .is_some()
}

#[gpui::test]
fn gallery_review_center_prioritizes_filters_and_file_evidence(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "review_center", "modern-light");
    visual.simulate_resize(size(px(1300.), px(980.)));
    ready(&mut visual, &view);
    node(&mut visual, &view, "heading", "Prepare the rollout");
    let queue = bounds(&mut visual, &view, "region", "Pending review");
    let stage = bounds(&mut visual, &view, "region", "Request evidence");
    let approve = bounds(&mut visual, &view, "button", "Approve");
    assert!((stage.x - queue.x - queue.width).abs() < 2.);
    assert!(
        (queue.y + queue.height - approve.y - approve.height).abs() < 24.,
        "the action strip belongs at the bottom of the same card: {queue:?}, {approve:?}"
    );
    assert_eq!(
        node(&mut visual, &view, "button", "rollout.toml").pressed,
        Some(true)
    );
    click(&mut visual, &view, "button", "release-notes.md");
    assert_eq!(
        node(&mut visual, &view, "button", "release-notes.md").pressed,
        Some(true)
    );
    click(
        &mut visual,
        &view,
        "document",
        "Before compared with Proposed",
    );
    visual.simulate_keystrokes(&crate::support::primary_keys("alt-secondary-c"));
    ready(&mut visual, &view);
    let patch = visual.update(|_, app| app.read_from_clipboard().and_then(|item| item.text()));
    assert!(
        patch
            .as_deref()
            .is_some_and(|p| p.contains("+Pilot with the canary group.")),
        "the actual native viewer must copy the second file's patch: {patch:?}"
    );
    click(&mut visual, &view, "button", "Unsure");
    assert!(!shown(
        &mut visual,
        &view,
        "button",
        "Review Reply to Jordan"
    ));
    node(&mut visual, &view, "button", "Review Plan the backup");
    click(&mut visual, &view, "button", "High risk");
    assert!(!shown(
        &mut visual,
        &view,
        "button",
        "Review Plan the backup"
    ));
    node(&mut visual, &view, "button", "Review Prepare the rollout");
    click(&mut visual, &view, "button", "All");
    click(&mut visual, &view, "button", "Review Prepare the rollout");
    visual.simulate_keystrokes("down");
    ready(&mut visual, &view);
    node(&mut visual, &view, "heading", "Reply to Jordan");
    click(&mut visual, &view, "button", "Skip");
    node(&mut visual, &view, "heading", "Plan the backup");
    node(&mut visual, &view, "button", "Review Reply to Jordan");
    node(&mut visual, &view, "status", "Skipped without resolving");
}

#[gpui::test]
fn gallery_review_center_decline_keeps_edited_fields_and_undo_restores_original(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "review_center", "clean-dark");
    visual.simulate_resize(size(px(1320.), px(960.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Decline next decision");
    click(&mut visual, &view, "button", "Review Reply to Jordan");
    click(&mut visual, &view, "button", "Edit");
    let message = "Thanks for your patience.\nThe provider will confirm the date.";
    visual.simulate_resize(size(px(620.), px(1260.)));
    ready(&mut visual, &view);
    paste(&mut visual, &view, message);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Proposed reply").value,
        Some(UiValue::String(message.into()))
    );
    visual.simulate_resize(size(px(1320.), px(960.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "text_field", "Recipient");
    paste(&mut visual, &view, "Jordan Lee");
    click(&mut visual, &view, "button", "Approve with edits");
    node(&mut visual, &view, "status", "Waiting for the demo Host…");
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined this proposal.",
    );
    assert_eq!(
        node(&mut visual, &view, "text_field", "Recipient").value,
        Some(UiValue::String("Jordan Lee".into()))
    );
    assert_eq!(
        node(&mut visual, &view, "text_field", "Proposed reply").value,
        Some(UiValue::String(message.into()))
    );
    click(&mut visual, &view, "button", "Approve with edits");
    later(&mut visual, &view);
    node(&mut visual, &view, "status", "Approved: Reply to Jordan");
    node(
        &mut visual,
        &view,
        "status",
        &format!("Accepted edits · audience: Jordan Lee · reply: {message}"),
    );
    assert!(!shown(
        &mut visual,
        &view,
        "button",
        "Review Reply to Jordan"
    ));
    click(&mut visual, &view, "button", "Undo");
    node(&mut visual, &view, "status", "Restored Reply to Jordan");
    click(&mut visual, &view, "button", "Review Reply to Jordan");
    click(&mut visual, &view, "button", "Edit");
    assert_eq!(
        node(&mut visual, &view, "text_field", "Recipient").value,
        Some(UiValue::String("Jordan".into()))
    );
}

#[gpui::test]
fn gallery_review_center_narrow_queue_and_versioned_rejection_receipt(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "review_center", "clean-light");
    visual.simulate_resize(size(px(620.), px(1180.)));
    ready(&mut visual, &view);
    let queue = bounds(&mut visual, &view, "region", "Pending review");
    let detail = bounds(&mut visual, &view, "region", "Request evidence");
    assert!(detail.y >= queue.y + queue.height - 1.);
    let diff = bounds(
        &mut visual,
        &view,
        "document",
        "Before compared with Proposed",
    );
    assert!(detail.y + detail.height >= diff.y + diff.height - 1.);
    click(&mut visual, &view, "button", "Reject");
    node(&mut visual, &view, "button", "Needs more evidence");
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    assert!(!shown(&mut visual, &view, "button", "Needs more evidence"));
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "button", "Needs more evidence");
    visual.simulate_keystrokes("3");
    ready(&mut visual, &view);
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Rejected: Prepare the rollout",
    );
    node(
        &mut visual,
        &view,
        "status",
        "Recorded rejection: needs_evidence",
    );
    node(&mut visual, &view, "button", "Undo");
    click(&mut visual, &view, "button", "Advance demo clock");
    assert!(!shown(&mut visual, &view, "button", "Undo"));
}

#[gpui::test]
fn review_center_controlled_filter_only_changes_after_host_acceptance(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(
        r#"
        import "patterns/review_center" as center;
        fn state_schema(){#{fields:#{
            filter:#{schema:#{type:"string"},"default":#{type:"string",value:"all"}},
            proposal:#{schema:#{type:"string"},"default":#{type:"string",value:"none"}},
        }}}
        fn choose(ctx,p){ctx.set_state("proposal",p);}
        fn apply(ctx,p){ctx.set_state("filter",ctx.get_state("proposal"));}
        fn view(ctx){column([
            center::ReviewCenter(#{key:"controlled",now_ms:100000,order:"oldest",filter:ctx.get_state("filter"),on_filter_change:Fn("choose"),
                items:[
                    #{id:"high",revision:0,title:"High item",agent:"One",kind:"Review",risk:"high",confidence:0.9,created_ms:70000},
                    #{id:"older",revision:0,title:"Older item",agent:"Two",kind:"Review",risk:"low",confidence:0.9,created_ms:50000},
                ]}),
            row([text("Apply filter").with_key("label")]).with_key("apply").accessibility_role("button")
                .accessibility_label("Apply filter").tab_stop(true).on_click(Fn("apply")),
            text(ctx.get_state("proposal")).with_key("proposal").accessibility_role("status"),
        ]).with_style(style().gap(px(9)).padding(px(9)))}
    "#,
    )
    .registry("patterns/review_center")
    .bundled_assets()
    .mount(cx);
    visual.simulate_resize(size(px(1000.), px(860.)));
    ready(&mut visual, &view);
    node(&mut visual, &view, "heading", "Older item");
    click(&mut visual, &view, "button", "High risk");
    node(&mut visual, &view, "status", "high");
    node(&mut visual, &view, "button", "Review Older item");
    assert_eq!(
        node(&mut visual, &view, "button", "All").pressed,
        Some(true)
    );
    click(&mut visual, &view, "button", "Apply filter");
    assert!(!shown(&mut visual, &view, "button", "Review Older item"));
    node(&mut visual, &view, "heading", "High item");
}
