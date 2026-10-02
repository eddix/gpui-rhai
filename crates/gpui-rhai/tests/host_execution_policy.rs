use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

const THEME: &str = include_str!("../../../registry/themes/default_dark.rhai");
const FINITE_WORK: &str = r"fn work() { let n = 0; for i in 0..450000 { n += 1; } n }
fn view() { text(`${work()}`) }
";

fn text_value(node: &UiNode) -> &str {
    match node.kind() {
        UiNodeKind::Text { text } => text.as_str(),
        kind => panic!("expected text, received {kind:?}"),
    }
}

fn render_timing(engine: &RuntimeEngine) -> ExecutionTiming {
    engine
        .take_timings()
        .into_iter()
        .find(|timing| timing.operation == ExecutionOperation::Render)
        .expect("real render timing")
}

#[test]
fn same_finite_work_exceeds_default_and_obeys_raised_and_lowered_quotas() {
    let mut engine = RuntimeEngine::new();
    let compiled = engine.compile(FINITE_WORK).unwrap();
    let _ = engine.take_timings();
    assert!(engine.render(&compiled).is_err());
    let rejected = render_timing(&engine);
    assert!(!rejected.succeeded);
    assert!(rejected.operations > DEFAULT_SCRIPT_OPERATION_LIMIT);
    assert_eq!(rejected.operation_limit, DEFAULT_SCRIPT_OPERATION_LIMIT);

    engine.set_operation_limit(2_000_000);
    assert_eq!(text_value(&engine.render(&compiled).unwrap()), "450000");
    let admitted = render_timing(&engine);
    assert!(admitted.succeeded);
    assert!(admitted.operations > DEFAULT_SCRIPT_OPERATION_LIMIT);
    assert!(admitted.operations < 2_000_000);
    assert_eq!(admitted.operation_limit, 2_000_000);
    assert!(admitted.duration > std::time::Duration::ZERO);

    // Another independently fresh round gives exactly the same result/count.
    assert_eq!(text_value(&engine.render(&compiled).unwrap()), "450000");
    assert_eq!(render_timing(&engine).operations, admitted.operations);
    engine.set_operation_limit(500_000);
    assert!(engine.render(&compiled).is_err());
    let lowered = render_timing(&engine);
    assert!(!lowered.succeeded);
    assert!(lowered.operations > 500_000);
    assert_eq!(lowered.operation_limit, 500_000);
    println!(
        "same finite workload: default={}, raised={}, lowered={}, raised_duration_us={} (quota is not a frame guarantee)",
        rejected.operations,
        admitted.operations,
        lowered.operations,
        admitted.duration.as_micros()
    );
}

const FORMAL: &str = r#"
define_component(#{metadata:#{id:"tests/heavy", "export":"Heavy", version:"0.1.8",
 runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
 schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},
 iterations:#{schema:#{type:"integer"},required:true,sensitive:false},
 nested:#{schema:#{type:"bool"},required:true,sensitive:false}},
 state:#{fields:#{heavy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}},
 events:#{},slots:#{},parts:["root"]},render:Fn("heavy")});
