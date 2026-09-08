use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
fn main() {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        RuntimeEngine::new().compile("fn view() { const m = #{x:1}; m.x=2; text(\"done\") }")
    }));
    assert!(matches!(
        result,
        Ok(Err(RuntimeError::InvalidAssignmentTarget(_)))
    ));
    println!("const_map_compile_rejected_without_panic=true");
    assert!(UiValue::from_dynamic(rhai::Dynamic::from(f64::NAN)).is_err());
    assert!(serde_json::to_string(&UiValue::Float(f64::INFINITY)).is_err());
    println!("nonfinite_rhai_and_serde_rejected=true");
    let swap_component = r#"
define_component(#{
  metadata: #{ id: "components/swap", "export": "Swap", version: "0.1.0", runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
  schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } }, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"], effects: ["sync"] },
  render: Fn("render_Swap")
});
fn Swap(props) { render_component("components/swap", props) }
fn render_Swap(ctx, props) { effect("sync", props.key, Fn("start"), Fn("cleanup")); text(props.key) }
fn start(ctx, key) { if key == "b" { throw "new effect failed"; } }
fn cleanup(ctx, key) { () }
"#;
    let mut resolver = RestrictedModuleResolver::new();
    resolver.insert("components/swap", swap_component).unwrap();
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(resolver);
    let compiled = engine
        .compile_self_contained_named(
            "swap-rollback",
            r#"
import "components/swap" as s;
fn view(ctx) { if ctx.get_state("swap") { s::Swap(#{key: "b"}) } else { s::Swap(#{key: "a"}) } }
fn delivered(ctx, value) { () }
"#,
        )
        .unwrap();
    let callback = engine.callback(&compiled, "delivered").unwrap();
    let generation = compiled.generation();
    let schema = ComponentStateSchema::new(BTreeMap::from([(
        "swap".into(),
        StateField::new(ValueSchema::Bool, UiValue::Bool(false)),
    )]))
    .unwrap();
    let state = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root = ComponentInstancePath::root("App", "swap");
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        state.clone(),
        root.clone(),
        None,
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    let owner = engine
        .component_invocations()
        .next()
        .unwrap()
        .path()
        .clone();
    let (_, emitter) = state
        .borrow_mut()
        .subscriptions
        .subscribe(SubscriptionRegistration::new(
            "old-owned-resource",
            AsyncScope::Component(owner),
            generation,
            callback.clone(),
            callback,
            ValueSchema::string(),
        ));
    let old_tree = format!("{:?}", lifecycle.root());
    state
        .borrow_mut()
        .set_component_state_from_host(&root, "swap", UiValue::Bool(true))
        .unwrap();
    let result = lifecycle.render_dirty(&mut engine);
    println!(
        "effect_swap_failed={} old_tree_restored={} old_subscription_active={} closed={:?}",
        result.is_err(),
        format!("{:?}", lifecycle.root()) == old_tree,
        state.borrow().subscriptions.active_count(),
        emitter.close_reason()
    );
}
