//! Async workers, effects and subscriptions deliver into a mounted view without input or a
//! manual poll, including across effect restarts.

use crate::support::{Mount, by_test_id, dispatch, node_texts, wait_for_view_text};
use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{ModuleId, UiRuntimeState, UiValue};

#[gpui::test]
fn async_workers_wake_the_view_without_input_or_manual_poll(cx: &mut TestAppContext) {
    // This test deliberately coordinates GPUI with real OS worker threads.
    // gpui-pre's deterministic scheduler requires that boundary to be explicit.
    cx.executor().allow_parking();
    // This intentionally does not advance either the GPUI timer or the Rhai
    // runtime clock. Worker completion itself must schedule the owning entity.
    use gpui_rhai::{
        AsyncCapabilityHandler, CapabilityDescriptor, CapabilityId, CapabilityMethod,
        ScriptViewExtension, SubscriptionCapabilityHandler, SubscriptionWork, TaskWork,
        ValueSchema,
    };

    struct DelayedText;
    impl AsyncCapabilityHandler for DelayedText {
        fn start(&mut self, _method: &str, input: UiValue) -> Result<TaskWork, String> {
            let UiValue::String(value) = input else {
                return Err("load expects a string".to_owned());
            };
            Ok(TaskWork::new(move || {
                Ok(UiValue::String(value.to_uppercase()))
            }))
        }
    }

    struct Ticker {
        gate: std::sync::Arc<(std::sync::Mutex<u8>, std::sync::Condvar)>,
    }
    impl SubscriptionCapabilityHandler for Ticker {
        fn subscribe(&mut self, _m: &str, _input: UiValue) -> Result<SubscriptionWork, String> {
            let gate = std::sync::Arc::clone(&self.gate);
            Ok(SubscriptionWork::new(move |emitter| {
                for tick in 1..=2 {
                    let (lock, ready) = &*gate;
                    let mut released = lock.lock().unwrap();
                    while *released < tick {
                        released = ready.wait(released).unwrap();
                    }
                    drop(released);
                    if emitter.emit(UiValue::Integer(i64::from(tick))).is_err() {
                        break;
                    }
                }
            }))
        }
    }

    struct AsyncExtension {
        gate: std::sync::Arc<(std::sync::Mutex<u8>, std::sync::Condvar)>,
    }
    impl ScriptViewExtension for AsyncExtension {
        fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
            let delayed = CapabilityId::parse("app.delayed_text").map_err(|e| e.to_string())?;
            runtime
                .capabilities
                .register_async(
                    CapabilityDescriptor {
                        id: delayed,
                        version: semver::Version::new(1, 0, 0),
                        methods: std::collections::BTreeMap::from([(
                            "load".to_owned(),
                            CapabilityMethod {
                                input: ValueSchema::string(),
                                output: ValueSchema::string(),
                            },
                        )]),
                    },
                    DelayedText,
                )
                .map_err(|e| e.to_string())?;
            let ticker = CapabilityId::parse("app.ticker").map_err(|e| e.to_string())?;
            runtime
                .capabilities
                .register_subscription(
                    CapabilityDescriptor {
                        id: ticker,
                        version: semver::Version::new(1, 0, 0),
                        methods: std::collections::BTreeMap::from([(
                            "watch".to_owned(),
                            CapabilityMethod {
                                input: ValueSchema::Null,
                                output: ValueSchema::integer(),
                            },
                        )]),
                    },
                    Ticker {
                        gate: std::sync::Arc::clone(&self.gate),
                    },
                )
                .map_err(|e| e.to_string())
        }
    }

    let gate = std::sync::Arc::new((std::sync::Mutex::new(0), std::sync::Condvar::new()));
    let manifest = gpui_rhai::AppManifest::new(ModuleId::parse("main").unwrap())
        .with_capability("app.delayed_text", "*")
        .unwrap()
        .with_capability("app.ticker", "*")
        .unwrap();
    let extension = AsyncExtension {
        gate: std::sync::Arc::clone(&gate),
    };
    let (window, view) = Mount::new(
        r#"
                import "helpers/state" as state;
                define_component(#{
                    metadata: #{ id: "tests/ticker", "export": "Ticker", version: "0.1.0",
                        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
                        dependencies: [], capabilities: #{ "app.ticker": "*" } },
                    schema: #{
                        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
                        state: #{ fields: #{ tick: #{ schema: #{ type: "integer" },
                            "default": #{ type: "integer", value: 0 } } } },
                        events: #{}, slots: #{}, parts: [], effects: ["watch"],
                    },
                    render: Fn("render_Ticker"),
                });
                fn state_schema() { #{ fields: #{
                    message: #{ schema: #{ type: "string" },
                        "default": #{ type: "string", value: "waiting" } },
                } } }
                fn loaded(ctx, value) { state::set_message(ctx, value); }
                fn failed(ctx, error) { state::set_message(ctx, `error: ${error}`); }
                fn clicked(ctx, value) { state::set_message(ctx, value); }
                fn ticked(ctx, value) { ctx.set_state("tick", value); }
                fn tick_failed(ctx, error) { ctx.set_state("tick", -1); }
                fn start_ticker(ctx, deps) {
                    ctx.start_subscription("app.ticker", "watch", (),
                        Fn("ticked"), Fn("tick_failed"), #{ delivery: "all" });
                }
                fn stop_ticker(ctx, deps) { () }
                fn render_Ticker(ctx, props) {
                    effect("watch", (), Fn("start_ticker"), Fn("stop_ticker"));
                    text(`tick: ${ctx.get_state("tick")}`)
                }
                fn Ticker() { render_component("tests/ticker", #{ key: "ticker" }) }
                fn init(ctx) {
                    ctx.start_task("app.delayed_text", "load", "background ready",
                        Fn("loaded"), Fn("failed"));
                }
                fn view(ctx) {
                    column([
                        text(`message: ${ctx.get_state("message")}`)
                            .test_id("message")
                            .on_click_value(Fn("clicked"), "clicked through import"),
                        Ticker()
                    ])
                }
                "#,
    )
    .module(
        "helpers/state",
        r#"
                fn set_message(ctx, value) { ctx.set_state("message", value); }
                fn set_tick(ctx, value) { ctx.set_state("tick", value); }
                "#,
    )
    .bundled_assets()
    .with(move |view| view.manifest(manifest).extension(extension))
    .ids("async-delivery-window", "async-delivery-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let mut visual = VisualTestContext::from_window(*window, cx);

    let release_tick = |tick: u8| {
        let (lock, ready) = &*gate;
        *lock.lock().unwrap() = tick;
        ready.notify_all();
    };
    let wait_for = |visual: &mut VisualTestContext, expected: [&str; 2]| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            visual.run_until_parked();
            let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
            let mut texts = Vec::new();
            node_texts(&root, &mut texts);
            if expected
                .iter()
                .all(|expected| texts.iter().any(|text| text == expected))
            {
                return;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "worker completion did not wake the view; expected {expected:?}, got {texts:?}"
            );
            std::thread::yield_now();
        }
    };

    release_tick(1);
    wait_for(&mut visual, ["message: BACKGROUND READY", "tick: 1"]);
    release_tick(2);
    wait_for(&mut visual, ["message: BACKGROUND READY", "tick: 2"]);

    dispatch(&mut visual, &view, by_test_id("message"), "click").unwrap();
    visual.run_until_parked();
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut texts = Vec::new();
    node_texts(&root, &mut texts);
    assert!(
        texts.contains(&"message: clicked through import".to_owned()),
        "entry-module event callback lost its import context: {texts:?}"
    );
}

