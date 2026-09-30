//! Positive acceptance and adversarial boundaries for d77b4b49.
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::{Duration, Instant}};
use gpui_rhai::{AsyncCapabilityHandler, CapabilityDescriptor, CapabilityId, CapabilityMethod,
    ComponentInstancePath, EmbeddedScriptSource, RestrictedModuleResolver, RuntimeEngine,
    ScriptLifecycle, TaskWork, UiRuntimeState, UiValue, ValueSchema, SubscriptionRegistration,
    AsyncScope, CapabilityHandler, TaskRegistry, SubscriptionRegistry};
use semver::{Version, VersionReq};

const APP: &str = r#"
fn state_schema() { #{ fields: #{ status: #{ schema: #{ type: "string" },
    "default": #{ type: "string", value: "idle" } }, hits: #{schema:#{type:"integer"},
    "default":#{type:"integer",value:0}}, errors: #{schema:#{type:"integer"},
    "default":#{type:"integer",value:0}} } } }
fn loaded(ctx, out) {
    ctx.call_capability("app.marker", "mark", ());
    let size = out.len();
    ctx.set_state("hits", ctx.get_state("hits") + 1);
    ctx.set_state("status", "loaded");
}
fn failed(ctx, error) {
    let size = error.message.len();
    ctx.set_state("errors", ctx.get_state("errors") + 1);
    ctx.set_state("status", "failed");
}
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
struct Marker(Rc<RefCell<usize>>);
impl CapabilityHandler for Marker {
    fn call(&mut self, _: &str, _: UiValue) -> Result<UiValue, String> {
        *self.0.borrow_mut() += 1;
        Ok(UiValue::Null)
    }
}

fn setup(source: &str, output: Result<UiValue, String>, schema: ValueSchema) ->
    (RuntimeEngine, ScriptLifecycle, Rc<RefCell<UiRuntimeState>>, ComponentInstancePath, Rc<RefCell<usize>>) {
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(
        &EmbeddedScriptSource::new(BTreeMap::new())).unwrap());
    let compiled = engine.compile_self_contained_named("ui/app.rhai", source).unwrap();
    let mut state = UiRuntimeState::new();
    let cap = CapabilityId::parse("app.big").unwrap();
    state.capabilities.register_async(CapabilityDescriptor {
        id: cap.clone(), version: Version::new(1, 0, 0), methods: BTreeMap::from([
            ("get".to_owned(), CapabilityMethod {input: ValueSchema::string(), output: schema})
        ])}, Big(output)).unwrap();
    let marker = CapabilityId::parse("app.marker").unwrap();
    let seen = Rc::new(RefCell::new(0));
    state.capabilities.register(CapabilityDescriptor {
        id: marker.clone(), version: Version::new(1,0,0), methods: BTreeMap::from([
            ("mark".to_owned(), CapabilityMethod {input: ValueSchema::Null, output: ValueSchema::Null})
        ])}, Marker(Rc::clone(&seen))).unwrap();
    state.capabilities.activate(&BTreeMap::from([(cap, VersionReq::STAR),(marker,VersionReq::STAR)])).unwrap();
    let runtime = Rc::new(RefCell::new(state));
    let root = ComponentInstancePath::root("App", "root");
    let schema = engine.root_state_schema(&compiled).unwrap();
    let mut lifecycle = ScriptLifecycle::new(compiled, Rc::clone(&runtime), root.clone(),
        Some("main".to_owned()), BTreeMap::new(), &schema).unwrap();
    lifecycle.start(&mut engine).unwrap();
    (engine, lifecycle, runtime, root, seen)
}

fn kick(engine: &RuntimeEngine, lifecycle: &ScriptLifecycle) {
    let callback = lifecycle.root().unwrap().handler("click").unwrap().as_script().unwrap().clone();
    let _ = lifecycle.invoke_callback_transactional(engine, &callback, UiValue::Null).unwrap();
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
fn field<'a>(payload: &'a UiValue, key: &str) -> &'a UiValue {
    let UiValue::Map(value) = payload else { panic!("object required") };
    value.get(key).unwrap()
}
fn nested(depth: usize) -> UiValue {
    (0..depth).fold(UiValue::Null, |value,_| UiValue::Array(vec![value]))
}

