use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use gpui_rhai::{
    ActionId, AsyncCapabilityHandler, CalendarClock, CapabilityDescriptor, CapabilityId,
    CapabilityMethod, ComponentInstancePath, ComponentStateSchema, EmbeddedScriptSource,
    ExecutionOperation, GregorianDate, ModuleId, RestrictedModuleResolver, RuntimeEngine,
    ScriptLifecycle, StateField, StoreId, TaskWork, UiNodeKind, UiRuntimeState, UiValue,
    ValueSchema,
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
        dependencies: [], capabilities: #{ "app.echo": "^1" }
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
    ctx.start_task(
        "app.echo", "echo", `${dependency}`,
        Fn("effect_loaded"), Fn("effect_failed")
    );
}
fn effect_loaded(ctx, value) { () }
fn effect_failed(ctx, error) { () }
fn cleanup_sync(ctx, dependency) {
    let count = ctx.get_app_store("effect_audit", "cleanups");
    ctx.set_app_store("effect_audit", "cleanups", count + 1);
    ctx.set_app_store("effect_audit", "last_cleanup", dependency);
}
fn EffectProbe(props) { render_component("components/effect_probe", props) }
fn render_EffectProbe(ctx, props) {
    effect("sync", props.dependency, Fn("start_sync"), Fn("cleanup_sync"));
    timeout("probe", 1000, false, Fn("timeout_fired"), ());
    text(`${props.dependency}`)
}
fn timeout_fired(ctx, payload) { () }
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

const EFFECT_REUSE_APP: &str = r#"
import "components/effect_probe" as probe;
fn state_schema() { #{ fields: #{
    revision: #{ schema: #{ type: "integer" },
        "default": #{ type: "integer", value: 0 } }
} } }
fn view(ctx) {
    column([
        text(`revision:${ctx.get_state("revision")}`),
        probe::EffectProbe(#{ key: "primary", dependency: 1 })
    ])
}
"#;

const EQUIVALENCE_COUNTER: &str = r#"
define_component(#{
    metadata: #{
        id: "components/equivalence_counter", "export": "EquivalenceCounter", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            step: #{ schema: #{ type: "integer" }, required: true, sensitive: false }
        },
        state: #{ fields: #{ count: #{ schema: #{ type: "integer" },
            "default": #{ type: "integer", value: 0 } } } },
        events: #{}, slots: #{}, parts: ["root"]
    },
    render: Fn("render_EquivalenceCounter")
});
fn increment_equivalence(step, ctx, payload) {
    ctx.set_state("count", ctx.get_state("count") + step);
}
fn EquivalenceCounter(props) {
    render_component("components/equivalence_counter", props)
}
fn render_EquivalenceCounter(ctx, props) {
    text(`${props.label}:${ctx.get_state("count")}`)
        .with_key(props.key)
        .on_click(Fn("increment_equivalence").curry(props.step))
}
"#;

const EQUIVALENCE_APP: &str = r#"
import "components/equivalence_counter" as counter;
fn view(ctx) {
    row([
        counter::EquivalenceCounter(#{ key: "left", label: "L", step: 1 }),
        counter::EquivalenceCounter(#{ key: "right", label: "R", step: 3 })
    ])
}
"#;

const COUNTER_PANEL: &str = r#"
import "components/equivalence_counter" as counter;
define_component(#{
    metadata: #{
        id: "components/counter_panel", "export": "CounterPanel", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/equivalence_counter"], capabilities: #{}
    },
    schema: #{
        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"]
    },
    render: Fn("render_CounterPanel")
});
fn CounterPanel(props) { render_component("components/counter_panel", props) }
fn render_CounterPanel(ctx, props) {
    row([
        counter::EquivalenceCounter(#{ key: "left", label: "L", step: 1 }),
        counter::EquivalenceCounter(#{ key: "right", label: "R", step: 3 })
    ])
}
"#;

const ROOT_DIRTY_APP: &str = r#"
import "components/counter_panel" as panel;
fn state_schema() { #{ fields: #{
    revision: #{ schema: #{ type: "integer" },
        "default": #{ type: "integer", value: 0 } }
} } }
fn view(ctx) {
    column([
        text(`revision:${ctx.get_state("revision")}`),
        panel::CounterPanel(#{ key: "panel" })
    ])
}
"#;

const STORE_READER: &str = r#"
define_component(#{
    metadata: #{
        id: "components/store_reader", "export": "StoreReader", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}
    },
    schema: #{
        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"]
    },
    render: Fn("render_StoreReader")
});
fn StoreReader(props) { render_component("components/store_reader", props) }
fn render_StoreReader(ctx, props) { text(ctx.get_app_store("model", "label")) }
"#;

