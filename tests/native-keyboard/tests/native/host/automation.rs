//! Automation commands run through the mounted handlers, actions and runtime clock.

use crate::support::{Mount, by_test_id, dispatch, node_texts};
use gpui::{TestAppContext, VisualTestContext};

#[gpui::test]
fn automation_commands_use_mounted_handlers_actions_and_clock(cx: &mut TestAppContext) {
    let manual = gpui_rhai::ManualRuntimeClock::new(std::time::Instant::now());
    let clock = manual.clock();
    let (window, view) = Mount::new(
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
            "#,
    )
    .bundled_assets()
    .with(move |view| view.runtime_clock(clock))
    .ids("automation-window", "automation-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let mut visual = VisualTestContext::from_window(*window, cx);
    let locator = by_test_id("increment");
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

    dispatch(&mut visual, &view, locator, "click").unwrap();
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