fn heavy(ctx,props){let iterations=if ctx.get_state("heavy"){100000}else{props.iterations};
 let n=0;for i in 0..iterations{n+=1;}
 if props.nested {column([text(`${n}`),render_component("tests/heavy",#{key:"child",iterations:100000,nested:false})])}
 else{text(`${n}`)}}
"#;

fn lifecycle(
    source: &str,
    quota: u64,
) -> (RuntimeEngine, ScriptLifecycle, Rc<RefCell<UiRuntimeState>>) {
    let mut engine = RuntimeEngine::new();
    engine.set_operation_limit(quota);
    let compiled = engine.compile_named("host-policy", source).unwrap();
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let life = ScriptLifecycle::new(
        compiled,
        runtime.clone(),
        ComponentInstancePath::root("App", "policy"),
        None,
        BTreeMap::new(),
        &schema,
    )
    .unwrap();
    (engine, life, runtime)
}

#[test]
fn nested_and_sibling_components_still_share_a_nondefault_round_quota() {
    let one = r#"render_component("tests/heavy",#{key:"one",iterations:100000,nested:false})"#;
    let nested = r#"render_component("tests/heavy",#{key:"parent",iterations:100000,nested:true})"#;
    let siblings = r#"column([render_component("tests/heavy",#{key:"one",iterations:100000,nested:false}),render_component("tests/heavy",#{key:"two",iterations:100000,nested:false})])"#;
    for (label, body, succeeds) in [
        ("one", one, true),
        ("nested", nested, false),
        ("siblings", siblings, false),
    ] {
        let source = format!("{FORMAL}\nfn view(ctx){{{body}}}");
        let (mut engine, mut life, _) = lifecycle(&source, 500_000);
        let result = life.start(&mut engine);
        assert_eq!(result.is_ok(), succeeds, "{label}: {result:?}");
        let timing = render_timing(&engine);
        assert_eq!(timing.operation_limit, 500_000);
        if succeeds {
            assert!(timing.operations < 500_000);
        } else {
            assert!(timing.operations > 500_000);
        }
    }

    // The same cumulative contract holds when sibling targets are reevaluated
    // together after an initially cheap mount, not only during initial render.
    let source = format!(
        r#"{FORMAL}
fn view(ctx){{column([render_component("tests/heavy",#{{key:"one",iterations:0,nested:false}}),render_component("tests/heavy",#{{key:"two",iterations:0,nested:false}})])}}"#
    );
    let (mut engine, mut life, runtime) = lifecycle(&source, 500_000);
    life.start(&mut engine).unwrap();
    let _ = engine.take_timings();
    let owners = engine
        .component_invocations()
        .map(|recipe| recipe.path().clone())
        .collect::<Vec<_>>();
    assert_eq!(owners.len(), 2);
    for owner in owners {
        runtime
            .borrow_mut()
            .set_component_state_from_host(&owner, "heavy", UiValue::Bool(true))
            .unwrap();
    }
    let error = life.render_dirty(&mut engine).unwrap_err();
    let failed = engine.last_failed_timing().unwrap();
    assert_eq!(failed.operation_limit, 500_000);
    assert!(failed.round_operations > 500_000);
    assert!(
        failed.operations < 500_000,
        "the second component consumes only the remainder of the shared round"
    );
    let LifecycleError::Runtime(runtime_error) = error else {
        panic!("expected Rhai operation rejection, got {error:?}")
    };
    engine.set_operation_limit(2_000_000);
    let diagnostic = Diagnostic::from_runtime(
        &runtime_error,
        &DiagnosticContext {
            execution_timing: Some(failed.clone()),
            ..DiagnosticContext::default()
        },
    );
    let budget = diagnostic.operation_budget.unwrap();
    assert_eq!(budget.consumed, failed.round_operations);
    assert_eq!(
        budget.maximum, 500_000,
        "diagnostics retain quota from start, not later Host changes"
    );
    assert!(budget.consumed > budget.maximum);
    assert!(
        engine
            .take_timings()
            .iter()
            .filter(|timing| timing.operation == ExecutionOperation::Render)
            .map(|timing| timing.operations)
            .sum::<u64>()
            > 500_000
    );
}

#[test]
fn retained_component_and_delayed_delivery_begin_fresh_nondefault_rounds() {
    let mut counts = Vec::new();
    for parent_iterations in [0, 100_000] {
        let source = format!(
            r#"{FORMAL}
fn view(ctx){{let n=0;for i in 0..{parent_iterations}{{n+=1;}}
 render_component("tests/heavy",#{{key:"child",iterations:0,nested:false}})}}"#
        );
        let (mut engine, mut life, runtime) = lifecycle(&source, 500_000);
        life.start(&mut engine).unwrap();
        let owner = engine
            .component_invocations()
            .next()
            .unwrap()
            .path()
            .clone();
        let _ = engine.take_timings();
        runtime
            .borrow_mut()
            .set_component_state_from_host(&owner, "heavy", UiValue::Bool(true))
            .unwrap();
        assert!(life.render_dirty(&mut engine).unwrap());
        let timing = render_timing(&engine);
        assert!(timing.operations < 500_000);
        assert_eq!(timing.operation_limit, 500_000);
        counts.push(timing.operations);
    }
    assert_eq!(
        counts[0], counts[1],
        "retained work must not recount parent history"
    );

    let source = r"
fn view(ctx){let n=0;for i in 0..100000{n+=1;}text(`${n}`)}
fn delayed(ctx,payload){let n=0;for i in 0..100000{n+=1;}n}
";
    let (mut engine, mut life, _) = lifecycle(source, 500_000);
    life.start(&mut engine).unwrap();
    let _ = engine.take_timings();
    let callback = engine.callback(&life.compiled(), "delayed").unwrap();
    for _ in 0..3 {
        let result = life
            .invoke_async_delivery_transactional(
                &engine,
                AsyncDelivery {
                    callback: callback.clone(),
                    payload: UiValue::Null,
                    scope: AsyncScope::App,
                },
            )
            .unwrap();
        assert_eq!(result.as_int().unwrap(), 100_000);
        let timings = engine.take_timings();
        let timing = timings
            .iter()
            .find(|entry| matches!(entry.operation, ExecutionOperation::Callback(_)))
            .unwrap();
        assert!(timing.operations < 500_000);
        assert_eq!(timing.operation_limit, 500_000);
    }
}

#[test]
fn restricted_imports_obey_quota_and_failed_reload_preserves_active_policy() {
    let mut resolver = RestrictedModuleResolver::new();
    resolver
        .insert("work", "fn work(){let n=0;for i in 0..450000{n+=1;}n}")
        .unwrap();
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(resolver);
    engine.set_operation_limit(2_000_000);
    let compiled = engine
        .compile_self_contained_named(
            "imports",
            r#"import "work" as work;fn view(){text(`${work::work()}`)}"#,
        )
        .unwrap();
    assert_eq!(text_value(&engine.render(&compiled).unwrap()), "450000");
    engine.set_operation_limit(500_000);
    assert!(engine.render(&compiled).is_err());

    let source = r#"fn view(ctx){text("last-good")}fn after(ctx,payload){let n=0;for i in 0..100000{n+=1;}n}"#;
    let (mut engine, mut life, _) = lifecycle(source, 500_000);
    life.start(&mut engine).unwrap();
    let previous = life.generation();
    let old_callback = engine.callback(&life.compiled(), "after").unwrap();
    let candidate = engine.compile(FINITE_WORK).unwrap();
    assert!(
        life.reload(&mut engine, candidate, &ComponentStateSchema::default())
            .is_err()
    );
    assert_eq!(life.generation(), previous);
    assert_eq!(life.state(), LifecycleState::Running);
    assert_eq!(text_value(life.root().unwrap()), "last-good");
    assert_eq!(engine.operation_limit(), 500_000);
    assert_eq!(
        life.invoke_callback_transactional(&engine, &old_callback, UiValue::Null)
            .unwrap()
            .as_int()
            .unwrap(),
        100_000
    );
}

fn deep_source() -> String {
    format!(
        "fn view(ctx){{text({}\"depth-ok\"{})}}",
        "(".repeat(24),
        ")".repeat(24)
    )
}

#[derive(Clone)]
struct PolicyProbe {
    seen: Rc<RefCell<Vec<Policy>>>,
    override_policy: Option<Policy>,
}
type Policy = (u64, (usize, usize));
impl ScriptViewExtension for PolicyProbe {
    fn configure_engine(&self, engine: &mut RuntimeEngine) -> Result<(), String> {
        self.seen
            .borrow_mut()
            .push((engine.operation_limit(), engine.expression_depth_limits()));
        if let Some((quota, (global, functions))) = self.override_policy {
            engine.set_operation_limit(quota);
            engine.set_expression_depth_limits(global, functions);
        }
        Ok(())
    }
}

fn embedded(source: &str) -> EmbeddedScriptView {
    let entry = ModuleId::parse("main").unwrap();
    EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, source.into())])),
        THEME,
    )
}

fn file(source: &str) -> (tempfile::TempDir, FileScriptView) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("main.rhai"), source).unwrap();
    std::fs::write(dir.path().join("theme.rhai"), THEME).unwrap();
    std::fs::write(
        dir.path().join("app.toml"),
        "entry = \"main\"\nruntime_api = 2\n",
    )
    .unwrap();
    let builder = FileScriptView::new(dir.path().join("main.rhai")).development(false);
    (dir, builder)
}

#[test]
fn parser_depth_defaults_and_independent_host_configuration_cover_all_constructors() {
    let source = deep_source();
    for functions in [32, 48, 64] {
        let mut engine = RuntimeEngine::new();
        assert_eq!(engine.expression_depth_limits(), (64, 32));
        let data_limits = (
            engine.engine().max_call_levels(),
            engine.engine().max_array_size(),
            engine.engine().max_map_size(),
            engine.engine().max_string_size(),
        );
        engine.set_expression_depth_limits(64, functions);
        assert_eq!(
            (
                engine.engine().max_call_levels(),
                engine.engine().max_array_size(),
                engine.engine().max_map_size(),
                engine.engine().max_string_size()
            ),
            data_limits
        );
        let result = engine.compile(&source);
        assert_eq!(
            result.is_ok(),
            functions == 64,
            "function depth={functions}: {:?}",
            result.as_ref().err()
        );
        if let Ok(compiled) = result {
            assert_eq!(
                text_value(
                    &engine
                        .render_with_context(
                            &compiled,
                            UiContext::new(
                                Rc::new(RefCell::new(UiRuntimeState::new())),
                                ComponentInstancePath::root("App", "depth"),
                                None,
                                ExecutionPhase::Render,
                                BTreeMap::new()
                            )
                        )
                        .unwrap()
                ),
                "depth-ok"
            );
        }
        assert_eq!(
            embedded(&source)
                .expression_depth_limits(64, functions)
                .prepare()
                .is_ok(),
            functions == 64
        );
        let (_dir, builder) = file(&source);
        assert_eq!(
            builder
                .expression_depth_limits(64, functions)
                .prepare()
                .is_ok(),
            functions == 64
        );
    }
    assert!(embedded(&source).prepare().is_err());
    let (_dir, builder) = file(&source);
    assert!(builder.prepare().is_err());

    // Global expression depth is a distinct parser setting, not a function
    // call/data/operation budget. The parser counts more than one level per
    // parenthesized expression in pinned Rhai 1.26.
    let global = format!(
        "let value={}\"global\"{};fn view(){{text(value)}}",
        "(".repeat(24),
        ")".repeat(24)
    );
    let mut engine = RuntimeEngine::new();
    assert!(engine.compile(&global).is_ok());
    engine.set_expression_depth_limits(48, 64);
    assert!(engine.compile(&global).is_err());
    engine.set_expression_depth_limits(64, 32);
    assert!(engine.compile(&global).is_ok());
}

#[test]
fn builder_policy_precedes_extensions_and_zero_never_means_unlimited() {
    let mut engine = RuntimeEngine::new();
    engine.set_operation_limit(0);
    engine.set_expression_depth_limits(0, 0);
    assert_eq!(engine.operation_limit(), 1);
    assert_eq!(engine.expression_depth_limits(), (1, 1));
    assert!(engine.compile("fn view(){text(\"blocked\")}").is_err());

    for (quota, depths) in [(300_000, (64, 48)), (0, (0, 0))] {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let probe = PolicyProbe {
            seen: seen.clone(),
            override_policy: Some((2_000_000, (64, 64))),
        };
        let source = deep_source();
        embedded(&source)
            .operation_limit(quota)
            .expression_depth_limits(depths.0, depths.1)
            .extension(probe.clone())
            .prepare()
            .unwrap();
        let (_dir, builder) = file(&source);
        builder
            .operation_limit(quota)
            .expression_depth_limits(depths.0, depths.1)
            .extension(probe)
            .prepare()
            .unwrap();
        assert_eq!(
            *seen.borrow(),
            vec![(quota.max(1), (depths.0.max(1), depths.1.max(1))); 2]
        );
    }
}
