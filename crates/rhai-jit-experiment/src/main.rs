use std::error::Error;
#[cfg(target_arch = "aarch64")]
use std::hint::black_box;
use std::time::{Duration, Instant};

use rhai::grain::Compiler;
use rhai::{CallFnOptions, Engine, Scope};
use rhai_jit_experiment::typed;
#[cfg(target_arch = "aarch64")]
use rhai_jit_experiment::{adaptive, jit_aarch64, tier};
use serde::Serialize;

const HOTSPOT: &str = r"
fn hotspot(limit) {
    let total = 0;
    let index = 0;
    while index < limit {
        if index % 3 == 0 {
            total += index * 2;
        } else if index % 5 == 0 {
            total -= index;
        } else {
            total += 1;
        }
        index += 1;
    }
    total
}
";

#[cfg(target_arch = "aarch64")]
const PAGINATION_SOURCE: &str = include_str!("../../../registry/components/pagination.rhai");
#[cfg(target_arch = "aarch64")]
const PRODUCT_FUNCTION: &str = "total_pages";
#[cfg(target_arch = "aarch64")]
const PRODUCT_THRESHOLD: u64 = 3;
#[cfg(target_arch = "aarch64")]
const PRODUCT_SAMPLE_ITERATIONS: usize = if cfg!(debug_assertions) { 4 } else { 1_000 };

const LIMIT: i64 = 10_000;
const LIMIT_FLOAT: rhai::FLOAT = 10_000.0;
const MAX_OPERATIONS: u64 = 1_000_000;

#[cfg(target_arch = "aarch64")]
const WARMUP_ITERATIONS: usize = if cfg!(debug_assertions) { 1 } else { 10 };
#[cfg(target_arch = "aarch64")]
const SAMPLE_ITERATIONS: usize = if cfg!(debug_assertions) { 2 } else { 100 };

#[derive(Serialize)]
struct InspectionReport {
    release: bool,
    function: String,
    ast_value: i64,
    grain_value: i64,
    typed_value: i64,
    jit_value: Option<i64>,
    #[cfg(target_arch = "aarch64")]
    guard_hit_backend: Option<jit_aarch64::GuardedBackend>,
    #[cfg(target_arch = "aarch64")]
    guard_miss_backend: Option<jit_aarch64::GuardedBackend>,
    #[cfg(target_arch = "aarch64")]
    guard_miss_value: Option<i64>,
    operations: u64,
    jit_operations: Option<u64>,
    opcode_counts: std::collections::BTreeMap<&'static str, u64>,
    grain_code_bytes: usize,
    native_code_bytes: Option<usize>,
    compile_ns: CompileReport,
    execution_ns: Option<ExecutionReport>,
    #[cfg(target_arch = "aarch64")]
    product_tier: ProductTierReport,
}

#[derive(Serialize)]
struct CompileReport {
    ast: u64,
    grain: u64,
    typed_lowering: u64,
    jit: Option<u64>,
}

#[derive(Serialize)]
struct ExecutionReport {
    ast: LatencyReport,
    grain_fresh_vm: LatencyReport,
    typed_trace: LatencyReport,
    jit: LatencyReport,
    guarded_jit: LatencyReport,
    jit_speedup_over_grain_at_p95: f64,
    guarded_jit_speedup_over_grain_at_p95: f64,
}

#[derive(Clone, Copy, Serialize)]
struct LatencyReport {
    p50: u64,
    p95: u64,
    min: u64,
    max: u64,
}

#[cfg(target_arch = "aarch64")]
#[derive(Serialize)]
struct ProductTierReport {
    entry_function: &'static str,
    function: &'static str,
    threshold: u64,
    transition_backends: Vec<tier::TierBackend>,
    transition_compiled_now: Vec<bool>,
    stats_after_transition: tier::FunctionTierStats,
    stats_after_benchmark: tier::FunctionTierStats,
    value: i64,
    grain_reused_vm: LatencyReport,
    cached_internal_tier: LatencyReport,
    cached_speedup_over_grain_at_p95: f64,
    adaptive_transition_backends: Vec<tier::TierBackend>,
    adaptive_stats_after_transition: AdaptiveStatsReport,
    cached_adaptive_tier: LatencyReport,
    adaptive_speedup_over_grain_at_p95: f64,
}

#[cfg(target_arch = "aarch64")]
#[derive(Serialize)]
struct AdaptiveStatsReport {
    grain_calls: u64,
    jit_calls: u64,
    compile_attempts: u64,
    code_bytes: usize,
    state: &'static str,
}

