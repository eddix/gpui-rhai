#![cfg(target_arch = "aarch64")]

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use rhai::grain::{Compiler, SharedProgram, Vm};
use rhai::{
    CallFnOptions, Dynamic, Engine, EvalAltResult, FnPtr, NativeCallContext, Scope, Shared,
};
use rhai_jit_experiment::adaptive::{AdaptiveFunctionTier, AdaptiveTierConfig};

fn tier() -> AdaptiveFunctionTier {
    AdaptiveFunctionTier::new(AdaptiveTierConfig {
        min_calls: 1,
        min_self_grain_time: Duration::ZERO,
        ..AdaptiveTierConfig::default()
    })
}

fn program(engine: &Engine, source: &str) -> SharedProgram {
    let ast = engine.compile(source).unwrap();
    Shared::new(Compiler::new().compile(&ast))
}

fn call(
    engine: &Engine,
    program: &SharedProgram,
    tier: &AdaptiveFunctionTier,
    name: &str,
    arguments: Vec<Dynamic>,
) -> Result<Dynamic, Box<EvalAltResult>> {
    Vm::new(engine)
        .with_function_accelerator(tier.accelerator())
        .call_fn_with_callbacks(
            CallFnOptions::new().eval_ast(false),
            &mut Scope::new(),
            program,
            name,
            arguments,
        )
}

fn assert_native(tier: &AdaptiveFunctionTier, name: &str) {
    let stats = tier.stats();
    assert!(
        stats
            .iter()
            .any(|entry| entry.name == name && entry.jit_calls > 0),
        "{stats:#?}"
    );
}

#[test]
fn managed_values_and_float_arithmetic_match_ast_without_mutating_arguments() {
    let engine = Engine::new();
    let source = r"
        fn build(name, scale, values, enabled) {
            let result = #{ name: name, amount: scale * 2.0, enabled: enabled, values: values };
            result.values.push(7);
            result.name += `:${result.amount}`;
            result
        }
    ";
    let ast = engine.compile(source).unwrap();
    let program = program(&engine, source);
    let tier = tier();
    let values = Dynamic::from_array(vec![1_i64.into(), 2_i64.into()]);
    let arguments = vec![
        "widget".into(),
        1.25_f64.into(),
        values.clone(),
        true.into(),
    ];
    let expected: Dynamic = engine
        .call_fn(&mut Scope::new(), &ast, "build", arguments.clone())
        .unwrap();
    for _ in 0..3 {
        let result = call(&engine, &program, &tier, "build", arguments.clone()).unwrap();
        assert_eq!(format!("{result:?}"), format!("{expected:?}"));
    }
    assert_eq!(values.cast::<rhai::Array>().len(), 2);
    assert_native(&tier, "build");
}

#[test]
fn managed_iteration_branches_and_early_returns_match_ast() {
    let engine = Engine::new();
    let source = r"
        fn sum(values, enabled) {
            if !enabled { return 0.0; }
            let total = 0.0;
            for value in values {
                if value < 0.0 { continue; }
                total += value;
            }
            total
        }
    ";
    let program = program(&engine, source);
    let tier = tier();
    for enabled in [true, true, false] {
        let values = vec![1.5_f64.into(), (-1.0_f64).into(), 2.5_f64.into()];
        let result = call(
            &engine,
            &program,
            &tier,
            "sum",
            vec![Dynamic::from_array(values), enabled.into()],
        )
        .unwrap();
        assert_eq!(
            result.as_float().unwrap().to_bits(),
            if enabled {
                4.0_f64.to_bits()
            } else {
                0.0_f64.to_bits()
            }
        );
    }
    assert_native(&tier, "sum");
}

#[test]
fn a_native_callback_can_reenter_the_same_tier() {
    let mut engine = Engine::new();
    engine.register_fn(
        "bounce",
        |context: NativeCallContext, function: FnPtr, value: Dynamic| {
            function.call_within_context::<Dynamic>(&context, (value,))
        },
    );
    let source = r#"
        fn inner(value) { value + "!" }
        fn outer(value) { bounce(Fn("inner"), value) }
    "#;
    let program = program(&engine, source);
    let tier = tier();
    for _ in 0..4 {
        assert_eq!(
            call(&engine, &program, &tier, "outer", vec!["hello".into()])
                .unwrap()
                .into_string()
                .unwrap(),
            "hello!"
        );
    }
    assert_native(&tier, "outer");
    assert_native(&tier, "inner");
}

