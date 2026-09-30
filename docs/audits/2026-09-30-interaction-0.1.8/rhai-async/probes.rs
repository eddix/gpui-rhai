//! Independent characterization against 0.1.8 HEAD 0b9b887c.
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::{Duration, Instant}};
use gpui_rhai::{AsyncCapabilityHandler, CapabilityDescriptor, CapabilityId, CapabilityMethod,
    ComponentInstancePath, EmbeddedScriptSource, RestrictedModuleResolver, RuntimeEngine,
    ScriptLifecycle, TaskWork, UiRuntimeState, UiValue, ValueSchema, SubscriptionRegistration,
    AsyncScope, CapabilityHandler};
use semver::{Version, VersionReq};

const APP: &str = r#"
fn state_schema() { #{ fields: #{ status: #{ schema: #{ type: "string" },
    "default": #{ type: "string", value: "idle" } } } } }
fn loaded(ctx, out) { let size = out.len(); ctx.set_state("status", "loaded"); }
fn failed(ctx, error) { let size = error.message.len(); ctx.set_state("status", "failed"); }
fn kick(ctx, payload) {
    ctx.set_state("status", "loading");
    ctx.start_task("app.big", "get", "", Fn("loaded"), Fn("failed"));
}
fn view(ctx) { text(ctx.get_state("status")).on_click(Fn("kick")) }
"#;

struct Big(Result<UiValue, String>);
impl AsyncCapabilityHandler for Big {
    fn start(&mut self, _: &str, _: UiValue) -> Result<TaskWork, String> {
        let result = self.0.clone();
        Ok(TaskWork::new(move || result))
    }
}

fn setup(source: &str, output: Result<UiValue, String>) ->
    (RuntimeEngine, ScriptLifecycle, Rc<RefCell<UiRuntimeState>>, ComponentInstancePath) {
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(
        &EmbeddedScriptSource::new(BTreeMap::new())).unwrap());
    let compiled = engine.compile_self_contained_named("ui/app.rhai", source).unwrap();
    let mut state = UiRuntimeState::new();
    let cap = CapabilityId::parse("app.big").unwrap();
    state.capabilities.register_async(CapabilityDescriptor {
        id: cap.clone(), version: Version::new(1, 0, 0), methods: BTreeMap::from([
            ("get".to_owned(), CapabilityMethod {input: ValueSchema::string(), output: ValueSchema::UiValue})
        ])}, Big(output)).unwrap();
    state.capabilities.activate(&BTreeMap::from([(cap, VersionReq::STAR)])).unwrap();
    let runtime = Rc::new(RefCell::new(state));
    let root = ComponentInstancePath::root("App", "root");
    let schema = engine.root_state_schema(&compiled).unwrap();
    let mut lifecycle = ScriptLifecycle::new(compiled, Rc::clone(&runtime), root.clone(),
        Some("main".to_owned()), BTreeMap::new(), &schema).unwrap();
    lifecycle.start(&mut engine).unwrap();
    (engine, lifecycle, runtime, root)
}