const STORE_REUSE_APP: &str = r#"
import "components/store_reader" as reader;
fn state_schema() { #{ fields: #{
    revision: #{ schema: #{ type: "integer" },
        "default": #{ type: "integer", value: 0 } }
} } }
fn change_model(ctx, payload) { ctx.set_app_store("model", "label", "beta"); }
fn view(ctx) {
    column([
        text(`revision:${ctx.get_state("revision")}`),
        reader::StoreReader(#{ key: "reader" })
    ])
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
fn state_schema() { #{ fields: #{
    visible: #{ schema: #{ type: "bool" },
        "default": #{ type: "bool", value: true } },
    revision: #{ schema: #{ type: "integer" },
        "default": #{ type: "integer", value: 0 } }
} } }
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
fn state_schema() { #{ fields: #{
    visible: #{ schema: #{ type: "bool" },
        "default": #{ type: "bool", value: true } },
    revision: #{ schema: #{ type: "integer" },
        "default": #{ type: "integer", value: 0 } }
} } }
fn hide(ctx, payload) { ctx.set_state("visible", false); }
fn view(ctx) {
    if ctx.get_state("visible") { probe::RefProbe(#{ key: "primary" }) }
    else { text("hidden") }
}
"#;

const NODE_PROP_STATEFUL: &str = r#"
define_component(#{
    metadata: #{ id: "components/node_prop_stateful", "export": "NodePropStateful",
        version: "0.1.0", runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{} },
    schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{ value: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "initial" } } } },
        events: #{}, slots: #{}, parts: ["root"] },
    render: Fn("render_NodePropStateful"),
});
fn NodePropStateful(props) { render_component("components/node_prop_stateful", props) }
fn render_NodePropStateful(ctx, props) { text(ctx.get_state("value")) }
"#;

const NODE_PROP_RECEIVER: &str = r#"
define_component(#{
    metadata: #{ id: "components/node_prop_receiver", "export": "NodePropReceiver",
        version: "0.1.0", runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{} },
    schema: #{ props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
        },
        state: #{ fields: #{
            revision: #{ schema: #{ type: "integer" },
                "default": #{ type: "integer", value: 0 } },
            fail: #{ schema: #{ type: "bool" },
                "default": #{ type: "bool", value: false } },
            click_count: #{ schema: #{ type: "integer" },
                "default": #{ type: "integer", value: 0 } },
        } },
        events: #{}, slots: #{}, parts: ["root"] },
    render: Fn("render_NodePropReceiver"),
});
fn NodePropReceiver(props) { render_component("components/node_prop_receiver", props) }
fn receiver_content_clicked(ctx, payload) {
    ctx.set_state("click_count", ctx.get_state("click_count") + 1);
}
fn render_NodePropReceiver(ctx, props) {
    if ctx.get_state("fail") { throw "receiver rejected replay"; }
    let content = props.content.on_click(Fn("receiver_content_clicked"));
    column([content, text(`receiver:${ctx.get_state("revision")}`)])
}
"#;

const NODE_PROP_APP: &str = r#"
import "components/node_prop_stateful" as stateful;
import "components/node_prop_receiver" as receiver;
fn state_schema() { #{ fields: #{ revision: #{ schema: #{ type: "integer" },
    "default": #{ type: "integer", value: 0 } } } } }
fn view(ctx) {
    let revision = ctx.get_state("revision");
    receiver::NodePropReceiver(#{ key: "receiver",
        content: stateful::NodePropStateful(#{ key: "stateful" }) })
}
"#;

const NODE_PROP_EFFECT_APP: &str = r#"
import "components/effect_probe" as probe;
import "components/node_prop_receiver" as receiver;
fn state_schema() { #{ fields: #{ visible: #{ schema: #{ type: "bool" },
    "default": #{ type: "bool", value: true } } } } }
fn view(ctx) {
    receiver::NodePropReceiver(#{ key: "receiver",
        content: if ctx.get_state("visible") {
            probe::EffectProbe(#{ key: "effect", dependency: 1 })
        } else { text("removed") } })
}
"#;

fn source() -> EmbeddedScriptSource {
    EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/counter").unwrap(),
        COUNTER.to_owned(),
    )]))
}

fn equivalence_engine_and_compiled() -> (RuntimeEngine, gpui_rhai::CompiledUi) {
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/equivalence_counter").unwrap(),
        EQUIVALENCE_COUNTER.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/equivalence.rhai", EQUIVALENCE_APP)
        .unwrap();
    (engine, compiled)
}

struct RootDirtyFixture {
    engine: RuntimeEngine,
    runtime: Rc<RefCell<UiRuntimeState>>,
    lifecycle: ScriptLifecycle,
    root: ComponentInstancePath,
    panel: ComponentInstancePath,
    left: ComponentInstancePath,
    right: ComponentInstancePath,
}

fn root_dirty_fixture() -> RootDirtyFixture {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/equivalence_counter").unwrap(),
            EQUIVALENCE_COUNTER.to_owned(),
        ),
        (
            ModuleId::parse("components/counter_panel").unwrap(),
            COUNTER_PANEL.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/root_dirty.rhai", ROOT_DIRTY_APP)
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root = ComponentInstancePath::root("App", "root");
    let panel = root.child("CounterPanel", "panel");
    let left = panel.child("EquivalenceCounter", "left");
    let right = panel.child("EquivalenceCounter", "right");
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
    RootDirtyFixture {
        engine,
        runtime,
        lifecycle,
        root,
        panel,
        left,
        right,
    }
}

struct AsyncEcho;