#[test]
fn fixed_task_preflight_and_error_routing_have_exactly_one_callback() {
    let cases = [
        ("string_exact", Ok(UiValue::String("x".repeat(1_048_576))), ValueSchema::UiValue, true),
        ("string_plus_one", Ok(UiValue::String("x".repeat(1_048_577))), ValueSchema::UiValue, false),
        ("nested_string_exact", Ok(UiValue::Array(vec![UiValue::String("x".repeat(524_288));2])), ValueSchema::UiValue, true),
        ("nested_string_excess", Ok(UiValue::Array(vec![UiValue::String("x".repeat(600_000));2])), ValueSchema::UiValue, false),
        ("array_exact", Ok(UiValue::Array(vec![UiValue::Null;10_000])), ValueSchema::UiValue, true),
        ("array_plus_one", Ok(UiValue::Array(vec![UiValue::Null;10_001])), ValueSchema::UiValue, false),
        ("nested_array_exact", Ok(UiValue::Array(vec![UiValue::Array(vec![UiValue::Null;4_999]);2])), ValueSchema::UiValue, true),
        ("nested_array_excess", Ok(UiValue::Array(vec![UiValue::Array(vec![UiValue::Null;5_000]);2])), ValueSchema::UiValue, false),
        ("depth64", Ok(nested(64)), ValueSchema::UiValue, true),
        ("depth65", Ok(nested(65)), ValueSchema::UiValue, false),
        ("large_utf8_error", Err("界".repeat(500_000)), ValueSchema::UiValue, false),
        ("large_schema_error", Ok(UiValue::String("界".repeat(500_000))), ValueSchema::enumeration(["allowed"]), false),
    ];
    for (name, output, schema, accepted) in cases {
        let (engine,lifecycle,runtime,root,seen) = setup(APP,output,schema);
        kick(&engine,&lifecycle);
        let deliveries = wait_tasks(&runtime,&lifecycle);
        assert_eq!(deliveries.len(),1);
        let delivery = deliveries.into_iter().next().unwrap();
        assert_eq!(delivery.callback.name(),if accepted {"loaded"} else {"failed"});
        if !accepted {
            let UiValue::String(message) = field(&delivery.payload,"message") else { panic!() };
            assert!(message.len() <= 1_027);
        }
        let _ = lifecycle.invoke_async_delivery_transactional(&engine,delivery).unwrap();
        assert_eq!(*seen.borrow(),usize::from(accepted));
        assert_eq!(runtime.borrow().component_state.get(&root,"hits"),Some(&UiValue::Integer(i64::from(accepted))));
        assert_eq!(runtime.borrow().component_state.get(&root,"errors"),Some(&UiValue::Integer(i64::from(!accepted))));
        assert!(runtime.borrow_mut().tasks.drain(lifecycle.generation()).is_empty());
        assert_eq!(runtime.borrow().tasks.active_count(),0);
        println!("TASK {name} accepted={accepted} exactly_one_callback=true host_success_side_effects={}",*seen.borrow());
    }
}

fn callbacks() -> (RuntimeEngine, gpui_rhai::CompiledUi, gpui_rhai::ScriptCallback, gpui_rhai::ScriptCallback) {
    let mut engine = RuntimeEngine::new();
    let compiled = engine.compile("fn view(){text(\"audit\")} fn success(ctx,value){value} fn failure(ctx,value){value}").unwrap();
    engine.render(&compiled).unwrap();
    let success = engine.callback(&compiled,"success").unwrap();
    let failure = engine.callback(&compiled,"failure").unwrap();
    (engine,compiled,success,failure)
}

