//! Motions driven natively by hover and press (`motion_hover`, `motion_press`)
//! start from pointer events, outside any render, prepaint or paint.

use std::time::{Duration, Instant};

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{ManualRuntimeClock, MotionPreference};

use crate::support::{Mount, Step, bounds, center, filled, last_error, settle};

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
