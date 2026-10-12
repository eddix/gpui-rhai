//! Automation commands run through the mounted handlers, actions and runtime clock.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::{SingleEmbeddedHost, node_texts, official_icon_assets};
use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{
    EmbeddedScriptSource, EmbeddedScriptView, ModuleId, ScriptViewConfig, ScriptViewHost,
};

#[gpui::test]
fn automation_commands_use_mounted_handlers_actions_and_clock(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let manual = gpui_rhai::ManualRuntimeClock::new(std::time::Instant::now());
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(std::collections::BTreeMap::from([(
            entry,
            r#"
                fn state_schema() { #{ fields: #{
                    count: #{ schema: #{ type: "integer" },
                        "default": #{ type: "integer", value: 0 } }
                } } }
                fn increment(ctx, payload) {
                    ctx.set_state("count", ctx.get_state("count") + 1);
                }
                fn tick(ctx, payload) { increment(ctx, payload); }
                fn fail(ctx, payload) { throw "deliberate automation failure"; }
                fn init(ctx) {
                    ctx.register_action("counter.bump", Fn("increment"));
                    ctx.register_action("counter.fail", Fn("fail"));
                }
                fn view(ctx) {
                    timeout("tick", 100, false, Fn("tick"), ());
                    column([
                        text(`Count: ${ctx.get_state("count")}`),
                        text("Increment").with_key("increment")
                            .test_id("increment")
                            .accessibility_role("button")
                            .accessibility_label("Increment")
                            .on_click(Fn("increment"))
                    ])
                }
            "#
            .to_owned(),
        )])),
        include_str!("../../../../../registry/themes/default_dark.rhai"),
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .runtime_clock(manual.clock())
    .asset_sources(official_icon_assets())
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("automation-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("automation-view"),
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
    let locator = gpui_rhai::AutomationLocator::TestId {
        id: "increment".to_owned(),
    };
    let query = visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Query {
                    locator: locator.clone(),
                },
                window,
                cx,
            )
        })
        .unwrap();
    assert!(matches!(query, gpui_rhai::AutomationResult::Node { .. }));

    visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Dispatch {
                    locator,
                    event: "click".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Action {
                    id: "counter.bump".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::AdvanceTime { millis: 100 },
                window,
                cx,
            )
        })
        .unwrap();
    visual.run_until_parked();

    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut texts = Vec::new();
    node_texts(&root, &mut texts);
    assert!(texts.contains(&"Count: 3".to_owned()), "{texts:?}");
    let error = visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Action {
                    id: "counter.fail".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap_err();
    assert!(error.to_string().contains("deliberate automation failure"));
}
