//! SplitPane's separator lane takes its hover and drag fill from the `handle_hover` and
//! `handle_drag` parts (#132), so the instance's `part_styles` and the application stylesheet
//! (`ui/styles.rhai`, `EmbeddedScriptView::component_styles` here) can change or remove the
//! default `surface_hover` band. Before, the band was a plain `signal_style` merged after the
//! node's base style, so it always won over the `handle` part.
//!
//! The SplitPane mirrors an application shell separator: `line: false`, an empty `handle`
//! (the grip alone shows hover and drag) and a `handle` part style that paints the lane in
//! the neighbouring pane's colour. Harness copied from pseudo_paint_order.rs.

use std::collections::BTreeMap;

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px, rgba};
use gpui_rhai::{MotionPreference, Rgba8, ScriptViewHandle, ThemeTokenOverrides};

use crate::support::{Mount, Step, painted_quads};

/// `surface_hover`, overridden so the band is unambiguous.
const HOVER: u32 = 0x0000_ffff;
/// The application's colour for the separator lane.
const PANE: u32 = 0x00ff_00ff;
/// A `handle_hover` and a `handle_drag` colour, for the states told apart.
const HOVER_PART: u32 = 0xffff_00ff;
const DRAG_PART: u32 = 0x00ff_ffff;
/// `accent`, overridden: the grip's border in `grip_hover` / `grip_drag`, which shows that
/// the pointer reached the native handle.
const ACCENT: u32 = 0xff00_ffff;
/// The separator lane's thickness.
const LANE: f32 = 8.0;

fn mount(
    cx: &mut TestAppContext,
    main: &str,
    sheet: Option<&str>,
) -> (VisualTestContext, ScriptViewHandle) {
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([
            ("surface_hover".to_owned(), Rgba8::from_rgba_hex(HOVER)),
            ("accent".to_owned(), Rgba8::from_rgba_hex(ACCENT)),
        ]),
        ..Default::default()
    };
    let sheet = sheet.map(str::to_owned);
    let (mut visual, view) = Mount::components(main)
        .motion(MotionPreference::None)
        .with(move |view| {
            let view = view.theme_token_overrides(overrides);
            match sheet {
                Some(sheet) => view.component_styles(sheet),
                None => view,
            }
        })
        .id("split")
        .mount(cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Refresh, 3);
}

/// Logical sizes of painted quads.
type Sizes = Vec<(f32, f32)>;

/// Logical sizes of the quads filled with `color`.
fn filled(visual: &mut VisualTestContext, color: u32) -> Sizes {
    crate::support::filled(visual, color)
        .iter()
        .map(|quad| (quad.bounds.width, quad.bounds.height))
        .collect()
}

/// Quads with a start (left) border painted in `color`.
fn bordered(visual: &mut VisualTestContext, color: u32) -> usize {
    let color: gpui::Hsla = rgba(color).into();
    painted_quads(visual)
        .iter()
        .filter(|quad| quad.border_widths[3] > 0.0 && quad.border_color == color)
        .count()
}

/// Whether a quad covers the whole lane: `LANE` across and more than 100px along it.
fn lane(quads: &[(f32, f32)], horizontal: bool) -> bool {
    quads.iter().any(|&(w, h)| {
        let (across, along) = if horizontal { (w, h) } else { (h, w) };
        (across - LANE).abs() < 0.5 && along > 100.0
    })
}

fn separator(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> (f64, f64) {
    let bounds = crate::support::bounds(visual, view, "separator", "Resize panels");
    (bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.)
}

fn script(orientation: &str, part_styles: &str) -> String {
    format!(
        r#"import "components/split_pane" as split_pane;
fn resized(ctx, value) {{}}
fn view(ctx) {{ column([
    split_pane::SplitPane(#{{ key: "layout", label: "Resize panels", size: 0.5,
        orientation: "{orientation}", min_start: 0.0, min_end: 0.0,
        start: text("Start"), end: text("End"), line: false, handle: box([]),{part_styles}
        on_resize: Fn("resized") }})
        .with_style(style().width(px(420)).height(px(180))),
]) }}"#
    )
}

/// The `handle`, `handle_hover` and `handle_drag` entries of a part style map.
fn parts(hover: u32, drag: u32) -> String {
    format!(
        "handle: style().background(rgba({PANE:#x})), \
         handle_hover: style().background(rgba({hover:#x})), \
         handle_drag: style().background(rgba({drag:#x}))"
    )
}

