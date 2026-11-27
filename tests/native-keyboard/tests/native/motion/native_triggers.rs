//! Motions driven natively by hover and press (`motion_hover`, `motion_press`)
//! start from pointer events, outside any render, prepaint or paint.

use std::time::{Duration, Instant};

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{ManualRuntimeClock, MotionPreference};

use crate::support::{
    ActiveHost, Mount, Step, bounds, by_role, center, dispatch, filled, last_error, settle,
};

const HOVER: u32 = 0xff00_ffff;
const PRESS: u32 = 0x00ff_ffff;
const CLICK: u32 = 0xffff_00ff;
const FOCUS: u32 = 0x8080_ffff;

fn source() -> String {
    format!(
        r#"
fn noop(ctx, payload) {{ () }}
fn view(ctx) {{
    let timing = #{{ duration_ms: 80, easing: "linear" }};
    column([
        box([]).with_key("hover")
            .accessibility_role("button").accessibility_label("Hover me")
            .progress_motion(motion_hover(motion_transition("opacity", 0.4, 1.0, timing)))
            .with_style(style().width(px(120)).height(px(40)).background(rgba({HOVER:#x}))),
        box([]).with_key("press")
            .accessibility_role("button").accessibility_label("Press me")
            .progress_motion(motion_press(motion_transition("opacity", 1.0, 0.4, timing)))
            .with_style(style().width(px(120)).height(px(40)).background(rgba({PRESS:#x}))),
        box([]).with_key("click")
            .accessibility_role("button").accessibility_label("Click me").on_click(Fn("noop"))
            .progress_motion(motion_hover(motion_transition("opacity", 0.4, 1.0, timing)))
            .with_style(style().width(px(120)).height(px(40)).background(rgba({CLICK:#x}))),
        box([]).with_key("focus").tab_stop(true)
            .accessibility_role("button").accessibility_label("Focus me")
            .progress_motion(motion_focus(motion_transition("opacity", 0.4, 1.0, timing)))
            .with_style(style().width(px(120)).height(px(40)).background(rgba({FOCUS:#x}))),
    ]).with_style(style().gap(px(8)).padding(px(16)))
}}
"#
    )
}

/// Whether the box filled with `color` is painted fully opaque.
fn opaque(visual: &mut VisualTestContext, color: u32) -> bool {
    !filled(visual, color).is_empty()
}

#[gpui::test]
fn first_presented_frame_uses_each_native_drivers_declared_rest_value(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(source())
        .motion(MotionPreference::Normal)
        .mount(cx);
    // No settle or second frame: Press is the draw positive control, while
    // Hover must already carry its authored 0.4 rather than default opacity.
    assert_eq!(filled(&mut visual, PRESS).len(), 1);
    assert_eq!(filled(&mut visual, 0xff00_ff66).len(), 1);
    assert!(!opaque(&mut visual, HOVER));
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn a_child_consuming_mouse_up_cannot_prevent_its_owners_native_press_cleanup(
    cx: &mut TestAppContext,
) {
    for consume in [false, true] {
        let script = format!(
            r#"
        fn state_schema(){{#{{fields:#{{bubbles:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:0}}}}}}}}}}
        fn child_up(ctx,value){{if {consume}{{handled()}}else{{event_response()}}}}
        fn outer_up(ctx,value){{ctx.set_state("bubbles",ctx.get_state("bubbles")+1);}}
        fn view(ctx){{column([
            box([text("Child").accessibility_role("button").on("pointer_up",Fn("child_up"))])
                .with_key("owner").on_bubble("pointer_up",Fn("outer_up"))
                .with_style(style().width(px(120)).height(px(40)).background(rgba({PRESS:#x})))
                .progress_motion(motion_press(motion_transition("opacity",1.0,0.4,#{{duration_ms:80,easing:"linear"}}))),
            text(`${{ctx.get_state("bubbles")}}`).test_id("bubbles")
        ]).with_style(style().padding(px(16)).gap(px(8)))}}
    "#
        );
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let (mut visual, view) = Mount::new(script)
            .motion(MotionPreference::Normal)
            .with(move |view| view.runtime_clock(clock))
            .mount(cx);
        settle(&mut visual, Step::Refresh, 3);
        let child = center(bounds(&mut visual, &view, "button", "Child"));
        visual.simulate_mouse_down(child, MouseButton::Left, Modifiers::none());
        settle(&mut visual, Step::Refresh, 3);
        manual.advance(Duration::from_millis(120));
        settle(&mut visual, Step::Refresh, 3);
        assert!(!opaque(&mut visual, PRESS));
        visual.simulate_mouse_up(child, MouseButton::Left, Modifiers::none());
        settle(&mut visual, Step::Refresh, 3);
        manual.advance(Duration::from_millis(120));
        settle(&mut visual, Step::Refresh, 3);
        assert!(opaque(&mut visual, PRESS));
        assert_eq!(
            crate::support::text_of(&mut visual, &view, "bubbles"),
            if consume { "0" } else { "1" },
            "native cleanup must not change business propagation"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn suspension_cancels_press_without_discarding_other_motion_time(cx: &mut TestAppContext) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let script=source().replacen("column([",r#"column([
        box([]).with_key("independent").with_style(style().width(px(20)).height(px(20)).background(rgba(0x0012abff)))
            .motion(motion_transition("translate_x",0.,100.,#{duration_ms:1000,easing:"linear"})),"#,1);
    let (window, view) = Mount::new(script)
        .motion(MotionPreference::Normal)
        .with(move |view| view.runtime_clock(clock))
        .mount_root(cx, |host, view| ActiveHost { host, view });
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Refresh, 3);
    let press = center(bounds(&mut visual, &view, "button", "Press me"));
    visual.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(120));
    settle(&mut visual, Step::Refresh, 3);
    assert!(!opaque(&mut visual, PRESS));
    let before = filled(&mut visual, 0x0012_abff)[0].bounds.x;
    assert!(
        visual
            .update(|window, cx| view.suspend(window, cx))
            .unwrap()
    );
    settle(&mut visual, Step::Refresh, 3);
    visual.simulate_mouse_up(
        point(px(500.), px(500.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    manual.advance(Duration::from_secs(60));
    assert!(visual.update(|_, cx| view.resume(cx)).unwrap());
    settle(&mut visual, Step::Refresh, 3);
    assert!(
        opaque(&mut visual, PRESS),
        "resume starts at rest rather than replaying the abandoned press"
    );
    manual.advance(Duration::from_millis(120));
    settle(&mut visual, Step::Refresh, 3);
    assert!(
        opaque(&mut visual, PRESS),
        "the absent mouse-up during suspension cannot preserve an active press"
    );
    let after = filled(&mut visual, 0x0012_abff)[0].bounds.x;
    assert!(
        (after - before - 12.).abs() < 0.6,
        "unrelated source must retain active time, {before} -> {after}"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn hiding_or_detaching_a_press_binding_cannot_reactivate_an_old_target(cx: &mut TestAppContext) {
    for hidden in [false, true] {
        let script = format!(
            r#"
            fn state_schema(){{#{{fields:#{{enabled:#{{schema:#{{type:"bool"}},"default":#{{type:"bool",value:true}}}}}}}}}}
            fn toggle(ctx,payload){{ctx.set_state("enabled",!ctx.get_state("enabled"));}}
            fn view(ctx){{
                let enabled=ctx.get_state("enabled");
                let target=box([]).with_key("press").accessibility_role("button").accessibility_label("Press me")
                    .with_style(style().width(px(120)).height(px(40)).background(rgba({PRESS:#x})));
                if enabled || {hidden} {{target=target.progress_motion(motion_press(motion_transition("opacity",1.0,0.4,#{{duration_ms:80,easing:"linear"}})));}}
                column([target.retained_page(enabled || !{hidden}),text("Toggle").accessibility_role("button").on_click(Fn("toggle"))])
                    .with_style(style().padding(px(16)).gap(px(8)))
            }}
        "#
        );
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let (mut visual, view) = Mount::new(script)
            .motion(MotionPreference::Normal)
            .with(move |view| view.runtime_clock(clock))
            .mount(cx);
        settle(&mut visual, Step::Refresh, 3);
        let press = center(bounds(&mut visual, &view, "button", "Press me"));
        visual.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
        settle(&mut visual, Step::Refresh, 3);
        manual.advance(Duration::from_millis(120));
        settle(&mut visual, Step::Refresh, 3);
        assert!(!opaque(&mut visual, PRESS));
        dispatch(&mut visual, &view, by_role("button", "Toggle"), "click").unwrap();
        settle(&mut visual, Step::Refresh, 3);
        visual.simulate_mouse_up(
            point(px(500.), px(500.)),
            MouseButton::Left,
            Modifiers::none(),
        );
        dispatch(&mut visual, &view, by_role("button", "Toggle"), "click").unwrap();
        settle(&mut visual, Step::Refresh, 3);
        assert!(
            opaque(&mut visual, PRESS),
            "hidden={hidden}: restored binding starts at rest immediately"
        );
        manual.advance(Duration::from_millis(120));
        settle(&mut visual, Step::Refresh, 3);
        assert!(
            opaque(&mut visual, PRESS),
            "hidden={hidden}: restoring presentation cannot revive a held target"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn releasing_outside_the_owner_returns_native_press_motion_to_rest(cx: &mut TestAppContext) {
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::new(source())
        .motion(MotionPreference::Normal)
        .with(move |view| view.runtime_clock(clock))
        .mount(cx);
    settle(&mut visual, Step::Refresh, 3);
    let press = center(bounds(&mut visual, &view, "button", "Press me"));
    visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap();
    visual.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(120));
    settle(&mut visual, Step::Refresh, 3);
    assert!(!opaque(&mut visual, PRESS));
    let outside = point(px(500.), px(500.));
    visual.simulate_mouse_move(outside, Some(MouseButton::Left), Modifiers::none());
    visual.simulate_mouse_up(outside, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(120));
    settle(&mut visual, Step::Refresh, 3);
    assert!(
        opaque(&mut visual, PRESS),
        "release outside must not leave the native press target active"
    );
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(
        visual
            .update(|_, cx| view.take_performance_snapshot(cx))
            .unwrap()
            .timings
            .is_empty(),
        "native press frames must not execute Rhai"
    );
}

#[gpui::test]
fn disabling_a_pressed_owner_cancels_its_target_before_reenable(cx: &mut TestAppContext) {
    let script = format!(
        r#"
        fn state_schema(){{#{{fields:#{{blocked:#{{schema:#{{type:"bool"}},"default":#{{type:"bool",value:false}}}}}}}}}}
        fn toggle(ctx,payload){{ctx.set_state("blocked",!ctx.get_state("blocked"));}}
        fn view(ctx){{column([
            box([]).with_key("press").accessibility_role("button").accessibility_label("Press me")
                .progress_motion(motion_press(motion_transition("opacity",1.0,0.4,#{{duration_ms:80,easing:"linear"}})))
                .with_style(style().width(px(120)).height(px(40)).background(rgba({PRESS:#x})))
                .disabled(ctx.get_state("blocked")),
            text("Toggle blocked").accessibility_role("button").on_click(Fn("toggle"))
        ]).with_style(style().padding(px(16)).gap(px(8)))}}
    "#
    );
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::new(script)
        .motion(MotionPreference::Normal)
        .with(move |view| view.runtime_clock(clock))
        .mount(cx);
    settle(&mut visual, Step::Refresh, 3);
    let press = center(bounds(&mut visual, &view, "button", "Press me"));
    visual.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(120));
    settle(&mut visual, Step::Refresh, 3);
    assert!(!opaque(&mut visual, PRESS));
    dispatch(
        &mut visual,
        &view,
        by_role("button", "Toggle blocked"),
        "click",
    )
    .unwrap();
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(120));
    settle(&mut visual, Step::Refresh, 3);
    assert!(
        opaque(&mut visual, PRESS),
        "disabled owner must leave its pointer-driven target"
    );
    visual.simulate_mouse_up(
        point(px(500.), px(500.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    dispatch(
        &mut visual,
        &view,
        by_role("button", "Toggle blocked"),
        "click",
    )
    .unwrap();
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(120));
    settle(&mut visual, Step::Refresh, 3);
    assert!(
        opaque(&mut visual, PRESS),
        "reenabling cannot resurrect a released press"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn hover_and_press_start_their_motions_from_pointer_events(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let manual = ManualRuntimeClock::new(Instant::now());
    let clock = manual.clock();
    let (mut visual, view) = Mount::new(source())
        .with(move |view| view.runtime_clock(clock))
        .motion(MotionPreference::Normal)
        .mount(cx);
    settle(&mut visual, Step::Refresh, 3);
    let hover = center(bounds(&mut visual, &view, "button", "Hover me"));
    let press = center(bounds(&mut visual, &view, "button", "Press me"));
    let click = center(bounds(&mut visual, &view, "button", "Click me"));
    assert!(!opaque(&mut visual, HOVER), "the hover box rests at 0.4");
    assert!(opaque(&mut visual, PRESS), "the press box rests at 1.0");

    // The test window drops the first pointer move it sees.
    visual.simulate_mouse_move(point(px(500.0), px(500.0)), None, Modifiers::none());
    visual.simulate_mouse_move(hover, None, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(120));
    settle(&mut visual, Step::Refresh, 3);
    assert!(
        opaque(&mut visual, HOVER),
        "hovering brought the box to 1.0"
    );

    visual.simulate_mouse_move(press, None, Modifiers::none());
    visual.simulate_mouse_down(press, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(120));
    settle(&mut visual, Step::Refresh, 3);
    assert!(!opaque(&mut visual, PRESS), "pressing took the box to 0.4");

    visual.simulate_mouse_up(press, MouseButton::Left, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(120));
    settle(&mut visual, Step::Refresh, 3);
    assert!(
        opaque(&mut visual, PRESS),
        "releasing brought the box back to 1.0"
    );

    // A node with its own pointer handlers drives the motion from the same
    // hover listener.
    assert!(
        !opaque(&mut visual, CLICK),
        "the clickable box rests at 0.4"
    );
    visual.simulate_mouse_move(click, None, Modifiers::none());
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(120));
    settle(&mut visual, Step::Refresh, 3);
    assert!(
        opaque(&mut visual, CLICK),
        "hovering brought the clickable box to 1.0"
    );

    // A focus motion follows the node's focus handle, not pointer events, and
    // keeps running beside them.
    assert!(
        !opaque(&mut visual, FOCUS),
        "the focusable box rests at 0.4"
    );
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Refresh, 2);
    // The clickable box is the first tab stop, the focusable box the second.
    visual.simulate_keystrokes("tab tab");
    settle(&mut visual, Step::Refresh, 3);
    manual.advance(Duration::from_millis(120));
    settle(&mut visual, Step::Refresh, 3);
    assert!(opaque(&mut visual, FOCUS), "focus brought the box to 1.0");
    assert_eq!(last_error(&mut visual, &view), None);
}
