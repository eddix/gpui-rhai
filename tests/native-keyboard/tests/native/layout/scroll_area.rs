//! ScrollArea's thumb drag updates the retained scroll handle, and its overlay scrollbar adds
//! nothing to the extent a viewport scrolls through.

use crate::support::{Mount, bounds, center};
use gpui::{Modifiers, MouseButton, ScrollDelta, ScrollWheelEvent, TestAppContext, point, px};

#[gpui::test]
fn scroll_area_thumb_drag_updates_the_retained_scroll_handle(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(
        r#"
                    import "components/scroll_area" as scroll_area;
                    fn view(ctx) {
                        let rows = [];
                        for index in 0..12 {
                            rows.push(text(`Row ${index}`).with_key(`${index}`)
                                .with_style(style().height(px(28)).flex_shrink(false).items_center()));
                        }
                        scroll_area::ScrollArea(#{ key: "logs", label: "Logs",
                            width: px(280), height: px(112), vertical_scrollbar: "always",
                            content: column(rows).with_style(style().padding_x(px(8))) })
                    }
                "#,
    )
    .registry("components/scroll_area")
    .bundled_assets()
    .ids("scroll-area-window", "scroll-area-view")
    .mount(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let region = bounds(&mut visual, &view, "region", "Logs");
    let before = bounds(&mut visual, &view, "text", "Row 11");
    let start = point(
        px((region.x + region.width - 6.0) as f32),
        px((region.y + 12.0) as f32),
    );
    let end = point(
        px((region.x + region.width - 6.0) as f32),
        px((region.y + region.height - 12.0) as f32),
    );
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();

    let after = bounds(&mut visual, &view, "text", "Row 11");
    assert!(
        after.y < before.y - 100.0,
        "dragging the themed thumb must move content: before={before:?} after={after:?}"
    );
}

/// Twenty 50px rows (1000px) in a 300px viewport `body` builds from `rows()`, wheeled far past
/// the end. Returns the space between the last row and the viewport's bottom edge.
fn space_below_the_end(cx: &mut TestAppContext, body: &str) -> f64 {
    let main = format!(
        r#"
import "components/scroll_area" as scroll_area;
fn rows() {{
    let rows = [];
    for index in 0..20 {{
        rows.push(text(`Row ${{index}}`).with_key(`${{index}}`)
            .with_style(style().height(px(50)).flex_shrink(false)));
    }}
    column(rows)
}}
fn view(ctx) {{ column([{body}]).with_style(style().width(px(400)).height(px(600))) }}
"#
    );
    let (mut visual, view) = Mount::components(main).id("overscroll").mount(cx);
    cx.run_until_parked();
    let viewport = bounds(&mut visual, &view, "region", "Logs");
    for _ in 0..10 {
        visual.simulate_event(ScrollWheelEvent {
            position: center(viewport),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-500.0))),
            ..ScrollWheelEvent::default()
        });
        visual.run_until_parked();
    }
    let last = bounds(&mut visual, &view, "text", "Row 19");
    (viewport.y + viewport.height) - (last.y + last.height)
}

#[gpui::test]
fn the_overlay_scrollbar_adds_nothing_to_the_scrolled_extent(cx: &mut TestAppContext) {
    // ScrollArea's viewport is a block box. The overlay scrollbar is an absolute child of the
    // viewport; at its static position (after the content) it added a viewport's height, so
    // the end of a scroll showed 300px of nothing.
    let area = space_below_the_end(
        cx,
        r#"scroll_area::ScrollArea(#{ key: "logs", label: "Logs", width: px(400), height: px(300), content: rows() })"#,
    );
    assert!(
        area.abs() < 0.5,
        "ScrollArea stops at its last row: {area}px below it"
    );
    // A flex column with the same overlay was never affected.
    let column = space_below_the_end(
        cx,
        r#"column([rows()]).accessibility_role("region").accessibility_label("Logs")
            .with_style(style().height(px(300)).overflow_y_scroll()).scrollbars("hidden", "auto")"#,
    );
    assert!(
        column.abs() < 0.5,
        "a scrolling column stops at its last row: {column}px below it"
    );
}
