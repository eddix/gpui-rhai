//! A fill-height VirtualCollection uses the resolved flex viewport.

use crate::support::{Mount, by_test_id, wait_for_view_text};
use gpui::{TestAppContext, VisualTestContext};

#[gpui::test]
fn virtual_collection_fill_height_uses_the_resolved_flex_viewport(cx: &mut TestAppContext) {
    let main = r#"
                fn render_item(ctx, item) {
                    text(item.item.label).with_style(style().height(px(24)))
                }
                fn view(ctx) {
                    let data = [];
                    for index in 0..40 {
                        data.push(#{ key: `row-${index}`, label: `Row ${index}` });
                    }
                    column([
                        text("Header").with_style(style().height(px(40))),
                        virtual_collection(#{
                            key: "fill", label: "Fill list", data: data,
                            estimated_height: 24, fill_height: true,
                            overdraw_pixels: 48, alignment: "top", follow_tail: false
                        }, Fn("render_item"))
                            .test_id("fill-list")
                            .accessibility_role("list")
                            .accessibility_label("Fill list")
                    ]).with_style(style().width(px(320)).height(px(240)))
                }
            "#;
    let (window, view) = Mount::new(main)
        .bundled_assets()
        .ids("fill-list-window", "fill-list-view")
        .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    cx.background_executor
        .advance_clock(std::time::Duration::from_millis(16));
    wait_for_view_text(
        &mut visual,
        &view,
        "Row 8",
        "measured fill viewport should request and realize its visible window",
    );
    let result = visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Query {
                    locator: by_test_id("fill-list"),
                },
                window,
                cx,
            )
        })
        .unwrap();
    let gpui_rhai::AutomationResult::Node { node } = result else {
        panic!("expected virtual collection node");
    };
    let bounds = node.bounds.expect("fill list has committed bounds");
    assert!(
        (190.0..=205.0).contains(&bounds.height),
        "fill viewport should consume 240px parent minus 40px header, got {bounds:?}"
    );
}
