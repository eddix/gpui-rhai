//! SplitPane's default separator is flush (#221): one 1px `border` line between panes that
//! reach it, unchanged through hover and drag, with a transparent 7px hit zone over both panes
//! that takes the drag; the grip bar shows only while hovered, dragged or focused.

use std::collections::BTreeMap;

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{GeometryBounds, MotionPreference, Rgba8, ScriptViewHandle, ThemeTokenOverrides};

use crate::support::{Mount, Step, filled, last_error, nodes, text_of};

const BORDER: u32 = 0x0000_ffff;
const MUTED: u32 = 0xff00_ffff;
const RING: u32 = 0x00ff_00ff;
const HOVER: u32 = 0xffff_00ff;

fn mount(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
    mount_with(cx, "")
}

fn mount_with(cx: &mut TestAppContext, extra: &str) -> (VisualTestContext, ScriptViewHandle) {
    let main = r#"import "components/split_pane" as split_pane;
fn state_schema() { #{ fields: #{
    size: #{ schema: #{ type: "number" }, "default": #{ type: "float", value: 0.5 } },
    log: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } } } } }
fn resized(ctx, size) { ctx.set_state("size", size); ctx.set_state("log", `${size}`); }
fn view(ctx) { column([
    split_pane::SplitPane(#{ key: "layout", label: "Resize panes", size: ctx.get_state("size"),
        min_start: 40.0, min_end: 40.0, start: text("Start"), end: text("End"), EXTRA
        on_resize: Fn("resized") }),
    text(ctx.get_state("log")).test_id("log"),
]).with_style(style().width(px(400)).height(px(200))) }"#;
    let main = main.replace("EXTRA", extra);
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([
            ("border".to_owned(), Rgba8::from_rgba_hex(BORDER)),
            ("text_muted".to_owned(), Rgba8::from_rgba_hex(MUTED)),
            ("focus_ring".to_owned(), Rgba8::from_rgba_hex(RING)),
            ("surface_hover".to_owned(), Rgba8::from_rgba_hex(HOVER)),
        ]),
        ..Default::default()
    };
    let (mut visual, view) = Mount::components(main)
        .with(move |view| view.theme_token_overrides(overrides))
        .motion(MotionPreference::None)
        .id("flush")
        .mount(cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Refresh, 3);
}

fn separator(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> GeometryBounds {
    nodes(visual, view)
        .into_iter()
        .find(|node| node.role == "separator")
        .and_then(|node| node.geometry)
        .unwrap()
        .visual
}

/// Visible quads in `color` within 12px of the separator, as (width, height).
fn near_line(visual: &mut VisualTestContext, line: GeometryBounds, color: u32) -> Vec<(f32, f32)> {
    filled(visual, color)
        .into_iter()
        .filter(|quad| {
            let x = f64::from(quad.bounds.x);
            x >= line.x - 12.0 && x <= line.x + line.width + 12.0
        })
        .map(|quad| (quad.bounds.width, quad.bounds.height))
        .collect()
}

fn drag(visual: &mut VisualTestContext, x: f64, y: f64, by: f64) {
    #[allow(clippy::cast_possible_truncation)]
    let at = |t: f64| point(px((x + by * t) as f32), px(y as f32));
    visual.simulate_mouse_move(at(0.0), None, Modifiers::default());
    visual.simulate_mouse_down(at(0.0), MouseButton::Left, Modifiers::default());
    for step in 1..=4 {
        visual.simulate_mouse_move(
            at(f64::from(step) / 4.0),
            MouseButton::Left,
            Modifiers::default(),
        );
        visual.run_until_parked();
    }
    visual.simulate_mouse_up(at(1.0), MouseButton::Left, Modifiers::default());
    settle(visual);
}

#[gpui::test]
fn a_flush_separator_is_one_still_line_whose_grip_shows_on_hover_and_focus(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = mount(cx);
    assert_eq!(last_error(&mut visual, &view), None);
    let line = separator(&mut visual, &view);
    assert!((line.width - 1.0).abs() < 0.01, "{line:?}");
    #[allow(clippy::cast_possible_truncation)]
    let full = (1.0, line.height as f32);
    // At rest: the line, no grip.
    assert_eq!(near_line(&mut visual, line, BORDER), [full]);
    assert!(near_line(&mut visual, line, MUTED).is_empty());
    // Hovered: the line stays as it is, with no lane fill, and the grip bar shows.
    #[allow(clippy::cast_possible_truncation)]
    let center = point(px((line.x + 0.5) as f32), px(100.0));
    visual.simulate_mouse_move(center, None, Modifiers::default());
    settle(&mut visual);
    assert!(near_line(&mut visual, line, BORDER).contains(&full));
    assert!(
        near_line(&mut visual, line, HOVER).is_empty(),
        "no lane fill"
    );
    assert_eq!(near_line(&mut visual, line, MUTED), [(4.0, 24.0)]);
    // Focused from the keyboard: the bar takes the focus color.
    visual.simulate_mouse_move(point(px(20.0), px(150.0)), None, Modifiers::default());
    visual.update(|window, cx| view.focus(window, cx)).unwrap();
    visual.simulate_keystrokes("tab");
    settle(&mut visual);
    assert_eq!(near_line(&mut visual, line, RING), [(4.0, 24.0)]);
}

#[gpui::test]
fn the_flush_hit_zone_takes_a_drag_three_pixels_off_the_line(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx);
    let line = separator(&mut visual, &view);
    // 3px off the line is inside the 7px zone: one resize, 60px to the right.
    drag(&mut visual, line.x + 0.5 + 3.0, 100.0, 60.0);
    let log = text_of(&mut visual, &view, "log");
    let size: f64 = log
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("no resize: {log:?}"));
    assert!(
        (size * 400.0 - (line.x + 60.0)).abs() < 2.0,
        "{size} from {line:?}"
    );
    // 6px off the line is outside it: nothing moves.
    let line = separator(&mut visual, &view);
    drag(&mut visual, line.x + 0.5 - 6.0, 100.0, -60.0);
    assert_eq!(
        text_of(&mut visual, &view, "log"),
        log,
        "a press outside the zone drags nothing"
    );
}
