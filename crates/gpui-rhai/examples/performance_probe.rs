use std::hint::black_box;
use std::time::{Duration, Instant};

use gpui_rhai::{GpuiNodeRenderer, RuntimeEngine};

const ITERATIONS: usize = 50;

fn percentile95(samples: &mut [Duration]) -> Duration {
    samples.sort_unstable();
    samples[(samples.len() * 95 / 100).min(samples.len() - 1)]
}

fn main() {
    let source = r"
        fn view() {
            let children = [];
            for index in 0..1000 {
                children.push(text(`row ${index}`));
            }
            column(children)
        }
    ";
    let mut runtime = RuntimeEngine::new();
    let compile_started = Instant::now();
    let compiled = runtime
        .compile(source)
        .expect("performance source compiles");
    let compile = compile_started.elapsed();
    let root = runtime
        .render(&compiled)
        .expect("performance source renders");

    let mut views = Vec::with_capacity(ITERATIONS);
    let mut conversions = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let started = Instant::now();
        black_box(runtime.render(&compiled).expect("repeat view renders"));
        views.push(started.elapsed());

        let started = Instant::now();
        black_box(GpuiNodeRenderer::render(&root));
        conversions.push(started.elapsed());
    }

    println!(
        "compile={compile:?} view_p95={:?} conversion_1000_nodes_p95={:?}",
        percentile95(&mut views),
        percentile95(&mut conversions),
    );
}