#[cfg(target_arch = "aarch64")]
impl LatencyReport {
    fn from_samples(mut samples: Vec<Duration>) -> Self {
        samples.sort_unstable();
        Self {
            p50: duration_ns(percentile(&samples, 50)),
            p95: duration_ns(percentile(&samples, 95)),
            min: duration_ns(samples[0]),
            max: duration_ns(samples[samples.len() - 1]),
        }
    }
}

struct ProbeRun {
    ast_compile: Duration,
    grain_compile: Duration,
    typed_lowering: Duration,
    ast_value: i64,
    grain_value: i64,
    typed: typed::TypedChunk,
    typed_execution: typed::TypedExecution,
    grain_code_bytes: usize,
    #[cfg(target_arch = "aarch64")]
    jit_compile: Duration,
    #[cfg(target_arch = "aarch64")]
    compiled: jit_aarch64::CompiledChunk,
    #[cfg(target_arch = "aarch64")]
    jit_execution: jit_aarch64::JitExecution,
    #[cfg(target_arch = "aarch64")]
    guard_hit: jit_aarch64::GuardedExecution,
    #[cfg(target_arch = "aarch64")]
    guard_miss: jit_aarch64::GuardedExecution,
    #[cfg(target_arch = "aarch64")]
    execution: ExecutionReport,
    #[cfg(target_arch = "aarch64")]
    product_tier: ProductTierReport,
}

impl ProbeRun {
    fn into_report(self) -> InspectionReport {
        InspectionReport {
            release: !cfg!(debug_assertions),
            function: self.typed.function,
            ast_value: self.ast_value,
            grain_value: self.grain_value,
            typed_value: self.typed_execution.value,
            #[cfg(target_arch = "aarch64")]
            jit_value: Some(self.jit_execution.value),
            #[cfg(not(target_arch = "aarch64"))]
            jit_value: None,
            #[cfg(target_arch = "aarch64")]
            guard_hit_backend: Some(self.guard_hit.backend),
            #[cfg(target_arch = "aarch64")]
            guard_miss_backend: Some(self.guard_miss.backend),
            #[cfg(target_arch = "aarch64")]
            guard_miss_value: Some(self.guard_miss.value),
            operations: self.typed_execution.operations,
            #[cfg(target_arch = "aarch64")]
            jit_operations: Some(self.jit_execution.operations),
            #[cfg(not(target_arch = "aarch64"))]
            jit_operations: None,
            opcode_counts: self.typed_execution.opcode_counts,
            grain_code_bytes: self.grain_code_bytes,
            #[cfg(target_arch = "aarch64")]
            native_code_bytes: Some(self.compiled.code_size()),
            #[cfg(not(target_arch = "aarch64"))]
            native_code_bytes: None,
            compile_ns: CompileReport {
                ast: duration_ns(self.ast_compile),
                grain: duration_ns(self.grain_compile),
                typed_lowering: duration_ns(self.typed_lowering),
                #[cfg(target_arch = "aarch64")]
                jit: Some(duration_ns(self.jit_compile)),
                #[cfg(not(target_arch = "aarch64"))]
                jit: None,
            },
            #[cfg(target_arch = "aarch64")]
            execution_ns: Some(self.execution),
            #[cfg(not(target_arch = "aarch64"))]
            execution_ns: None,
            #[cfg(target_arch = "aarch64")]
            product_tier: self.product_tier,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", serde_json::to_string_pretty(&run_probe()?)?);
    Ok(())
}

fn run_probe() -> Result<InspectionReport, Box<dyn Error>> {
    let mut engine = Engine::new();
    engine.set_max_call_levels(64);
    engine.set_max_expr_depths(64, 32);
    engine.set_max_operations(MAX_OPERATIONS);
    let started = Instant::now();
    let ast = engine.compile(HOTSPOT)?;
    let ast_compile = started.elapsed();
    let started = Instant::now();
    let program = Compiler::new().compile(&ast);
    let grain_compile = started.elapsed();
    program
        .verify()
        .map_err(|error| std::io::Error::other(format!("Grain verification failed: {error:?}")))?;
    let started = Instant::now();
    let typed = typed::lower(&program)?;
    let typed_lowering = started.elapsed();

    let ast_value: i64 = engine.call_fn_with_options(
        CallFnOptions::new().eval_ast(false),
        &mut Scope::new(),
        &ast,
        "hotspot",
        (LIMIT,),
    )?;
    let grain_value: i64 = rhai::grain::Vm::new(&engine).call_fn_with_options(
        CallFnOptions::new().eval_ast(false),
        &mut Scope::new(),
        &program,
        "hotspot",
        (LIMIT,),
    )?;
    let typed_execution = typed.execute(&[LIMIT], MAX_OPERATIONS)?;
    #[cfg(target_arch = "aarch64")]
    let started = Instant::now();
    #[cfg(target_arch = "aarch64")]
    let compiled = jit_aarch64::compile(&typed)?;
    #[cfg(target_arch = "aarch64")]
    let jit_compile = started.elapsed();
    #[cfg(target_arch = "aarch64")]
    let jit_execution = compiled.execute(&[LIMIT], MAX_OPERATIONS)?;
    #[cfg(target_arch = "aarch64")]
    let guard_hit = compiled.execute_guarded(
        &engine,
        &program,
        vec![rhai::Dynamic::from(LIMIT)],
        MAX_OPERATIONS,
    )?;
    #[cfg(target_arch = "aarch64")]
    let guard_miss = compiled.execute_guarded(
        &engine,
        &program,
        vec![rhai::Dynamic::from_float(LIMIT_FLOAT)],
        MAX_OPERATIONS,
    )?;

    ensure_equal("Grain", ast_value, grain_value)?;
    ensure_equal("typed trace", ast_value, typed_execution.value)?;
    #[cfg(target_arch = "aarch64")]
    ensure_equal("JIT", ast_value, jit_execution.value)?;
    #[cfg(target_arch = "aarch64")]
    ensure_equal("guard hit", ast_value, guard_hit.value)?;
    #[cfg(target_arch = "aarch64")]
    ensure_equal("guard miss fallback", ast_value, guard_miss.value)?;

    #[cfg(target_arch = "aarch64")]
    let execution = benchmark(&engine, &ast, &program, &typed, &compiled)?;
    #[cfg(target_arch = "aarch64")]
    let product_tier = benchmark_product_tier(&engine)?;

    Ok(ProbeRun {
        ast_compile,
        grain_compile,
        typed_lowering,
        ast_value,
        grain_value,
        grain_code_bytes: program.code().len(),
        typed,
        typed_execution,
        #[cfg(target_arch = "aarch64")]
        jit_compile,
        #[cfg(target_arch = "aarch64")]
        compiled,
        #[cfg(target_arch = "aarch64")]
        jit_execution,
        #[cfg(target_arch = "aarch64")]
        guard_hit,
        #[cfg(target_arch = "aarch64")]
        guard_miss,
        #[cfg(target_arch = "aarch64")]
        execution,
        #[cfg(target_arch = "aarch64")]
        product_tier,
    }
    .into_report())
}

fn ensure_equal(backend: &str, expected: i64, actual: i64) -> Result<(), Box<dyn Error>> {
    if actual == expected {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "{backend} mismatch: AST={expected}, actual={actual}"
        ))
        .into())
    }
}

