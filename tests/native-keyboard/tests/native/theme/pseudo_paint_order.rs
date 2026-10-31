//! Pseudo paint on ordinary nodes applies `base → hover → focus → active`: while a
//! node holds focus its focus paint covers its hover paint, and a press still paints
//! over both.

use std::collections::BTreeMap;

use gpui::{Modifiers, MouseButton, Pixels, Point, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::*;

use crate::support::{Mount, Step, bounds};

const HOVER: u32 = 0x0000_ffff;
const FOCUS: u32 = 0x00ff_00ff;
const ACTIVE: u32 = 0xff00_00ff;

fn mount(
    cx: &mut TestAppContext,
    main: &str,
    overrides: ThemeTokenOverrides,
) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(main)
        .motion(MotionPreference::None)
        .with(move |view| view.theme_token_overrides(overrides))
        // Paint order with the pointer over a focused node: focus shows without a key.
        .config(|config| config.focus_indicators(FocusIndicators::Always))
        .id("pseudo")
        .mount(cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Refresh, 3);
}

/// Logical sizes of the quads filled with `color`.
fn filled(visual: &mut VisualTestContext, color: u32) -> Vec<(f32, f32)> {
    crate::support::filled(visual, color)
        .iter()
        .map(|quad| (quad.bounds.width, quad.bounds.height))
        .collect()
}

fn has_width(quads: &[(f32, f32)], width: f32) -> bool {
    quads.iter().any(|(w, _)| (w - width).abs() < 0.5)
}

fn center(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> Point<Pixels> {
    crate::support::center(bounds(visual, view, role, name))
}

fn focus_first_tab_stop(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.update(|window, cx| view.focus(window, cx)).unwrap();
    visual.simulate_keystrokes("tab");
    settle(visual);
}

#[gpui::test]
fn focus_paints_over_hover_and_a_press_paints_over_focus(cx: &mut TestAppContext) {
    let main = format!(
        r#"fn pressed(ctx, payload) {{}}
fn view(ctx) {{ column([
    box([]).with_style(style().width(px(40)).height(px(40)).background(rgba(0x202020ff))
            .hover(style().background(rgba({HOVER:#x})))
            .focus(style().background(rgba({FOCUS:#x})))
            .active(style().background(rgba({ACTIVE:#x}))))
        .on_click(Fn("pressed")).accessibility_role("button").accessibility_label("Probe"),
]).with_style(style().width(px(200)).height(px(120)).padding(px(20))) }}"#
    );
    let (mut visual, view) = mount(cx, &main, ThemeTokenOverrides::default());
    let probe = center(&mut visual, &view, "button", "Probe");
    let away = point(px(180.), px(110.));

    visual.simulate_mouse_move(probe, None, Modifiers::none());
    settle(&mut visual);
    assert!(has_width(&filled(&mut visual, HOVER), 40.0), "hover paints");
    visual.simulate_mouse_move(away, None, Modifiers::none());
    settle(&mut visual);

    // Keyboard input suppresses hover until the pointer moves again, so the
    // pointer comes back after the node takes focus.
    focus_first_tab_stop(&mut visual, &view);
    assert!(has_width(&filled(&mut visual, FOCUS), 40.0), "focus paints");
    visual.simulate_mouse_move(probe, None, Modifiers::none());
    settle(&mut visual);
    let focus = filled(&mut visual, FOCUS);
    let hover = filled(&mut visual, HOVER);
    assert!(
        has_width(&focus, 40.0) && !has_width(&hover, 40.0),
        "focus must paint over hover: focus {focus:?}, hover {hover:?}"
    );

    visual.simulate_mouse_down(probe, MouseButton::Left, Modifiers::none());
    settle(&mut visual);
    let active = filled(&mut visual, ACTIVE);
    assert!(
        has_width(&active, 40.0),
        "a press paints over focus: {active:?}"
    );
    visual.simulate_mouse_up(probe, MouseButton::Left, Modifiers::none());
    settle(&mut visual);

    visual.simulate_mouse_move(away, None, Modifiers::none());
    settle(&mut visual);
    assert!(
        has_width(&filled(&mut visual, FOCUS), 40.0),
        "focus paints without the pointer"
    );
}

#[gpui::test]
fn split_pane_handle_keeps_its_focus_bar_under_the_pointer(cx: &mut TestAppContext) {
    let main = r#"import "components/split_pane" as split_pane;
fn resized(ctx, value) {}
fn view(ctx) { column([
    split_pane::SplitPane(#{ key: "layout", label: "Resize panels", size: 0.5,
        start: text("Start"), end: text("End"), on_resize: Fn("resized") })
        .with_style(style().width(px(420)).height(px(180))),
]) }"#;
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([
            ("surface_hover".to_owned(), Rgba8::from_rgba_hex(HOVER)),
            ("focus_ring".to_owned(), Rgba8::from_rgba_hex(FOCUS)),
        ]),
        ..Default::default()
    };
    let (mut visual, view) = mount(cx, main, overrides);
    let handle = center(&mut visual, &view, "separator", "Resize panels");

    // The native handle takes the pointer; its state signal fills the zone on hover.
    visual.simulate_mouse_move(handle, None, Modifiers::none());
    settle(&mut visual);
    let hover = filled(&mut visual, HOVER);
    assert!(
        has_width(&hover, 8.0),
        "the hovered handle fills its zone: {hover:?}"
    );
    visual.simulate_mouse_move(point(px(10.), px(10.)), None, Modifiers::none());
    settle(&mut visual);

    focus_first_tab_stop(&mut visual, &view);
    assert!(
        has_width(&filled(&mut visual, FOCUS), 8.0),
        "the focused handle is an ink bar"
    );
    visual.simulate_mouse_move(handle, None, Modifiers::none());
    settle(&mut visual);
    let focus = filled(&mut visual, FOCUS);
    let hover = filled(&mut visual, HOVER);
    assert!(
        has_width(&focus, 8.0) && !has_width(&hover, 8.0),
        "the focused handle stays an ink bar under the pointer: focus {focus:?}, hover {hover:?}"
    );
}