#[test]
fn subscription_close_preserves_order_and_cancel_or_stale_discards() {
    let (mut engine,compiled,success,failure)=callbacks();
    let mut registry=SubscriptionRegistry::new();
    let registration=|| SubscriptionRegistration::new("audit",AsyncScope::App,compiled.generation(),success.clone(),failure.clone(),ValueSchema::UiValue);
    let (_,emitter)=registry.subscribe(registration());
    emitter.emit(UiValue::String("before".into())).unwrap();
    emitter.emit(UiValue::String("x".repeat(1_100_000))).unwrap();
    emitter.emit_error("界".repeat(500_000)).unwrap();
    emitter.emit(UiValue::String("after".into())).unwrap();
    emitter.close();
    let deliveries=registry.drain(compiled.generation());
    assert_eq!(deliveries.iter().map(|d|d.callback.name()).collect::<Vec<_>>(),vec!["success","failure","failure","success"]);
    for d in deliveries.iter().filter(|d|d.callback.name()=="failure") {
        engine.engine().ensure_data_size_within_limits(&d.payload.clone().into_dynamic()).unwrap();
    }
    assert_eq!(registry.active_count(),0);
    assert!(registry.drain(compiled.generation()).is_empty());
    assert!(emitter.emit(UiValue::Null).is_err());
    let (id,emitter)=registry.subscribe(registration());
    emitter.emit(UiValue::String("x".repeat(1_100_000))).unwrap();
    assert!(registry.cancel(id));
    assert!(registry.drain(compiled.generation()).is_empty());
    let (_,emitter)=registry.subscribe(registration());
    emitter.emit_error("x".repeat(1_100_000)).unwrap();
    let next=engine.compile("fn view(){text(\"next\")}").unwrap();
    assert!(registry.drain(next.generation()).is_empty());
    assert!(emitter.emit(UiValue::Null).is_err());
    println!("SUBSCRIPTION ordered=[success,failure,failure,success] close_flush_once=true cancel_and_stale_drop=true");
}

#[test]
fn map_accounting_matches_actual_rhai_including_keys() {
    let (engine,compiled,success,failure)=callbacks();
    for count in [100_000,100_001] {
        let value=UiValue::Map((0..count).map(|i|(i.to_string(),UiValue::Null)).collect());
        let expected=engine.engine().ensure_data_size_within_limits(&value.clone().into_dynamic()).is_ok();
        let mut registry=SubscriptionRegistry::new();
        let (_,emitter)=registry.subscribe(SubscriptionRegistration::new("map",AsyncScope::App,compiled.generation(),success.clone(),failure.clone(),ValueSchema::UiValue));
        emitter.emit(value).unwrap();
        let delivery=registry.drain(compiled.generation()).pop().unwrap();
        assert_eq!(delivery.callback.name()=="success",expected);
        println!("MAP entries={count} accepted={expected}");
    }
    let value=UiValue::Map(BTreeMap::from([("x".repeat(1_100_000),UiValue::Null)]));
    engine.engine().ensure_data_size_within_limits(&value.clone().into_dynamic()).unwrap();
    let mut registry=SubscriptionRegistry::new();
    let (_,emitter)=registry.subscribe(SubscriptionRegistration::new("map-key",AsyncScope::App,compiled.generation(),success.clone(),failure.clone(),ValueSchema::UiValue));
    emitter.emit(value).unwrap();
    assert_eq!(registry.drain(compiled.generation()).pop().unwrap().callback.name(),"success");
    println!("MAP large_key_excluded_from_rhai_string_quota=true");
}