fn wait_tasks(runtime: &Rc<RefCell<UiRuntimeState>>, lifecycle: &ScriptLifecycle)
    -> Vec<gpui_rhai::AsyncDelivery> {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let deliveries = runtime.borrow_mut().tasks.drain(lifecycle.generation());
        if !deliveries.is_empty() { return deliveries; }
        assert!(Instant::now() < until, "task timeout");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn task_delivery_result_and_error_size_characterization() {
    let cases = [
        ("small_success", Ok(UiValue::String("x".repeat(1_000_000))), true, "loaded"),
        ("large_success", Ok(UiValue::String("x".repeat(1_100_000))), false, "loading"),
        ("small_error", Err("x".repeat(100)), true, "failed"),
        ("large_error", Err("x".repeat(1_100_000)), false, "loading"),
        ("large_array", Ok(UiValue::Array(vec![UiValue::Integer(0); 10_001])), false, "loading"),
        ("nested_strings", Ok(UiValue::Array(vec![UiValue::String("x".repeat(600_000)); 2])), false, "loading"),
    ];
    for (name, output, expect_ok, expected_status) in cases {
        let (engine, lifecycle, runtime, root) = setup(APP, output);
        let callback = lifecycle.root().unwrap().handler("click").unwrap().as_script().unwrap().clone();
        let _ = lifecycle.invoke_callback_transactional(&engine, &callback, UiValue::Null).unwrap();
        let delivery = wait_tasks(&runtime, &lifecycle).pop().unwrap();
        let result = lifecycle.invoke_async_delivery_transactional(&engine, delivery);
        let status = runtime.borrow().component_state.get(&root, "status").cloned();
        println!("CASE={name} result={} status={status:?}", match &result {
            Ok(_) => "Ok".to_owned(), Err(error) => error.to_string()});
        assert_eq!(result.is_ok(), expect_ok, "{name}");
        assert_eq!(status, Some(UiValue::String(expected_status.to_owned())), "{name}");
    }
}

#[test]
fn subscription_delivery_result_and_error_size_characterization() {
    for error_payload in [false, true] {
        let (engine, lifecycle, runtime, root) = setup(APP, Ok(UiValue::Null));
        let callback = lifecycle.root().unwrap().handler("click").unwrap().as_script().unwrap().clone();
        let _ = lifecycle.invoke_callback_transactional(&engine, &callback, UiValue::Null).unwrap();
        // Reuse callbacks retained by the real task creation path, including its call context.
        let mut success = wait_tasks(&runtime, &lifecycle).pop().unwrap();
        let callback = success.callback.clone();
        let registration = SubscriptionRegistration::new("audit", AsyncScope::Component(root.clone()),
            lifecycle.generation(), callback.clone(), callback, ValueSchema::UiValue);
        let (_, emitter) = runtime.borrow_mut().subscriptions.subscribe(registration);
        if error_payload { emitter.emit_error("x".repeat(1_100_000)).unwrap(); }
        else { emitter.emit(UiValue::String("x".repeat(1_100_000))).unwrap(); }
        success = runtime.borrow_mut().subscriptions.drain(lifecycle.generation()).pop().unwrap();
        let result = lifecycle.invoke_async_delivery_transactional(&engine, success);
        println!("SUBSCRIPTION error_payload={error_payload} result={:?}", result.as_ref().map(|_| ()));
        assert!(result.is_err());
        assert_eq!(runtime.borrow().component_state.get(&root, "status"), Some(&UiValue::String("loading".to_owned())));
    }
}

#[test]
fn real_retained_task_chain_resets_depth_and_operation_baseline() {
    let source = APP.replace("fn loaded(ctx, out) { let size = out.len(); ctx.set_state(\"status\", \"loaded\"); }", r#"
fn loaded(ctx, out) {
    // Each turn uses >10k evaluator operations, aggregate >1m, but each is independent.
    let cost = 0; for i in 0..2000 { cost += i; }
    let hop = if ctx.get_state("status") == "loading" { 1 } else { parse_int(ctx.get_state("status")) + 1 };
    ctx.set_state("status", hop.to_string());
    if hop < 200 { ctx.start_task("app.big", "get", "", Fn("loaded"), Fn("failed")); }
}
"#);
    let (engine, lifecycle, runtime, root) = setup(&source, Ok(UiValue::Null));
    let callback = lifecycle.root().unwrap().handler("click").unwrap().as_script().unwrap().clone();
    let _ = lifecycle.invoke_callback_transactional(&engine, &callback, UiValue::Null).unwrap();
    for hop in 1..=200 {
        let delivery = wait_tasks(&runtime, &lifecycle).pop().unwrap();
        let _ = lifecycle.invoke_async_delivery_transactional(&engine, delivery).unwrap();
        assert_eq!(runtime.borrow().component_state.get(&root, "status"), Some(&UiValue::String(hop.to_string())));
    }
    println!("CHAIN completed=200 depth_stable=true independent_operation_budget=true");
}

#[test]
fn retained_callback_still_bounds_real_recursion_and_infinite_loop() {
    for (label, body, expected) in [
        ("recursion", "loaded(ctx, out);", "Stack overflow"),
        ("operation_budget", "loop { let n = 1 + 1; }", "Too many operations"),
    ] {
        let source = APP.replace("let size = out.len(); ctx.set_state(\"status\", \"loaded\");", body);
        let (engine, lifecycle, runtime, root) = setup(&source, Ok(UiValue::Null));
        let callback = lifecycle.root().unwrap().handler("click").unwrap().as_script().unwrap().clone();
        let _ = lifecycle.invoke_callback_transactional(&engine, &callback, UiValue::Null).unwrap();
        let delivery = wait_tasks(&runtime, &lifecycle).pop().unwrap();
        let error = lifecycle.invoke_async_delivery_transactional(&engine, delivery).unwrap_err().to_string();
        println!("GUARD {label}: {error}");
        // Runtime's progress hook reports termination, rather than Rhai's disabled absolute operations limit.
        assert!(error.contains(expected) || label == "operation_budget" && error.contains("terminated"));
        assert_eq!(runtime.borrow().component_state.get(&root, "status"), Some(&UiValue::String("loading".to_owned())));
    }
}

struct Marker(Rc<RefCell<usize>>);
impl CapabilityHandler for Marker {
    fn call(&mut self, _: &str, _: UiValue) -> Result<UiValue, String> {
        *self.0.borrow_mut() += 1;
        Ok(UiValue::Null)
    }
}

#[test]
fn oversize_argument_failure_is_lazy_and_can_follow_host_side_effects() {
    for read_payload in [false, true] {
        let body = if read_payload {
            "ctx.call_capability(\"app.marker\", \"mark\", ()); let size = out.len();"
        } else {
            "ctx.call_capability(\"app.marker\", \"mark\", ());"
        };
        let source = APP.replace("let size = out.len();", body);
        let (engine, lifecycle, runtime, root) = setup(&source, Ok(UiValue::String("x".repeat(1_100_000))));
        let seen = Rc::new(RefCell::new(0));
        let id = CapabilityId::parse("app.marker").unwrap();
        runtime.borrow_mut().capabilities.register(CapabilityDescriptor {
            id: id.clone(), version: Version::new(1,0,0), methods: BTreeMap::from([
                ("mark".to_owned(), CapabilityMethod {input: ValueSchema::Null, output: ValueSchema::Null})
            ])}, Marker(Rc::clone(&seen))).unwrap();
        runtime.borrow_mut().capabilities.activate(&BTreeMap::from([
            (CapabilityId::parse("app.big").unwrap(), VersionReq::STAR), (id, VersionReq::STAR)
        ])).unwrap();
        let callback = lifecycle.root().unwrap().handler("click").unwrap().as_script().unwrap().clone();
        let _ = lifecycle.invoke_callback_transactional(&engine, &callback, UiValue::Null).unwrap();
        let delivery = wait_tasks(&runtime, &lifecycle).pop().unwrap();
        let result = lifecycle.invoke_async_delivery_transactional(&engine, delivery);
        let status = runtime.borrow().component_state.get(&root, "status").cloned();
        println!("LAZY read_payload={read_payload} callback_result={:?} host_side_effects={} state={status:?}",
            result.as_ref().map(|_| ()), *seen.borrow());
        assert_eq!(*seen.borrow(), 1, "callback was entered and Host effect ran before any failure");
        assert_eq!(result.is_err(), read_payload);
    }
}
