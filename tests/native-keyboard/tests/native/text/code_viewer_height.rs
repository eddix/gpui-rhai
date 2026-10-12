//! CodeViewer `height: "content"` (#139d): the native viewer measures its display rows at
//! layout time from its current width, so a code sample in a scrolling Region takes
//! exactly its height, wrapped lines included, and `max_height` caps it. The default,
//! `fill`, still fills the viewer's box.

use gpui::{
    Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, TestAppContext, VisualTestContext, point,
    px,
};
use gpui_rhai::{MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::{BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID};

use crate::support::{Mount, Step, last_error, settle, snapshot};

/// The body line height of the default tokens (`typography.body.line_height`); an
/// unstyled `text` part sets CodeViewer's row height from it.
const ROW: f64 = 22.0;

const SCRIPT: &str = r#"
fn state_schema() { #{ fields: #{ hit: #{ schema: #{ type: "string" },
    "default": #{ type: "string", value: "none" } } } } }
fn activated(ctx, payload) { ctx.set_state("hit", `${payload.line}:${payload.column}`); }
fn lines(count) {
    let out = [];
    for index in 1..=count { out.push(`let value_${index} = ${index};`); }
    out.reduce(|sum, line| sum + "\n" + line, "").sub_string(1)
}
fn guide(width, viewer) {
    region::Region(#{ label: "Guide", title: "Guide", scroll: true,
        body: column([viewer, text("after")]) })
        .with_style(style().width(width).height(px(400)))
}
"#;

fn mount(cx: &mut TestAppContext, view_body: &str) -> (VisualTestContext, ScriptViewHandle) {
    let main = format!(
        r#"
import "components/code_viewer" as code_viewer;
import "layouts/region" as region;
{SCRIPT}
fn view(ctx) {{ column([{view_body}, text(`hit:${{ctx.get_state("hit")}}`)]) }}
"#
    );
    let (mut visual, view) = Mount::components(main)
        .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .id("code-viewer-height")
        .mount(cx);
    // The document is prepared, and viewport wrapping rebuilt, in the background.
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

/// Layout bounds (x, y, width, height) of the node with this role and name.
fn bounds(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> (f64, f64, f64, f64) {
    let g = snapshot(visual, view)
        .find_by_role_and_name(role, name)
        .next()
        .unwrap_or_else(|| panic!("no {role} {name:?}"))
        .geometry
        .unwrap()
        .layout;
    (g.x, g.y, g.width, g.height)
}

/// The `line:column` reported by the latest double-click, here at (x, y); a click that
/// misses the text leaves the previous value (`none` at first).
fn hit(visual: &mut VisualTestContext, view: &ScriptViewHandle, x: f64, y: f64) -> String {
    let position = point(px(x as f32), px(y as f32));
    for click_count in 1..=2 {
        visual.simulate_event(MouseDownEvent {
            position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count,
            first_mouse: false,
        });
        visual.simulate_event(MouseUpEvent {
            position,
            modifiers: Modifiers::default(),
            button: MouseButton::Left,
            click_count,
        });
    }
    visual.run_until_parked();
    settle(visual, Step::Frame, 1);
    snapshot(visual, view)
        .nodes()
        .filter(|node| node.role == "text")
        .find_map(|node| node.name.strip_prefix("hit:").map(ToOwned::to_owned))
        .unwrap_or_default()
}

fn assert_near(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < 0.5,
        "{what}: {actual}px, expected {expected}px"
    );
}

#[gpui::test]
fn the_default_still_fills_its_box(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (mut visual, view) = mount(
        cx,
        r#"column([code_viewer::CodeViewer(#{ key: "sample", label: "Sample", source: lines(3) })])
            .with_style(style().width(px(400)).height(px(300)))"#,
    );
    let viewer = bounds(&mut visual, &view, "document", "Sample");
    eprintln!("fill: {viewer:?}");
    assert_near(viewer.3, 300.0, "a fill viewer in a 300px column");
    assert_near(viewer.2, 400.0, "its width");
}

#[gpui::test]
fn a_short_sample_takes_exactly_its_rows(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let (mut visual, view) = mount(
        cx,
        r#"guide(px(480), code_viewer::CodeViewer(#{ key: "sample", label: "Sample",
            source: lines(3), height: "content" }))"#,
    );
    let viewer = bounds(&mut visual, &view, "document", "Sample");
    let after = bounds(&mut visual, &view, "text", "after");
    eprintln!("short: {viewer:?}, after {after:?}");
    assert_near(viewer.3, 3.0 * ROW, "three unwrapped lines");
    assert_near(after.1, viewer.1 + viewer.3, "the next sibling's top");
}