#[cfg(target_arch = "aarch64")]
fn benchmark_product_tier(engine: &Engine) -> Result<ProductTierReport, Box<dyn Error>> {
    const ENTRY_FUNCTION: &str = "workload";
    let source =
        format!("{PAGINATION_SOURCE}\nfn {ENTRY_FUNCTION}() {{ {PRODUCT_FUNCTION}(128, 10) }}");
    let ast = engine.compile(source)?;
    let program = Compiler::new().compile(&ast);
    program
        .verify()
        .map_err(|error| std::io::Error::other(format!("Grain verification failed: {error:?}")))?;

    let function_tier = tier::FunctionTier::for_function(PRODUCT_THRESHOLD, PRODUCT_FUNCTION, 2);
    let mut accelerated_vm =
        rhai::grain::Vm::new(engine).with_function_accelerator(function_tier.accelerator());
    let mut grain_vm = rhai::grain::Vm::new(engine);
    let adaptive_tier = adaptive::AdaptiveFunctionTier::new(adaptive::AdaptiveTierConfig {
        min_calls: 2,
        min_self_grain_time: Duration::ZERO,
        ..Default::default()
    });
    let mut adaptive_vm =
        rhai::grain::Vm::new(engine).with_function_accelerator(adaptive_tier.accelerator());
    let mut transition_backends = Vec::new();
    let mut transition_compiled_now = Vec::new();
    let mut value = 0_i64;
    for _ in 0..4 {
        let before = function_tier.stats(&program, PRODUCT_FUNCTION, 2);
        value = call_product_workload(&mut accelerated_vm, &program, ENTRY_FUNCTION)?;
        let after = function_tier
            .stats(&program, PRODUCT_FUNCTION, 2)
            .ok_or_else(|| std::io::Error::other("missing product tier transition stats"))?;
        transition_backends.push(
            if after.jit_calls > before.as_ref().map_or(0, |s| s.jit_calls) {
                tier::TierBackend::Jit
            } else if after.state == "rejected" {
                tier::TierBackend::GrainRejected
            } else {
                tier::TierBackend::GrainCold
            },
        );
        transition_compiled_now.push(
            after.compile_attempts > before.as_ref().map_or(0, |s| s.compile_attempts)
                && after.state == "compiled",
        );
    }
    let stats_after_transition = function_tier
        .stats(&program, PRODUCT_FUNCTION, 2)
        .ok_or_else(|| std::io::Error::other("missing product tier stats after transition"))?;

    let (adaptive_transition_backends, adaptive_stats_after_transition) =
        warm_adaptive_product_tier(&adaptive_tier, &mut adaptive_vm, &program, value)?;

    let mut grain_samples = Vec::with_capacity(PRODUCT_SAMPLE_ITERATIONS);
    let mut tier_samples = Vec::with_capacity(PRODUCT_SAMPLE_ITERATIONS);
    let mut adaptive_samples = Vec::with_capacity(PRODUCT_SAMPLE_ITERATIONS);
    for _ in 0..PRODUCT_SAMPLE_ITERATIONS {
        grain_samples.push(measure(|| {
            call_product_workload(&mut grain_vm, &program, ENTRY_FUNCTION)
        })?);
        tier_samples.push(measure(|| {
            call_product_workload(&mut accelerated_vm, &program, ENTRY_FUNCTION)
        })?);
        adaptive_samples.push(measure(|| {
            call_product_workload(&mut adaptive_vm, &program, ENTRY_FUNCTION)
        })?);
    }

    let grain_reused_vm = LatencyReport::from_samples(grain_samples);
    let cached_internal_tier = LatencyReport::from_samples(tier_samples);
    let cached_adaptive_tier = LatencyReport::from_samples(adaptive_samples);
    let stats_after_benchmark = function_tier
        .stats(&program, PRODUCT_FUNCTION, 2)
        .ok_or_else(|| std::io::Error::other("missing product tier stats after benchmark"))?;
    Ok(ProductTierReport {
        entry_function: ENTRY_FUNCTION,
        function: PRODUCT_FUNCTION,
        threshold: PRODUCT_THRESHOLD,
        transition_backends,
        transition_compiled_now,
        stats_after_transition,
        stats_after_benchmark,
        value,
        cached_speedup_over_grain_at_p95: ratio(grain_reused_vm.p95, cached_internal_tier.p95),
        adaptive_speedup_over_grain_at_p95: ratio(grain_reused_vm.p95, cached_adaptive_tier.p95),
        grain_reused_vm,
        cached_internal_tier,
        adaptive_transition_backends,
        adaptive_stats_after_transition,
        cached_adaptive_tier,
    })
}

