//! Scripts reach the system services through capabilities when the Host mounts
//! `SystemServices` (feedback #29): a notification posted from a click reaches
//! the platform at the view's next poll, and the user's click on it comes back
//! to the script through the `responses` subscription.

use gpui::{SystemNotificationResponse, TestAppContext, VisualTestContext};
use gpui_rhai::system::SystemServices;
use gpui_rhai::*;

use crate::support::{Mount, Step, by_test_id, dispatch, last_error, settle, text_of};

const SOURCE: &str = r#"
define_component(#{
    metadata: #{ id: "tests/alerts", "export": "Alerts", version: "0.1.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{ "system.notification": "*", "system.badge": "*" } },
    schema: #{
        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{ clicked: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "none" } } } },
        events: #{}, slots: #{}, parts: [], effects: ["responses"],
    },
    render: Fn("render_Alerts"),
});
fn responded(ctx, response) {
    ctx.set_state("clicked", `${response.tag}:${if response.action_id == () { "body" } else { response.action_id }}`);
}
fn failed(ctx, error) { ctx.set_state("clicked", `error ${error}`); }
fn start(ctx, deps) {
    ctx.start_subscription("system.notification", "responses", (), Fn("responded"), Fn("failed"), #{ delivery: "all" });
}
fn stop(ctx, deps) { () }
fn notify(ctx, payload) {
    ctx.call_capability("system.notification", "show", #{ tag: "waiting", title: "A session needs you",
        body: "Approve the command", actions: [#{ id: "approve", label: "Approve" }] });
    ctx.call_capability("system.badge", "set", #{ label: "1" });
}
fn render_Alerts(ctx, props) {
    effect("responses", (), Fn("start"), Fn("stop"));
    column([
        text("Notify").test_id("notify").on_click(Fn("notify")),
        text(`clicked ${ctx.get_state("clicked")}`).test_id("clicked"),
    ])
}
fn view(ctx) { render_component("tests/alerts", #{ key: "alerts" }) }
"#;

#[gpui::test]
fn a_script_notification_reaches_the_platform_and_its_click_comes_back(cx: &mut TestAppContext) {
    // The responses subscription forwards on gpui-rhai's real worker thread.
    cx.executor().allow_parking();
    // GPUI's test platform, like macOS without a bundle, posts nothing without an identity.
    cx.update(|cx| cx.set_app_identity("dev.gpui-rhai.tests", "gpui-rhai tests"));
    let manifest = AppManifest::new(ModuleId::parse("main").unwrap())
        .with_capability("system.notification", "*")
        .unwrap()
        .with_capability("system.badge", "*")
        .unwrap();
    let (mut visual, view): (VisualTestContext, ScriptViewHandle) = Mount::new(SOURCE)
        .with(move |view| view.manifest(manifest).extension(SystemServices::default()))
        .id("alerts")
        .mount(cx);
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(last_error(&mut visual, &view), None);

    dispatch(&mut visual, &view, by_test_id("notify"), "click").unwrap();
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(last_error(&mut visual, &view), None);
    let shown = cx.shown_system_notifications();
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert_eq!(shown[0].tag.as_ref(), "waiting");
    assert_eq!(shown[0].title.as_ref(), "A session needs you");
    assert_eq!(shown[0].actions[0].id.as_ref(), "approve");

    cx.simulate_system_notification_response(SystemNotificationResponse {
        tag: "waiting".into(),
        action_id: Some("approve".into()),
    });
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while text_of(&mut visual, &view, "clicked") != "clicked waiting:approve" {
        assert!(std::time::Instant::now() < deadline, "the click never came back");
        std::thread::sleep(std::time::Duration::from_millis(20));
        settle(&mut visual, Step::Tick, 1);
    }
}
