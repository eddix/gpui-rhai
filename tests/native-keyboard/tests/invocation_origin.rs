//! Capability handlers see what a call responds to (#91): a click, an
//! automation command, the completion of a task a click started, an effect,
//! the lifecycle; with the view and the calling component. An action dispatched
//! or an event emitted from any of them runs with the same origin, although it
//! runs after the callback that queued it has returned.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, point, px,
};
use gpui_rhai::*;

/// (label, origin, view, component) for every `record` call.
type Records = Rc<RefCell<Vec<(String, InvocationOrigin, Option<String>, String)>>>;

#[derive(Clone)]
struct Capture {
    records: Records,
}

impl CapabilityHandler for Capture {
    fn call(&mut self, _: &str, _: UiValue) -> Result<UiValue, String> {
        Err("call_with is the entry point".into())
    }

    fn call_with(
        &mut self,
        context: &InvocationContext,
        _: &str,
        input: UiValue,
    ) -> Result<UiValue, String> {
        let UiValue::String(label) = input else {
            return Err("record takes a string".into());
        };
        self.records.borrow_mut().push((
            label,
            context.origin.clone(),
            context.view_id.clone(),
            context.component.to_string(),
        ));
        Ok(UiValue::Null)
    }
}

struct Echo;

impl AsyncCapabilityHandler for Echo {
    fn start(&mut self, _: &str, input: UiValue) -> Result<TaskWork, String> {
        Ok(TaskWork::new(move || Ok(input)))
    }
}

fn descriptor(id: &str) -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: CapabilityId::parse(id).unwrap(),
        version: semver::Version::new(1, 0, 0),
        methods: BTreeMap::from([(
            "run".into(),
            CapabilityMethod {
                input: ValueSchema::string(),
                output: ValueSchema::Null,
            },
        )]),
    }
}

impl ScriptViewExtension for Capture {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .capabilities
            .register(descriptor("app.capture"), self.clone())
            .map_err(|error| error.to_string())?;
        let mut work = descriptor("app.work");
        work.methods.get_mut("run").unwrap().output = ValueSchema::string();
        runtime
            .capabilities
            .register_async(work, Echo)
            .map_err(|error| error.to_string())
    }
}

const MAIN: &str = r#"
define_component(#{ metadata: #{ id: "tests/probe", "export": "Probe", version: "0.0.1",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [],
        capabilities: #{ "app.capture": "*" } },
    schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            on_ping: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false,
                sensitive: false } },
        state: #{ fields: #{} }, events: #{ ping: #{ payload: #{ type: "null" } } }, slots: #{},
        parts: [], effects: ["probe"] },
    render: Fn("probe") });