fn instance(orientation: &str, hover: u32, drag: u32) -> String {
    script(
        orientation,
        &format!(
            "\n        part_styles: #{{ grip: style().border(px(1)), {} }},",
            parts(hover, drag)
        ),
    )
}

fn sheet(hover: u32, drag: u32) -> String {
    format!(
        r#"fn component_styles() {{
    #{{ "components/split_pane": #{{ grip: style().border(px(1)), {} }} }}
}}"#,
        parts(hover, drag)
    )
}

/// Idle the lane has the `handle` colour; hovered it has `hover`, mid-drag `drag`, and no
/// lane-sized quad is painted with `surface_hover` in either state.
fn assert_lane_states(
    cx: &mut TestAppContext,
    main: &str,
    sheet: Option<&str>,
    horizontal: bool,
    hover: u32,
    drag: u32,
) {
    let (mut visual, view) = mount(cx, main, sheet);
    let idle = filled(&mut visual, PANE);
    assert!(
        lane(&idle, horizontal),
        "idle: the handle part style paints the lane: {idle:?}"
    );

    // 40px along the separator from its centre, so the pointer misses the (empty) grip box
    // and lands on the native handle; the drag moves 40px across it.
    let (x, y) = separator(&mut visual, &view);
    let (along, across) = if horizontal {
        (point(px(0.), px(40.)), point(px(40.), px(0.)))
    } else {
        (point(px(40.), px(0.)), point(px(0.), px(40.)))
    };
    let at = point(px(x as f32), px(y as f32)) + along;
    visual.simulate_mouse_move(at, None, Modifiers::none());
    settle(&mut visual);
    let hovered = (filled(&mut visual, HOVER), filled(&mut visual, hover));
    let grip_hover = bordered(&mut visual, ACCENT);

    let to = at + across;
    visual.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
    settle(&mut visual);
    let dragged = (filled(&mut visual, HOVER), filled(&mut visual, drag));
    let (dragged_x, dragged_y) = separator(&mut visual, &view);
    visual.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    settle(&mut visual);

    // Preconditions: the pointer reached the native handle (its state signal put the grip
    // in `grip_hover`) and the drag moved the separator.
    assert!(
        grip_hover > 0,
        "hover did not reach the handle: no grip_hover border"
    );
    let moved = if horizontal {
        dragged_x - x
    } else {
        dragged_y - y
    };
    assert!(
        moved > 20.,
        "the drag did not move the separator: ({x}, {y}) -> ({dragged_x}, {dragged_y})"
    );

    let keeps = |(band, own): &(Sizes, Sizes)| !lane(band, horizontal) && lane(own, horizontal);
    assert!(
        keeps(&hovered) && keeps(&dragged),
        "the lane should take handle_hover / handle_drag and show no surface_hover band\n  \
         hover: surface_hover quads {:?}, handle_hover quads {:?}\n  \
         drag:  surface_hover quads {:?}, handle_drag quads {:?}",
        hovered.0,
        hovered.1,
        dragged.0,
        dragged.1
    );
}

#[gpui::test]
fn instance_part_styles_keep_the_lane_still(cx: &mut TestAppContext) {
    let main = instance("horizontal", PANE, PANE);
    assert_lane_states(cx, &main, None, true, PANE, PANE);
}

#[gpui::test]
fn stylesheet_rules_keep_the_lane_still(cx: &mut TestAppContext) {
    let main = script("horizontal", "");
    assert_lane_states(cx, &main, Some(&sheet(PANE, PANE)), true, PANE, PANE);
}

#[gpui::test]
fn instance_part_styles_restyle_hover_and_drag(cx: &mut TestAppContext) {
    let main = instance("vertical", HOVER_PART, DRAG_PART);
    assert_lane_states(cx, &main, None, false, HOVER_PART, DRAG_PART);
}

#[gpui::test]
fn stylesheet_rules_restyle_hover_and_drag(cx: &mut TestAppContext) {
    let main = script("vertical", "");
    let sheet = sheet(HOVER_PART, DRAG_PART);
    assert_lane_states(cx, &main, Some(&sheet), false, HOVER_PART, DRAG_PART);
}
