//! Task and subscription capabilities see what started them (#133), as a sync
//! capability does through `CapabilityHandler::call_with`: an async handler
//! overriding `AsyncCapabilityHandler::start_with` tells a real click from an
//! automation command before any work runs, so it can gate a native save
//! dialog or a package install on user input. A subscription handler
//! overriding `SubscriptionCapabilityHandler::subscribe_with` sees the effect
//! that started it, with the view and the component.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext};
use gpui_rhai::*;

use crate::support::{Mount, Step, bounds, by_role, center, dispatch};

/// What a handler saw: the method, the origin, the view and the component.
#[derive(Clone, Debug, PartialEq)]
struct Call {
    method: String,
    origin: InvocationOrigin,
    view: Option<String>,
    component: String,
}

impl Call {
    fn new(context: &InvocationContext, method: &str) -> Self {
        Self {
            method: method.to_owned(),
            origin: context.origin.clone(),
            view: context.view_id.clone(),
            component: context.component.to_string(),
        }
    }
}

/// What the handlers saw, in order.
#[derive(Clone, Debug, PartialEq)]
enum Seen {
    /// `CapabilityHandler::call_with`, with the label it was called with.
    Sync(String, Call),
    /// `AsyncCapabilityHandler::start_with`, with its input.
    Start(Call, UiValue),
    /// `SubscriptionCapabilityHandler::subscribe_with`.
    Subscribe(Call),
    /// The task's work running on the worker thread.
    Work,
}

type Log = Arc<Mutex<Vec<Seen>>>;

#[derive(Clone)]
struct Gate(Log);

impl CapabilityHandler for Gate {
    fn call(&mut self, _: &str, _: UiValue) -> Result<UiValue, String> {
        Err("call_with is the entry point".into())
    }

    fn call_with(
        &mut self,
        context: &InvocationContext,
        method: &str,
        input: UiValue,
    ) -> Result<UiValue, String> {
        let UiValue::String(label) = input else {
            return Err("check takes a string".into());
        };
        self.0
            .lock()
            .unwrap()
            .push(Seen::Sync(label, Call::new(context, method)));
        Ok(UiValue::Bool(context.origin.is_user_input()))
    }
}

struct Saver(Log);

impl AsyncCapabilityHandler for Saver {
    fn start(&mut self, _: &str, _: UiValue) -> Result<TaskWork, String> {
        Err("start_with is the entry point".into())
    }

    fn start_with(
        &mut self,
        context: &InvocationContext,
        method: &str,
        input: UiValue,
    ) -> Result<TaskWork, String> {
        self.0
            .lock()
            .unwrap()
            .push(Seen::Start(Call::new(context, method), input));
        let log = self.0.clone();
        Ok(TaskWork::new(move || {
            log.lock().unwrap().push(Seen::Work);
            Ok(UiValue::String("saved".into()))
        }))
    }
}

struct Feed(Log);

impl SubscriptionCapabilityHandler for Feed {
    fn subscribe(&mut self, _: &str, _: UiValue) -> Result<SubscriptionWork, String> {
        Err("subscribe_with is the entry point".into())
    }

    fn subscribe_with(
        &mut self,
        context: &InvocationContext,
        method: &str,
        _: UiValue,
    ) -> Result<SubscriptionWork, String> {
        self.0
            .lock()
            .unwrap()
            .push(Seen::Subscribe(Call::new(context, method)));
        Ok(SubscriptionWork::new(|_| {}))
    }
}

#[derive(Clone)]
struct Capabilities(Log);

impl ScriptViewExtension for Capabilities {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        let descriptor = |id: &str, name: &str, input, output| CapabilityDescriptor {
            id: CapabilityId::parse(id).unwrap(),
            version: semver::Version::new(1, 0, 0),
            methods: BTreeMap::from([(name.to_owned(), CapabilityMethod { input, output })]),
        };
        runtime
            .capabilities
            .register(
                descriptor(
                    "app.gate",
                    "check",
                    ValueSchema::string(),
                    ValueSchema::Bool,
                ),
                Gate(self.0.clone()),
            )
            .map_err(|error| error.to_string())?;
        runtime
            .capabilities
            .register_async(
                descriptor(
                    "app.files",
                    "save",
                    ValueSchema::string(),
                    ValueSchema::string(),
                ),
                Saver(self.0.clone()),
            )
            .map_err(|error| error.to_string())?;
        runtime
            .capabilities
            .register_subscription(
                descriptor("app.feed", "watch", ValueSchema::Null, ValueSchema::Null),
                Feed(self.0.clone()),
            )
            .map_err(|error| error.to_string())
    }
}

const MAIN: &str = r#"
define_component(#{ metadata: #{ id: "tests/watcher", "export": "Watcher", version: "0.0.1",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [],
        capabilities: #{ "app.feed": "*" } },
    schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: [], effects: ["feed"] },
    render: Fn("watcher") });
