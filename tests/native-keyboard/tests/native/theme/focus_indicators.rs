//! `ScriptViewConfig::focus_indicators`: by default a focus frame shows after a navigation
//! key and hides on the next pointer press, a shortcut does not show it, and a text field
//! that holds focus always shows its frame. `Always` and `Never` override the default;
//! `Never` still frames a text field, where typing goes.

use std::collections::BTreeMap;

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, rgba};
use gpui_rhai::*;

use crate::support::{Mount, Step, bounds, center, painted_quads};

const FOCUS: u32 = 0x00ff_00ff;

const MAIN: &str = r#"import "components/button" as button;
import "components/input" as input;
fn noop(ctx, payload) {}
fn view(ctx) {
    column([
        button::Button(#{ key: "deploy", text: "Deploy", on_click: Fn("noop") }),
        input::Input(#{ key: "name", label: "Name", value: "", on_change: Fn("noop") }),
    ]).with_style(style().width(px(320)).padding(px(16)).gap(px(16)))
}"#;

fn mount(
    cx: &mut TestAppContext,
    indicators: Option<FocusIndicators>,
) -> (VisualTestContext, ScriptViewHandle) {
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([("focus_ring".to_owned(), Rgba8::from_rgba_hex(FOCUS))]),
        ..Default::default()
    };
    let mut mount = Mount::components(MAIN)
        .motion(MotionPreference::None)
        .with(move |view| view.theme_token_overrides(overrides))
        .id("indicators");
    if let Some(indicators) = indicators {
        mount = mount.config(move |config| config.focus_indicators(indicators));
    }
    let (mut visual, view) = mount.mount(cx);
    settle(&mut visual);
    visual.update(|window, cx| view.focus(window, cx).unwrap());
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Refresh, 3);
}

/// Whether a quad framed in the focus color covers `target`.
fn framed(visual: &mut VisualTestContext, target: GeometryBounds) -> bool {
    let color: gpui::Hsla = rgba(FOCUS).into();
    #[allow(clippy::cast_possible_truncation)]
    let (x, y) = (target.x as f32, target.y as f32);
    painted_quads(visual).iter().any(|quad| {
        quad.border_color == color
            && quad.has_border()
            && (quad.bounds.x - x).abs() < 1.0
            && (quad.bounds.y - y).abs() < 1.0
    })
}

fn keys(visual: &mut VisualTestContext, keys: &str) {
    visual.simulate_keystrokes(keys);
    settle(visual);
}

fn press(visual: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>) {
    visual.simulate_mouse_move(at, None, Modifiers::none());
    visual.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(visual);
}

#[gpui::test]
fn navigation_keys_show_focus_frames_a_press_hides_them_and_shortcuts_do_not(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = mount(cx, None);
    let deploy = bounds(&mut visual, &view, "button", "Deploy");
    keys(&mut visual, "tab");
    assert!(framed(&mut visual, deploy), "Tab shows the button's frame");
    // Pressing the button keeps the focus on it but hides the frame.
    press(&mut visual, center(deploy));
    assert!(!framed(&mut visual, deploy), "a pointer press hides it");
    keys(&mut visual, "cmd-k");
    assert!(!framed(&mut visual, deploy), "a shortcut does not show it");
    keys(&mut visual, "right");
    assert!(framed(&mut visual, deploy), "an arrow shows it again");
}

#[gpui::test]
fn a_text_field_shows_its_frame_after_a_click(cx: &mut TestAppContext) {
    for indicators in [None, Some(FocusIndicators::Never)] {
        let (mut visual, view) = mount(cx, indicators);
        let field = bounds(&mut visual, &view, "text_field", "Name");
        press(&mut visual, center(field));
        assert!(
            framed(&mut visual, field),
            "{indicators:?}: the focused field keeps its frame"
        );
    }
}

#[gpui::test]
fn never_shows_no_frame_after_navigation_keys(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, Some(FocusIndicators::Never));
    let deploy = bounds(&mut visual, &view, "button", "Deploy");
    keys(&mut visual, "tab");
    let focused = visual.update(|window, cx| window.focused(cx));
    assert!(focused.is_some(), "Tab still moves focus");
    assert!(!framed(&mut visual, deploy), "Never shows no frame");
}

#[gpui::test]
fn always_shows_the_frame_after_a_click(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, Some(FocusIndicators::Always));
    let deploy = bounds(&mut visual, &view, "button", "Deploy");
    press(&mut visual, center(deploy));
    assert!(
        framed(&mut visual, deploy),
        "Always frames the pressed button"
    );
}
