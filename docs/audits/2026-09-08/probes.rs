use gpui_rhai::*;
use gpui_rhai_cli::Project;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant},
};

fn main() {
    let dir = std::env::temp_dir().join(format!("gpui-rhai-audit-owned-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let imported = dir.join("owned_fixture.rhai");
    fs::write(&imported, "fn value() { \"external fixture loaded\" }").unwrap();
    let mut engine = RuntimeEngine::new();
    let script = format!(
        "import {:?} as fixture; fn view() {{ text(fixture::value()) }}",
        imported.with_extension("").to_str().unwrap()
    );
    let compiled = engine.compile(&script).unwrap();
    println!(
        "default_resolver_absolute_import_accepted={}",
        engine.render(&compiled).is_ok()
    );

    let panic_result = catch_unwind(AssertUnwindSafe(|| {
        let mut engine = RuntimeEngine::new();
        let compiled = engine
            .compile("fn view() { const m = #{ x: 1 }; m.x = 2; text(\"done\") }")
            .unwrap();
        engine.render(&compiled)
    }));
    println!("const_map_panicked={}", panic_result.is_err());

    let mut engine = RuntimeEngine::new();
    let compiled = engine.compile("fn view() { text(\"probe\") } fn success(ctx, value) { value } fn failure(ctx, value) { value }").unwrap();
    engine.render(&compiled).unwrap();
    let success = engine.callback(&compiled, "success").unwrap();
    let failure = engine.callback(&compiled, "failure").unwrap();
    let generation = compiled.generation();
    let mut state = UiRuntimeState::new();
    let (handle, emitter) = state.subscriptions.subscribe(SubscriptionRegistration::new(
        "audit",
        AsyncScope::App,
        generation,
        success.clone(),
        failure.clone(),
        ValueSchema::string(),
    ));
    let snapshot = state.snapshot().unwrap();
    assert!(state.subscriptions.cancel(handle));
    state.restore(snapshot).unwrap();
    println!(
        "cancel_rollback_subscription_active={} emitter_closed={:?}",
        state.subscriptions.active_count(),
        emitter.close_reason()
    );

    let (done_tx, done_rx) = mpsc::channel();
    struct Done(mpsc::Sender<()>);
    impl Drop for Done {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }
    state
        .tasks
        .spawn(
            AsyncScope::App,
            generation,
            success,
            failure,
            ValueSchema::string(),
            move || {
                let _done = Done(done_tx);
                panic!("intentional audit worker panic");
            },
        )
        .unwrap();
    done_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    println!(
        "worker_panic_deliveries={} active={}",
        state.tasks.drain(generation).len(),
        state.tasks.active_count()
    );

    let nan = rhai::Dynamic::from(f64::NAN);
    let valid = ValueSchema::UiValue.validate(&nan).is_ok();
    let value = UiValue::from_dynamic(nan).unwrap();
    let json = serde_json::to_string(&value).unwrap();
    println!(
        "nan_ui_value_accepted={valid} reflexive={} json={json} roundtrip={}",
        value == value.clone(),
        serde_json::from_str::<UiValue>(&json).is_ok()
    );

    for (label, source) in [
        (
            "nested_comment",
            "/* outer /* inner */ import \"../not_an_import\"; */ fn view() { text(\"ok\") }",
        ),
        (
            "backtick_string",
            "fn view() { text(`example: import \"../not_an_import\" as demo;`) }",
        ),
    ] {
        let mut engine = RuntimeEngine::new();
        println!(
            "lexer_{label}_rhai_compile={} import_scan={:?}",
            engine.compile(source).is_ok(),
            extract_imports(source)
        );
    }

    let project_dir = dir.join("partial-project");
    fs::create_dir_all(&project_dir).unwrap();
    let original = "[package]\nname = \"audit-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";
    fs::write(project_dir.join("Cargo.toml"), original).unwrap();
    let plan = Project::new(&project_dir).plan_init().unwrap();
    fs::create_dir_all(
        project_dir
            .join("src")
            .join(format!(".gpui-rhai-tmp-{}-1", std::process::id())),
    )
    .unwrap();
    let result = plan.apply();
    println!(
        "cli_partial_apply_failed={} cargo_already_changed={} ui_main_exists={}",
        result.is_err(),
        fs::read_to_string(project_dir.join("Cargo.toml")).unwrap() != original,
        project_dir.join("ui/main.rhai").exists()
    );

    let payload = UiValue::Array(
        (0..10_000)
            .map(|i| {
                UiValue::Map(BTreeMap::from([
                    ("id".into(), UiValue::Integer(i)),
                    ("label".into(), UiValue::String("x".repeat(100))),
                ]))
            })
            .collect(),
    );
    let schema = ComponentStateSchema::new(BTreeMap::from([(
        "items".into(),
        StateField::new(ValueSchema::UiValue, payload),
    )]))
    .unwrap();
    let mut runtime = UiRuntimeState::new();
    runtime
        .stores
        .declare(StoreId::app("bulk"), schema)
        .unwrap();
    let mut samples = vec![];
    for _ in 0..30 {
        let start = Instant::now();
        std::hint::black_box(runtime.snapshot().unwrap());
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "debug_snapshot_10000_map_items_p50_ms={:.3} p95_ms={:.3}",
        samples[15], samples[28]
    );

    for count in [200, 400, 800, 1600] {
        let mut store = StateStore::new();
        let start = Instant::now();
        for i in 0..count {
            store
                .mount_instance(
                    ComponentInstancePath::root("Empty", i.to_string()),
                    &ComponentStateSchema::default(),
                )
                .unwrap();
        }
        println!(
            "debug_mount_empty_instances count={count} elapsed_ms={:.3}",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }

    let nested_module = r#"
define_component(#{ metadata: #{ id: "components/heavy", "export": "Heavy", version: "0.1.0", runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} }, schema: #{ props: #{}, state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"] }, render: Fn("render_Heavy") });
fn Heavy(props) { render_component("components/heavy", props) }
fn render_Heavy(ctx, props) { let n = 0; for i in 0..200000 { n += 1; } text(`${n}`) }
"#;
    let mut resolver = RestrictedModuleResolver::new();
    resolver.insert("components/heavy", nested_module).unwrap();
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(resolver);
    let compiled = engine.compile_self_contained_named("nested-budget", "import \"components/heavy\" as h; fn view() { column([h::Heavy(#{}), h::Heavy(#{}), h::Heavy(#{}), h::Heavy(#{})]) }").unwrap();
    let _ = engine.take_timings();
    let result = engine.render(&compiled);
    let measured: u64 = engine.take_timings().iter().map(|t| t.operations).sum();
    println!(
        "nested_800000_iterations_accepted={} recorded_operations={measured}",
        result.is_ok()
    );
    engine.clear_component_exports().unwrap();
    let mut bad_resolver = RestrictedModuleResolver::new();
    bad_resolver
        .insert(
            "components/heavy",
            format!("let mutable_global = 1;\n{nested_module}"),
        )
        .unwrap();
    engine.set_module_resolver(bad_resolver);
    let preload =
        engine.compile_self_contained_named("reload-preload", "import \"components/heavy\" as h;");
    println!(
        "failed_preload={} old_generation_render_after_clear={:?}",
        preload.is_err(),
        engine.render(&compiled).map(|_| ())
    );

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

    for (root_loops, callback_loops) in [(0, 100_000), (250_000, 100_000), (300_000, 100_000)] {
        let module = format!(
            r#"
define_component(#{{ metadata: #{{ id: "components/probe", "export": "Probe", version: "0.1.0", runtime_api: #{{ min_inclusive: 1, max_exclusive: 2 }}, dependencies: [], capabilities: #{{}} }}, schema: #{{ props: #{{}}, state: #{{ fields: #{{}} }}, events: #{{}}, slots: #{{}}, parts: ["root"] }}, render: Fn("render_Probe") }});
fn Probe(props) {{ render_component("components/probe", props) }}
fn render_Probe(ctx, props) {{ text("probe").on_click(Fn("clicked")) }}
fn clicked(ctx, payload) {{ let n = 0; for i in 0..{callback_loops} {{ n += 1; }} n }}
"#
        );
        let mut resolver = RestrictedModuleResolver::new();
        resolver.insert("components/probe", module).unwrap();
        let mut engine = RuntimeEngine::new();
        engine.set_module_resolver(resolver);
        let source = format!(
            "import \"components/probe\" as p; fn view(ctx) {{ let n = 0; for i in 0..{root_loops} {{ n += 1; }} p::Probe(#{{}}) }}"
        );
        let compiled = engine
            .compile_self_contained_named("counter-probe", &source)
            .unwrap();
        let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
        let mut lifecycle = ScriptLifecycle::new(
            compiled,
            runtime,
            ComponentInstancePath::root("App", "probe"),
            None,
            BTreeMap::new(),
            &ComponentStateSchema::default(),
        )
        .unwrap();
        match lifecycle.start(&mut engine) {
            Err(e) => println!("callback_ops root_loops={root_loops} mount_error={e}"),
            Ok(root) => {
                let UiEventHandler::Script(cb) = root.handlers()["click"][0].handler() else {
                    unreachable!()
                };
                let cb = cb.clone();
                let render_ops: u64 = engine.take_timings().iter().map(|x| x.operations).sum();
                let result = lifecycle.invoke_callback_transactional(&engine, &cb, UiValue::Null);
                let callback_ops: u64 = engine.take_timings().iter().map(|x| x.operations).sum();
                println!(
                    "callback_ops root_loops={root_loops} callback_loops={callback_loops} render_ops={render_ops} callback_ops={callback_ops} result={result:?}"
                );
            }
        }
    }
}
