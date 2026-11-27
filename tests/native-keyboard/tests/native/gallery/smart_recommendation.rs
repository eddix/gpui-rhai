use std::time::{Duration, Instant};

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, ScriptViewHandle, UiValue};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::conversation::{click, gallery, node, ready};
use crate::support::{Host, Mount, bounds, snapshot};

fn finish(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(550));
    ready(visual, view);
}

#[gpui::test]
fn smart_recommendation_selection_survives_reordering_and_host_rejection(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "smart_recommendation", "modern-light");
    v.simulate_resize(size(px(1300.), px(900.)));
    ready(&mut v, &view);
    node(&mut v, &view, "group", "Strong signal");
    click(&mut v, &view, "button", "Alternatives");
    assert_eq!(
        node(&mut v, &view, "menuitem", "Incremental sync").checked,
        Some(UiValue::Bool(true))
    );
    click(&mut v, &view, "menuitem", "Full sync");
    node(&mut v, &view, "group", "Good signal");
    click(&mut v, &view, "button", "Reorder options");
    node(&mut v, &view, "button", "Accept full sync");
    click(&mut v, &view, "button", "Refuse next proposal");
    click(&mut v, &view, "button", "Accept full sync");
    node(&mut v, &view, "status", "Waiting for confirmation");
    assert!(node(&mut v, &view, "button", "Accept full sync").disabled);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("status", "Full sync accepted")
            .next()
            .is_none()
    );
    finish(&mut v, &view);
    node(&mut v, &view, "status", "Proposal refused by Host");
    assert!(!node(&mut v, &view, "button", "Accept full sync").disabled);
    click(&mut v, &view, "button", "Accept full sync");
    finish(&mut v, &view);
    node(&mut v, &view, "status", "Full sync accepted");
    click(&mut v, &view, "button", "Undo");
    node(&mut v, &view, "status", "Full sync accepted");
    finish(&mut v, &view);
    node(&mut v, &view, "status", "Acceptance undone by Host");
    node(&mut v, &view, "button", "Accept full sync");
}

#[gpui::test]
fn smart_recommendation_keyboard_and_long_labels_fit_the_narrow_card(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "smart_recommendation", "clean-dark");
    v.simulate_resize(size(px(1300.), px(900.)));
    ready(&mut v, &view);
    click(&mut v, &view, "button", "Smart recommendation");
    assert_eq!(
        node(&mut v, &view, "button", "Smart recommendation").expanded,
        Some(false)
    );
    click(&mut v, &view, "button", "Narrow recommendation");
    click(&mut v, &view, "button", "Alternatives");
    v.simulate_keystrokes("end enter");
    ready(&mut v, &view);
    let full_name = "Accept manual review before moving any recently edited files";
    node(&mut v, &view, "button", full_name);
    let button = bounds(&mut v, &view, "button", full_name);
    let frame = bounds(&mut v, &view, "group", "Smart recommendation");
    let confidence = bounds(&mut v, &view, "group", "No signal");
    assert!(
        button.y >= confidence.y + confidence.height
            && button.x + button.width <= frame.x + frame.width + 1.,
        "{button:?} {frame:?}"
    );
    assert_eq!(
        node(&mut v, &view, "button", "Smart recommendation").expanded,
        Some(true)
    );
    click(&mut v, &view, "button", "Alternatives");
    v.simulate_keystrokes("escape");
    ready(&mut v, &view);
    assert!(
        snapshot(&mut v, &view)
            .find_by_role_and_name("menu", "Alternatives")
            .next()
            .is_none()
    );
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    node(&mut v, &view, "menuitem", "AWS backup");
}

#[gpui::test]
fn smart_recommendation_local_acceptance_uses_the_same_stable_button(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::new(r#"
        import "patterns/smart_recommendation" as rec;
        fn view(ctx) {
            rec::SmartRecommendation(#{key:"local",session:"backup",options:[
                #{id:"cloud",label:"AWS backup",signal:1,reason:row([
                    text("Keep the regional copy").accessibility_role("heading").accessibility_label("Regional reason")
                ])}
            ]})
        }
    "#).registry("patterns/smart_recommendation").motion(MotionPreference::None).mount(cx);
    ready(&mut v, &view);
    node(&mut v, &view, "heading", "Regional reason");
    let start = node(&mut v, &view, "button", "Accept AWS backup").id;
    click(&mut v, &view, "button", "Accept AWS backup");
    node(&mut v, &view, "status", "AWS backup accepted");
    assert_eq!(node(&mut v, &view, "button", "Undo").id, start);
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    assert_eq!(node(&mut v, &view, "button", "Accept AWS backup").id, start);
}

#[gpui::test]
fn smart_recommendation_disclosure_moves_the_footer_on_native_frames(cx: &mut TestAppContext) {
    let clock = ManualRuntimeClock::new(Instant::now());
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "smart_recommendation".into(),
        theme: "modern-light".into(),
        ..Default::default()
    })
    .unwrap()
    .runtime_clock(clock.clock())
    .motion_preference(MotionPreference::Normal)
    .prepare()
    .unwrap();
    let (win, view) =
        super::mount_prepared(cx, prepared, ("rec", "gallery"), false, |host, view| Host {
            host,
            view,
        });
    let mut v = VisualTestContext::from_window(*win, cx);
    v.update(|win, _| win.activate_window());
    v.simulate_resize(size(px(1300.), px(900.)));
    clock.advance(Duration::from_millis(400));
    ready(&mut v, &view);
    let start = bounds(&mut v, &view, "group", "Strong signal").y;
    click(&mut v, &view, "button", "Smart recommendation");
    v.update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    clock.advance(Duration::from_millis(80));
    ready(&mut v, &view);
    let middle = bounds(&mut v, &view, "group", "Strong signal").y;
    clock.advance(Duration::from_millis(360));
    ready(&mut v, &view);
    let end = bounds(&mut v, &view, "group", "Strong signal").y;
    assert!(start > end + 2., "footer must move: {start} → {end}");
    if gpui_rhai::EXTENDED_RENDERING {
        assert!(
            start > middle + 2. && middle > end + 2.,
            "footer y at 0ms/80ms/440ms: {start}, {middle}, {end}"
        );
    } else {
        assert!((middle - end).abs() < 1., "{middle} → {end}");
    }
    assert!(
        v.update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty()
    );
}