#[gpui::test]
fn effect_restart_still_delivers_async_task_results(cx: &mut TestAppContext) {
    // The capability runs on gpui-rhai's real worker pool by contract.
    cx.executor().allow_parking();
    // Companion to the async delivery guard above: the first effect
    // activation delivers fine, but a *replacement* activation (dependency
    // change -> cleanup + start) must also deliver its task results.
    // Downstream (omb, 2026-08-31) observed replacement-activation tasks
    // silently never calling back.
    use gpui_rhai::{
        AsyncCapabilityHandler, CapabilityDescriptor, CapabilityId, CapabilityMethod,
        ScriptViewExtension, TaskWork, ValueSchema,
    };

    struct Echo;
    impl AsyncCapabilityHandler for Echo {
        fn start(&mut self, _m: &str, input: UiValue) -> Result<TaskWork, String> {
            Ok(TaskWork::new(move || Ok(input)))
        }
    }

    struct EchoExtension;
    impl ScriptViewExtension for EchoExtension {
        fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
            runtime
                .capabilities
                .register_async(
                    CapabilityDescriptor {
                        id: CapabilityId::parse("app.echo").map_err(|e| e.to_string())?,
                        version: semver::Version::new(1, 0, 0),
                        methods: std::collections::BTreeMap::from([(
                            "run".to_owned(),
                            CapabilityMethod {
                                input: ValueSchema::string(),
                                output: ValueSchema::string(),
                            },
                        )]),
                    },
                    Echo,
                )
                .map_err(|e| e.to_string())
        }
    }

    let manifest = gpui_rhai::AppManifest::new(ModuleId::parse("main").unwrap())
        .with_capability("app.echo", "*")
        .unwrap();
    let extension = EchoExtension;
    let (window, view) = Mount::new(
        r#"
                import "widgets/loader" as loader;
                fn state_schema() { #{ fields: #{
                    dep: #{ schema: #{ type: "string" },
                        "default": #{ type: "string", value: "first" } } } } }
                fn bump(ctx, payload) { ctx.set_state("dep", "second"); }
                fn view(ctx) {
                    column([
                        loader::Loader(#{ key: "loader", dep: ctx.get_state("dep") }),
                        text("Bump").with_key("bump").test_id("bump").on_click(Fn("bump"))
                    ])
                }
                "#,
    )
    .module(
        "widgets/loader",
        r#"
                define_component(#{
                    metadata: #{ id: "widgets/loader", "export": "Loader", version: "0.1.0",
                        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
                        dependencies: [], capabilities: #{ "app.echo": "*" } },
                    schema: #{
                        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                                  dep: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
                        state: #{ fields: #{
                            got: #{ schema: #{ type: "string" },
                                "default": #{ type: "string", value: "none" } } } },
                        events: #{}, slots: #{}, parts: [], effects: ["load"],
                    },
                    render: Fn("render_Loader"),
                });
                fn Loader(props) { render_component("widgets/loader", props) }
                fn start_load(ctx, deps) {
                    ctx.start_task("app.echo", "run", `echo:${deps.dep}`,
                        Fn("loaded"), Fn("failed"));
                }
                fn cleanup_load(ctx, deps) { () }
                fn loaded(ctx, value) { ctx.set_state("got", value); }
                fn failed(ctx, error) { ctx.set_state("got", `err:${error}`); }
                fn render_Loader(ctx, props) {
                    effect("load", #{ dep: props.dep }, Fn("start_load"), Fn("cleanup_load"));
                    text(`got ${ctx.get_state("got")}`)
                }
                "#,
    )
    .bundled_assets()
    .with(move |view| view.manifest(manifest).extension(extension))
    .ids("effect-restart-window", "effect-restart-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let mut visual = VisualTestContext::from_window(*window, cx);

    wait_for_view_text(
        &mut visual,
        &view,
        "got echo:first",
        "first activation should deliver",
    );

    // Dependency change restarts the effect; the replacement activation's
    // task must deliver too.
    dispatch(&mut visual, &view, by_test_id("bump"), "click").unwrap();
    wait_for_view_text(
        &mut visual,
        &view,
        "got echo:second",
        "replacement activation should deliver",
    );
}

#[gpui::test]
fn effect_start_state_write_restarts_sibling_effect_and_delivers(cx: &mut TestAppContext) {
    // The capability runs on gpui-rhai's real worker pool by contract.
    cx.executor().allow_parking();
    // Exact downstream shape (omb detail widget): effect A's start callback
    // writes component state; that state is effect B's dependency, so B
    // restarts; B's replacement activation starts a task. The task result
    // must deliver.
    use gpui_rhai::{
        AsyncCapabilityHandler, CapabilityDescriptor, CapabilityId, CapabilityMethod,
        ScriptViewExtension, TaskWork, ValueSchema,
    };

    struct Echo;
    impl AsyncCapabilityHandler for Echo {
        fn start(&mut self, _m: &str, input: UiValue) -> Result<TaskWork, String> {
            Ok(TaskWork::new(move || Ok(input)))
        }
    }
    struct EchoExtension;
    impl ScriptViewExtension for EchoExtension {
        fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
            runtime
                .capabilities
                .register_async(
                    CapabilityDescriptor {
                        id: CapabilityId::parse("app.echo").map_err(|e| e.to_string())?,
                        version: semver::Version::new(1, 0, 0),
                        methods: std::collections::BTreeMap::from([(
                            "run".to_owned(),
                            CapabilityMethod {
                                input: ValueSchema::string(),
                                output: ValueSchema::string(),
                            },
                        )]),
                    },
                    Echo,
                )
                .map_err(|e| e.to_string())
        }
    }

    let manifest = gpui_rhai::AppManifest::new(ModuleId::parse("main").unwrap())
        .with_capability("app.echo", "*")
        .unwrap();
    let extension = EchoExtension;
    let (window, view) = Mount::new(
        r#"
                import "widgets/nested" as nested;
                fn view(ctx) { nested::Nested(#{ key: "nested" }) }
                "#,
    )
    .module(
        "widgets/nested",
        r#"
                define_component(#{
                    metadata: #{ id: "widgets/nested", "export": "Nested", version: "0.1.0",
                        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
                        dependencies: [], capabilities: #{ "app.echo": "*" } },
                    schema: #{
                        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
                        state: #{ fields: #{
                            id: #{ schema: #{ type: "string" },
                                "default": #{ type: "string", value: "" } },
                            got: #{ schema: #{ type: "string" },
                                "default": #{ type: "string", value: "none" } } } },
                        events: #{}, slots: #{}, parts: [], effects: ["watch", "load"],
                    },
                    render: Fn("render_Nested"),
                });
                fn Nested(props) { render_component("widgets/nested", props) }
                fn start_watch(ctx, deps) {
                    // Downstream this is "read the shared store on startup";
                    // the write is what restarts the sibling load effect.
                    ctx.set_state("id", "selected");
                }
                fn cleanup_watch(ctx, deps) { () }
                fn start_load(ctx, deps) {
                    if deps.id == "" { return; }
                    ctx.start_task("app.echo", "run", `echo:${deps.id}`,
                        Fn("loaded"), Fn("failed"));
                }
                fn cleanup_load(ctx, deps) { () }
                fn loaded(ctx, value) { ctx.set_state("got", value); }
                fn failed(ctx, error) { ctx.set_state("got", `err:${error}`); }
                fn render_Nested(ctx, props) {
                    effect("watch", (), Fn("start_watch"), Fn("cleanup_watch"));
                    effect("load", #{ id: ctx.get_state("id") },
                        Fn("start_load"), Fn("cleanup_load"));
                    text(`got ${ctx.get_state("got")}`)
                }
                "#,
    )
    .bundled_assets()
    .with(move |view| view.manifest(manifest).extension(extension))
    .ids("nested-effect-window", "nested-effect-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let mut visual = VisualTestContext::from_window(*window, cx);
    wait_for_view_text(
        &mut visual,
        &view,
        "got echo:selected",
        "task started by a sibling-restarted effect must deliver",
    );
}

#[gpui::test]
fn subscription_callback_state_write_restarts_effect_and_delivers(cx: &mut TestAppContext) {
    // The subscription and restarted task intentionally use real worker threads.
    cx.executor().allow_parking();
    // Third companion: the sibling-effect restart is triggered from a
    // *subscription* callback (downstream: a store-watch push), not a click
    // or an effect start. The restarted effect's task must still deliver.
    use gpui_rhai::{
        AsyncCapabilityHandler, CapabilityDescriptor, CapabilityId, CapabilityMethod,
        ScriptViewExtension, SubscriptionCapabilityHandler, SubscriptionWork, TaskWork,
        ValueSchema,
    };

    struct Echo;
    impl AsyncCapabilityHandler for Echo {
        fn start(&mut self, _m: &str, input: UiValue) -> Result<TaskWork, String> {
            Ok(TaskWork::new(move || Ok(input)))
        }
    }
    struct Push;
    impl SubscriptionCapabilityHandler for Push {
        fn subscribe(&mut self, _m: &str, _input: UiValue) -> Result<SubscriptionWork, String> {
            Ok(SubscriptionWork::new(|emitter| {
                let _ = emitter.emit(UiValue::String("selected".to_owned()));
            }))
        }
    }
    struct Extension;
    impl ScriptViewExtension for Extension {
        fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
            runtime
                .capabilities
                .register_async(
                    CapabilityDescriptor {
                        id: CapabilityId::parse("app.echo").map_err(|e| e.to_string())?,
                        version: semver::Version::new(1, 0, 0),
                        methods: std::collections::BTreeMap::from([(
                            "run".to_owned(),
                            CapabilityMethod {
                                input: ValueSchema::string(),
                                output: ValueSchema::string(),
                            },
                        )]),
                    },
                    Echo,
                )
                .map_err(|e| e.to_string())?;
            runtime
                .capabilities
                .register_subscription(
                    CapabilityDescriptor {
                        id: CapabilityId::parse("app.push").map_err(|e| e.to_string())?,
                        version: semver::Version::new(1, 0, 0),
                        methods: std::collections::BTreeMap::from([(
                            "watch".to_owned(),
                            CapabilityMethod {
                                input: ValueSchema::Null,
                                output: ValueSchema::string(),
                            },
                        )]),
                    },
                    Push,
                )
                .map_err(|e| e.to_string())
        }
    }

    let manifest = gpui_rhai::AppManifest::new(ModuleId::parse("main").unwrap())
        .with_capability("app.echo", "*")
        .unwrap()
        .with_capability("app.push", "*")
        .unwrap();
    let extension = Extension;
    let (window, view) = Mount::new(
        r#"
                import "widgets/watcher" as watcher;
                fn view(ctx) { watcher::Watcher(#{ key: "watcher" }) }
                "#,
    )
    .module(
        "widgets/watcher",
        r#"
                define_component(#{
                    metadata: #{ id: "widgets/watcher", "export": "Watcher", version: "0.1.0",
                        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
                        dependencies: [], capabilities: #{ "app.echo": "*", "app.push": "*" } },
                    schema: #{
                        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
                        state: #{ fields: #{
                            id: #{ schema: #{ type: "string" },
                                "default": #{ type: "string", value: "" } },
                            got: #{ schema: #{ type: "string" },
                                "default": #{ type: "string", value: "none" } } } },
                        events: #{}, slots: #{}, parts: [], effects: ["watch", "load"],
                    },
                    render: Fn("render_Watcher"),
                });
                fn Watcher(props) { render_component("widgets/watcher", props) }
                fn start_watch(ctx, deps) {
                    ctx.start_subscription("app.push", "watch", (),
                        Fn("pushed"), Fn("push_failed"), #{ delivery: "all" });
                }
                fn cleanup_watch(ctx, deps) { () }
                fn pushed(ctx, value) { ctx.set_state("id", value); }
                fn push_failed(ctx, error) { ctx.set_state("got", `suberr:${error}`); }
                fn start_load(ctx, deps) {
                    if deps.id == "" { return; }
                    ctx.start_task("app.echo", "run", `echo:${deps.id}`,
                        Fn("loaded"), Fn("failed"));
                }
                fn cleanup_load(ctx, deps) { () }
                fn loaded(ctx, value) { ctx.set_state("got", value); }
                fn failed(ctx, error) { ctx.set_state("got", `err:${error}`); }
                fn render_Watcher(ctx, props) {
                    effect("watch", (), Fn("start_watch"), Fn("cleanup_watch"));
                    effect("load", #{ id: ctx.get_state("id") },
                        Fn("start_load"), Fn("cleanup_load"));
                    text(`got ${ctx.get_state("got")}`)
                }
                "#,
    )
    .bundled_assets()
    .with(move |view| view.manifest(manifest).extension(extension))
    .ids("sub-restart-window", "sub-restart-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let mut visual = VisualTestContext::from_window(*window, cx);
    wait_for_view_text(
        &mut visual,
        &view,
        "got echo:selected",
        "task from a subscription-triggered effect restart must deliver",
    );
}
