use std::time::Duration;

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::ScriptViewHandle;

use super::conversation::{click, gallery, node, ready};
use crate::support::{bounds, snapshot};

fn later(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(610));
    ready(visual, view);
}

fn missing(v: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    assert!(
        snapshot(v, view)
            .find_by_role_and_name(role, name)
            .next()
            .is_none(),
        "unexpected {role} {name}"
    );
}

#[gpui::test]
fn gallery_agent_monitor_clock_freezes_and_resume_excludes_pause(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_monitor", "modern-light");
    v.simulate_resize(size(px(1280.), px(1070.)));
    later(&mut v, &view);
    node(&mut v, &view, "status", "Elapsed: 20 s");
    let pause = node(&mut v, &view, "button", "Pause").id;
    click(&mut v, &view, "button", "Refuse next transition");
    click(&mut v, &view, "button", "Pause");
    node(&mut v, &view, "status", "Waiting for the demo Host…");
    assert!(node(&mut v, &view, "button", "Pause").disabled);
    later(&mut v, &view);
    node(&mut v, &view, "status", "The Host refused the transition.");
    node(&mut v, &view, "status", "Running");
    click(&mut v, &view, "button", "Pause");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Paused");
    assert_eq!(node(&mut v, &view, "button", "Resume").id, pause);
    // Pause and Resume are the same native control.
    v.simulate_keystrokes("enter");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Resumed the run.");
    click(&mut v, &view, "button", "Pause");
    later(&mut v, &view);
    click(&mut v, &view, "button", "Advance monitor clock");
    node(&mut v, &view, "status", "Elapsed: 20 s");
    assert!(
        node(&mut v, &view, "button", "Details for Build the preview")
            .description
            .contains("6 s")
    );
    click(&mut v, &view, "button", "Resume");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Elapsed: 20 s");
    click(&mut v, &view, "button", "Advance monitor clock");
    node(&mut v, &view, "status", "Elapsed: 32 s");
    assert!(
        node(&mut v, &view, "button", "Details for Build the preview")
            .description
            .contains("18 s")
    );
    click(&mut v, &view, "button", "Finish monitor run");
    node(&mut v, &view, "text", "Finished in 32 s");
    node(&mut v, &view, "text", "Preview saved locally.");
    click(&mut v, &view, "button", "Advance monitor clock");
    node(&mut v, &view, "status", "Elapsed: 32 s");
}

#[gpui::test]
fn gallery_agent_monitor_strip_reveals_scrolled_native_step_and_keeps_fold_focus(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "agent_monitor", "clean-dark");
    v.simulate_resize(size(px(1280.), px(1040.)));
    later(&mut v, &view);
    let first = bounds(&mut v, &view, "button", "Duration: Read preview inputs");
    let second = bounds(
        &mut v,
        &view,
        "button",
        "Duration: Find reference baselines",
    );
    assert!(
        second.width > first.width * 1.3,
        "strip widths are duration-weighted"
    );
    let history = bounds(&mut v, &view, "list", "Recent steps");
    let row = bounds(&mut v, &view, "button", "Details for Read preview inputs");
    assert!(row.y + row.height > history.y + history.height);
    click(&mut v, &view, "button", "Duration: Read preview inputs");
    later(&mut v, &view);
    node(&mut v, &view, "dialog", "Read preview inputs");
    node(&mut v, &view, "text", "Fixture inputs: 4 palettes");
    let history = bounds(&mut v, &view, "list", "Recent steps");
    let row = bounds(&mut v, &view, "button", "Details for Read preview inputs");
    assert!(row.y >= history.y - 1.0 && row.y + row.height <= history.y + history.height + 1.0);
    v.simulate_keystrokes("escape");
    later(&mut v, &view);
    missing(&mut v, &view, "dialog", "Read preview inputs");
    let fold = node(&mut v, &view, "button", "Fold monitor").id;
    click(&mut v, &view, "button", "Fold monitor");
    missing(&mut v, &view, "list", "Recent steps");
    assert_eq!(node(&mut v, &view, "button", "Open monitor").id, fold);
    v.simulate_keystrokes("enter");
    later(&mut v, &view);
    node(&mut v, &view, "list", "Recent steps");
}

#[gpui::test]
fn gallery_agent_monitor_approval_is_scoped_and_retry_is_versioned(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_monitor", "clean-light");
    v.simulate_resize(size(px(1280.), px(1140.)));
    later(&mut v, &view);
    click(&mut v, &view, "button", "Request monitor approval");
    v.simulate_keystrokes("escape");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Waiting on you");
    node(&mut v, &view, "text", "preview --fixture local");
    click(&mut v, &view, "button", "Refuse next transition");
    click(&mut v, &view, "region", "Current activity");
    v.simulate_keystrokes("enter");
    later(&mut v, &view);
    node(&mut v, &view, "status", "The Host refused the transition.");
    node(&mut v, &view, "text", "preview --fixture local");
    click(&mut v, &view, "region", "Current activity");
    v.simulate_keystrokes("enter");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Allowed request/配色/1");
    missing(&mut v, &view, "text", "preview --fixture local");
    click(&mut v, &view, "button", "Request monitor approval");
    click(&mut v, &view, "region", "Current activity");
    v.simulate_keystrokes("escape");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Declined request/配色/3");
    node(&mut v, &view, "status", "Cancelled");
    click(&mut v, &view, "button", "Reset monitor");
    click(&mut v, &view, "button", "Fail monitor run");
    node(
        &mut v,
        &view,
        "text",
        "The fixture is missing an output image.",
    );
    click(&mut v, &view, "button", "Try again");
    click(&mut v, &view, "button", "Reset monitor");
    later(&mut v, &view);
    missing(&mut v, &view, "status", "Started a new run.");
    click(&mut v, &view, "button", "Fail monitor run");
    click(&mut v, &view, "button", "Try again");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Started a new run.");
    node(&mut v, &view, "status", "Running");
}
