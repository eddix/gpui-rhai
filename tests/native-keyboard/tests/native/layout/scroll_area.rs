//! ScrollArea's thumb drag updates the retained scroll handle.

use crate::support::{Mount, bounds};
use gpui::{Modifiers, MouseButton, TestAppContext, point, px};

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
