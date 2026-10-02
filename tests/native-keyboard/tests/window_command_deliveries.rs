//! Entry points share enqueue-time ownership; a peer may pump, but not authorize.
use gpui::{
    Context, IntoElement, Modifiers, ParentElement, Render, TestAppContext, VisualTestContext,
    Window, WindowHandle, point, px,
};
use gpui_rhai::*;
use std::{
    collections::BTreeMap,
    sync::mpsc::{Receiver, Sender, channel},
    time::{Duration, Instant},
};

struct Host {
    domain: ScriptViewHost,
    view: Option<ScriptViewHandle>,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.domain.container(
            gpui::div().children(self.view.iter().filter_map(|view| view.element().ok())),
        )
    }
}

struct Echo;
impl AsyncCapabilityHandler for Echo {
    fn start(&mut self, _: &str, input: UiValue) -> Result<TaskWork, String> {
        Ok(TaskWork::new(move || Ok(input)))
    }
}

struct Push(Sender<SubscriptionEmitter>);
impl SubscriptionCapabilityHandler for Push {
    fn subscribe(&mut self, _: &str, _: UiValue) -> Result<SubscriptionWork, String> {
        let ready = self.0.clone();
        Ok(SubscriptionWork::new(move |emitter| {
            // The test uses the public thread-safe emitter to enqueue a complete
            // ordered batch before any foreground pump is allowed to run.
            if ready.send(emitter.clone()).is_err() {
                return;
            }
            while emitter.close_reason().is_none() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }))
    }
}

struct Extension(Sender<SubscriptionEmitter>);
impl ScriptViewExtension for Extension {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        runtime
            .capabilities
            .register_async(descriptor("app.echo", "run", ValueSchema::string()), Echo)
            .map_err(|error| error.to_string())?;
        runtime
            .capabilities
            .register_subscription(
                descriptor("app.push", "watch", ValueSchema::Null),
                Push(self.0.clone()),
            )
            .map_err(|error| error.to_string())
    }
}

fn descriptor(id: &str, method: &str, input: ValueSchema) -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: CapabilityId::parse(id).unwrap(),
        version: semver::Version::new(1, 0, 0),
        methods: BTreeMap::from([(
            method.into(),
            CapabilityMethod {
                input,
                output: ValueSchema::string(),
            },
        )]),
    }
}