#[gpui::test]
fn a_sample_taller_than_its_scrolling_region_keeps_every_row(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    // 30 rows (660px) in a 400px Region: the Region scrolls, the viewer does not shrink.
    let (mut visual, view) = mount(
        cx,
        r#"guide(px(480), code_viewer::CodeViewer(#{ key: "sample", label: "Sample",
            source: lines(30), height: "content" }))"#,
    );
    let viewer = bounds(&mut visual, &view, "document", "Sample");
    let after = bounds(&mut visual, &view, "text", "after");
    eprintln!("tall: {viewer:?}, after {after:?}");
    assert_near(viewer.3, 30.0 * ROW, "thirty unwrapped lines");
    assert_near(after.1, viewer.1 + viewer.3, "the next sibling's top");
}

#[gpui::test]
fn max_height_caps_a_long_sample(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    for (max, expected) in [
        ("200", 200.0),
        // `metrics.row` is 32px in the default comfortable density.
        (r#"theme_length("metrics.row") * 5"#, 160.0),
    ] {
        let (mut visual, view) = mount(
            cx,
            &format!(
                r#"guide(px(480), code_viewer::CodeViewer(#{{ key: "sample", label: "Sample",
                    source: lines(40), height: "content", max_height: {max},
                    on_location_activate: Fn("activated") }}))"#
            ),
        );
        let viewer = bounds(&mut visual, &view, "document", "Sample");
        let after = bounds(&mut visual, &view, "text", "after");
        eprintln!("max_height {max}: {viewer:?}, after {after:?}");
        assert_near(
            viewer.3,
            expected,
            &format!("40 lines under max_height {max}"),
        );
        assert_near(after.1, viewer.1 + viewer.3, "the next sibling's top");
        // Above the cap the viewer scrolls as before: its last visible row is an early line.
        let last_row = (viewer.3 / ROW).floor();
        let reported = hit(
            &mut visual,
            &view,
            viewer.0 + 46.0,
            viewer.1 + (last_row - 0.5) * ROW,
        );
        assert_eq!(reported, format!("{last_row}:1"), "max_height {max}");
    }
}

#[gpui::test]
fn wrapped_rows_grow_the_height_in_a_narrow_window(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    // One long line, then a short one that is always the last display row.
    let viewer_at = |cx: &mut TestAppContext, width: &str| {
        let (mut visual, view) = mount(
            cx,
            &format!(
                r#"column([guide({width}, code_viewer::CodeViewer(#{{ key: "sample", label: "Sample",
                    source: "let message = \"a long line that wraps when the window is narrow enough\";\nend",
                    height: "content", wrap: "viewport", on_location_activate: Fn("activated") }}))])
                    .with_style(style().width(px(1195)))"#
            ),
        );
        let viewer = bounds(&mut visual, &view, "document", "Sample");
        let after = bounds(&mut visual, &view, "text", "after");
        eprintln!("width {width}: {viewer:?}, after {after:?}");
        assert_near(after.1, viewer.1 + viewer.3, "the next sibling's top");
        let rows = viewer.3 / ROW;
        assert_near(rows.round() * ROW, viewer.3, "a whole number of rows");
        // The first row starts line 1 and the bottom row is all of line 2: nothing is
        // scrolled out of view and nothing is left blank below the last row.
        let top = hit(&mut visual, &view, viewer.0 + 46.0, viewer.1 + ROW / 2.0);
        assert_eq!(top, "1:1", "width {width}: the top row");
        let bottom = hit(
            &mut visual,
            &view,
            viewer.0 + 46.0,
            viewer.1 + viewer.3 - ROW / 2.0,
        );
        assert_eq!(bottom, "2:1", "width {width}: the bottom row");
        rows.round()
    };
    let wide = viewer_at(cx, "px(960)");
    let narrow = viewer_at(cx, "px(300)");
    assert_eq!(wide, 2.0, "unwrapped at 960px");
    assert!(narrow > 3.0, "the long line wraps at 300px: {narrow} rows");
    // 25.008% of 1195px leaves the viewer about 274.85px, which fits 23 columns, while
    // its bounds round to 275px, which fits 24: its rows wrap at the width its height
    // was measured from.
    let fractional = viewer_at(cx, "relative(0.25008)");
    assert_eq!(
        fractional, 5.0,
        "23 columns: four rows of line 1, one of line 2"
    );
}