#[test]
fn task_cancel_and_stale_never_deliver_rejected_payload() {
    let (mut engine,compiled,success,failure)=callbacks();
    let mut tasks=TaskRegistry::new();
    let (send,recv)=std::sync::mpsc::channel();
    let handle=tasks.spawn(AsyncScope::App,compiled.generation(),success.clone(),failure.clone(),ValueSchema::UiValue,move || {
        recv.recv().unwrap(); Ok(UiValue::String("x".repeat(1_100_000)))
    }).unwrap();
    assert!(tasks.cancel(handle)); send.send(()).unwrap();
    std::thread::sleep(Duration::from_millis(30));
    assert!(tasks.drain(compiled.generation()).is_empty());
    let handle=tasks.spawn(AsyncScope::App,compiled.generation(),success,failure,ValueSchema::UiValue,|| Ok(UiValue::String("x".repeat(1_100_000)))).unwrap();
    let next=engine.compile("fn view(){text(\"next\")}").unwrap();
    let until=Instant::now()+Duration::from_secs(10);
    while tasks.active_count()>0 {
        assert!(tasks.drain(next.generation()).is_empty());
        assert!(Instant::now()<until);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!tasks.cancel(handle));
    println!("TASK cancellation_and_stale_drop=true");
}

#[test]
fn preflight_rejection_cost_characterization() {
    let (_,compiled,success,failure)=callbacks();
    for count in [10_001,100_000,900_000] {
        for item_schema in [ValueSchema::Null,ValueSchema::integer()] {
            let mut registry=SubscriptionRegistry::new();
            let expected=if item_schema==ValueSchema::Null {"valid_schema"} else {"invalid_schema"};
            let schema=ValueSchema::Array{items:Box::new(item_schema),max_items:None};
            let (_,emitter)=registry.subscribe(SubscriptionRegistration::new("cost",AsyncScope::App,compiled.generation(),success.clone(),failure.clone(),schema));
            emitter.emit(UiValue::Array(vec![UiValue::Null;count])).unwrap();
            let begin=Instant::now();
            let delivery=registry.drain(compiled.generation()).pop().unwrap();
            let elapsed=begin.elapsed();
            assert_eq!(delivery.callback.name(),"failure");
            println!("COST items={count} schema={expected} elapsed_ms={:.3} error_kind={:?}",elapsed.as_secs_f64()*1000.0,field(&delivery.payload,"kind"));
        }
    }
    let schema=ValueSchema::Array{items:Box::new(ValueSchema::integer()),max_items:None};
    let value=UiValue::Array(vec![UiValue::Null;900_000]);
    let errors=schema.validate_ui_value(&value).unwrap_err();
    let message=errors.to_string();
    assert_eq!(errors.issues.len(),900_000);
    println!("SCHEMA intermediates issues={} formatted_message_bytes={} final_bound=1027",errors.issues.len(),message.len());
}

#[test]
fn callback_failure_does_not_retry_error_and_retained_guard_remains() {
    let throwing=APP.replace("let size = out.len();","throw \"callback-failed\";");
    let (engine,lifecycle,runtime,root,seen)=setup(&throwing,Ok(UiValue::String("ok".into())),ValueSchema::UiValue);
    kick(&engine,&lifecycle);
    let delivery=wait_tasks(&runtime,&lifecycle).pop().unwrap();
    assert!(lifecycle.invoke_async_delivery_transactional(&engine,delivery).is_err());
    assert_eq!(*seen.borrow(),1);
    assert_eq!(runtime.borrow().component_state.get(&root,"errors"),Some(&UiValue::Integer(0)));
    assert!(runtime.borrow_mut().tasks.drain(lifecycle.generation()).is_empty());
    println!("CALLBACK execution_failure_not_retried=true external_effects=1 error_callback=0");
    let recursion=APP.replace("let size = out.len();","loaded(ctx, out);");
    let (engine,lifecycle,runtime,_,_)=setup(&recursion,Ok(UiValue::String("ok".into())),ValueSchema::UiValue);
    kick(&engine,&lifecycle);
    let delivery=wait_tasks(&runtime,&lifecycle).pop().unwrap();
    let error=lifecycle.invoke_async_delivery_transactional(&engine,delivery).unwrap_err().to_string();
    assert!(error.contains("Stack overflow"));
    println!("GUARD actual_recursion_still_limited=true");
}