const SOURCE: &str = r#"
define_component(#{
    metadata: #{ id: "tests/window-deliveries", "export": "Probe", version: "0.1.0",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 }, dependencies: [],
        capabilities: #{ "app.echo": "*", "app.push": "*" } },
    schema: #{
        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            watch: #{ schema: #{ type: "bool" }, required: true, sensitive: false } },
        state: #{ fields: #{ delivered: #{ schema: #{ type: "integer" },
            "default": #{ type: "integer", value: 0 } } } },
        events: #{}, slots: #{}, parts: [], effects: ["watch"] },
    render: Fn("render_Probe")
});
fn init(ctx) {
    if ctx.window_id() == "domain" {
        ctx.open_window("child", "Child", 400, 300, false);
    }
}
fn start_watch(ctx, dependencies) {
    ctx.start_subscription("app.push", "watch", (), Fn("delivered"),
        Fn("failed"), #{ delivery: "all" });
}
fn cleanup_watch(ctx, dependencies) { () }
fn failed(ctx, value) { throw "independent delivery failure"; }
fn delivered(ctx, value) {
    if value == "fail" { throw "independent delivery failure"; }
    if value == "open" { ctx.open_window("late", "Late", 400, 300, false); }
    if value == "close" { ctx.close_window("child"); }
    if value == "focus" { ctx.focus_window("child"); }
    if value == "self-focus" { ctx.focus_window(ctx.window_id()); }
    ctx.set_state("delivered", ctx.get_state("delivered") + 1);
}
fn focus_me(ctx, value) { ctx.focus_window(ctx.window_id()); }
fn open_late(ctx, value) { ctx.open_window("late", "Late", 400, 300, false); }
fn open_child(ctx, value) { ctx.open_window("child", "Replacement child", 400, 300, false); }
fn focus_child(ctx, value) { ctx.focus_window("child"); }
fn close_child(ctx, value) { ctx.close_window("child"); }
fn task_focus(ctx, value) {
    ctx.start_task("app.echo", "run", "self-focus", Fn("delivered"), Fn("failed"));
}
fn open_focus(ctx, value) {
    ctx.open_window("ephemeral", "Ephemeral", 400, 300, false);
    ctx.focus_window("ephemeral");
}
fn open_close(ctx, value) {
    ctx.open_window("ephemeral", "Ephemeral", 400, 300, false);
    ctx.focus_window("ephemeral");
    ctx.close_window("ephemeral");
}
fn render_Probe(ctx, props) {
    if props.watch { effect("watch", (), Fn("start_watch"), Fn("cleanup_watch")); }
    column([
        text("Focus myself").test_id("focus").accessibility_role("button")
            .with_style(style().width(px(180)).height(px(36))).on_click(Fn("focus_me")),
        text("Open late").test_id("open-late").accessibility_role("button")
            .with_style(style().width(px(180)).height(px(36))).on_click(Fn("open_late")),
        text("Task focus").test_id("task-focus").accessibility_role("button")
            .with_style(style().width(px(180)).height(px(36))).on_click(Fn("task_focus")),
        text("Open focus").test_id("open-focus").accessibility_role("button")
            .with_style(style().width(px(180)).height(px(36))).on_click(Fn("open_focus")),
        text("Open close").test_id("open-close").accessibility_role("button")
            .with_style(style().width(px(180)).height(px(36))).on_click(Fn("open_close")),
        text(`${ctx.get_state("delivered")}`).accessibility_role("status"),
        text("Reopen child").test_id("open-child").accessibility_role("button")
            .with_style(style().width(px(180)).height(px(36))).on_click(Fn("open_child")),
        text("Focus child").test_id("focus-child").accessibility_role("button")
            .with_style(style().width(px(180)).height(px(36))).on_click(Fn("focus_child")),
        text("Close child").test_id("close-child").accessibility_role("button")
            .with_style(style().width(px(180)).height(px(36))).on_click(Fn("close_child"))
    ]).with_style(style().width(px(180)))
}
fn Probe(props) { render_component("tests/window-deliveries", props) }
fn view(ctx) { Probe(#{ key: "probe", watch: ctx.window_id() == "domain" }) }
"#;

fn prepared(source: &str, ready: Sender<SubscriptionEmitter>) -> PreparedScriptView {
    let entry = ModuleId::parse("main").unwrap();
    let manifest = AppManifest::new(entry.clone())
        .with_capability("app.echo", "*")
        .unwrap()
        .with_capability("app.push", "*")
        .unwrap();
    EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, source.into())])),
        include_str!("../../../registry/themes/default_dark.rhai"),
    )
    .manifest(manifest)
    .extension(Extension(ready))
    .runtime_clock(ManualRuntimeClock::new(Instant::now()).clock())
    .prepare()
    .unwrap()
}