#[cfg(target_arch = "aarch64")]
fn warm_adaptive_product_tier(
    adaptive_tier: &adaptive::AdaptiveFunctionTier,
    adaptive_vm: &mut rhai::grain::Vm<'_>,
    program: &rhai::grain::Program<'_>,
    expected: i64,
) -> Result<(Vec<tier::TierBackend>, AdaptiveStatsReport), Box<dyn Error>> {
    let mut backends = Vec::new();
    for _ in 0..4 {
        let before = adaptive_tier
            .stats()
            .into_iter()
            .find(|stats| stats.name == PRODUCT_FUNCTION)
            .map_or(0, |stats| stats.jit_calls);
        let value = call_product_workload(adaptive_vm, program, "workload")?;
        ensure_equal("adaptive product tier", expected, value)?;
        let after = adaptive_tier
            .stats()
            .into_iter()
            .find(|stats| stats.name == PRODUCT_FUNCTION)
            .ok_or_else(|| std::io::Error::other("missing adaptive product tier stats"))?;
        backends.push(if after.jit_calls > before {
            tier::TierBackend::Jit
        } else if after.state == "rejected" {
            tier::TierBackend::GrainRejected
        } else {
            tier::TierBackend::GrainCold
        });
    }
    let stats = adaptive_tier
        .stats()
        .into_iter()
        .find(|stats| stats.name == PRODUCT_FUNCTION)
        .ok_or_else(|| std::io::Error::other("missing adaptive transition stats"))?;
    Ok((
        backends,
        AdaptiveStatsReport {
            grain_calls: stats.grain_calls,
            jit_calls: stats.jit_calls,
            compile_attempts: stats.compile_attempts,
            code_bytes: stats.code_bytes,
            state: stats.state,
        },
    ))
}

