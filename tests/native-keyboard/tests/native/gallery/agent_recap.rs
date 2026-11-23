use std::time::{Duration, Instant};

use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, ScriptViewHandle};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, Mount, bounds, center, snapshot};

const RELEASE: &str = "Publish the draft release";
const FAILED: &str = "Preview sync failed";
const DONE: &str = "Indexed the support queue";

fn wait_for_host(v: &mut VisualTestContext, view: &ScriptViewHandle) {
    v.executor().advance_clock(Duration::from_millis(650));
    ready(v, view);
}

#[gpui::test]
fn gallery_agent_recap_proposes_actions_and_uses_accepted_outcomes(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_recap", "modern-light");
    v.simulate_resize(size(px(1260.), px(1000.)));
    ready(&mut v, &view);
    node(&mut v, &view, "listitem", "Done: 2");
    node(&mut v, &view, "listitem", "Needs you: 2");
    node(&mut v, &view, "listitem", "Failed: 1");
    let original = node(&mut v, &view, "listitem", RELEASE);
    click(&mut v, &view, "button", &format!("Review: {RELEASE}"));
    node(
        &mut v,
        &view,
        "status",
        &format!("Review requested: {RELEASE}"),
    );
    click(&mut v, &view, "button", "Reject next recap action");
    node(
        &mut v,
        &view,
        "status",
        "The next command will not be accepted.",
    );
    click(&mut v, &view, "button", &format!("Approve: {RELEASE}"));
    node(&mut v, &view, "status", "Waiting for the demo Host…");
    node(&mut v, &view, "listitem", "Needs you: 2");
    wait_for_host(&mut v, &view);
    node(&mut v, &view, "status", &format!("Not accepted: {RELEASE}"));
    assert_eq!(original.id, node(&mut v, &view, "listitem", RELEASE).id);
    click(&mut v, &view, "button", &format!("Approve: {RELEASE}"));
    let focus = v
        .update(|window, cx| window.focused(cx))
        .expect("the pressed action retains focus");
    wait_for_host(&mut v, &view);
    node(&mut v, &view, "status", &format!("Accepted: {RELEASE}"));
    node(&mut v, &view, "listitem", "Done: 3");
    node(&mut v, &view, "listitem", "Needs you: 1");
    assert_eq!(v.update(|window, cx| window.focused(cx)), Some(focus));
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", &format!("Undone: {RELEASE}"));
    node(&mut v, &view, "listitem", "Done: 2");
    click(&mut v, &view, "button", &format!("Retry: {FAILED}"));
    wait_for_host(&mut v, &view);
    node(&mut v, &view, "listitem", "Failed: 0");
    click(&mut v, &view, "button", &format!("Undo: {DONE}"));
    click(&mut v, &view, "button", &format!("Redo: {DONE}"));
    node(&mut v, &view, "status", &format!("Redone: {DONE}"));
}