fn setup(cx: &mut TestAppContext) -> (WindowHandle<Host>, SubscriptionEmitter) {
    cx.executor().allow_parking();
    cx.update(gpui_rhai::install);
    let (ready, received): (_, Receiver<SubscriptionEmitter>) = channel();
    let prepared = prepared(SOURCE, ready);
    let source = cx.add_window(move |window, cx| {
        let domain = ScriptViewHost::new("domain", cx).unwrap();
        let view = prepared
            .mount_window(ScriptViewConfig::new("owner"), domain.clone(), window, cx)
            .unwrap();
        Host {
            domain,
            view: Some(view),
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    assert_eq!(cx.windows().len(), 2, "init must create an actual child");
    let emitter = received.recv_timeout(Duration::from_secs(2)).unwrap();
    (source, emitter)
}

fn dispatch(view: &ScriptViewHandle, id: &str, window: &mut Window, cx: &mut gpui::App) {
    view.automate(
        AutomationCommand::Dispatch {
            locator: AutomationLocator::TestId { id: id.into() },
            event: "click".into(),
            payload: None,
        },
        window,
        cx,
    )
    .unwrap();
}

fn queue_failed_batch(
    view: &ScriptViewHandle,
    emitter: &SubscriptionEmitter,
    operation: &str,
    fail_first: bool,
    window: &mut Window,
    cx: &mut gpui::App,
) {
    let values = if fail_first {
        ["fail", operation]
    } else {
        [operation, "fail"]
    };
    for value in values {
        emitter.emit(UiValue::String(value.into())).unwrap();
    }
    assert!(
        view.automate(AutomationCommand::AdvanceTime { millis: 0 }, window, cx)
            .is_err(),
        "the independent failure must leave the successful command for a later pump"
    );
    let status = view
        .accessibility_snapshot(cx)
        .unwrap()
        .find_by_role_and_name("status", "1")
        .count();
    assert_eq!(status, 1, "the successful delivery must actually commit");
}

fn advance_peer(cx: &mut TestAppContext) {
    cx.background_executor
        .advance_clock(Duration::from_millis(32));
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
}

fn cleanup(cx: &mut TestAppContext) {
    for window in cx.windows() {
        window
            .update(cx, |_, window, _| window.remove_window())
            .unwrap();
    }
    cx.run_until_parked();
}

#[gpui::test]
fn actual_mouse_and_worker_task_focus_the_qualified_source(cx: &mut TestAppContext) {
    let (source, _) = setup(cx);
    let mut visual = VisualTestContext::from_window(*source, cx);
    visual.deactivate_window();
    // Fixed dimensions describe this fixture's controls. It is real native
    // pointer input, not a successful Automation enqueue standing in for focus.
    visual.simulate_click(point(px(90.0), px(18.0)), Modifiers::none());
    visual.run_until_parked();
    assert!(visual.update(|window, _| window.is_window_active()));
    visual.deactivate_window();
    visual.simulate_click(point(px(90.0), px(90.0)), Modifiers::none());
    let deadline = Instant::now() + Duration::from_secs(2);
    while !visual.update(|window, _| window.is_window_active()) {
        visual.run_until_parked();
        assert!(Instant::now() < deadline, "worker focus was not presented");
        std::thread::yield_now();
    }
    let delivered = source
        .update(cx, |host, _, cx| {
            host.view
                .as_ref()
                .unwrap()
                .accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("status", "1")
                .count()
        })
        .unwrap();
    assert_eq!(delivered, 1, "focus must come from the task's callback");
    cleanup(cx);
}

#[gpui::test]
fn suspended_source_keeps_origin_while_peer_pumps_subscription_commands(cx: &mut TestAppContext) {
    for operation in ["open", "close", "focus"] {
        let (source, emitter) = setup(cx);
        let child = cx.windows().into_iter().find(|w| *w != *source).unwrap();
        let mut visual = VisualTestContext::from_window(child, cx);
        visual.deactivate_window();
        drop(visual);
        source
            .update(cx, |host, window, cx| {
                let owner = host.view.as_ref().unwrap();
                queue_failed_batch(owner, &emitter, operation, false, window, cx);
                owner.suspend(window, cx).unwrap();
            })
            .unwrap();
        advance_peer(cx);
        match operation {
            "open" => assert_eq!(cx.windows().len(), 3),
            "close" => assert!(!cx.windows().contains(&child)),
            "focus" => assert!(
                child
                    .update(cx, |_, window, _| window.is_window_active())
                    .unwrap()
            ),
            _ => unreachable!(),
        }
        cleanup(cx);
    }
}

#[gpui::test]
fn subscription_origin_is_revoked_by_dispose_drop_native_close_and_remount(
    cx: &mut TestAppContext,
) {
    for (operation, ending, fail_first) in [
        ("open", "dispose", false),
        ("close", "drop", true),
        ("focus", "native-close", false),
        ("open", "remount", true),
        ("close", "dispose", false),
        ("focus", "drop", true),
    ] {
        let (source, emitter) = setup(cx);
        let child = cx.windows().into_iter().find(|w| *w != *source).unwrap();
        let mut visual = VisualTestContext::from_window(child, cx);
        visual.deactivate_window();
        drop(visual);
        let retained = source
            .update(cx, |host, window, cx| {
                let old = host.view.take().unwrap();
                queue_failed_batch(&old, &emitter, operation, fail_first, window, cx);
                if ending == "dispose" {
                    old.dispose(cx).unwrap();
                }
                if ending == "native-close" {
                    // The native close lifecycle must revoke an externally
                    // retained Handle as well, not just rely on its Drop.
                    let retained = old.clone();
                    host.view = Some(old);
                    window.remove_window();
                    Some(retained)
                } else {
                    drop(old);
                    if ending == "remount" {
                        let (unused, _) = channel();
                        let replacement =
                            prepared("fn view(ctx) { text(\"replacement\") }", unused)
                                .mount_window(
                                    ScriptViewConfig::new("owner"),
                                    host.domain.clone(),
                                    window,
                                    cx,
                                )
                                .unwrap();
                        host.view = Some(replacement);
                    }
                    cx.notify();
                    None
                }
            })
            .unwrap();
        advance_peer(cx);
        if let Some(retained) = &retained {
            assert_eq!(retained.state(), ScriptViewState::Disposed);
        }
        assert!(cx.windows().contains(&child), "{operation}, {ending}");
        assert_eq!(
            cx.windows().len(),
            if ending == "native-close" { 1 } else { 2 }
        );
        assert!(
            !child
                .update(cx, |_, window, _| window.is_window_active())
                .unwrap(),
            "a stale {operation} from {ending} must not focus the peer"
        );
        cleanup(cx);
    }
}

#[gpui::test]
fn cancelled_open_releases_reservation_for_a_live_peer(cx: &mut TestAppContext) {
    let (source, emitter) = setup(cx);
    let child = cx.windows().into_iter().find(|w| *w != *source).unwrap();
    source
        .update(cx, |host, window, cx| {
            let old = host.view.take().unwrap();
            queue_failed_batch(&old, &emitter, "open", false, window, cx);
            old.dispose(cx).unwrap();
            drop(old);
            cx.notify();
        })
        .unwrap();
    advance_peer(cx);
    assert_eq!(cx.windows().len(), 2, "revoked Open must not materialize");
    let mut visual = VisualTestContext::from_window(child, cx);
    visual.simulate_click(point(px(90.0), px(54.0)), Modifiers::none());
    visual.run_until_parked();
    assert_eq!(
        cx.windows().len(),
        3,
        "the peer must actually reopen the same late ID"
    );
    cleanup(cx);
}

#[gpui::test]
fn live_source_cannot_redirect_stale_focus_or_close_to_a_same_name_target(cx: &mut TestAppContext) {
    for operation in ["focus", "close"] {
        let (source, emitter) = setup(cx);
        let old = cx.windows().into_iter().find(|w| *w != *source).unwrap();
        let mut visual = VisualTestContext::from_window(old, cx);
        visual.deactivate_window();
        drop(visual);
        source
            .update(cx, |host, window, cx| {
                let owner = host.view.as_ref().unwrap();
                queue_failed_batch(owner, &emitter, operation, false, window, cx);
                assert_eq!(owner.state(), ScriptViewState::Active);
                // No executor yield, frame clock advance, or second poll may
                // drain the stale command between enqueue and target closure.
                assert!(cx.windows().contains(&old));
                assert!(
                    !old.update(cx, |_, window, _| window.is_window_active())
                        .unwrap()
                );
                old.update(cx, |_, window, _| window.remove_window())
                    .unwrap();
                assert!(!cx.windows().contains(&old));
            })
            .unwrap();
        // Flush the native close observer without advancing a frame timer.
        // It must release the target registration, not revoke the live source.
        cx.run_until_parked();
        source
            .update(cx, |host, window, cx| {
                assert_eq!(host.view.as_ref().unwrap().state(), ScriptViewState::Active);
                dispatch(host.view.as_ref().unwrap(), "open-child", window, cx);
            })
            .unwrap();
        cx.run_until_parked();
        let replacement = cx.windows().into_iter().find(|w| *w != *source).unwrap();
        assert_ne!(old.window_id(), replacement.window_id());
        let mut visual = VisualTestContext::from_window(replacement, cx);
        visual.deactivate_window();
        drop(visual);
        advance_peer(cx);
        assert!(
            cx.windows().contains(&replacement),
            "old Close cannot retarget child"
        );
        assert!(
            !replacement
                .update(cx, |_, window, _| window.is_window_active())
                .unwrap(),
            "old Focus cannot retarget child"
        );
        // Positive control: the same still-live source can operate on the new
        // registration. Dropping all commands is not a valid implementation.
        source
            .update(cx, |host, window, cx| {
                assert_eq!(host.view.as_ref().unwrap().state(), ScriptViewState::Active);
                dispatch(
                    host.view.as_ref().unwrap(),
                    if operation == "focus" {
                        "focus-child"
                    } else {
                        "close-child"
                    },
                    window,
                    cx,
                );
            })
            .unwrap();
        cx.run_until_parked();
        if operation == "focus" {
            assert!(
                replacement
                    .update(cx, |_, window, _| window.is_window_active())
                    .unwrap()
            );
        } else {
            assert!(!cx.windows().contains(&replacement));
            assert!(cx.windows().contains(&*source));
        }
        cleanup(cx);
    }
}

#[gpui::test]
fn same_batch_open_focus_and_open_focus_close_keep_the_pending_target_identity(
    cx: &mut TestAppContext,
) {
    for close in [false, true] {
        let (source, _) = setup(cx);
        let child = cx.windows().into_iter().find(|w| *w != *source).unwrap();
        source
            .update(cx, |host, window, cx| {
                dispatch(
                    host.view.as_ref().unwrap(),
                    if close { "open-close" } else { "open-focus" },
                    window,
                    cx,
                );
                assert_eq!(
                    cx.windows().len(),
                    3,
                    "Open must create its actual native target before defer"
                );
            })
            .unwrap();
        cx.run_until_parked();
        assert_eq!(cx.windows().len(), if close { 2 } else { 3 });
        if !close {
            let ephemeral = cx
                .windows()
                .into_iter()
                .find(|w| *w != *source && *w != child)
                .unwrap();
            assert!(
                ephemeral
                    .update(cx, |_, window, _| window.is_window_active())
                    .unwrap()
            );
        }
        cleanup(cx);
    }
}
