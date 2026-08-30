use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui_rhai::{
    ActionId, AsyncCapabilityHandler, CapabilityDescriptor, CapabilityId, CapabilityMethod,
    ComponentInstancePath, ComponentStateSchema, EmbeddedScriptSource, ModuleId,
    RestrictedModuleResolver, RuntimeEngine, ScriptLifecycle, StateField, StoreId, TaskWork,
    UiNodeKind, UiRuntimeState, UiValue, ValueSchema,
};
use semver::{Version, VersionReq};

const COUNTER: &str = r#"
/* gpui-rhai
{
  "id": "components/counter",
  "export": "Counter",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Stateful counter used by the formal-component integration contract.
// Props: stable key.
// State: local count integer. Events: none.
// Example: Counter(#{ key: "primary" })
define_component(#{
    metadata: #{
        id: "components/counter", "export": "Counter", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false }
        },
        state: #{ fields: #{
            count: #{ schema: #{ type: "integer" },
                "default": #{ type: "integer", value: 0 } }
        } },
        events: #{ change: #{ payload: #{ type: "integer" } } },
        slots: #{}, parts: ["root"]
    },
    render: Fn("render_Counter")
});

fn reset(ctx, payload) { ctx.set_state("count", 0); }
fn loaded(ctx, payload) { () }
fn failed(ctx, payload) { () }
fn increment(ctx, payload) {
    let next = ctx.get_state("count") + 1;
    ctx.set_state("count", next);
    ctx.emit("change", next);
    ctx.register_action("counter.reset", Fn("reset"));
    ctx.start_task("app.echo", "echo", "work", Fn("loaded"), Fn("failed"));
}
fn Counter(props) { render_component("components/counter", props) }
fn render_Counter(ctx, props) {
    text(`${ctx.get_state("count")}`).on_click(Fn("increment"))
}
"#;

const APP: &str = r#"
import "components/counter" as counter;
fn state_schema() {
    #{ fields: #{ observed: #{ schema: #{ type: "integer" },
        "default": #{ type: "integer", value: -1 } } } }
}
fn changed(ctx, value) { ctx.set_state("observed", value); }
fn view(ctx) { counter::Counter(#{ key: "primary", on_change: Fn("changed") }) }
"#;

const EFFECT_PROBE: &str = r#"
define_component(#{
    metadata: #{
        id: "components/effect_probe", "export": "EffectProbe", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            dependency: #{ schema: #{ type: "integer" }, required: true, sensitive: false }
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"],
        effects: ["sync"]
    },
    render: Fn("render_EffectProbe")
});
fn start_sync(ctx, dependency) {
    if dependency == 3 { throw "effect start rejected dependency 3"; }
    let count = ctx.get_app_store("effect_audit", "starts");
    ctx.set_app_store("effect_audit", "starts", count + 1);
    ctx.set_app_store("effect_audit", "last_start", dependency);
}
fn cleanup_sync(ctx, dependency) {
    let count = ctx.get_app_store("effect_audit", "cleanups");
    ctx.set_app_store("effect_audit", "cleanups", count + 1);
    ctx.set_app_store("effect_audit", "last_cleanup", dependency);
}
fn EffectProbe(props) { render_component("components/effect_probe", props) }
fn render_EffectProbe(ctx, props) {
    effect("sync", props.dependency, Fn("start_sync"), Fn("cleanup_sync"));
    text(`${props.dependency}`)
}
"#;

const EFFECT_APP: &str = r#"
import "components/effect_probe" as probe;
fn state_schema() { #{ fields: #{
    dependency: #{ schema: #{ type: "integer" },
        "default": #{ type: "integer", value: 1 } },
    visible: #{ schema: #{ type: "bool" },
        "default": #{ type: "bool", value: true } }
} } }
fn change(ctx, payload) { ctx.set_state("dependency", 2); }
fn fail_effect(ctx, payload) { ctx.set_state("dependency", 3); }
fn hide(ctx, payload) { ctx.set_state("visible", false); }
fn view(ctx) {
    if ctx.get_state("visible") {
        probe::EffectProbe(#{ key: "primary", dependency: ctx.get_state("dependency") })
    } else {
        text("hidden")
    }
}
"#;

const SIGNAL_PROBE: &str = r#"
define_component(#{
    metadata: #{
        id: "components/signal_probe", "export": "SignalProbe", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}
    },
    schema: #{
        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"]
    },
    render: Fn("render_SignalProbe")
});
fn advance(ctx, payload) {
    ctx.set_signal("progress", ctx.get_signal("progress") + 0.25);
}
fn SignalProbe(props) { render_component("components/signal_probe", props) }
fn render_SignalProbe(ctx, props) {
    let progress = signal("progress", 0.0);
    text("meter").bind_signal("opacity", progress).on_click(Fn("advance"))
}
"#;

const SIGNAL_APP: &str = r#"
import "components/signal_probe" as probe;
fn state_schema() { #{ fields: #{ visible: #{ schema: #{ type: "bool" },
    "default": #{ type: "bool", value: true } } } } }
fn hide(ctx, payload) { ctx.set_state("visible", false); }
fn view(ctx) {
    if ctx.get_state("visible") {
        probe::SignalProbe(#{ key: "primary" })
    } else {
        text("hidden")
    }
}
"#;

const REF_PROBE: &str = r#"
define_component(#{
    metadata: #{
        id: "components/ref_probe", "export": "RefProbe", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}
    },
    schema: #{
        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"]
    },
    render: Fn("render_RefProbe")
});
fn RefProbe(props) { render_component("components/ref_probe", props) }
fn render_RefProbe(ctx, props) {
    let field = element_ref("field");
    text("field").with_key("field").with_ref(field)
}
"#;

const REF_APP: &str = r#"
import "components/ref_probe" as probe;
fn state_schema() { #{ fields: #{ visible: #{ schema: #{ type: "bool" },
    "default": #{ type: "bool", value: true } } } } }
fn hide(ctx, payload) { ctx.set_state("visible", false); }
fn view(ctx) {
    if ctx.get_state("visible") { probe::RefProbe(#{ key: "primary" }) }
    else { text("hidden") }
}
"#;

fn source() -> EmbeddedScriptSource {
    EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/counter").unwrap(),
        COUNTER.to_owned(),
    )]))
}

struct AsyncEcho;

impl AsyncCapabilityHandler for AsyncEcho {
    fn start(&mut self, _: &str, input: UiValue) -> Result<TaskWork, String> {
        Ok(Box::new(move || Ok(input)))
    }
}

fn runtime_with_async_echo() -> Rc<RefCell<UiRuntimeState>> {
    let mut runtime = UiRuntimeState::new();
    let capability = CapabilityId::parse("app.echo").unwrap();
    runtime
        .capabilities
        .register_async(
            CapabilityDescriptor {
                id: capability.clone(),
                version: Version::new(1, 0, 0),
                methods: BTreeMap::from([(
                    "echo".to_owned(),
                    CapabilityMethod {
                        input: ValueSchema::string(),
                        output: ValueSchema::string(),
                    },
                )]),
            },
            AsyncEcho,
        )
        .unwrap();
    runtime
        .capabilities
        .activate(&BTreeMap::from([(capability, VersionReq::STAR)]))
        .unwrap();
    Rc::new(RefCell::new(runtime))
}

fn effect_audit_runtime() -> Rc<RefCell<UiRuntimeState>> {
    let mut runtime = UiRuntimeState::new();
    let fields = ["starts", "cleanups", "last_start", "last_cleanup"]
        .into_iter()
        .map(|name| {
            (
                name.to_owned(),
                StateField::new(ValueSchema::integer(), UiValue::Integer(0)),
            )
        })
        .collect();
    runtime
        .stores
        .declare(
            StoreId::app("effect_audit"),
            ComponentStateSchema::new(fields).unwrap(),
        )
        .unwrap();
    Rc::new(RefCell::new(runtime))
}

fn effect_audit_values(runtime: &Rc<RefCell<UiRuntimeState>>) -> BTreeMap<String, UiValue> {
    runtime.borrow().stores.inspect()[0]
        .fields
        .iter()
        .map(|(name, value)| (name.clone(), value.value.clone()))
        .collect()
}

fn assert_counter_recipe(engine: &RuntimeEngine, component_path: &ComponentInstancePath) {
    let recipe = engine
        .component_invocations()
        .find(|recipe| recipe.path() == component_path)
        .expect("formal component render records one retained invocation recipe");
    assert_eq!(recipe.component().as_str(), "components/counter");
    assert_eq!(recipe.render_name(), "render_Counter");
    assert!(recipe.has_invocation_context());
    assert!(matches!(
        recipe.props().get("on_change"),
        Some(gpui_rhai::ComponentPropValue::Callback(_))
    ));
}

fn assert_root_text(lifecycle: &ScriptLifecycle, expected: &str) {
    assert!(matches!(
        lifecycle.root().unwrap().kind(),
        UiNodeKind::Text { text } if text == expected
    ));
}

fn script_handler(lifecycle: &ScriptLifecycle, event: &str) -> gpui_rhai::ScriptCallback {
    lifecycle
        .root()
        .unwrap()
        .handler(event)
        .unwrap()
        .as_script()
        .unwrap()
        .clone()
}

fn assert_incremental_counter_timing(engine: &mut RuntimeEngine) {
    assert!(engine.take_timings().iter().any(|timing| {
        matches!(timing.operation, gpui_rhai::ExecutionOperation::Render)
            && timing.source == "components/counter"
    }));
}

#[test]
fn local_state_callback_scope_and_reload_cleanup_are_end_to_end() {
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source()).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/formal_counter.rhai", APP)
        .unwrap();
    let runtime = runtime_with_async_echo();
    let root_path = ComponentInstancePath::root("App", "root");
    let component_path = root_path.child("Counter", "primary");
    let root_schema = engine.root_state_schema(&compiled).unwrap();
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        root_path,
        Some("main".to_owned()),
        BTreeMap::new(),
        &root_schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    assert_counter_recipe(&engine, &component_path);
    assert_eq!(
        runtime
            .borrow()
            .component_state
            .get(&component_path, "count"),
        Some(&UiValue::Integer(0))
    );
    let click = script_handler(&lifecycle, "click");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &click, UiValue::Null)
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_root_text(&lifecycle, "1");
    assert_incremental_counter_timing(&mut engine);
    let batch = runtime.borrow_mut().drain_batch();
    assert_eq!(batch.events.len(), 1);
    for event in batch.events {
        let _ = lifecycle
            .invoke_component_event_transactional(&engine, event)
            .unwrap();
    }
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_root_text(&lifecycle, "1");
    assert_eq!(
        runtime
            .borrow()
            .component_state
            .get(&component_path, "count"),
        Some(&UiValue::Integer(1))
    );
    assert_eq!(
        runtime
            .borrow()
            .component_state
            .get(lifecycle.root_path(), "observed"),
        Some(&UiValue::Integer(1))
    );
    assert_eq!(runtime.borrow().tasks.active_count(), 1);
    let reset = runtime
        .borrow()
        .actions
        .dispatch(&ActionId::parse("counter.reset").unwrap(), UiValue::Null)
        .unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &reset.callback, reset.payload)
        .unwrap();
    assert_eq!(
        runtime
            .borrow()
            .component_state
            .get(&component_path, "count"),
        Some(&UiValue::Integer(0))
    );

    let candidate = engine
        .compile("fn view(ctx) { text(\"removed\") }")
        .unwrap();
    lifecycle
        .reload(&mut engine, candidate, &ComponentStateSchema::default())
        .unwrap();
    assert!(
        runtime
            .borrow()
            .component_state
            .get(&component_path, "count")
            .is_none()
    );
    assert_eq!(runtime.borrow().tasks.active_count(), 0);
    assert!(
        runtime
            .borrow()
            .actions
            .dispatch(&ActionId::parse("counter.reset").unwrap(), UiValue::Null)
            .is_err()
    );
}

#[test]
fn duplicate_stateful_component_keys_are_rejected() {
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source()).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/duplicate_counter.rhai",
            r#"
                import "components/counter" as counter;
                fn view(ctx) {
                    column([
                        counter::Counter(#{ key: "same" }),
                        counter::Counter(#{ key: "same" })
                    ])
                }
            "#,
        )
        .unwrap();
    let context = gpui_rhai::UiContext::new(
        Rc::new(RefCell::new(UiRuntimeState::new())),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        gpui_rhai::ExecutionPhase::Render,
        BTreeMap::new(),
    );
    assert!(engine.render_with_context(&compiled, context).is_err());
}

#[test]
fn callback_props_execute_in_the_caller_state_scope() {
    let button = include_str!("../../../registry/components/button.rhai");
    let module = ModuleId::parse("components/button").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([(module, button.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/caller_scope.rhai",
            r#"
                import "components/button" as button;
                fn state_schema() {
                    #{ fields: #{ count: #{ schema: #{ type: "integer" },
                        "default": #{ type: "integer", value: 0 } } } }
                }
                fn increment(ctx, payload) {
                    ctx.set_state("count", ctx.get_state("count") + 1);
                }
                fn view(ctx) { button::Button(#{ text: "Add", on_click: Fn("increment") }) }
            "#,
        )
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root = ComponentInstancePath::root("App", "root");
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        root.clone(),
        Some("main".to_owned()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    let click = script_handler(&lifecycle, "click");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &click, UiValue::Null)
        .unwrap();
    assert_eq!(
        runtime.borrow().component_state.get(&root, "count"),
        Some(&UiValue::Integer(1))
    );
}

#[test]
fn native_semantic_callback_props_execute_in_the_caller_state_scope() {
    let dropdown = include_str!("../../../registry/components/dropdown.rhai");
    let module = ModuleId::parse("components/dropdown").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([(module, dropdown.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/native_caller_scope.rhai",
            r#"
                import "components/dropdown" as dropdown;
                fn state_schema() {
                    #{ fields: #{ open: #{ schema: #{ type: "bool" },
                        "default": #{ type: "bool", value: true } } } }
                }
                fn set_open(ctx, value) { ctx.set_state("open", value); }
                fn view(ctx) {
                    dropdown::Dropdown(#{
                        key: "theme", options: [#{ value: "dark", label: "Dark" }],
                        open: ctx.get_state("open"), on_open_change: Fn("set_open")
                    })
                }
            "#,
        )
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root = ComponentInstancePath::root("App", "root");
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        root.clone(),
        Some("main".to_owned()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    let open_change = script_handler(&lifecycle, "open_change");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &open_change, UiValue::Bool(false))
        .unwrap();
    assert_eq!(
        runtime.borrow().component_state.get(&root, "open"),
        Some(&UiValue::Bool(false))
    );
}

#[test]
fn declarative_effects_start_restart_and_cleanup_in_imported_module_context() {
    let module = ModuleId::parse("components/effect_probe").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([(module, EFFECT_PROBE.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/effect_probe.rhai", EFFECT_APP)
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = effect_audit_runtime();
    let root = ComponentInstancePath::root("App", "root");
    let mut lifecycle = ScriptLifecycle::new(
        compiled.clone(),
        Rc::clone(&runtime),
        root.clone(),
        Some("main".to_owned()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    assert_eq!(runtime.borrow().effects.len(), 1);

    assert_eq!(effect_audit_values(&runtime)["starts"], UiValue::Integer(1));
    assert_eq!(
        effect_audit_values(&runtime)["cleanups"],
        UiValue::Integer(0)
    );

    let change = engine.callback(&compiled, "change").unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &change, UiValue::Null)
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_eq!(effect_audit_values(&runtime)["starts"], UiValue::Integer(2));
    assert_eq!(
        effect_audit_values(&runtime)["cleanups"],
        UiValue::Integer(1)
    );
    assert_eq!(
        effect_audit_values(&runtime)["last_start"],
        UiValue::Integer(2)
    );
    assert_eq!(
        effect_audit_values(&runtime)["last_cleanup"],
        UiValue::Integer(1)
    );

    let fail_effect = engine.callback(&compiled, "fail_effect").unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &fail_effect, UiValue::Null)
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).is_err());
    assert_root_text(&lifecycle, "2");
    assert!(runtime.borrow().dirty_components().contains(&root));
    assert_eq!(runtime.borrow().effects.len(), 1);
    assert_eq!(effect_audit_values(&runtime)["starts"], UiValue::Integer(2));
    assert_eq!(
        effect_audit_values(&runtime)["cleanups"],
        UiValue::Integer(1)
    );

    let _ = lifecycle
        .invoke_callback_transactional(&engine, &change, UiValue::Null)
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());

    let hide = engine.callback(&compiled, "hide").unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &hide, UiValue::Null)
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert!(runtime.borrow().effects.is_empty());
    assert_eq!(effect_audit_values(&runtime)["starts"], UiValue::Integer(2));
    assert_eq!(
        effect_audit_values(&runtime)["cleanups"],
        UiValue::Integer(2)
    );
    assert_eq!(
        effect_audit_values(&runtime)["last_cleanup"],
        UiValue::Integer(2)
    );
}

#[test]
fn hot_reload_cleans_old_effect_context_before_starting_new_generation() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/effect_probe").unwrap(),
        EFFECT_PROBE.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let active = engine
        .compile_self_contained_named("ui/effect_probe.rhai", EFFECT_APP)
        .unwrap();
    let schema = engine.root_state_schema(&active).unwrap();
    let runtime = effect_audit_runtime();
    let mut lifecycle = ScriptLifecycle::new(
        active,
        Rc::clone(&runtime),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();

    let candidate = engine
        .compile_self_contained_named("ui/effect_probe.rhai", EFFECT_APP)
        .unwrap();
    let candidate_generation = candidate.generation();
    lifecycle.reload(&mut engine, candidate, &schema).unwrap();
    assert_eq!(lifecycle.generation(), candidate_generation);
    assert_eq!(effect_audit_values(&runtime)["starts"], UiValue::Integer(2));
    assert_eq!(
        effect_audit_values(&runtime)["cleanups"],
        UiValue::Integer(1)
    );

    lifecycle.dispose(&mut engine).unwrap();
    assert!(runtime.borrow().effects.is_empty());
    assert_eq!(
        effect_audit_values(&runtime)["cleanups"],
        UiValue::Integer(2)
    );
}

#[test]
fn native_signal_updates_preserve_identity_without_component_invalidation() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/signal_probe").unwrap(),
        SIGNAL_PROBE.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/signal_probe.rhai", SIGNAL_APP)
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root = ComponentInstancePath::root("App", "root");
    let component = root.child("SignalProbe", "primary");
    let mut lifecycle = ScriptLifecycle::new(
        compiled.clone(),
        Rc::clone(&runtime),
        root.clone(),
        Some("main".to_owned()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    assert_eq!(lifecycle.root().unwrap().signal_bindings().len(), 1);
    let signal = runtime
        .borrow()
        .signals
        .resolve(&component, "progress")
        .unwrap();
    let click = script_handler(&lifecycle, "click");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &click, UiValue::Null)
        .unwrap();
    assert_eq!(
        runtime.borrow().signals.read(&signal).unwrap(),
        gpui_rhai::SignalValue::Float(0.25)
    );
    assert!(runtime.borrow().dirty_components().is_empty());

    lifecycle.render(&mut engine).unwrap();
    assert_eq!(
        runtime.borrow().signals.read(&signal).unwrap(),
        gpui_rhai::SignalValue::Float(0.25)
    );

    let hide = engine.callback(&compiled, "hide").unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &hide, UiValue::Null)
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert!(runtime.borrow().signals.read(&signal).is_err());
}

#[test]
fn element_refs_follow_retained_node_identity_and_fail_stale_after_unmount() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/ref_probe").unwrap(),
        REF_PROBE.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/ref_probe.rhai", REF_APP)
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root_path = ComponentInstancePath::root("App", "root");
    let mut lifecycle = ScriptLifecycle::new(
        compiled.clone(),
        Rc::clone(&runtime),
        root_path,
        Some("main".to_owned()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    let reference = lifecycle.root().unwrap().element_ref().unwrap().clone();
    let node_id = runtime.borrow().element_refs.resolve(&reference).unwrap();
    assert_eq!(Some(node_id), lifecycle.retained().root_id());

    lifecycle.render(&mut engine).unwrap();
    assert_eq!(
        runtime.borrow().element_refs.resolve(&reference).unwrap(),
        node_id
    );

    let hide = engine.callback(&compiled, "hide").unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &hide, UiValue::Null)
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert!(runtime.borrow().element_refs.resolve(&reference).is_err());
}