impl AsyncCapabilityHandler for AsyncEcho {
    fn start(&mut self, _: &str, input: UiValue) -> Result<TaskWork, String> {
        Ok(TaskWork::new(move || Ok(input)))
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

fn normalized_root(lifecycle: &ScriptLifecycle) -> String {
    regex::Regex::new(r"generation: ScriptGeneration\(\d+\)")
        .unwrap()
        .replace_all(
            &format!("{:?}", lifecycle.root()),
            "generation: <runtime-local>",
        )
        .into_owned()
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

fn child_script_handler(lifecycle: &ScriptLifecycle, index: usize) -> gpui_rhai::ScriptCallback {
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("equivalence root must be a Box");
    };
    children[index]
        .handler("click")
        .unwrap()
        .as_script()
        .unwrap()
        .clone()
}

fn panel_counter_handler(lifecycle: &ScriptLifecycle, index: usize) -> gpui_rhai::ScriptCallback {
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("root-dirty test root must be a Box");
    };
    let UiNodeKind::Box { children } = children[1].kind() else {
        panic!("counter panel must be a Box");
    };
    children[index]
        .handler("click")
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
fn composed_semantic_callback_props_execute_in_the_caller_state_scope() {
    let combobox = include_str!("../../../registry/components/combobox.rhai");
    let input = include_str!("../../../registry/components/input.rhai");
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/combobox").unwrap(),
            combobox.to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            input.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/native_caller_scope.rhai",
            r#"
                import "components/combobox" as combobox;
                fn state_schema() {
                    #{ fields: #{ open: #{ schema: #{ type: "bool" },
                        "default": #{ type: "bool", value: true } } } }
                }
                fn set_open(ctx, value) { ctx.set_state("open", value); }
                fn view(ctx) {
                    combobox::Combobox(#{
                        key: "theme", options: [#{ value: "dark", label: "Dark" }],
                        selected: [], open: ctx.get_state("open"), query: "",
                        on_open_change: Fn("set_open")
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
    let events = runtime.borrow_mut().drain_batch().events;
    for event in events {
        let _ = lifecycle
            .invoke_component_event_transactional(&engine, event)
            .unwrap();
    }
    assert_eq!(
        runtime.borrow().component_state.get(&root, "open"),
        Some(&UiValue::Bool(false))
    );
}

fn first_click_handler(node: &gpui_rhai::UiNode) -> Option<gpui_rhai::ScriptCallback> {
    node.handler("click")
        .and_then(gpui_rhai::UiEventHandler::as_script)
        .cloned()
        .or_else(|| match node.kind() {
            UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
                children.iter().find_map(first_click_handler)
            }
            _ => None,
        })
}

#[test]
fn raw_node_slots_keep_the_callers_callback_provenance() {
    let title_bar = include_str!("../../../registry/components/title_bar.rhai");
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/title_bar").unwrap(),
        title_bar.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/titlebar_slot_scope.rhai",
            r#"
                import "components/title_bar" as title_bar;
                fn state_schema() { #{ fields: #{ menu_open: #{ schema: #{ type: "bool" },
                    "default": #{ type: "bool", value: false } } } } }
                fn toggle_menu(ctx, payload) { ctx.set_state("menu_open", true); }
                fn view(ctx) {
                    let menu = text("Menu").on_click(Fn("toggle_menu"));
                    title_bar::TitleBar(#{
                        label: "Chrome", title: "Workspace", end: [menu],
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

    let handler = first_click_handler(lifecycle.root().unwrap())
        .expect("raw TitleBar slot must retain its click callback");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &handler, UiValue::Null)
        .unwrap();
    assert_eq!(
        runtime.borrow().component_state.get(&root, "menu_open"),
        Some(&UiValue::Bool(true))
    );
}

#[test]
fn nested_callback_scope_is_never_inferred_from_a_private_function_name() {
    let inner = r#"
        define_component(#{
            metadata: #{ id: "components/inner_collision", "export": "InnerCollision",
                version: "0.1.0", runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
                dependencies: [], capabilities: #{} },
            schema: #{ props: #{ on_change: #{ schema: #{ type: "callback" },
                    required: true, sensitive: false } },
                state: #{ fields: #{} }, events: #{ change: #{ payload: #{ type: "integer" } } },
                slots: #{}, parts: ["root"] },
            render: Fn("render_InnerCollision")
        });
        fn InnerCollision(props) { render_component("components/inner_collision", props) }
        fn collide(ctx, value) { ctx.emit("change", value + 100); }
        fn render_InnerCollision(ctx, props) { text("inner").on_click_value(Fn("collide"), 1) }
    "#;
    let outer = r#"
        import "components/inner_collision" as inner;
        define_component(#{
            metadata: #{ id: "components/outer_collision", "export": "OuterCollision",
                version: "0.1.0", runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
                dependencies: ["components/inner_collision"], capabilities: #{} },
            schema: #{ props: #{ on_change: #{ schema: #{ type: "callback" },
                    required: true, sensitive: false } },
                state: #{ fields: #{} }, events: #{ change: #{ payload: #{ type: "integer" } } },
                slots: #{}, parts: ["root"] },
            render: Fn("render_OuterCollision")
        });
        fn OuterCollision(props) { render_component("components/outer_collision", props) }
        fn collide(ctx, value) { ctx.emit("change", value + 10); }
        fn render_OuterCollision(ctx, props) {
            inner::InnerCollision(#{ on_change: Fn("collide") })
        }
    "#;
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/inner_collision").unwrap(),
            inner.to_owned(),
        ),
        (
            ModuleId::parse("components/outer_collision").unwrap(),
            outer.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/callback_name_collision.rhai",
            r#"
                import "components/outer_collision" as outer;
                fn state_schema() { #{ fields: #{ observed: #{ schema: #{ type: "integer" },
                    "default": #{ type: "integer", value: 0 } } } } }
                fn collide(offset, ctx, value) { ctx.set_state("observed", value + offset); }
                fn view(ctx) { outer::OuterCollision(#{ on_change: Fn("collide").curry(1000) }) }
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
        .invoke_callback_transactional(&engine, &click, UiValue::Integer(1))
        .unwrap();
    loop {
        let events = runtime.borrow_mut().drain_batch().events;
        if events.is_empty() {
            break;
        }
        for event in events {
            let _ = lifecycle
                .invoke_component_event_transactional(&engine, event)
                .unwrap();
        }
    }
    assert_eq!(
        runtime.borrow().component_state.get(&root, "observed"),
        Some(&UiValue::Integer(1111))
    );
}

#[test]
fn dirty_transparent_child_promotes_to_the_nearest_replaceable_component() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/select").unwrap(),
            include_str!("../../../registry/components/select.rhai").to_owned(),
        ),
        (
            ModuleId::parse("components/combobox").unwrap(),
            include_str!("../../../registry/components/combobox.rhai").to_owned(),
        ),
        (
            ModuleId::parse("components/input").unwrap(),
            include_str!("../../../registry/components/input.rhai").to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named(
            "ui/transparent_select.rhai",
            r#"
                import "components/select" as select;
                fn view(ctx) {
                    select::Select(#{
                        key: "region",
                        options: [#{ value: "cn", label: "China" }],
                        value: (), open: true, query: "",
                        empty_text: "No regions",
                    })
                }
            "#,
        )
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root = ComponentInstancePath::root("App", "root");
    let select = root.child("Select", "region");
    let combobox = select.child("Combobox", "region-combobox");
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        root,
        Some("main".to_owned()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();

    assert_eq!(lifecycle.root().unwrap().component_root(), Some(&select));
    assert!(
        engine
            .component_invocations()
            .any(|recipe| recipe.path() == &combobox)
    );
    let navigate = script_handler(&lifecycle, "key:down");
    let payload = lifecycle
        .root()
        .unwrap()
        .handler_payload("key:down")
        .cloned()
        .unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &navigate, payload)
        .unwrap();
    assert!(runtime.borrow().dirty_components().contains(&combobox));

    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert!(runtime.borrow().dirty_components().is_empty());
    assert_eq!(lifecycle.root().unwrap().component_root(), Some(&select));
    let UiNodeKind::Overlay { spec, .. } = lifecycle.root().unwrap().kind() else {
        panic!("Select must continue to render its Combobox overlay root");
    };
    assert!(spec.open);
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
    assert_eq!(runtime.borrow().tasks.active_count(), 1);
    assert_eq!(runtime.borrow().timers.active_count(), 1);

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
    assert_eq!(runtime.borrow().tasks.active_count(), 1);
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
    assert_eq!(runtime.borrow().tasks.active_count(), 1);
    assert_eq!(runtime.borrow().timers.active_count(), 1);
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
    assert_eq!(runtime.borrow().tasks.active_count(), 0);
    assert_eq!(runtime.borrow().timers.active_count(), 0);
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
fn suspension_cleans_effects_and_resume_restarts_fresh_activations() {
    let module = ModuleId::parse("components/effect_probe").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([(module, EFFECT_PROBE.to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/effect_suspend.rhai", EFFECT_APP)
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = effect_audit_runtime();
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        ComponentInstancePath::root("App", "root"),
        Some("main".to_owned()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    assert_eq!(runtime.borrow().effects.len(), 1);
    assert_eq!(runtime.borrow().tasks.active_count(), 1);

    assert!(lifecycle.suspend(&mut engine).unwrap());
    assert_eq!(
        effect_audit_values(&runtime)["cleanups"],
        UiValue::Integer(1)
    );
    assert!(runtime.borrow().effects.is_empty());
    assert_eq!(runtime.borrow().tasks.active_count(), 0);
    assert!(
        runtime
            .borrow()
            .timers
            .inspect(runtime.borrow().clock.now())[0]
            .view_paused
    );

    assert!(lifecycle.resume(&mut engine).unwrap());
    assert_eq!(effect_audit_values(&runtime)["starts"], UiValue::Integer(2));
    assert_eq!(runtime.borrow().effects.len(), 1);
    assert_eq!(runtime.borrow().tasks.active_count(), 1);
    assert!(
        !runtime
            .borrow()
            .timers
            .inspect(runtime.borrow().clock.now())[0]
            .interaction_paused
    );
}

#[test]
fn incremental_component_renders_match_forced_full_renders_over_event_sequences() {
    let (mut incremental_engine, incremental_compiled) = equivalence_engine_and_compiled();
    let (mut full_engine, full_compiled) = equivalence_engine_and_compiled();
    let incremental_runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let full_runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root = ComponentInstancePath::root("App", "root");
    let mut incremental = ScriptLifecycle::new(
        incremental_compiled,
        Rc::clone(&incremental_runtime),
        root.clone(),
        Some("main".to_owned()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    let mut full = ScriptLifecycle::new(
        full_compiled,
        Rc::clone(&full_runtime),
        root.clone(),
        Some("main".to_owned()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    incremental.start(&mut incremental_engine).unwrap();
    full.start(&mut full_engine).unwrap();
    assert_eq!(normalized_root(&incremental), normalized_root(&full));
    let _ = incremental_engine.take_timings();
    let _ = full_engine.take_timings();

    let mut random = 0x9e37_79b9_7f4a_7c15_u64;
    for batch in 0..128 {
        random = random
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let event_count = usize::try_from(random % 3 + 1).unwrap();
        let mut selected = BTreeSet::new();
        for _ in 0..event_count {
            random = random
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let index = usize::try_from(random & 1).unwrap();
            selected.insert(index);
            let incremental_callback = child_script_handler(&incremental, index);
            let full_callback = child_script_handler(&full, index);
            let _ = incremental
                .invoke_callback_transactional(
                    &incremental_engine,
                    &incremental_callback,
                    UiValue::Null,
                )
                .unwrap();
            let _ = full
                .invoke_callback_transactional(&full_engine, &full_callback, UiValue::Null)
                .unwrap();
        }

        assert!(incremental.render_dirty(&mut incremental_engine).unwrap());
        let incremental_renders = incremental_engine
            .take_timings()
            .into_iter()
            .filter(|timing| matches!(timing.operation, gpui_rhai::ExecutionOperation::Render))
            .collect::<Vec<_>>();
        assert_eq!(incremental_renders.len(), selected.len());
        assert!(
            incremental_renders
                .iter()
                .all(|timing| timing.source == "components/equivalence_counter")
        );
        assert!(!full_runtime.borrow_mut().drain_batch().dirty.is_empty());
        full.render(&mut full_engine).unwrap();
        let _ = full_engine.take_timings();

        assert_eq!(
            normalized_root(&incremental),
            normalized_root(&full),
            "node snapshots diverged after event batch {batch}"
        );
        assert_eq!(
            incremental_runtime.borrow().component_state.inspect(),
            full_runtime.borrow().component_state.inspect(),
            "component state diverged after event batch {batch}"
        );
        assert_eq!(
            incremental_engine
                .component_invocations()
                .map(|recipe| recipe.path().clone())
                .collect::<Vec<_>>(),
            full_engine
                .component_invocations()
                .map(|recipe| recipe.path().clone())
                .collect::<Vec<_>>(),
            "invocation identities diverged after event batch {batch}"
        );
    }
}

#[test]
fn root_dirty_render_reuses_unchanged_components_and_respects_dirty_descendants() {
    let RootDirtyFixture {
        mut engine,
        runtime,
        mut lifecycle,
        root,
        panel,
        left,
        right,
    } = root_dirty_fixture();
    let retained_ids = [panel.clone(), left.clone(), right.clone()]
        .map(|path| lifecycle.retained().component_node(&path).unwrap());
    let _ = engine.take_timings();

    assert!(
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "revision", UiValue::Integer(1))
            .unwrap()
    );
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    let reuse = engine
        .take_timings()
        .into_iter()
        .filter_map(|timing| match timing.operation {
            gpui_rhai::ExecutionOperation::ComponentReuse(components) => {
                Some((timing.source, components))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(reuse, [("components/counter_panel".to_owned(), 3)]);
    assert_eq!(
        retained_ids,
        [panel.clone(), left.clone(), right.clone()]
            .map(|path| lifecycle.retained().component_node(&path).unwrap())
    );
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("root-dirty test root must be a Box");
    };
    assert!(matches!(&children[0].kind(), UiNodeKind::Text { text } if text == "revision:1"));

    let increment_left = panel_counter_handler(&lifecycle, 0);
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &increment_left, UiValue::Null)
        .unwrap();
    assert!(
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "revision", UiValue::Integer(2))
            .unwrap()
    );
    assert!(runtime.borrow().dirty_components().contains(&left));
    assert!(runtime.borrow().dirty_components().contains(&root));
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    let reuse = engine
        .take_timings()
        .into_iter()
        .filter_map(|timing| match timing.operation {
            gpui_rhai::ExecutionOperation::ComponentReuse(components) => {
                Some((timing.source, components))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(reuse, [("components/equivalence_counter".to_owned(), 1)]);
    assert_eq!(
        runtime.borrow().component_state.get(&left, "count"),
        Some(&UiValue::Integer(1))
    );
    assert_eq!(
        runtime.borrow().component_state.get(&right, "count"),
        Some(&UiValue::Integer(0))
    );
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("root-dirty test root must be a Box");
    };
    let UiNodeKind::Box { children } = children[1].kind() else {
        panic!("counter panel must be a Box");
    };
    assert!(matches!(&children[0].kind(), UiNodeKind::Text { text } if text == "L:1"));
    assert!(matches!(&children[1].kind(), UiNodeKind::Text { text } if text == "R:0"));
}

#[test]
fn component_reuse_invalidates_on_calendar_environment_change() {
    let RootDirtyFixture {
        mut engine,
        runtime,
        mut lifecycle,
        root,
        ..
    } = root_dirty_fixture();
    let _ = engine.take_timings();
    runtime.borrow_mut().calendar_clock =
        CalendarClock::fixed(GregorianDate::new(2026, 9, 3).unwrap());
    assert!(
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "revision", UiValue::Integer(1))
            .unwrap()
    );
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert!(!engine.take_timings().iter().any(|timing| {
        matches!(
            timing.operation,
            gpui_rhai::ExecutionOperation::ComponentReuse(_)
        )
    }));

    assert!(
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "revision", UiValue::Integer(2))
            .unwrap()
    );
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert!(engine.take_timings().iter().any(|timing| {
        timing.operation == gpui_rhai::ExecutionOperation::ComponentReuse(3)
            && timing.source == "components/counter_panel"
    }));
}

#[test]
fn reused_component_preserves_effect_and_async_ownership() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/effect_probe").unwrap(),
        EFFECT_PROBE.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/effect_reuse.rhai", EFFECT_REUSE_APP)
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = effect_audit_runtime();
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
    assert_eq!(effect_audit_values(&runtime)["starts"], UiValue::Integer(1));
    assert_eq!(runtime.borrow().effects.len(), 1);
    assert_eq!(runtime.borrow().tasks.active_count(), 1);
    assert_eq!(runtime.borrow().timers.active_count(), 1);
    let _ = runtime.borrow_mut().drain_batch();
    let _ = engine.take_timings();

    assert!(
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "revision", UiValue::Integer(1))
            .unwrap()
    );
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    let timings = engine.take_timings();
    assert!(
        timings.iter().any(|timing| {
            timing.operation == gpui_rhai::ExecutionOperation::ComponentReuse(1)
                && timing.source == "components/effect_probe"
        }),
        "{timings:#?}"
    );
    assert_eq!(effect_audit_values(&runtime)["starts"], UiValue::Integer(1));
    assert_eq!(
        effect_audit_values(&runtime)["cleanups"],
        UiValue::Integer(0)
    );
    assert_eq!(runtime.borrow().effects.len(), 1);
    assert_eq!(runtime.borrow().tasks.active_count(), 1);
    assert_eq!(runtime.borrow().timers.active_count(), 1);
}

#[test]
fn reused_component_keeps_store_reader_dependency() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/store_reader").unwrap(),
        STORE_READER.to_owned(),
    )]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/store_reuse.rhai", STORE_REUSE_APP)
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let mut state = UiRuntimeState::new();
    state
        .stores
        .declare(
            StoreId::app("model"),
            ComponentStateSchema::new(BTreeMap::from([(
                "label".to_owned(),
                StateField::new(ValueSchema::string(), UiValue::String("alpha".to_owned())),
            )]))
            .unwrap(),
        )
        .unwrap();
    let runtime = Rc::new(RefCell::new(state));
    let root = ComponentInstancePath::root("App", "root");
    let reader = root.child("StoreReader", "reader");
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

    assert!(
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "revision", UiValue::Integer(1))
            .unwrap()
    );
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("store reuse root must be a Box");
    };
    assert!(matches!(&children[1].kind(), UiNodeKind::Text { text } if text == "alpha"));

    let change_model = engine.callback(&compiled, "change_model").unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &change_model, UiValue::Null)
        .unwrap();
    assert!(runtime.borrow().dirty_components().contains(&reader));
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("store reuse root must remain a Box");
    };
    assert!(matches!(&children[1].kind(), UiNodeKind::Text { text } if text == "beta"));
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
    assert_eq!(runtime.borrow().tasks.active_count(), 1);

    let candidate = engine
        .compile_self_contained_named("ui/effect_probe.rhai", EFFECT_APP)
        .unwrap();
    let candidate_generation = candidate.generation();
    lifecycle.reload(&mut engine, candidate, &schema).unwrap();
    assert_eq!(lifecycle.generation(), candidate_generation);
    assert_eq!(runtime.borrow().tasks.active_count(), 1);
    assert_eq!(effect_audit_values(&runtime)["starts"], UiValue::Integer(2));
    assert_eq!(
        effect_audit_values(&runtime)["cleanups"],
        UiValue::Integer(1)
    );

    lifecycle.dispose(&mut engine).unwrap();
    assert!(runtime.borrow().effects.is_empty());
    assert_eq!(runtime.borrow().tasks.active_count(), 0);
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

    assert!(
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root, "revision", UiValue::Integer(1))
            .unwrap()
    );
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
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
        root_path.clone(),
        Some("main".to_owned()),
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    let reference = lifecycle.root().unwrap().element_ref().unwrap().clone();
    let node_id = runtime.borrow().element_refs.resolve(&reference).unwrap();
    assert_eq!(Some(node_id), lifecycle.retained().root_id());

    assert!(
        runtime
            .borrow_mut()
            .set_component_state_from_host(&root_path, "revision", UiValue::Integer(1))
            .unwrap()
    );
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
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

struct NodePropFixture {
    engine: RuntimeEngine,
    runtime: Rc<RefCell<UiRuntimeState>>,
    lifecycle: ScriptLifecycle,
    root: ComponentInstancePath,
    stateful: ComponentInstancePath,
    receiver: ComponentInstancePath,
}

fn node_prop_fixture() -> NodePropFixture {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/node_prop_stateful").unwrap(),
            NODE_PROP_STATEFUL.to_owned(),
        ),
        (
            ModuleId::parse("components/node_prop_receiver").unwrap(),
            NODE_PROP_RECEIVER.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/node_prop_replay.rhai", NODE_PROP_APP)
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root = ComponentInstancePath::root("App", "root");
    let stateful = root.child("NodePropStateful", "stateful");
    let receiver = root.child("NodePropReceiver", "receiver");
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
    NodePropFixture {
        engine,
        runtime,
        lifecycle,
        root,
        stateful,
        receiver,
    }
}

fn assert_receiver_content(lifecycle: &ScriptLifecycle, expected: &str, handlers: usize) {
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        panic!("receiver must render a column");
    };
    assert!(matches!(children[0].kind(), UiNodeKind::Text { text } if text == expected));
    assert_eq!(children[0].event_handlers("click").len(), handlers);
}

#[test]
fn receiver_rerender_replays_the_latest_node_prop_component_snapshot() {
    let NodePropFixture {
        mut engine,
        runtime,
        mut lifecycle,
        root,
        stateful,
        receiver,
    } = node_prop_fixture();

    runtime
        .borrow_mut()
        .set_component_state_from_host(&stateful, "value", UiValue::String("updated".to_owned()))
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_receiver_content(&lifecycle, "updated", 1);

    runtime
        .borrow_mut()
        .set_component_state_from_host(&receiver, "revision", UiValue::Integer(1))
        .unwrap();
    let _ = engine.take_timings();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_eq!(
        runtime.borrow().component_state.get(&stateful, "value"),
        Some(&UiValue::String("updated".to_owned())),
        "the caller-owned component state itself must remain mounted"
    );
    assert_receiver_content(&lifecycle, "updated", 1);
    let timings = engine.take_timings();
    assert!(timings.iter().any(|timing| {
        matches!(timing.operation, ExecutionOperation::Render)
            && timing.source == "components/node_prop_receiver"
    }));
    assert!(!timings.iter().any(|timing| {
        matches!(timing.operation, ExecutionOperation::Render)
            && timing.source == "components/node_prop_stateful"
    }));

    {
        let mut runtime = runtime.borrow_mut();
        runtime
            .set_component_state_from_host(
                &stateful,
                "value",
                UiValue::String("same-batch".to_owned()),
            )
            .unwrap();
        runtime
            .set_component_state_from_host(&receiver, "revision", UiValue::Integer(2))
            .unwrap();
    }
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_receiver_content(&lifecycle, "same-batch", 1);
    let UiNodeKind::Box { children } = lifecycle.root().unwrap().kind() else {
        unreachable!()
    };
    let receiver_handler = children[0]
        .handler("click")
        .and_then(gpui_rhai::UiEventHandler::as_script)
        .cloned()
        .expect("receiver presentation callback");
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &receiver_handler, UiValue::Null)
        .unwrap();
    assert_eq!(
        runtime
            .borrow()
            .component_state
            .get(&receiver, "click_count"),
        Some(&UiValue::Integer(1)),
        "presentation callback must retain the receiving component context"
    );
    assert!(lifecycle.render_dirty(&mut engine).unwrap());