#[cfg(target_arch = "aarch64")]
fn call_product_workload(
    vm: &mut rhai::grain::Vm<'_>,
    program: &rhai::grain::Program<'_>,
    entry_function: &str,
) -> Result<i64, Box<rhai::EvalAltResult>> {
    vm.call_fn_with_options(
        CallFnOptions::new().eval_ast(false),
        &mut Scope::new(),
        program,
        entry_function,
        (),
    )
}

#[cfg(target_arch = "aarch64")]
fn benchmark(
    engine: &Engine,
    ast: &rhai::AST,
    program: &rhai::grain::Program<'_>,
    typed: &typed::TypedChunk,
    compiled: &jit_aarch64::CompiledChunk,
) -> Result<ExecutionReport, Box<dyn Error>> {
    for _ in 0..WARMUP_ITERATIONS {
        let _ = black_box(execute_ast(engine, ast)?);
        let _ = black_box(execute_grain(engine, program)?);
        let _ = black_box(typed.execute(&[LIMIT], MAX_OPERATIONS)?);
        let _ = black_box(compiled.execute(&[LIMIT], MAX_OPERATIONS)?);
        let _ = black_box(compiled.execute_guarded(
            engine,
            program,
            vec![rhai::Dynamic::from(LIMIT)],
            MAX_OPERATIONS,
        )?);
    }

    let mut ast_samples = Vec::with_capacity(SAMPLE_ITERATIONS);
    let mut grain_samples = Vec::with_capacity(SAMPLE_ITERATIONS);
    let mut typed_samples = Vec::with_capacity(SAMPLE_ITERATIONS);
    let mut jit_samples = Vec::with_capacity(SAMPLE_ITERATIONS);
    let mut guarded_jit_samples = Vec::with_capacity(SAMPLE_ITERATIONS);
    for _ in 0..SAMPLE_ITERATIONS {
        ast_samples.push(measure(|| execute_ast(engine, ast))?);
        grain_samples.push(measure(|| execute_grain(engine, program))?);
        typed_samples.push(measure(|| typed.execute(&[LIMIT], MAX_OPERATIONS))?);
        jit_samples.push(measure(|| compiled.execute(&[LIMIT], MAX_OPERATIONS))?);
        guarded_jit_samples.push(measure(|| {
            compiled.execute_guarded(
                engine,
                program,
                vec![rhai::Dynamic::from(LIMIT)],
                MAX_OPERATIONS,
            )
        })?);
    }

    let ast = LatencyReport::from_samples(ast_samples);
    let grain_fresh_vm = LatencyReport::from_samples(grain_samples);
    let typed_trace = LatencyReport::from_samples(typed_samples);
    let jit = LatencyReport::from_samples(jit_samples);
    let guarded_jit = LatencyReport::from_samples(guarded_jit_samples);
    Ok(ExecutionReport {
        jit_speedup_over_grain_at_p95: ratio(grain_fresh_vm.p95, jit.p95),
        guarded_jit_speedup_over_grain_at_p95: ratio(grain_fresh_vm.p95, guarded_jit.p95),
        ast,
        grain_fresh_vm,
        typed_trace,
        jit,
        guarded_jit,
    })
}

fn execute_ast(engine: &Engine, ast: &rhai::AST) -> Result<i64, Box<rhai::EvalAltResult>> {
    engine.call_fn_with_options(
        CallFnOptions::new().eval_ast(false),
        &mut Scope::new(),
        ast,
        "hotspot",
        (LIMIT,),
    )
}

fn execute_grain(
    engine: &Engine,
    program: &rhai::grain::Program<'_>,
) -> Result<i64, Box<rhai::EvalAltResult>> {
    rhai::grain::Vm::new(engine).call_fn_with_options(
        CallFnOptions::new().eval_ast(false),
        &mut Scope::new(),
        program,
        "hotspot",
        (LIMIT,),
    )
}

#[cfg(target_arch = "aarch64")]
fn measure<T, E>(execute: impl FnOnce() -> Result<T, E>) -> Result<Duration, E> {
    let started = Instant::now();
    let _ = black_box(execute()?);
    Ok(started.elapsed())
}

#[cfg(target_arch = "aarch64")]
fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    let rank = samples
        .len()
        .saturating_mul(percentile)
        .div_ceil(100)
        .saturating_sub(1);
    samples[rank.min(samples.len() - 1)]
}