fn start_feed(ctx, deps) {
    ctx.start_subscription("app.feed", "watch", (), Fn("received"), Fn("failed"), #{});
}
fn stop_feed(ctx, deps) {}
fn received(ctx, value) {}
fn watcher(ctx, props) {
    effect("feed", (), Fn("start_feed"), Fn("stop_feed"));
    text("watching")
}
fn clicked(ctx, payload) {
    ctx.call_capability("app.gate", "check", "click");
    ctx.start_task("app.files", "save", "report.csv", Fn("saved"), Fn("failed"));
}
fn saved(ctx, value) { ctx.call_capability("app.gate", "check", "saved"); }
fn failed(ctx, error) {}
fn view(ctx) {
    column([
        render_component("tests/watcher", #{ key: "watcher" }),
        box([text("Save")]).accessibility_role("button").accessibility_label("Save")
            .on_click(Fn("clicked")).with_style(style().width(px(80)).height(px(32))),
    ])
}
"#;

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Frame, 3);
}

/// Settle until the log holds `count` completions.
fn wait_for_completions(visual: &mut VisualTestContext, log: &Log, count: usize) {
    for _ in 0..40 {
        let done = log
            .lock()
            .unwrap()
            .iter()
            .filter(|seen| matches!(seen, Seen::Sync(label, _) if label == "saved"))
            .count();
        if done >= count {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        settle(visual);
    }
    panic!("task {count} did not complete: {:?}", log.lock().unwrap());
}

#[gpui::test]
fn task_and_subscription_starts_see_the_invocation_context(cx: &mut TestAppContext) {
    // Task work runs on a runtime worker thread.
    cx.executor().allow_parking();
    let log = Log::default();
    let manifest = AppManifest::new(ModuleId::parse("main").unwrap())
        .with_capability("app.gate", "*")
        .unwrap()
        .with_capability("app.files", "*")
        .unwrap()
        .with_capability("app.feed", "*")
        .unwrap();
    let capabilities = Capabilities(log.clone());
    let (mut visual, view) = Mount::new(MAIN)
        .theme(r#"fn theme(){#{family:"Origin",name:"Dark",mode:"dark",tokens:#{}}}"#)
        .no_token_base()
        .with(move |view| view.manifest(manifest).extension(capabilities))
        .id("origin")
        .mount(cx);
    settle(&mut visual);

    // The subscription the Watcher's effect started on mount.
    let subscriptions = log
        .lock()
        .unwrap()
        .drain(..)
        .filter_map(|seen| match seen {
            Seen::Subscribe(call) => Some(call),
            _ => None,
        })
        .collect::<Vec<_>>();
    println!("{subscriptions:?}");
    assert_eq!(subscriptions.len(), 1, "{subscriptions:?}");
    assert_eq!(subscriptions[0].method, "watch");
    assert_eq!(subscriptions[0].origin, InvocationOrigin::Effect);
    assert_eq!(subscriptions[0].view.as_deref(), Some("origin"));
    assert!(
        subscriptions[0].component.contains("/Watcher[watcher]"),
        "{subscriptions:?}"
    );

    // 1. A real click.
    let position = center(bounds(&mut visual, &view, "button", "Save"));
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::none());
    settle(&mut visual);
    wait_for_completions(&mut visual, &log, 1);

    // 2. The same callback through automation: not user input.
    dispatch(&mut visual, &view, by_role("button", "Save"), "click").unwrap();
    settle(&mut visual);
    wait_for_completions(&mut visual, &log, 2);

    let log = log.lock().unwrap().clone();
    for seen in &log {
        println!("{seen:?}");
    }
    // The start sees the context the sync call in the same callback sees.
    let Seen::Sync(_, first) = &log[0] else {
        panic!("{log:?}");
    };
    let call = |method: &str, origin: InvocationOrigin| Call {
        method: method.into(),
        origin,
        view: first.view.clone(),
        component: first.component.clone(),
    };
    let click = InvocationOrigin::UserInput {
        event: "click".into(),
    };
    let input = UiValue::String("report.csv".into());
    assert_eq!(first.view.as_deref(), Some("origin"));
    assert_eq!(
        log,
        vec![
            Seen::Sync("click".into(), call("check", click.clone())),
            Seen::Start(call("save", click.clone()), input.clone()),
            Seen::Work,
            Seen::Sync(
                "saved".into(),
                call(
                    "check",
                    InvocationOrigin::TaskCompletion {
                        started_by: Box::new(click),
                    }
                ),
            ),
            // Automation: `start_with` sees the difference before the work runs.
            Seen::Sync("click".into(), call("check", InvocationOrigin::Automation)),
            Seen::Start(call("save", InvocationOrigin::Automation), input),
            Seen::Work,
            Seen::Sync(
                "saved".into(),
                call(
                    "check",
                    InvocationOrigin::TaskCompletion {
                        started_by: Box::new(InvocationOrigin::Automation),
                    }
                ),
            ),
        ]
    );
}