    let last_good = lifecycle.root().unwrap().clone();
    runtime
        .borrow_mut()
        .set_component_state_from_host(&receiver, "fail", UiValue::Bool(true))
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).is_err());
    assert_eq!(lifecycle.root(), Some(&last_good));
    {
        let mut runtime = runtime.borrow_mut();
        runtime
            .set_component_state_from_host(&receiver, "fail", UiValue::Bool(false))
            .unwrap();
        runtime
            .set_component_state_from_host(&receiver, "revision", UiValue::Integer(3))
            .unwrap();
    }
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_receiver_content(&lifecycle, "same-batch", 1);

    runtime
        .borrow_mut()
        .set_component_state_from_host(&root, "revision", UiValue::Integer(1))
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_receiver_content(&lifecycle, "same-batch", 1);
}

#[test]
fn receiver_rerender_does_not_restart_replayed_node_prop_resources() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([
        (
            ModuleId::parse("components/effect_probe").unwrap(),
            EFFECT_PROBE.to_owned(),
        ),
        (
            ModuleId::parse("components/node_prop_receiver").unwrap(),
            NODE_PROP_RECEIVER.to_owned(),
        ),
    ]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled = engine
        .compile_self_contained_named("ui/node_prop_effect.rhai", NODE_PROP_EFFECT_APP)
        .unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = effect_audit_runtime();
    let root = ComponentInstancePath::root("App", "root");
    let receiver = root.child("NodePropReceiver", "receiver");
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
    assert_eq!(effect_audit_values(&runtime)["starts"], UiValue::Integer(1));
    assert_eq!(
        effect_audit_values(&runtime)["cleanups"],
        UiValue::Integer(0)
    );
    assert_eq!(runtime.borrow().effects.len(), 1);
    assert_eq!(runtime.borrow().timers.active_count(), 1);
    assert_eq!(runtime.borrow().tasks.active_count(), 1);

    runtime
        .borrow_mut()
        .set_component_state_from_host(&receiver, "revision", UiValue::Integer(1))
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_eq!(effect_audit_values(&runtime)["starts"], UiValue::Integer(1));
    assert_eq!(
        effect_audit_values(&runtime)["cleanups"],
        UiValue::Integer(0)
    );
    assert_eq!(runtime.borrow().effects.len(), 1);
    assert_eq!(runtime.borrow().timers.active_count(), 1);
    assert_eq!(runtime.borrow().tasks.active_count(), 1);

    runtime
        .borrow_mut()
        .set_component_state_from_host(&root, "visible", UiValue::Bool(false))
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_eq!(effect_audit_values(&runtime)["starts"], UiValue::Integer(1));
    assert_eq!(
        effect_audit_values(&runtime)["cleanups"],
        UiValue::Integer(1)
    );
    assert!(runtime.borrow().effects.is_empty());
    assert_eq!(runtime.borrow().timers.active_count(), 0);
    assert_eq!(runtime.borrow().tasks.active_count(), 0);
}

