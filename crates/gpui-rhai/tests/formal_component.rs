use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui_rhai::{
    ActionId, AsyncCapabilityHandler, CapabilityDescriptor, CapabilityId, CapabilityMethod,
    ComponentInstancePath, ComponentStateSchema, EmbeddedScriptSource, ModuleId,
    RestrictedModuleResolver, RuntimeEngine, ScriptLifecycle, TaskWork, UiNodeKind, UiRuntimeState,
    UiValue, ValueSchema,
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
export_component(#{
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
    }
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
fn Counter(props) { component_render("components/counter", props, Fn("render_Counter")) }
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
    assert_eq!(
        runtime
            .borrow()
            .component_state
            .get(&component_path, "count"),
        Some(&UiValue::Integer(0))
    );
    let click = lifecycle.root().unwrap().handlers()["click"].clone();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &click, UiValue::Null)
        .unwrap();
    let batch = runtime.borrow_mut().drain_batch();
    assert_eq!(batch.events.len(), 1);
    for event in batch.events {
        let _ = lifecycle
            .invoke_component_event_transactional(&engine, event)
            .unwrap();
    }
    lifecycle.render(&mut engine).unwrap();
    assert!(matches!(
        lifecycle.root().unwrap().kind(),
        UiNodeKind::Text { text } if text == "1"
    ));
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
    let click = lifecycle.root().unwrap().handlers()["click"].clone();
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
    let open_change = lifecycle.root().unwrap().handlers()["open_change"].clone();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &open_change, UiValue::Bool(false))
        .unwrap();
    assert_eq!(
        runtime.borrow().component_state.get(&root, "open"),
        Some(&UiValue::Bool(false))
    );
}
