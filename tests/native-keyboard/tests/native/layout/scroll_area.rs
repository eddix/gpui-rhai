//! ScrollArea's thumb drag updates the retained scroll handle.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{SingleEmbeddedHost, official_icon_assets};
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{
    EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptViewConfig, ScriptViewHost,
};

#[gpui::test]
fn scroll_area_thumb_drag_updates_the_retained_scroll_handle(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([
            (
                entry,
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
                "#
                .to_owned(),
            ),
            (
                ModuleId::parse("components/scroll_area").unwrap(),
                include_str!("../../../../../registry/components/scroll_area.rhai").to_owned(),
            ),
        ])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
        .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .asset_sources(official_icon_assets())
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("scroll-area-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("scroll-area-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some(view.clone());
        SingleEmbeddedHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    let (region, before) = visual.update(|_, cx| {
        let snapshot = view.accessibility_snapshot(cx).unwrap();
        let region = snapshot
            .find_by_role_and_name("region", "Logs")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual;
        let row = snapshot
            .find_by_role_and_name("text", "Row 11")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual;
        (region, row)
    });
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

    let after = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("text", "Row 11")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual
    });
    assert!(
        after.y < before.y - 100.0,
        "dragging the themed thumb must move content: before={before:?} after={after:?}"
    );
}