#[test]
fn rejected_child_candidate_cannot_leak_through_the_shared_node_snapshot() {
    let NodePropFixture {
        mut engine,
        runtime,
        mut lifecycle,
        stateful,
        receiver,
        ..
    } = node_prop_fixture();
    runtime
        .borrow_mut()
        .set_component_state_from_host(&stateful, "value", UiValue::String("accepted".to_owned()))
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_receiver_content(&lifecycle, "accepted", 1);

    let retained_budget = runtime.borrow().budgets.retained_nodes;
    {
        let mut runtime = runtime.borrow_mut();
        runtime.budgets.retained_nodes = 1;
        runtime
            .set_component_state_from_host(
                &stateful,
                "value",
                UiValue::String("rejected".to_owned()),
            )
            .unwrap();
    }
    assert!(lifecycle.render_dirty(&mut engine).is_err());
    assert_receiver_content(&lifecycle, "accepted", 1);

    {
        let mut runtime = runtime.borrow_mut();
        runtime.budgets.retained_nodes = retained_budget;
        let _ = runtime.drain_batch();
        runtime
            .set_component_state_from_host(&receiver, "revision", UiValue::Integer(1))
            .unwrap();
    }
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert_receiver_content(&lifecycle, "accepted", 1);
}

#[test]
fn data_backed_virtual_collection_realizes_only_requested_items_off_layout_path() {
    let mut engine = RuntimeEngine::new();
    let compiled = engine
        .compile(
            r#"
                define_component(#{
                    metadata: #{
                        id: "components/virtual_item", "export": "VirtualItem", version: "0.1.0",
                        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
                        dependencies: [], capabilities: #{}
                    },
                    schema: #{
                        props: #{
                            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                            label: #{ schema: #{ type: "string" }, required: true, sensitive: false }
                        },
                        state: #{ fields: #{ mounted: #{ schema: #{ type: "bool" },
                            "default": #{ type: "bool", value: true } } } },
                        events: #{}, slots: #{}, parts: ["root"]
                    },
                    render: Fn("render_VirtualItem")
                });
                fn VirtualItem(props) { render_component("components/virtual_item", props) }
                fn render_VirtualItem(ctx, props) { text(props.label) }
                fn item(ctx, payload) {
                    VirtualItem(#{ key: payload.key, label: payload.item.label })
                }
                fn view(ctx) {
                    let data = [];
                    for index in 0..100 {
                        data.push(#{ key: `item-${index}`, label: `Item ${index}` });
                    }
                    virtual_collection(#{
                        key: "messages", label: "Messages", data: data,
                        estimated_height: 20, height: 100, overdraw_pixels: 40,
                        alignment: "top", follow_tail: false
                    }, Fn("item"))
                }
            "#,
        )
        .unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root = ComponentInstancePath::root("App", "root");
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        Rc::clone(&runtime),
        root.clone(),
        Some("main".to_owned()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    let gpui_rhai::UiNodeKind::VirtualCollection { spec } = lifecycle.root().unwrap().kind() else {
        panic!("root must be virtual collection");
    };
    assert_eq!(spec.data.len(), 100);
    assert_eq!(spec.realized.len(), 8);
    assert_eq!(runtime.borrow().component_state.instance_count(), 9);
    let id = spec.id.clone();

    runtime
        .borrow()
        .virtual_requests
        .request(id, 50usize..56usize);
    assert!(lifecycle.realize_virtual_requests(&mut engine).unwrap());
    let gpui_rhai::UiNodeKind::VirtualCollection { spec } = lifecycle.root().unwrap().kind() else {
        panic!("root must remain virtual collection");
    };
    assert_eq!(
        spec.realized.keys().copied().collect::<Vec<_>>(),
        (50..56).collect::<Vec<_>>()
    );
    assert_eq!(runtime.borrow().component_state.instance_count(), 7);
    assert!(matches!(
        spec.realized[&50].kind(),
        UiNodeKind::Text { text } if text == "Item 50"
    ));
}
