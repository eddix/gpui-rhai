//! A fill-height VirtualCollection uses the resolved flex viewport.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{SingleEmbeddedHost, official_icon_assets, wait_for_view_text};
use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{
    EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptViewConfig, ScriptViewHost,
};

#[gpui::test]
fn virtual_collection_fill_height_uses_the_resolved_flex_viewport(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
            entry,
            r#"
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
            "#
            .to_owned(),
        )])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .asset_sources(official_icon_assets())
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("fill-list-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("fill-list-view"),
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
                    locator: gpui_rhai::AutomationLocator::TestId {
                        id: "fill-list".to_owned(),
                    },
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