#[test]
fn errors_after_native_effects_do_not_replay_the_function() {
    let mut engine = Engine::new();
    let effects = Rc::new(Cell::new(0));
    let sink = Rc::clone(&effects);
    engine.register_fn("effect", move || sink.set(sink.get() + 1));
    let program = program(&engine, r"fn fail(value) { effect(); throw value; }");
    let tier = tier();
    for expected in 1..=3 {
        assert!(call(&engine, &program, &tier, "fail", vec!["failure".into()]).is_err());
        assert_eq!(effects.get(), expected);
    }
    assert_native(&tier, "fail");
}

struct Tracked(Rc<Cell<i64>>);

impl Tracked {
    fn new(live: &Rc<Cell<i64>>) -> Self {
        live.set(live.get() + 1);
        Self(Rc::clone(live))
    }
}

impl Clone for Tracked {
    fn clone(&self) -> Self {
        Self::new(&self.0)
    }
}

impl Drop for Tracked {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}

#[test]
fn arbitrary_host_values_are_dropped_after_success_and_error() {
    let engine = Engine::new();
    let program = program(
        &engine,
        r#"
        fn retain(value, fail) {
            let local = #{ value: value, nested: [value] };
            if fail { throw "failed"; }
            local
        }
    "#,
    );
    let tier = tier();
    let live = Rc::new(Cell::new(0));
    let value = Dynamic::from(Tracked::new(&live));
    for fail in [false, false, true, false, true] {
        let result = call(
            &engine,
            &program,
            &tier,
            "retain",
            vec![value.clone(), fail.into()],
        );
        assert_eq!(result.is_err(), fail);
        drop(result);
        assert_eq!(live.get(), 1);
    }
    assert_native(&tier, "retain");
    drop(value);
    assert_eq!(live.get(), 0);
}

#[test]
fn managed_loops_share_the_outer_operation_limit() {
    let mut engine = Engine::new();
    engine.set_max_operations(40);
    let program = program(
        &engine,
        "fn count(value) { while value < 1000.0 { value += 1.0; } value }",
    );
    let tier = tier();
    for _ in 0..3 {
        let error = call(&engine, &program, &tier, "count", vec![0.0_f64.into()]).unwrap_err();
        assert!(
            matches!(*error, EvalAltResult::ErrorTooManyOperations(_)),
            "{error}"
        );
    }
    assert_native(&tier, "count");
}

