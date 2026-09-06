//! Interleaved release measurements independent of GPUI applications.

#[cfg(target_arch = "aarch64")]
const SOURCE: &str = r#"
        fn numeric(limit, scale) {
            let i = 0; let total = 0.0;
            while i < limit { total += scale * i / (i + 1.0); i += 1; }
            total
        }
        fn objects(values) {
            let sum = 0.0; let names = "";
            for row in values { sum += row.amount; names += row.name; }
            #{ sum: sum, names: names }
        }
        fn native_mix(limit, name) {
            let values = []; let i = 0;
            while i < limit {
                let n = node(`${name}-${i}`, 2.5);
                values.push(#{ node: n, weight: n.weight() });
                i += 1;
            }
            values
        }
    "#;

#[cfg(target_arch = "aarch64")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};

    use rhai::grain::{CallbackBindings, Compiler, Vm};
    use rhai::{CallFnOptions, Dynamic, Engine, Scope, Shared};
    use rhai_jit_experiment::adaptive::{AdaptiveFunctionTier, AdaptiveTierConfig};
    use serde_json::json;

    #[derive(Clone, Debug)]
    struct HostNode(usize);

    let mut engine = Engine::new();
    engine.register_fn("node", |name: rhai::ImmutableString, width: f64| {
        HostNode(name.len() + usize::from(width > 0.0))
    });
    engine.register_fn("weight", |node: &mut HostNode| {
        i64::try_from(node.0).unwrap_or(i64::MAX)
    });
    let started = Instant::now();
    let ast = engine.compile(SOURCE)?;
    let ast_compile_ns = started.elapsed().as_nanos();
    let started = Instant::now();
    let program = Shared::new(Compiler::new().compile(&ast));
    let grain_compile_ns = started.elapsed().as_nanos();
    let bindings = CallbackBindings::new(program.clone());
    if std::env::var_os("JIT_DUMP").is_some() {
        for function in program.functions() {
            println!("{}", program.view().name(function.name).unwrap_or("?"));
            for (pc, op) in function.chunk.ops(program.code()) {
                println!("  {pc}: {op:?}");
            }
        }
        return Ok(());
    }
    let rows = fixture_rows();
    let workloads = [
        ("numeric", vec![512_i64.into(), 1.25_f64.into()]),
        ("objects", vec![Dynamic::from_array(rows)]),
        ("native_mix", vec![16_i64.into(), "item".into()]),
    ];
    let mut reports = Vec::new();
    for (name, args) in workloads {
        let tier = AdaptiveFunctionTier::new(AdaptiveTierConfig {
            check_profitability: std::env::var_os("JIT_CHECK_PROFITABILITY").is_some(),
            min_calls: 2,
            min_self_grain_time: Duration::ZERO,
            ..Default::default()
        });
        let mut grain = Vm::new(&engine);
        let mut jit = Vm::new(&engine).with_function_accelerator(tier.accelerator());
        let mut run = |backend| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
            match backend {
                0 => engine.call_fn(&mut Scope::new(), &ast, name, args.clone()),
                1 => grain.call_fn_with_bindings(
                    CallFnOptions::new().eval_ast(false),
                    &mut Scope::new(),
                    &bindings,
                    name,
                    args.clone(),
                ),
                _ => jit.call_fn_with_bindings(
                    CallFnOptions::new().eval_ast(false),
                    &mut Scope::new(),
                    &bindings,
                    name,
                    args.clone(),
                ),
            }
        };
        let expected = format!("{:?}", run(0)?);
        for _ in 0..8 {
            for backend in 0..3 {
                assert_eq!(format!("{:?}", run(backend)?), expected);
            }
        }
        let times = measure(&mut run)?;
        let stats = tier.stats();
        let stats = stats
            .iter()
            .find(|entry| entry.name == name)
            .ok_or("missing tier stats")?;
        reports.push(json!({
            "function": name, "lane": stats.native_lane, "state": stats.state,
            "rejection": stats.rejection,
            "ast_p50_ns": times[0][20], "ast_p95_ns": times[0][38],
            "grain_p50_ns": times[1][20], "grain_p95_ns": times[1][38],
            "jit_p50_ns": times[2][20], "jit_p95_ns": times[2][38],
            "compile_ns": stats.compile_time.as_nanos(),
            "code_bytes": stats.code_bytes, "allocated_code_bytes": stats.allocated_code_bytes, "jit_calls": stats.jit_calls,
        }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "release": !cfg!(debug_assertions),
            "architecture": std::env::consts::ARCH,
            "ast_compile_ns": ast_compile_ns,
            "grain_compile_ns": grain_compile_ns,
            "workloads": reports,
        }))?
    );
    Ok(())
}

#[cfg(target_arch = "aarch64")]
fn fixture_rows() -> rhai::Array {
    use rhai::Dynamic;
    (0..32)
        .map(|_| {
            let mut row = rhai::Map::new();
            row.insert("amount".into(), 1.25_f64.into());
            row.insert("name".into(), "item".into());
            Dynamic::from_map(row)
        })
        .collect::<rhai::Array>()
}

#[cfg(target_arch = "aarch64")]
fn measure(
    run: &mut impl FnMut(usize) -> Result<rhai::Dynamic, Box<rhai::EvalAltResult>>,
) -> Result<[Vec<u128>; 3], Box<rhai::EvalAltResult>> {
    use std::hint::black_box;
    use std::time::Instant;
    let mut times = [Vec::new(), Vec::new(), Vec::new()];
    let repeats = if cfg!(debug_assertions) { 2 } else { 100 };
    for sample in 0..40 {
        for offset in 0..3 {
            let backend = (sample + offset) % 3;
            let started = Instant::now();
            for _ in 0..repeats {
                drop(black_box(run(backend)?));
            }
            times[backend].push(started.elapsed().as_nanos() / repeats);
        }
    }
    for samples in &mut times {
        samples.sort_unstable();
    }
    Ok(times)
}

#[cfg(not(target_arch = "aarch64"))]
fn main() {
    eprintln!("This experiment currently requires AArch64.");
}
