use gpui_rhai::*;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    time::{Duration, Instant},
};

fn main() {
    // Standalone windows share UiRuntimeState, but retained NodeId allocation is per tree.
    let shared = Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut ea = RuntimeEngine::new();
    let ca = ea
        .compile("fn view(ctx) { column([text(\"child a\")]) }")
        .unwrap();
    let pa = ComponentInstancePath::root("View", "a");
    let mut la = ScriptLifecycle::new(
        ca,
        shared.clone(),
        pa.clone(),
        Some("a".into()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    la.start(&mut ea).unwrap();
    let ra = la.retained().root_id().unwrap();
    let child = la.retained().nodes().find(|n| n.id() != ra).unwrap().id();
    let ga = GeometryBounds::new(0.0, 0.0, 100.0, 20.0).unwrap();
    let gb = GeometryBounds::new(0.0, 0.0, 300.0, 20.0).unwrap();
    shared.borrow().geometry.update(
        child,
        ElementGeometry {
            layout: ga,
            visual: ga,
            clip: None,
        },
    );
    shared.borrow().pointer_capture.capture(1, child);
    let mut eb = RuntimeEngine::new();
    let cb = eb.compile("fn view(ctx) { text(\"window b\") }").unwrap();
    let mut lb = ScriptLifecycle::new(
        cb,
        shared.clone(),
        ComponentInstancePath::root("View", "b"),
        Some("b".into()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    lb.start(&mut eb).unwrap();
    let rb = lb.retained().root_id().unwrap();
    println!(
        "shared_windows root_id_collision={} a_child_geometry_lost={} a_capture_lost={}",
        ra == rb,
        shared.borrow().geometry.read(child, &pa).is_err(),
        shared.borrow().pointer_capture.captured(1).is_none()
    );
    shared.borrow().geometry.update(
        ra,
        ElementGeometry {
            layout: ga,
            visual: ga,
            clip: None,
        },
    );
    shared.borrow().geometry.update(
        rb,
        ElementGeometry {
            layout: gb,
            visual: gb,
            clip: None,
        },
    );
    println!(
        "shared_windows a_root_expected_width=100 actual_width={}",
        shared.borrow().geometry.read(ra, &pa).unwrap().visual.width
    );

    // Program identity must distinguish candidates, not just the last committed counter.
    let mut e = RuntimeEngine::new();
    let a = e
        .compile("fn view() { text(\"a\") } fn callback() { \"a\" }")
        .unwrap();
    let cb = e.callback(&a, "callback").unwrap();
    let b = e
        .compile("fn view() { text(\"b\") } fn callback() { \"b\" }")
        .unwrap();
    e.render(&b).unwrap();
    println!(
        "candidate_alias a={} b={} stale_a_with_a={:?} stale_a_with_b={:?}",
        a.generation(),
        b.generation(),
        e.invoke_callback(&a, &cb, ()),
        e.invoke_callback(&b, &cb, ())
    );

    // A temporary suspension is a separate cause from an explicit user pause.
    let now = Instant::now();
    let owner = ComponentInstancePath::root("Owner", "pause");
    let tid = TimerId::new(owner.clone(), "one").unwrap();
    let mut timers = TimerRegistry::new();
    let descriptor = TimerDescriptor::new(
        tid.clone(),
        Duration::from_millis(100),
        false,
        cb.clone(),
        UiValue::Null,
    )
    .unwrap();
    timers.reconcile(&owner, BTreeMap::from([(tid.clone(), descriptor)]), now);
    assert!(timers.pause(&tid, now + Duration::from_millis(20)));
    timers.pause_component_scope(&owner, now + Duration::from_millis(30));
    timers.resume_component_scope(&owner, now + Duration::from_secs(1));
    println!(
        "pause_before_suspend_restored={} unexpectedly_fired={}",
        timers.inspect(now + Duration::from_secs(1))[0].interaction_paused,
        timers
            .drain(now + Duration::from_secs(2), a.generation())
            .len()
    );

    // A rejected insertion must not replace the provider of a live namespace.
    let assets = AssetRegistry::new();
    let data = AssetData {
        mime_type: "image/svg+xml".into(),
        bytes: b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'></svg>".to_vec(),
    };
    assets
        .register(
            "app",
            InMemoryAssetProvider::new(BTreeMap::from([
                ("a".into(), data.clone()),
                ("b".into(), data),
            ])),
        )
        .unwrap();
    assets
        .load_image(&AssetId::parse("app/a").unwrap())
        .unwrap();
    let duplicate = assets.register("app", InMemoryAssetProvider::default());
    println!(
        "duplicate_provider_rejected={} cached_a_ok={} previously_valid_b_ok={}",
        duplicate.is_err(),
        assets.load_image(&AssetId::parse("app/a").unwrap()).is_ok(),
        assets.load_image(&AssetId::parse("app/b").unwrap()).is_ok()
    );

    // Deserialization must not make state schema validation optional.
    let bad: ComponentStateSchema = serde_json::from_str(r#"{"fields":{"count":{"schema":{"type":"integer"},"default":{"type":"string","value":"wrong"}}}}"#).unwrap();
    let state_mount = StateStore::new().mount_instance(owner.clone(), &bad);
    let mut stores = StoreRegistry::new();
    let sid = StoreId::app("bad");
    let declaration = stores.declare(sid.clone(), bad);
    println!(
        "bad_schema_component_rejected={} store_accepted={} value={:?}",
        state_mount.is_err(),
        declaration.is_ok(),
        stores.read_tracked(&owner, &sid, "count")
    );

    // Component incarnation must be distinct from the reusable logical key.
    let module = r#"
define_component(#{
 metadata: #{ id:"components/incarnation", "export":"Counter", version:"0.1.1", runtime_api:#{min_inclusive:1,max_exclusive:2}, dependencies:[],capabilities:#{} },
 schema: #{ props: #{key:#{schema:#{type:"string"},required:true,sensitive:false}}, state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}, events:#{}, slots:#{},parts:["root"] }, render:Fn("render_Counter")
});
fn Counter(props) { render_component("components/incarnation", props) }
fn increment(ctx,payload) { ctx.set_state("count",ctx.get_state("count")+1); }
fn render_Counter(ctx,props) {
 let progress=signal("progress",0.0);
 let field=element_ref("field");
 text(`${ctx.get_state("count")}`).bind_signal("opacity",progress).with_key("field").with_ref(field).on_click(Fn("increment"))
}
"#;
    let mut resolver = RestrictedModuleResolver::new();
    resolver.insert("components/incarnation", module).unwrap();
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(resolver);
    let source = r#"import "components/incarnation" as c; fn view(ctx) { if ctx.get_state("visible") { c::Counter(#{key:"same"}) } else { text("hidden") } }"#;
    let compiled = engine
        .compile_self_contained_named("incarnation", source)
        .unwrap();
    let state = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root = ComponentInstancePath::root("App", "incarnation");
    let schema = ComponentStateSchema::new(BTreeMap::from([(
        "visible".into(),
        StateField::new(ValueSchema::Bool, UiValue::Bool(true)),
    )]))
    .unwrap();
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
    let component = engine
        .component_invocations()
        .next()
        .unwrap()
        .path()
        .clone();
    let old_cb = match lifecycle.root().unwrap().handlers()["click"][0].handler() {
        UiEventHandler::Script(cb) => cb.clone(),
        _ => unreachable!(),
    };
    let old_signal = state
        .borrow()
        .signals
        .resolve(&component, "progress")
        .unwrap();
    let old_ref = lifecycle.root().unwrap().element_ref().unwrap().clone();
    let old_node = state.borrow().element_refs.resolve(&old_ref).unwrap();
    state
        .borrow_mut()
        .set_component_state_from_host(&root, "visible", UiValue::Bool(false))
        .unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    let signal_stale = state.borrow().signals.read(&old_signal).is_err();
    state
        .borrow_mut()
        .set_component_state_from_host(&root, "visible", UiValue::Bool(true))
        .unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    let signal_write = state
        .borrow_mut()
        .signals
        .write(&old_signal, SignalValue::Float(0.75));
    let callback_result = lifecycle.invoke_callback_transactional(&engine, &old_cb, UiValue::Null);
    let new_node = state.borrow().element_refs.resolve(&old_ref).unwrap();
    println!(
        "remount_signal_was_stale={} old_signal_write={:?} old_callback_accepted={} new_instance_count={:?} old_ref_rebound={} old_node={} new_node={}",
        signal_stale,
        signal_write,
        callback_result.is_ok(),
        state.borrow().component_state.get(&component, "count"),
        old_node != new_node,
        old_node,
        new_node
    );
    println!(
        "host_signal_nan_accepted={}",
        state
            .borrow_mut()
            .signals
            .write(&old_signal, SignalValue::Float(f64::NAN))
            .is_ok()
    );

    // Retained timer identity includes the curry value, even if its function name does not change.
    let timer_module = r#"
define_component(#{metadata:#{id:"components/timer", "export":"Timer", version:"0.1.1",runtime_api:#{min_inclusive:1,max_exclusive:2},dependencies:[],capabilities:#{}}, schema:#{props:#{value:#{schema:#{type:"integer"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:["root"]},render:Fn("render_Timer")});
fn Timer(props) { render_component("components/timer", props) }
fn fired(value,ctx,payload) { () }
fn render_Timer(ctx,props) { timeout("one",100,false,Fn("fired").curry(props.value),()); text("timer") }
"#;
    let mut resolver = RestrictedModuleResolver::new();
    resolver.insert("components/timer", timer_module).unwrap();
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(resolver);
    let compiled=engine.compile_self_contained_named("timer-curry",r#"import "components/timer" as t; fn view(ctx) { t::Timer(#{value:ctx.get_state("value")}) }"#).unwrap();
    let state = Rc::new(RefCell::new(UiRuntimeState::new()));
    let root = ComponentInstancePath::root("App", "timer-curry");
    let schema = ComponentStateSchema::new(BTreeMap::from([(
        "value".into(),
        StateField::new(ValueSchema::integer(), UiValue::Integer(1)),
    )]))
    .unwrap();
    let generation = compiled.generation();
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
    let first = state
        .borrow_mut()
        .timers
        .drain(Instant::now() + Duration::from_secs(1), generation)
        .len();
    state
        .borrow_mut()
        .set_component_state_from_host(&root, "value", UiValue::Integer(2))
        .unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    println!(
        "changed_curry_completed_timer first_fired={} rearmed_active={}",
        first,
        state.borrow().timers.active_count()
    );
    // Backpressure must not be confused with end-of-stream by the built-in receiver adapter.
    struct ReceiverStream(Option<std::sync::mpsc::Receiver<UiValue>>);
    impl SubscriptionCapabilityHandler for ReceiverStream {
        fn subscribe(&mut self, _: &str, _: UiValue) -> Result<SubscriptionWork, String> {
            Ok(SubscriptionWork::from_receiver(self.0.take().unwrap()))
        }
    }
    let (tx, rx) = std::sync::mpsc::channel();
    for i in 0..65 {
        tx.send(UiValue::Integer(i)).unwrap();
    }
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let id = CapabilityId::parse("app.stream").unwrap();
    runtime
        .borrow_mut()
        .capabilities
        .register_subscription(
            CapabilityDescriptor {
                id: id.clone(),
                version: "1.0.0".parse().unwrap(),
                methods: BTreeMap::from([(
                    "watch".into(),
                    CapabilityMethod {
                        input: ValueSchema::Null,
                        output: ValueSchema::integer(),
                    },
                )]),
            },
            ReceiverStream(Some(rx)),
        )
        .unwrap();
    runtime
        .borrow_mut()
        .capabilities
        .activate(&BTreeMap::from([(id, "^1".parse().unwrap())]))
        .unwrap();
    let mut engine = RuntimeEngine::new();
    let compiled = engine.compile(r#"
define_component(#{metadata:#{id:"components/stream", "export":"Stream", version:"0.1.1",runtime_api:#{min_inclusive:1,max_exclusive:2},dependencies:[],capabilities:#{}}, schema:#{props:#{},state:#{fields:#{}},events:#{},slots:#{},parts:["root"],effects:["stream"]},render:Fn("render_Stream")});
fn render_Stream(ctx,props) { effect("stream",(),Fn("start"),Fn("stop")); text("stream") }
fn start(ctx,dep) { ctx.start_subscription("app.stream","watch",(),Fn("received"),Fn("failed"),#{delivery:"all",capacity:64}); }
fn stop(ctx,dep) { () }
fn received(ctx,value) { () }
fn failed(ctx,value) { () }
fn view(ctx) { render_component("components/stream",#{}) }
"#).unwrap();
    let generation = compiled.generation();
    let mut lifecycle = ScriptLifecycle::new(
        compiled,
        runtime.clone(),
        ComponentInstancePath::root("App", "stream"),
        None,
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    lifecycle.start(&mut engine).unwrap();
    let mut closed = false;
    for _ in 0..300 {
        if tx.send(UiValue::Integer(999)).is_err() {
            closed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let values = runtime.borrow_mut().subscriptions.drain(generation);
    println!(
        "receiver_adapter preloaded=65 source_disconnected={} delivered={} active={}",
        closed,
        values.len(),
        runtime.borrow().subscriptions.active_count()
    );
}
