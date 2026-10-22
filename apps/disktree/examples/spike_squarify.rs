//! Spike 1 (M0): can the rhai squarify port live inside gpui-rhai's
//! 1,000,000-operation script budget on realistic and stress tile counts?
//!
//! Runs ui/core.rhai's `squarify` through a plain rhai engine with the same
//! op cap, measuring wall time and true operation count per case, and checks
//! the two correctness properties from tobi's treemap tests.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use rhai::{Array, Dynamic, Engine, Map, Scope};

/// An engine configured like gpui-rhai's RuntimeEngine (engine.rs:2261-2262).
fn runtime_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_call_levels(64);
    engine.set_max_expr_depths(64, 32);
    engine
}

fn area(w: f64, h: f64) -> Map {
    let mut map = Map::new();
    map.insert("x".into(), Dynamic::from(0.0f64));
    map.insert("y".into(), Dynamic::from(0.0f64));
    map.insert("w".into(), Dynamic::from(w));
    map.insert("h".into(), Dynamic::from(h));
    map
}

fn from_raw(raw: &[f64]) -> Array {
    raw.iter().map(|v| Dynamic::from(*v)).collect()
}

/// A long-tailed distribution: the i-th entry is worth 100000/(i+1).
fn harmonic(count: usize) -> Array {
    (0..count)
        .map(|i| Dynamic::from(100_000.0f64 / (i + 1) as f64))
        .collect()
}

struct CaseResult {
    name: &'static str,
    values: usize,
    ops: u64,
    ms: f64,
    capped_ok: bool,
}

fn run_case(ast: &rhai::AST, name: &'static str, raw: Vec<f64>, w: f64, h: f64) -> CaseResult {
    // True op count and wall time, uncapped.
    let (ops, ms) = {
        let mut engine = runtime_engine();
        let counter = Rc::new(Cell::new(0u64));
        let probe = counter.clone();
        engine.on_progress(move |_| {
            probe.set(probe.get() + 1);
            None
        });
        let start = Instant::now();
        let mut scope = Scope::new();
        let rects: Array = engine
            .call_fn(&mut scope, ast, "squarify", (from_raw(&raw), area(w, h)))
            .expect("squarify runs");
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        let covered: f64 = rects
            .iter()
            .map(|r| {
                let m = r.as_map_ref().expect("rect map");
                let rw = m.get("w").expect("w").as_float().expect("w float");
                let rh = m.get("h").expect("h").as_float().expect("h float");
                rw.max(0.0) * rh.max(0.0)
            })
            .sum();
        assert!(
            (covered - w * h).abs() < w * h * 0.01,
            "{name}: covered {covered} of {}",
            w * h
        );
        (counter.get(), ms)
    };

    // The same call under the production 1M-op cap.
    let capped_ok = {
        let mut engine = runtime_engine();
        engine.set_max_operations(1_000_000);
        let mut scope = Scope::new();
        engine
            .call_fn::<Array>(&mut scope, ast, "squarify", (from_raw(&raw), area(w, h)))
            .is_ok()
    };

    CaseResult {
        name,
        values: raw.len(),
        ops,
        ms,
        capped_ok,
    }
}

fn harmonic_raw(count: usize) -> Vec<f64> {
    (0..count).map(|i| 100_000.0f64 / (i + 1) as f64).collect()
}

fn uniform_raw(count: usize) -> Vec<f64> {
    (0..count).map(|_| 100.0f64).collect()
}