#[gpui::test]
fn gallery_agent_recap_links_marks_and_cards_and_stacks_narrow_lanes(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_recap", "clean-dark");
    v.simulate_resize(size(px(1260.), px(1000.)));
    ready(&mut v, &view);
    let release = bounds(&mut v, &view, "listitem", RELEASE);
    let failed = bounds(&mut v, &view, "listitem", FAILED);
    assert!((failed.y - release.y).abs() < 1.0, "{release:?} {failed:?}");
    assert!(failed.x > release.x, "{release:?} {failed:?}");
    let mark = snapshot(&mut v, &view)
        .nodes()
        .find(|n| n.role.as_str() == "button" && n.name.ends_with(&format!("· {RELEASE}")))
        .unwrap()
        .clone();
    v.simulate_mouse_move(
        center(mark.geometry.unwrap().visual),
        None,
        Modifiers::none(),
    );
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Inspected event: release/签收");
    v.simulate_mouse_move(center(failed), None, Modifiers::none());
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Inspected event: pipeline");
    v.simulate_resize(size(px(720.), px(1000.)));
    ready(&mut v, &view);
    let release = bounds(&mut v, &view, "listitem", RELEASE);
    let failed = bounds(&mut v, &view, "listitem", FAILED);
    assert!((failed.x - release.x).abs() < 1.0 && failed.y > release.y);
    // Return to the wide layout so Gallery controls are within the viewport.
    v.simulate_resize(size(px(1260.), px(1000.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Show summary only");
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("listitem", RELEASE)
            .next()
            .is_none()
    );
    node(&mut v, &view, "listitem", "Needs you: 2");
}

#[gpui::test]
fn gallery_agent_recap_rtl_labels_follow_the_numeric_axis(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_recap", "clean-dark");
    v.simulate_resize(size(px(860.), px(1000.)));
    v.update(|_, cx| view.select_locale("ar", cx)).unwrap();
    ready(&mut v, &view);
    let tree = snapshot(&mut v, &view);
    let x = |prefix: &str| {
        tree.nodes()
            .find(|n| n.role == "text" && n.name.starts_with(prefix))
            .unwrap_or_else(|| panic!("missing {prefix}"))
            .geometry
            .unwrap()
            .visual
            .x
    };
    assert!(x("البداية:") < x("النهاية:"));
}

#[gpui::test]
fn gallery_agent_recap_replays_on_the_motion_clock_and_keeps_progress_on_resize(
    cx: &mut TestAppContext,
) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "agent_recap".into(),
        theme: "clean-light".into(),
        ..Default::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::Normal)
    .runtime_clock(manual.clock())
    .prepare()
    .unwrap();
    let (window, view) = super::mount_prepared(
        cx,
        prepared,
        ("recap-test", "gallery"),
        false,
        |host, view| Host { host, view },
    );
    let mut v = VisualTestContext::from_window(*window, cx);
    v.update(|window, _| window.activate_window());
    v.simulate_resize(size(px(1260.), px(1000.)));
    ready(&mut v, &view);
    assert_count(&mut v, &view, "Done: 0", "initial replay");
    manual.advance(Duration::from_millis(220));
    ready(&mut v, &view);
    assert_count(&mut v, &view, "Done: 1", "native 220 ms");
    assert_count(&mut v, &view, "Needs you: 1", "native 220 ms");
    v.simulate_resize(size(px(1100.), px(1000.)));
    ready(&mut v, &view);
    assert_count(&mut v, &view, "Needs you: 1", "resize at fixed clock");
    manual.advance(Duration::from_millis(1800));
    ready(&mut v, &view);
    assert_count(&mut v, &view, "Done: 2", "finished");
    assert_count(&mut v, &view, "Needs you: 2", "finished");
    click(&mut v, &view, "button", "Replay recap");
    assert_count(&mut v, &view, "Done: 0", "explicit restart");
    v.update(|_, cx| view.set_motion_preference(MotionPreference::Reduced, cx))
        .unwrap();
    ready(&mut v, &view);
    assert_count(
        &mut v,
        &view,
        "Done: 2",
        "reduced motion settles immediately",
    );
    v.update(|_, cx| view.set_motion_preference(MotionPreference::Normal, cx))
        .unwrap();
    ready(&mut v, &view);
    assert_count(&mut v, &view, "Done: 2", "restoring policy does not replay");
}

#[track_caller]
fn assert_count(v: &mut VisualTestContext, view: &ScriptViewHandle, name: &str, step: &str) {
    let tree = snapshot(v, view);
    assert!(
        tree.find_by_role_and_name("listitem", name)
            .next()
            .is_some(),
        "{step}: no {name}; current items: {:?}",
        tree.nodes()
            .filter(|n| n.role.as_str() == "listitem")
            .map(|n| n.name.as_str())
            .collect::<Vec<_>>()
    );
}

#[gpui::test]
fn recap_width_fraction_motion_uses_fresh_native_parent_geometry(cx: &mut TestAppContext) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut v, view) = Mount::components(r#"
        fn view(ctx){
            let moving=column([]).accessibility_role("group").accessibility_label("Width witness")
                .with_key("reveal").with_style(style().width(relative(1.0)).height(px(20)).clip())
                .motion(motion_transition("width_fraction",0.0,1.0,#{duration_ms:1000,easing:"linear"}));
            column([moving]).accessibility_role("group").accessibility_label("Parent witness")
                .with_style(style().min_width(px(0)).width(relative(1.0)).items_start())
        }
    "#).motion(MotionPreference::Normal).with(move |view| view.runtime_clock(clock)).mount(cx);
    v.simulate_resize(size(px(720.), px(360.)));
    ready(&mut v, &view);
    manual.advance(Duration::from_millis(500));
    ready(&mut v, &view);
    let small = bounds(&mut v, &view, "group", "Parent witness").width;
    let moving = bounds(&mut v, &view, "group", "Width witness").width;
    assert!((small * 0.5 - moving).abs() < 1.0, "{small} {moving}");
    v.simulate_resize(size(px(960.), px(360.)));
    ready(&mut v, &view);
    let large = bounds(&mut v, &view, "group", "Parent witness").width;
    let moving = bounds(&mut v, &view, "group", "Width witness").width;
    assert!(
        large > small + 100.0 && (large * 0.5 - moving).abs() < 1.0,
        "{large} {moving}"
    );
    manual.advance(Duration::from_millis(500));
    ready(&mut v, &view);
    let final_width = bounds(&mut v, &view, "group", "Width witness").width;
    assert!((large - final_width).abs() < 1.0);
}