fn duration_ns(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(target_arch = "aarch64")]
fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        f64::INFINITY
    } else {
        Duration::from_nanos(numerator).as_secs_f64()
            / Duration::from_nanos(denominator).as_secs_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn hotspot_matches_across_all_execution_paths() {
        let engine = Engine::new();
        let ast = engine.compile(HOTSPOT).unwrap();
        let program = Compiler::new().compile(&ast);
        let typed = typed::lower(&program).unwrap();
        let compiled = jit_aarch64::compile(&typed).unwrap();

        assert_eq!(
            (
                execute_ast(&engine, &ast).unwrap(),
                execute_grain(&engine, &program).unwrap(),
                typed.execute(&[LIMIT], MAX_OPERATIONS).unwrap().value,
                compiled.execute(&[LIMIT], MAX_OPERATIONS).unwrap().value,
            ),
            (26_678_664, 26_678_664, 26_678_664, 26_678_664)
        );
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn jit_operation_budget_resets_for_each_call() {
        let engine = Engine::new();
        let ast = engine.compile(HOTSPOT).unwrap();
        let program = Compiler::new().compile(&ast);
        let typed = typed::lower(&program).unwrap();
        let compiled = jit_aarch64::compile(&typed).unwrap();

        let limited = compiled.execute(&[LIMIT], 1).unwrap_err().to_string();
        let first = compiled.execute(&[LIMIT], MAX_OPERATIONS).unwrap();
        let second = compiled.execute(&[LIMIT], MAX_OPERATIONS).unwrap();
        assert_eq!(
            (limited, first.operations, second.operations),
            (
                "the JIT operation limit 1 was exceeded".to_owned(),
                20_001,
                20_001,
            )
        );
    }

    #[test]
    fn reused_grain_vm_resets_operation_budget_for_each_call() {
        let mut engine = Engine::new();
        engine.set_max_operations(6);
        let ast = engine
            .compile("fn f() { let x = 0; while x < 2 { x += 1; } x }")
            .unwrap();
        let program = Compiler::new().compile(&ast);
        let mut vm = rhai::grain::Vm::new(&engine);

        let first: i64 = vm
            .call_fn_with_options(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                &program,
                "f",
                (),
            )
            .unwrap();
        let first_operations = vm.num_operations();
        let second: i64 = vm
            .call_fn_with_options(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                &program,
                "f",
                (),
            )
            .unwrap();

        assert_eq!(
            (first, first_operations, second, vm.num_operations()),
            (2, 6, 2, 6)
        );
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn integer_guard_hit_uses_jit() {
        let engine = Engine::new();
        let ast = engine.compile(HOTSPOT).unwrap();
        let program = Compiler::new().compile(&ast);
        let typed = typed::lower(&program).unwrap();
        let compiled = jit_aarch64::compile(&typed).unwrap();

        let execution = compiled
            .execute_guarded(
                &engine,
                &program,
                vec![rhai::Dynamic::from(LIMIT)],
                MAX_OPERATIONS,
            )
            .unwrap();
        assert_eq!(
            (execution.backend, execution.value, execution.operations),
            (jit_aarch64::GuardedBackend::Jit, 26_678_664, Some(20_001),)
        );
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn non_integer_guard_miss_falls_back_to_grain() {
        let engine = Engine::new();
        let ast = engine.compile(HOTSPOT).unwrap();
        let program = Compiler::new().compile(&ast);
        let typed = typed::lower(&program).unwrap();
        let compiled = jit_aarch64::compile(&typed).unwrap();

        let execution = compiled
            .execute_guarded(
                &engine,
                &program,
                vec![rhai::Dynamic::from_float(LIMIT_FLOAT)],
                MAX_OPERATIONS,
            )
            .unwrap();
        assert_eq!(
            (execution.backend, execution.value, execution.operations),
            (jit_aarch64::GuardedBackend::GrainFallback, 26_678_664, None,)
        );
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn rhai_to_rhai_product_call_tiers_inside_the_grain_vm() {
        const ENTRY_FUNCTION: &str = "workload";
        let mut engine = Engine::new();
        engine.set_max_expr_depths(64, 32);
        engine.set_max_operations(MAX_OPERATIONS);
        let source =
            format!("{PAGINATION_SOURCE}\nfn {ENTRY_FUNCTION}() {{ {PRODUCT_FUNCTION}(128, 10) }}");
        let ast = engine.compile(source).unwrap();
        let program = Compiler::new().compile(&ast);
        let function_tier =
            tier::FunctionTier::for_function(PRODUCT_THRESHOLD, PRODUCT_FUNCTION, 2);
        let mut vm =
            rhai::grain::Vm::new(&engine).with_function_accelerator(function_tier.accelerator());

        let mut values = Vec::new();
        let mut stats = Vec::new();
        for _ in 0..4 {
            values.push(call_product_workload(&mut vm, &program, ENTRY_FUNCTION).unwrap());
            stats.push(function_tier.stats(&program, PRODUCT_FUNCTION, 2).unwrap());
        }

        assert_eq!(values, vec![13, 13, 13, 13]);
        assert_eq!(
            stats,
            vec![
                tier::FunctionTierStats {
                    calls: 1,
                    compile_attempts: 0,
                    jit_calls: 0,
                    guard_misses: 0,
                    state: "cold",
                    rejection: None,
                },
                tier::FunctionTierStats {
                    calls: 2,
                    compile_attempts: 0,
                    jit_calls: 0,
                    guard_misses: 0,
                    state: "cold",
                    rejection: None,
                },
                tier::FunctionTierStats {
                    calls: 3,
                    compile_attempts: 1,
                    jit_calls: 1,
                    guard_misses: 0,
                    state: "compiled",
                    rejection: None,
                },
                tier::FunctionTierStats {
                    calls: 4,
                    compile_attempts: 1,
                    jit_calls: 2,
                    guard_misses: 0,
                    state: "compiled",
                    rejection: None,
                },
            ]
        );
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn internal_tier_guard_miss_falls_back_without_touching_arguments() {
        let engine = Engine::new();
        let ast = engine
            .compile(
                "fn target(value) { value + 1 }\n\
                 fn integer_workload() { target(1) }\n\
                 fn float_workload() { target(1.5) }",
            )
            .unwrap();
        let program = Compiler::new().compile(&ast);
        let function_tier = tier::FunctionTier::for_function(1, "target", 1);
        let mut vm =
            rhai::grain::Vm::new(&engine).with_function_accelerator(function_tier.accelerator());

        let integer: i64 = vm
            .call_fn_with_options(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                &program,
                "integer_workload",
                (),
            )
            .unwrap();
        let float: rhai::FLOAT = vm
            .call_fn_with_options(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                &program,
                "float_workload",
                (),
            )
            .unwrap();

        assert_eq!((integer, float), (2, 2.5));
        assert_eq!(
            function_tier.stats(&program, "target", 1).unwrap(),
            tier::FunctionTierStats {
                calls: 2,
                compile_attempts: 1,
                jit_calls: 1,
                guard_misses: 1,
                state: "compiled",
                rejection: None,
            }
        );
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn internal_tier_declines_when_fast_operators_are_disabled() {
        let mut engine = Engine::new();
        engine.set_fast_operators(false);
        let ast = engine
            .compile("fn target(value) { value + 1 } fn workload() { target(1) }")
            .unwrap();
        let program = Compiler::new().compile(&ast);
        let function_tier = tier::FunctionTier::for_function(1, "target", 1);
        let mut vm =
            rhai::grain::Vm::new(&engine).with_function_accelerator(function_tier.accelerator());

        let value: i64 = vm
            .call_fn_with_options(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                &program,
                "workload",
                (),
            )
            .unwrap();

        assert_eq!(value, 2);
        assert_eq!(function_tier.stats(&program, "target", 1), None);
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn internal_tier_cache_is_isolated_across_program_reload() {
        let engine = Engine::new();
        let first_ast = engine
            .compile("fn target(value) { value + 1 } fn workload() { target(1) }")
            .unwrap();
        let second_ast = engine
            .compile("fn target(value) { value + 100 } fn workload() { target(1) }")
            .unwrap();
        let first_program = Compiler::new().compile(&first_ast);
        let second_program = Compiler::new().compile(&second_ast);
        let function_tier = tier::FunctionTier::for_function(1, "target", 1);
        let mut vm =
            rhai::grain::Vm::new(&engine).with_function_accelerator(function_tier.accelerator());

        let first: i64 = vm
            .call_fn_with_options(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                &first_program,
                "workload",
                (),
            )
            .unwrap();
        let first_jit_calls = function_tier
            .stats(&first_program, "target", 1)
            .unwrap()
            .jit_calls;
        let second: i64 = vm
            .call_fn_with_options(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                &second_program,
                "workload",
                (),
            )
            .unwrap();

        assert_eq!((first, second), (2, 101));
        assert_eq!(
            (
                first_jit_calls,
                function_tier
                    .stats(&second_program, "target", 1)
                    .unwrap()
                    .jit_calls,
            ),
            (1, 1)
        );
        assert_eq!(function_tier.stats(&first_program, "target", 1), None);
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn internal_jit_operations_are_charged_to_the_grain_vm_budget() {
        const LIMIT_WITHOUT_LAST_TICK: u64 = 10_002;
        let mut engine = Engine::new();
        engine.set_max_operations(LIMIT_WITHOUT_LAST_TICK);
        let source = format!("{HOTSPOT}\nfn workload() {{ hotspot({LIMIT}) }}");
        let ast = engine.compile(source).unwrap();
        let program = Compiler::new().compile(&ast);
        let function_tier = tier::FunctionTier::for_function(1, "hotspot", 1);
        let mut vm =
            rhai::grain::Vm::new(&engine).with_function_accelerator(function_tier.accelerator());

        let error = vm
            .call_fn_with_options::<i64>(
                CallFnOptions::new().eval_ast(false),
                &mut Scope::new(),
                &program,
                "workload",
                (),
            )
            .unwrap_err();

        assert!(
            error.to_string().contains("operations"),
            "unexpected operation-limit error: {error:?}"
        );
        assert_eq!(vm.num_operations(), LIMIT_WITHOUT_LAST_TICK + 1);
        assert_eq!(
            function_tier
                .stats(&program, "hotspot", 1)
                .unwrap()
                .jit_calls,
            1
        );
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn rejected_product_helper_is_not_recompiled() {
        const ENTRY_FUNCTION: &str = "workload";
        let mut engine = Engine::new();
        engine.set_max_expr_depths(64, 32);
        let source = format!(
            "{PAGINATION_SOURCE}\nfn {ENTRY_FUNCTION}() {{ prop_or(#{{ value: 7 }}, \"value\", 0) }}"
        );
        let ast = engine.compile(source).unwrap();
        let program = Compiler::new().compile(&ast);
        let function_tier = tier::FunctionTier::for_function(1, "prop_or", 3);
        let mut vm =
            rhai::grain::Vm::new(&engine).with_function_accelerator(function_tier.accelerator());

        assert_eq!(
            call_product_workload(&mut vm, &program, ENTRY_FUNCTION).unwrap(),
            7
        );
        assert_eq!(
            call_product_workload(&mut vm, &program, ENTRY_FUNCTION).unwrap(),
            7
        );
        let stats = function_tier.stats(&program, "prop_or", 3).unwrap();
        assert_eq!(
            (
                stats.calls,
                stats.compile_attempts,
                stats.jit_calls,
                stats.state
            ),
            (2, 1, 0, "rejected")
        );
        assert!(stats.rejection.is_some());
    }

    #[test]
    #[cfg(target_arch = "aarch64")]
    fn product_division_error_is_reported_without_fallback_reexecution() {
        const ENTRY_FUNCTION: &str = "workload";
        let mut engine = Engine::new();
        engine.set_max_expr_depths(64, 32);
        engine.set_max_operations(MAX_OPERATIONS);
        let source =
            format!("{PAGINATION_SOURCE}\nfn {ENTRY_FUNCTION}() {{ {PRODUCT_FUNCTION}(128, 0) }}");
        let ast = engine.compile(source).unwrap();
        let program = Compiler::new().compile(&ast);
        let function_tier = tier::FunctionTier::for_function(1, PRODUCT_FUNCTION, 2);
        let mut vm =
            rhai::grain::Vm::new(&engine).with_function_accelerator(function_tier.accelerator());

        let error = call_product_workload(&mut vm, &program, ENTRY_FUNCTION).unwrap_err();
        assert!(
            matches!(
                error.unwrap_inner(),
                rhai::EvalAltResult::ErrorArithmetic(..)
            ),
            "unexpected accelerated error: {error}"
        );
        let stats = function_tier.stats(&program, PRODUCT_FUNCTION, 2).unwrap();
        assert_eq!(
            (
                stats.calls,
                stats.compile_attempts,
                stats.jit_calls,
                stats.state,
                stats.rejection,
            ),
            (1, 1, 1, "compiled", None)
        );
    }

    #[test]
    fn typed_lowering_rejects_a_dynamic_type_change() {
        let engine = Engine::new();
        let ast = engine
            .compile("fn hotspot() { let value = 1; value = true; value + 1 }")
            .unwrap();
        let program = Compiler::new().compile(&ast);

        assert!(matches!(
            typed::lower(&program),
            Err(typed::TypedError::TypeMismatch { .. })
        ));
    }
}