fn main() {
    let core_source = std::fs::read_to_string("ui/core.rhai").expect("run from the repo root");
    let ast = {
        let engine = runtime_engine();
        engine.compile(&core_source).expect("core.rhai compiles")
    };

    // Correctness parity with tobi's treemap tests, on a fresh capped engine.
    {
        let mut engine = runtime_engine();
        engine.set_max_operations(1_000_000);
        let mut scope = Scope::new();

        let rects: Array = engine
            .call_fn(
                &mut scope,
                &ast,
                "squarify",
                (
                    from_raw(&[6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0]),
                    area(600.0, 400.0),
                ),
            )
            .expect("canonical squarify");
        let mut worst = 0.0f64;
        for r in rects.iter() {
            let m = r.as_map_ref().expect("rect map");
            let w = m.get("w").expect("w").as_float().expect("w float");
            let h = m.get("h").expect("h").as_float().expect("h float");
            worst = worst.max((w / h).max(h / w));
        }
        assert!(worst <= 4.0, "canonical aspect ratio {worst}");
        println!("correctness: canonical worst aspect ratio {worst:.3} (limit 4.0) — ok");

        let empty: Array = engine
            .call_fn(
                &mut scope,
                &ast,
                "squarify",
                (Array::new(), area(800.0, 500.0)),
            )
            .expect("empty squarify");
        assert!(empty.is_empty());
        let zeros: Array = engine
            .call_fn(
                &mut scope,
                &ast,
                "squarify",
                (from_raw(&[0.0, 0.0]), area(800.0, 500.0)),
            )
            .expect("zero squarify");
        assert!(zeros.iter().all(|r| {
            let m = r.as_map_ref().expect("rect map");
            let w = m.get("w").expect("w").as_float().expect("w float");
            let h = m.get("h").expect("h").as_float().expect("h float");
            (w * h).abs() < f64::EPSILON
        }));
        println!("correctness: degenerate inputs — ok");
    }

    println!();
    let mut results = vec![
        run_case(&ast, "dir_96 (typical)", harmonic_raw(96), 1200.0, 680.0),
        run_case(&ast, "dir_500", harmonic_raw(500), 1200.0, 680.0),
        run_case(&ast, "dir_1000", harmonic_raw(1000), 1200.0, 680.0),
        run_case(&ast, "dir_5000 (stress)", harmonic_raw(5000), 1200.0, 680.0),
        run_case(
            &ast,
            "uniform_2000 (adversarial)",
            uniform_raw(2000),
            1200.0,
            680.0,
        ),
        run_case(
            &ast,
            "uniform_10000 (adversarial)",
            uniform_raw(10000),
            1200.0,
            680.0,
        ),
    ];

    // A depth-3 layout simulation with tobi's real pruning rules: a home-like
    // root (40 dirs), each big-enough directory subdivided at 96 children
    // until depth 3; tiles under min_tile (5px, after padding) dropped, dirs
    // under 44px wide (or without 15px of body under a header band) kept whole.
    // Counts real squarify calls and their total operations.
    struct Sim {
        engine: Engine,
        ast: rhai::AST,
        counter: Rc<Cell<u64>>,
        calls: usize,
        tiles: usize,
    }
    impl Sim {
        fn squarify(&mut self, values: &Array, _x: f64, _y: f64, w: f64, h: f64) -> Vec<[f64; 4]> {
            self.calls += 1;
            let before = self.counter.get();
            let mut scope = Scope::new();
            let rects: Array = self
                .engine
                .call_fn::<Array>(
                    &mut scope,
                    &self.ast,
                    "squarify",
                    (values.clone(), area(w, h)),
                )
                .expect("sim squarify");
            let spent = self.counter.get() - before;
            if spent > 900_000 {
                panic!("single-dir squarify nearly at budget: {spent}");
            }
            rects
                .iter()
                .map(|r| {
                    let m = r.as_map_ref().expect("rect map");
                    let rx = m.get("x").unwrap().as_float().unwrap();
                    let ry = m.get("y").unwrap().as_float().unwrap();
                    let rw = m.get("w").unwrap().as_float().unwrap();
                    let rh = m.get("h").unwrap().as_float().unwrap();
                    [rx, ry, rw, rh]
                })
                .collect()
        }
        /// Mirrors place_children: inset by padding, drop min_tile, recurse
        /// into subdividable dirs below their header band.
        #[allow(clippy::too_many_arguments)]
        fn place(
            &mut self,
            values: &Array,
            x: f64,
            y: f64,
            w: f64,
            h: f64,
            depth: usize,
            max_depth: usize,
        ) {
            if w <= 0.0 || h <= 0.0 || values.is_empty() {
                return;
            }
            let padding = if depth == 0 { 3.0 } else { 1.0 };
            for rect in self.squarify(values, x, y, w, h) {
                let [rx, ry, rw, rh] = rect;
                let rw = rw - padding * 2.0;
                let rh = rh - padding * 2.0;
                if rw < 5.0 || rh < 5.0 {
                    continue;
                }
                self.tiles += 1;
                if depth + 1 >= max_depth {
                    continue;
                }
                let header = if depth == 0 { 20.0 } else { 15.0 };
                if rw < 44.0 || rh - header < 15.0 {
                    continue;
                }
                let body = area(rw, rh - header);
                let _ = body;
                self.place(
                    &harmonic(96),
                    rx + padding,
                    ry + padding + header,
                    rw,
                    rh - header,
                    depth + 1,
                    max_depth,
                );
            }
        }
    }
    let sim_result = {
        let counter = Rc::new(Cell::new(0u64));
        let probe = counter.clone();
        let mut engine = runtime_engine();
        engine.on_progress(move |_| {
            probe.set(probe.get() + 1);
            None
        });
        let mut sim = Sim {
            engine,
            ast: ast.clone(),
            counter,
            calls: 0,
            tiles: 0,
        };
        let start = Instant::now();
        sim.place(&harmonic(40), 0.0, 0.0, 1200.0, 680.0, 0, 3);
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        println!(
            "layout_sim_depth3: {} squarify calls, {} tiles kept, {} ops, {:.1}ms",
            sim.calls,
            sim.tiles,
            sim.counter.get(),
            ms
        );
        CaseResult {
            name: "layout_sim_depth3 (pruned)",
            values: sim.tiles,
            ops: sim.counter.get(),
            ms,
            capped_ok: sim.counter.get() <= 1_000_000,
        }
    };
    results.push(sim_result);

    println!(
        "{:<28} {:>8} {:>12} {:>10} {:>9}",
        "case", "values", "ops", "time", "1M cap"
    );
    for r in &results {
        println!(
            "{:<28} {:>8} {:>12} {:>10} {:>9}",
            r.name,
            r.values,
            r.ops,
            format!("{:.1}ms", r.ms),
            if r.capped_ok { "pass" } else { "FAIL" }
        );
    }
}
