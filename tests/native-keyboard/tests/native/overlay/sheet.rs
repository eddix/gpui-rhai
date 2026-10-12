//! Sheet uses the viewport's end edge and restores a controlled open state.

use crate::support::{Mount, bounds, palette_texts};
use gpui::{TestAppContext, VisualTestContext};

#[gpui::test]
fn sheet_uses_the_viewport_end_edge_and_restores_controlled_open_state(cx: &mut TestAppContext) {
    let (window, view) = Mount::new(
        r#"
                    import "components/sheet" as sheet;
                    fn state_schema() { #{ fields: #{
                        open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } }
                    } } }
                    fn opened(ctx, open) { ctx.set_state("open", open); }
                    fn view(ctx) {
                        column([
                            text(`sheet-open:${ctx.get_state("open")}`),
                            sheet::Sheet(#{ key: "inspector", open: ctx.get_state("open"),
                                side: "end", size: 280, title: "Inspector",
                                content: text("Sheet content"), on_open_change: Fn("opened") })
                        ])
                    }
        "#,
    )
    .registry("components/sheet")
    .bundled_assets()
    .ids("sheet-window", "sheet-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    let panel = bounds(&mut visual, &view, "dialog", "Inspector");
    let viewport_width = visual.update(|window, _| f64::from(window.viewport_size().width));
    assert!((panel.width - 280.0).abs() < 1.0);
    assert!((panel.x + panel.width - viewport_width).abs() < 1.0);

    visual.simulate_keystrokes("escape");
    visual.run_until_parked();
    assert!(palette_texts(&mut visual, &view).contains(&"sheet-open:false".to_owned()));
}