fn start_probe(ctx, deps) {
    ctx.call_capability("app.capture", "run", "effect");
    ctx.dispatch_action("app.followup", "effect");
}
fn stop_probe(ctx, deps) {}
fn ticked(ctx, payload) {
    ctx.call_capability("app.capture", "run", "timer");
    ctx.dispatch_action("app.followup", "timer");
    ctx.emit("ping", ());
}
fn probe(ctx, props) {
    effect("probe", (), Fn("start_probe"), Fn("stop_probe"));
    timeout("tick", 5, false, Fn("ticked"), ());
    text("probe")
}
fn init(ctx) {
    ctx.register_action("app.followup", Fn("followup"));
    ctx.call_capability("app.capture", "run", "init");
}
fn followup(ctx, label) { ctx.call_capability("app.capture", "run", `after-${label}`); }
fn pinged(ctx, payload) { ctx.call_capability("app.capture", "run", "emitted"); }
fn clicked(ctx, payload) {
    ctx.call_capability("app.capture", "run", "click");
    ctx.dispatch_action("app.followup", "click");
    ctx.start_task("app.work", "run", "task", Fn("done"), Fn("failed"));
}
fn done(ctx, value) {
    ctx.call_capability("app.capture", "run", "done");
    ctx.dispatch_action("app.followup", "done");
}
fn failed(ctx, error) {}
fn view(ctx) {
    column([
        render_component("tests/probe", #{ key: "probe", on_ping: Fn("pinged") }),
        box([text("Go")]).accessibility_role("button").accessibility_label("Go")
            .on_click(Fn("clicked")).with_style(style().width(px(80)).height(px(32))),
    ])
}
"#;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..3 {
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

#[gpui::test]
fn capability_calls_carry_their_origin(cx: &mut TestAppContext) {
    // Task work runs on a runtime worker thread.
    cx.executor().allow_parking();
    cx.update(gpui_rhai::install);
    let records = Records::default();
    let capture = Capture {
        records: records.clone(),
    };
    let entry = ModuleId::parse("main").unwrap();
    let manifest = AppManifest::new(entry.clone())
        .with_capability("app.capture", "*")
        .unwrap()
        .with_capability("app.work", "*")
        .unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, MAIN.to_owned())])),
        r#"fn theme(){#{family:"Origin",name:"Dark",mode:"dark",tokens:#{}}}"#,
    )
    .manifest(manifest)
    .runtime_clock(ManualRuntimeClock::new(std::time::Instant::now()).clock())
    .extension(capture)
    .prepare()
    .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("origin", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("origin"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);

    // A real click: user input, and the task it starts completes as started by it.
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let go = tree
        .find_by_role_and_name("button", "Go")
        .next()
        .and_then(|node| node.geometry)
        .unwrap()
        .visual;
    #[allow(clippy::cast_possible_truncation)]
    let position = point(
        px((go.x + go.width / 2.0) as f32),
        px((go.y + go.height / 2.0) as f32),
    );
    visual.simulate_mouse_down(position, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(position, MouseButton::Left, Modifiers::none());
    settle(&mut visual);
    for _ in 0..20 {
        if records
            .borrow()
            .iter()
            .any(|(label, ..)| label == "after-done")
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
        settle(&mut visual);
    }

    // A timer the mount scheduled.
    visual
        .update(|window, cx| {
            view.automate(AutomationCommand::AdvanceTime { millis: 10 }, window, cx)
        })
        .unwrap();
    settle(&mut visual);

    // The same callback through automation.
    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::RoleName {
                        role: "button".into(),
                        name: "Go".into(),
                    },
                    event: "click".into(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    settle(&mut visual);

    let records = records.borrow();
    for record in records.iter() {
        println!("{record:?}");
    }
    let find = |label: &str| {
        records
            .iter()
            .filter(|(found, ..)| found == label)
            .map(|(_, origin, view, component)| (origin.clone(), view.clone(), component.clone()))
            .collect::<Vec<_>>()
    };
    let click = InvocationOrigin::UserInput {
        event: "click".into(),
    };
    assert_eq!(find("init")[0].0, InvocationOrigin::Lifecycle);
    assert_eq!(find("effect")[0].0, InvocationOrigin::Effect);
    assert_eq!(
        find("timer")[0].0,
        InvocationOrigin::Timer {
            started_by: Box::new(InvocationOrigin::Lifecycle),
        }
    );
    assert!(
        find("effect")[0].2.contains("/Probe[probe]"),
        "{:?}",
        find("effect")
    );
    let clicks = find("click");
    assert_eq!(clicks.len(), 2, "{clicks:?}");
    assert_eq!(clicks[0].0, click);
    assert_eq!(clicks[0].1.as_deref(), Some("origin"));
    assert_eq!(clicks[1].0, InvocationOrigin::Automation);
    let done = find("done");
    assert_eq!(
        done[0].0,
        InvocationOrigin::TaskCompletion {
            started_by: Box::new(click),
        }
    );
    assert!(done[0].0.is_user_input());
    assert!(!InvocationOrigin::Automation.is_user_input());

    // Follow-ups queued by each of them keep its origin.
    assert_eq!(find("after-click")[0].0, clicks[0].0);
    assert_eq!(find("after-done")[0].0, done[0].0);
    assert_eq!(find("after-timer")[0].0, find("timer")[0].0);
    assert_eq!(find("emitted")[0].0, find("timer")[0].0);
    assert_eq!(find("after-effect")[0].0, InvocationOrigin::Effect);
}
