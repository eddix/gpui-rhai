use std::time::Duration;

use gpui::{TestAppContext, px, size};

use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, snapshot};

#[gpui::test]
fn notification_stack_keeps_an_accepted_front_and_reuses_keyboard_dismissal(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "notification_stack", "modern-light");
    v.simulate_resize(size(px(1240.), px(940.)));
    ready(&mut v, &view);
    let front = bounds(&mut v, &view, "group", "Notifications");
    let target = node(&mut v, &view, "button", "Dismiss Release ready").id;
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("heading", "Billing moved")
            .next()
            .is_none()
    );
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("button", "Dismiss Billing moved")
            .next()
            .is_none()
    );
    click(&mut v, &view, "button", "Release notes");
    node(&mut v, &view, "status", "Host action: notes");
    click(&mut v, &view, "button", "Refuse next request");
    click(&mut v, &view, "button", "Dismiss Release ready");
    node(&mut v, &view, "heading", "Release ready");
    node(&mut v, &view, "status", "Waiting for the Host…");
    v.executor().advance_clock(Duration::from_millis(390));
    ready(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "The Host kept the current snapshot.",
    );
    node(&mut v, &view, "heading", "Release ready");
    v.simulate_keystrokes("space");
    ready(&mut v, &view);
    v.executor().advance_clock(Duration::from_millis(390));
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Dismissed: release");
    node(&mut v, &view, "heading", "Billing moved");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("heading", "Release ready")
            .next()
            .is_none()
    );
    assert_eq!(
        node(&mut v, &view, "button", "Dismiss Billing moved").id,
        target
    );
    assert!(bounds(&mut v, &view, "group", "Notifications").height > front.height);
    v.simulate_keystrokes("space");
    ready(&mut v, &view);
    v.executor().advance_clock(Duration::from_millis(390));
    ready(&mut v, &view);
    node(&mut v, &view, "heading", "Shortcuts are ready");
    assert_eq!(
        node(&mut v, &view, "button", "Dismiss Shortcuts are ready").id,
        target
    );
}

#[gpui::test]
fn notification_stack_empty_after_last_accepted_dismissal(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "notification_stack", "clean-dark");
    v.simulate_resize(size(px(1240.), px(940.)));
    ready(&mut v, &view);
    for label in [
        "Release ready",
        "Billing moved",
        "Shortcuts are ready",
        "Background maintenance",
    ] {
        click(&mut v, &view, "button", &format!("Dismiss {label}"));
        v.executor().advance_clock(Duration::from_millis(390));
        ready(&mut v, &view);
    }
    let snapshot = snapshot(&mut v, &view);
    for label in [
        "Release ready",
        "Billing moved",
        "Shortcuts are ready",
        "Background maintenance",
    ] {
        assert!(
            snapshot
                .find_by_role_and_name("heading", label)
                .next()
                .is_none()
        );
    }
    assert!(bounds(&mut v, &view, "group", "Notifications").height <= 1.);
    click(&mut v, &view, "button", "Reset stack");
    node(&mut v, &view, "heading", "Release ready");
}
