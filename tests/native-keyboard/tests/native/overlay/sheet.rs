//! Sheet uses the viewport's end edge and restores a controlled open state.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{SingleEmbeddedHost, official_icon_assets, palette_texts};
use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{
    EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptViewConfig, ScriptViewHost,
};

#[gpui::test]
fn sheet_uses_the_viewport_end_edge_and_restores_controlled_open_state(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([
            (
                entry,
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
                "#
                .to_owned(),
            ),
            (
                ModuleId::parse("components/sheet").unwrap(),
                include_str!("../../../../../registry/components/sheet.rhai").to_owned(),
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
        let host = ScriptViewHost::new("sheet-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("sheet-view"),
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
    let (panel, viewport_width) = visual.update(|window, cx| {
        let panel = view
            .accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("dialog", "Inspector")
            .next()
            .unwrap()
            .geometry
            .unwrap()
            .visual;
        (panel, f64::from(window.viewport_size().width))
    });
    assert!((panel.width - 280.0).abs() < 1.0);
    assert!((panel.x + panel.width - viewport_width).abs() < 1.0);

    visual.simulate_keystrokes("escape");
    visual.run_until_parked();
    assert!(palette_texts(&mut visual, &view).contains(&"sheet-open:false".to_owned()));
}