#[test]
fn imported_named_functions_and_private_helpers_enter_the_tier() {
    let mut engine = Engine::new();
    let ast = engine
        .compile(
            r#"
        private fn suffix(value) { value + "!" }
        fn build(value) { #{ title: suffix(value), values: [1.5, 2.5] } }
    "#,
        )
        .unwrap();
    let module = rhai::Module::eval_ast_as_new(Scope::new(), &ast, &engine).unwrap();
    let mut resolver = rhai::module_resolvers::StaticModuleResolver::new();
    resolver.insert("library", module);
    engine.set_module_resolver(resolver);
    let ast = engine
        .compile_into_self_contained(
            &Scope::new(),
            r#"
        import "library" as lib;
        fn entry() { lib::build("module") }
    "#,
        )
        .unwrap();
    let program = Shared::new(Compiler::new().compile(&ast));
    let tier = tier();
    for _ in 0..4 {
        let result: Dynamic = Vm::new(&engine)
            .with_function_accelerator(tier.accelerator())
            .call_fn_with_callbacks(
                CallFnOptions::new(),
                &mut Scope::new(),
                &program,
                "entry",
                (),
            )
            .unwrap();
        assert_eq!(
            result.cast::<rhai::Map>()["title"]
                .clone()
                .into_string()
                .unwrap(),
            "module!"
        );
    }
    assert_native(&tier, "build");
    assert_native(&tier, "suffix");
}

#[test]
fn repeated_native_reentry_cannot_reset_the_operation_budget() {
    let mut engine = Engine::new();
    engine.register_fn(
        "bounce",
        |context: NativeCallContext, function: FnPtr, value: Dynamic| {
            function.call_within_context::<Dynamic>(&context, (value,))
        },
    );
    let program = program(
        &engine,
        r#"
        fn inner(value) { let n = value; while n < value + 8.0 { n += 1.0; } n }
        fn outer(value) {
            bounce(Fn("inner"), value);
            bounce(Fn("inner"), value);
            bounce(Fn("inner"), value)
        }
    "#,
    );
    let tier = tier();
    for _ in 0..2 {
        let _ = call(&engine, &program, &tier, "outer", vec![0.0_f64.into()]).unwrap();
    }
    engine.set_max_operations(35);
    let error = call(&engine, &program, &tier, "outer", vec![0.0_f64.into()]).unwrap_err();
    assert!(
        matches!(*error, EvalAltResult::ErrorTooManyOperations(_)),
        "{error}"
    );
    assert_native(&tier, "outer");
    assert_native(&tier, "inner");
}

#[test]
fn scalar_float_comparisons_preserve_nan_and_signed_zero() {
    let engine = Engine::new();
    let source = r"
        fn equal(a, b) { a == b }
        fn unequal(a, b) { a != b }
        fn less(a, b) { a < b }
        fn less_equal(a, b) { a <= b }
        fn greater(a, b) { a > b }
        fn greater_equal(a, b) { a >= b }
    ";
    let ast = engine.compile(source).unwrap();
    let program = program(&engine, source);
    let tier = tier();
    for name in [
        "equal",
        "unequal",
        "less",
        "less_equal",
        "greater",
        "greater_equal",
    ] {
        for (a, b) in [
            (0.0, -0.0),
            (1.0, 2.0),
            (1.0, 1.0 + f64::EPSILON),
            (f64::MIN_POSITIVE, 0.0),
            (f64::NAN, 0.0),
            (0.0, f64::NAN),
            (f64::INFINITY, f64::INFINITY),
        ] {
            let expected: bool = engine
                .call_fn(&mut Scope::new(), &ast, name, (a, b))
                .unwrap();
            let actual = call(&engine, &program, &tier, name, vec![a.into(), b.into()])
                .unwrap()
                .as_bool()
                .unwrap();
            assert_eq!(actual, expected, "{name}({a}, {b})");
        }
        assert_native(&tier, name);
        assert_eq!(
            tier.stats()
                .iter()
                .find(|entry| entry.name == name)
                .unwrap()
                .native_lane,
            Some("scalar")
        );
    }
}

#[test]
fn guarded_map_reads_preserve_host_getters_and_missing_key_errors() {
    #[derive(Clone)]
    struct Object(i64);
    let mut engine = Engine::new();
    let reads = Rc::new(Cell::new(0));
    let sink = Rc::clone(&reads);
    engine.register_get("value", move |object: &mut Object| {
        sink.set(sink.get() + 1);
        object.0
    });
    let program = program(&engine, "fn field(object) { object.value }");
    let tier = tier();
    for _ in 0..3 {
        let mut map = rhai::Map::new();
        map.insert("value".into(), 7_i64.into());
        assert_eq!(
            call(
                &engine,
                &program,
                &tier,
                "field",
                vec![Dynamic::from_map(map)]
            )
            .unwrap()
            .as_int()
            .unwrap(),
            7
        );
        assert_eq!(
            call(
                &engine,
                &program,
                &tier,
                "field",
                vec![Dynamic::from(Object(9))]
            )
            .unwrap()
            .as_int()
            .unwrap(),
            9
        );
    }
    assert_eq!(reads.get(), 3);
    assert!(
        call(
            &engine,
            &program,
            &tier,
            "field",
            vec![Dynamic::from_map(rhai::Map::new())]
        )
        .unwrap()
        .is_unit()
    );
    engine.set_fail_on_invalid_map_property(true);
    let error = call(
        &engine,
        &program,
        &tier,
        "field",
        vec![Dynamic::from_map(rhai::Map::new())],
    )
    .unwrap_err();
    assert!(
        matches!(
            error.unwrap_inner(),
            EvalAltResult::ErrorPropertyNotFound(..)
        ),
        "{error}"
    );
    assert_native(&tier, "field");
}

#[test]
#[allow(deprecated)]
fn retained_callbacks_start_with_a_fresh_budget() {
    use std::cell::RefCell;
    let retained = Rc::new(RefCell::new(None));
    let sink = Rc::clone(&retained);
    let mut engine = Engine::new();
    engine.register_fn(
        "retain",
        move |context: NativeCallContext, function: FnPtr| {
            *sink.borrow_mut() = Some((context.store_data(), function));
        },
    );
    let program = program(
        &engine,
        r#"
        fn callback(value) { value + 1 }
        fn setup() { let i = 0; while i < 20 { i += 1; } retain(Fn("callback")); }
    "#,
    );
    let tier = tier();
    let _ = call(&engine, &program, &tier, "setup", vec![]).unwrap();
    engine.set_max_operations(5);
    let retained = retained.borrow();
    let (stored, function) = retained.as_ref().unwrap();
    for _ in 0..3 {
        let result: i64 = function
            .call_within_context(&stored.create_context(&engine), (41_i64,))
            .unwrap();
        assert_eq!(result, 42);
    }
    assert_native(&tier, "callback");
}

#[test]
fn scalar_arithmetic_errors_preserve_grain_text_and_position() {
    let engine = Engine::new();
    let program = program(
        &engine,
        r"
        fn divide(a, b) { a / b }
        fn add(a, b) { a + b }
        fn update(a, b) { a += b; a }
    ",
    );
    let tier = tier();
    for (name, a, b) in [
        ("divide", 10_i64, 0_i64),
        ("divide", i64::MIN, -1),
        ("add", i64::MAX, 1),
        ("update", i64::MAX, 1),
    ] {
        let error = Vm::new(&engine)
            .call_fn::<Dynamic>(&mut Scope::new(), &program, name, (a, b))
            .unwrap_err();
        for _ in 0..3 {
            let native =
                call(&engine, &program, &tier, name, vec![a.into(), b.into()]).unwrap_err();
            assert_eq!(
                (native.to_string(), native.position()),
                (error.to_string(), error.position())
            );
        }
        assert_native(&tier, name);
    }
}

#[test]
fn native_code_and_embedded_sites_survive_eviction_during_reentry() {
    let tier = AdaptiveFunctionTier::new(AdaptiveTierConfig {
        min_calls: 1,
        min_self_grain_time: Duration::ZERO,
        max_programs: 1,
        check_profitability: false,
        ..Default::default()
    });
    let observed = Rc::new(Cell::new(false));
    let mut engine = Engine::new();
    let other = program(&engine, r#"fn other() { "other" }"#);
    let nested_tier = tier.clone();
    let observed_tier = Rc::clone(&observed);
    engine.register_fn("evict", move |context: NativeCallContext| {
        observed_tier.set(
            nested_tier
                .stats()
                .iter()
                .any(|entry| entry.name == "outer" && entry.state == "compiled"),
        );
        call(context.engine(), &other, &nested_tier, "other", vec![])
    });
    let program = program(
        &engine,
        r"fn outer(value, evict_now) { let before = value.name; if evict_now { evict(); } before + value.name }",
    );
    let mut value = rhai::Map::new();
    value.insert("name".into(), "a".into());
    let value = Dynamic::from_map(value);
    let _ = call(
        &engine,
        &program,
        &tier,
        "outer",
        vec![value.clone(), false.into()],
    )
    .unwrap();
    let result = call(&engine, &program, &tier, "outer", vec![value, true.into()]).unwrap();
    assert_eq!(result.into_string().unwrap(), "aa");
    assert!(observed.get());
    assert!(tier.summary().evictions > 0);
}

#[test]
fn helper_panics_terminate_the_invocation_and_release_managed_values() {
    let mut engine = Engine::new();
    engine.register_fn("host_panic", || -> () { panic!("intentional test panic") });
    let program = program(
        &engine,
        r"fn run(value, fail) { let local = [value]; if fail { host_panic(); } local }",
    );
    let tier = tier();
    let live = Rc::new(Cell::new(0));
    let value = Dynamic::from(Tracked::new(&live));
    drop(
        call(
            &engine,
            &program,
            &tier,
            "run",
            vec![value.clone(), false.into()],
        )
        .unwrap(),
    );
    let error = call(
        &engine,
        &program,
        &tier,
        "run",
        vec![value.clone(), true.into()],
    )
    .unwrap_err();
    assert!(error.is_system_exception(), "{error}");
    assert_eq!(live.get(), 1);
    assert_native(&tier, "run");
}

#[test]
fn native_code_can_call_a_script_by_name_without_a_script_fn_pointer() {
    let mut engine = Engine::new();
    engine.register_fn("by_name", |context: NativeCallContext, value: i64| {
        context.call_fn::<i64>("inner", (value,))
    });
    let program = program(
        &engine,
        "fn inner(value) { value + 1 } fn outer(value) { by_name(value) }",
    );
    assert!(!program.makes_fn_pointers());
    let tier = tier();
    for _ in 0..3 {
        assert_eq!(
            call(&engine, &program, &tier, "outer", vec![41_i64.into()])
                .unwrap()
                .as_int()
                .unwrap(),
            42
        );
    }
    assert_native(&tier, "inner");
    assert_native(&tier, "outer");
}

#[test]
fn scalar_admission_preserves_a_changed_variable_limit() {
    let mut engine = Engine::new();
    let program = program(&engine, "fn sum(a) { let x = a + 1; let y = a + 2; x + y }");
    let tier = tier();
    for _ in 0..2 {
        let _ = call(&engine, &program, &tier, "sum", vec![1_i64.into()]).unwrap();
    }
    assert_native(&tier, "sum");
    engine.set_max_variables(2);
    let error = call(&engine, &program, &tier, "sum", vec![1_i64.into()]).unwrap_err();
    assert!(
        matches!(*error, EvalAltResult::ErrorTooManyVariables(_)),
        "{error}"
    );
    assert_eq!(tier.stats()[0].guard_misses, 1);
}

#[test]
fn scalar_loops_charge_ticks_and_back_edges_like_grain() {
    let engine = Engine::new();
    let program = program(
        &engine,
        "fn sum(limit) { let n = 0.0; let total = 0.0; while n < limit { total += n; n += 1.0; } total }",
    );
    let tier = tier();
    let mut plain = Vm::new(&engine);
    let expected: f64 = plain
        .call_fn(&mut Scope::new(), &program, "sum", (20.0_f64,))
        .unwrap();
    let operations = plain.num_operations();
    for _ in 0..3 {
        let mut native = Vm::new(&engine).with_function_accelerator(tier.accelerator());
        let value: f64 = native
            .call_fn(&mut Scope::new(), &program, "sum", (20.0_f64,))
            .unwrap();
        assert_eq!((value, native.num_operations()), (expected, operations));
    }
    assert_native(&tier, "sum");
    assert_eq!(tier.stats()[0].native_lane, Some("scalar"));
}

#[test]
fn exact_progress_termination_prevents_later_effects() {
    let mut engine = Engine::new();
    let effects = Rc::new(Cell::new(0));
    let sink = Rc::clone(&effects);
    engine.register_fn("effect", move || sink.set(sink.get() + 1));
    let program = program(
        &engine,
        "fn work(limit) { let n = 0; while n < limit { n += 1; } effect(); n }",
    );
    let tier = tier();
    for _ in 0..2 {
        let _ = call(&engine, &program, &tier, "work", vec![3_i64.into()]).unwrap();
    }
    effects.set(0);
    engine.on_progress(|operations| (operations >= 5).then(|| "stop".into()));
    let error = call(&engine, &program, &tier, "work", vec![100_i64.into()]).unwrap_err();
    assert!(
        matches!(*error, EvalAltResult::ErrorTerminated(..)),
        "{error}"
    );
    assert_eq!(effects.get(), 0);
}
