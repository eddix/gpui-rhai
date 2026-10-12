//! Selectable text uses native selection and copy semantics.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{SingleEmbeddedHost, node_texts, official_icon_assets};
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{
    EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptViewConfig, ScriptViewHost,
};

#[gpui::test]
fn selectable_text_uses_native_selection_and_copy_semantics(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
            entry,
            r#"
                fn state_schema() { #{ fields: #{
                    clicks: #{ schema: #{ type: "integer" },
                        "default": #{ type: "integer", value: 0 } }
                } } }
                fn parent_clicked(ctx, payload) {
                    ctx.set_state("clicks", ctx.get_state("clicks") + 1);
                }
                fn view(ctx) {
                    column([
                        text("Copy this 错误\nsecond line").selectable(true)
                            .with_key("err")
                            .test_id("err")
                            .accessibility_role("document")
                            .accessibility_label("error text"),
                        text(`clicks: ${ctx.get_state("clicks")}`)
                    ]).on_click(Fn("parent_clicked"))
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
        let host = ScriptViewHost::new("selectable-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("selectable-view"),
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

    let result = visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Query {
                    locator: gpui_rhai::AutomationLocator::TestId {
                        id: "err".to_owned(),
                    },
                },
                window,
                cx,
            )
        })
        .unwrap();
    let gpui_rhai::AutomationResult::Node { node } = result else {
        panic!("expected node result");
    };
    let bounds = node.bounds.expect("selectable text has committed bounds");

    cx.write_to_clipboard(gpui::ClipboardItem::new_string("sentinel".to_owned()));
    // Selection itself must not mutate the clipboard or trigger the clickable
    // ancestor. Copy remains a separate, focus-routed platform action.
    let start = point(px((bounds.x + 2.0) as f32), px((bounds.y + 2.0) as f32));
    let end = point(
        px((bounds.x + bounds.width - 1.0) as f32),
        px((bounds.y + bounds.height - 1.0) as f32),
    );
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();

    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()),
        Some("sentinel".to_owned())
    );
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut texts = Vec::new();
    node_texts(&root, &mut texts);
    assert!(texts.contains(&"clicks: 0".to_owned()), "{texts:?}");

    cx.simulate_keystrokes(*window, "cmd-c");
    cx.run_until_parked();
    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()),
        Some("Copy this 错误\nsecond line".to_owned())
    );
}
