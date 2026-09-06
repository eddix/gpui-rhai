use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, px, size};
use gpui_rhai::{
    AutomationCommand, AutomationLocator, ExecutionOperation, ExecutionTiming, ScriptViewConfig,
    ScriptViewExtension, ScriptViewHandle, ScriptViewHost, ScriptViewPerformanceSnapshot,
    VirtualCollectionId,
};
use serde::Serialize;

#[allow(dead_code)]
#[path = "../../../crates/gpui-rhai/examples/table_1000.rs"]
mod table_1000_example;

struct BenchmarkHost {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for BenchmarkHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

#[derive(Serialize)]
struct BenchmarkMetadata {
    schema: &'static str,
    commit: String,
    dirty: bool,
    rustc: String,
    os: String,
    hardware: String,
    profile: &'static str,
    features: String,
    data_backend: &'static str,
    label: String,
    samples: usize,
    warmup: usize,
    window: [f64; 2],
}

#[derive(Clone, Debug, Serialize)]
struct Sample {
    total_us: u64,
    recorded_rhai_us: u64,
    root_rhai_us: u64,
    virtual_rhai_us: u64,
    operations: u64,
    root_operations: u64,
    virtual_operations: u64,
    timing_count: usize,
    reused_component_subtrees: usize,
    reused_components: usize,
    retained_nodes: usize,
    dirty_components: usize,
    virtual_items: usize,
    virtual_realized: usize,
}

#[derive(Serialize)]
struct ScenarioReport {
    name: String,
    samples: Vec<Sample>,
    total_p50_us: u64,
    total_p95_us: u64,
    total_p99_us: u64,
    rhai_p50_us: u64,
    rhai_p95_us: u64,
    root_rhai_p95_us: u64,
    virtual_rhai_p95_us: u64,
    operations_p50: u64,
    operations_p95: u64,
    root_operations_p95: u64,
    virtual_operations_p95: u64,
}

#[derive(Serialize)]
struct ColdReport {
    prepare_us: u64,
    mount_and_first_frame_us: u64,
    recorded_rhai_us: u64,
    root_rhai_us: u64,
    virtual_rhai_us: u64,
    operations: u64,
    root_operations: u64,
    virtual_operations: u64,
    retained_nodes: usize,
    virtual_items: usize,
    virtual_realized: usize,
}

#[derive(Serialize)]
struct BenchmarkReport {
    metadata: BenchmarkMetadata,
    cold: ColdReport,
    scenarios: Vec<ScenarioReport>,
}

fn env_usize(name: &str, fallback: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or(fallback)
}

fn command_output(root: &Path, program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map_or_else(
            || "unknown".to_owned(),
            |output| String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        )
}

fn metadata(samples: usize, warmup: usize) -> BenchmarkMetadata {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dirty = !command_output(&root, "git", &["status", "--porcelain"]).is_empty();
    BenchmarkMetadata {
        schema: "gpui-rhai-e2e-v2",
        commit: command_output(&root, "git", &["rev-parse", "HEAD"]),
        dirty,
        rustc: command_output(&root, "rustc", &["-Vv"]),
        os: command_output(&root, "sw_vers", &[]),
        hardware: command_output(&root, "sysctl", &["-n", "hw.model"]),
        profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        features: std::env::var("GPUI_RHAI_BENCH_FEATURES")
            .unwrap_or_else(|_| "default".to_owned()),
        data_backend: "native_collection",
        label: std::env::var("GPUI_RHAI_BENCH_LABEL").unwrap_or_default(),
        samples,
        warmup,
        window: [1_180.0, 820.0],
    }
}

fn micros(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)
}

fn timing_totals(timings: &[ExecutionTiming]) -> (u64, u64, u64, u64, u64, u64) {
    timings.iter().fold(
        (0, 0, 0, 0, 0, 0),
        |(duration, operations, root_duration, virtual_duration, root_ops, virtual_ops), timing| {
            let elapsed = u64::try_from(timing.duration.as_micros()).unwrap_or(u64::MAX);
            if matches!(timing.operation, ExecutionOperation::VirtualCollection(_)) {
                (
                    duration.saturating_add(elapsed),
                    operations.saturating_add(timing.operations),
                    root_duration,
                    virtual_duration.saturating_add(elapsed),
                    root_ops,
                    virtual_ops.saturating_add(timing.operations),
                )
            } else {
                (
                    duration.saturating_add(elapsed),
                    operations.saturating_add(timing.operations),
                    root_duration.saturating_add(elapsed),
                    virtual_duration,
                    root_ops.saturating_add(timing.operations),
                    virtual_ops,
                )
            }
        },
    )
}

fn sample_from(total_us: u64, snapshot: &ScriptViewPerformanceSnapshot) -> Sample {
    let (
        recorded_rhai_us,
        operations,
        root_rhai_us,
        virtual_rhai_us,
        root_operations,
        virtual_operations,
    ) = timing_totals(&snapshot.timings);
    let virtual_items = snapshot
        .virtual_collections
        .iter()
        .map(|collection| collection.item_count)
        .sum();
    let virtual_realized = snapshot
        .virtual_collections
        .iter()
        .map(|collection| collection.realized_count)
        .sum();
    let reused_component_subtrees = snapshot
        .timings
        .iter()
        .filter(|timing| {
            matches!(timing.operation, ExecutionOperation::ComponentReuse(_))
        })
        .count();
    let reused_components = snapshot
        .timings
        .iter()
        .filter_map(|timing| match timing.operation {
            ExecutionOperation::ComponentReuse(components) => Some(components),
            _ => None,
        })
        .sum();
    Sample {
        total_us,
        recorded_rhai_us,
        root_rhai_us,
        virtual_rhai_us,
        operations,
        root_operations,
        virtual_operations,
        timing_count: snapshot.timings.len(),
        reused_component_subtrees,
        reused_components,
        retained_nodes: snapshot.retained_nodes,
        dirty_components: snapshot.dirty_components,
        virtual_items,
        virtual_realized,
    }
}

fn take_snapshot(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> ScriptViewPerformanceSnapshot {
    visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap()
}

type VirtualSignature = Vec<(VirtualCollectionId, std::ops::Range<usize>, usize)>;

fn virtual_signature(snapshot: &ScriptViewPerformanceSnapshot) -> VirtualSignature {
    snapshot
        .virtual_collections
        .iter()
        .map(|collection| {
            (
                collection.id.clone(),
                collection.realized_range.clone(),
                collection.realized_count,
            )
        })
        .collect()
}

fn merge_snapshot(
    combined: &mut ScriptViewPerformanceSnapshot,
    mut next: ScriptViewPerformanceSnapshot,
) {
    combined.timings.append(&mut next.timings);
    combined.virtual_collections = next.virtual_collections;
    combined.retained_nodes = next.retained_nodes;
    combined.dirty_components = next.dirty_components;
    combined.pending_virtual_requests = next.pending_virtual_requests;
}

fn settle(
    cx: &TestAppContext,
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> ScriptViewPerformanceSnapshot {
    // Let the host frame-poll task install its first timer before advancing a
    // deterministic test clock. Otherwise a very fast mount can repeatedly
    // schedule the timer just after each artificial advance.
    visual.run_until_parked();
    let mut combined = take_snapshot(visual, view);
    let mut previous = virtual_signature(&combined);
    let mut stable_observations = 0usize;
    for _ in 0..12 {
        cx.background_executor
            .advance_clock(Duration::from_millis(16));
        visual
            .update(|window, app| {
                view.automate(AutomationCommand::AdvanceTime { millis: 16 }, window, app)
            })
            .unwrap();
        visual.run_until_parked();
        let next = take_snapshot(visual, view);
        let signature = virtual_signature(&next);
        if !next.pending_virtual_requests && signature == previous {
            stable_observations = stable_observations.saturating_add(1);
        } else {
            stable_observations = 0;
        }
        previous = signature;
        merge_snapshot(&mut combined, next);
        if stable_observations >= 1 {
            assert_eq!(combined.dirty_components, 0);
            assert!(!combined.pending_virtual_requests);
            return combined;
        }
    }
    panic!(
        "virtual collections did not settle: pending={} signature={previous:?} metrics={:?}",
        combined.pending_virtual_requests, combined.virtual_collections
    );
}

fn dispatch_button(visual: &mut VisualTestContext, view: &ScriptViewHandle, label: &str) {
    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::RoleName {
                        role: "button".to_owned(),
                        name: label.to_owned(),
                    },
                    event: "click".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
    visual.run_until_parked();
}

#[derive(Clone, Copy)]
struct ButtonScenario<'a> {
    name: &'a str,
    setup: Option<&'a str>,
    action: &'a str,
}

fn button_scenario(
    cx: &TestAppContext,
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    scenario: ButtonScenario<'_>,
    warmup: usize,
    samples: usize,
) -> ScenarioReport {
    let ButtonScenario {
        name,
        setup,
        action,
    } = scenario;
    for _ in 0..warmup {
        if let Some(setup) = setup {
            dispatch_button(visual, view, setup);
            let _ = settle(cx, visual, view);
        }
        dispatch_button(visual, view, action);
        let _ = settle(cx, visual, view);
    }
    let mut measured = Vec::with_capacity(samples);
    for _ in 0..samples {
        if let Some(setup) = setup {
            dispatch_button(visual, view, setup);
            let _ = settle(cx, visual, view);
        }
        let _ = take_snapshot(visual, view);
        let started = Instant::now();
        dispatch_button(visual, view, action);
        let snapshot = settle(cx, visual, view);
        let total_us = micros(started);
        assert_eq!(snapshot.dirty_components, 0, "{name} did not settle");
        let sample = sample_from(total_us, &snapshot);
        if name == "unchanged_data_rerender" {
            assert!(
                sample.reused_components >= 5,
                "unchanged root render did not reuse the four buttons and Table: {sample:#?}"
            );
        }
        measured.push(sample);
    }
    summarize(name, measured)
}

fn resize_scenario(
    cx: &TestAppContext,
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    warmup: usize,
    samples: usize,
) -> ScenarioReport {
    for index in 0..warmup {
        let dimensions = if index.is_multiple_of(2) {
            size(px(820.0), px(620.0))
        } else {
            size(px(1_180.0), px(820.0))
        };
        visual.simulate_resize(dimensions);
        let _ = settle(cx, visual, view);
    }
    let mut measured = Vec::with_capacity(samples);
    for offset in 0..samples {
        let index = warmup + offset;
        let dimensions = if index.is_multiple_of(2) {
            size(px(820.0), px(620.0))
        } else {
            size(px(1_180.0), px(820.0))
        };
        let _ = take_snapshot(visual, view);
        let started = Instant::now();
        visual.simulate_resize(dimensions);
        let snapshot = settle(cx, visual, view);
        let total_us = micros(started);
        assert_eq!(snapshot.dirty_components, 0, "resize did not settle");
        assert!(
            snapshot
                .timings
                .iter()
                .all(|timing| matches!(timing.operation, ExecutionOperation::VirtualCollection(_))),
            "resize may realize newly visible rows but must not rerun root/components: {:?}",
            snapshot.timings
        );
        measured.push(sample_from(total_us, &snapshot));
    }
    summarize("window_resize_native", measured)
}

fn percentile(sorted: &[u64], percentile: usize) -> u64 {
    let rank = (sorted.len() * percentile).div_ceil(100).saturating_sub(1);
    sorted[rank.min(sorted.len() - 1)]
}

fn percentiles(values: impl Iterator<Item = u64>) -> (u64, u64, u64) {
    let mut values = values.collect::<Vec<_>>();
    values.sort_unstable();
    (
        percentile(&values, 50),
        percentile(&values, 95),
        percentile(&values, 99),
    )
}

fn summarize(name: &str, samples: Vec<Sample>) -> ScenarioReport {
    let (total_p50_us, total_p95_us, total_p99_us) =
        percentiles(samples.iter().map(|sample| sample.total_us));
    let (rhai_p50_us, rhai_p95_us, _) =
        percentiles(samples.iter().map(|sample| sample.recorded_rhai_us));
    let (_, root_rhai_p95_us, _) = percentiles(samples.iter().map(|sample| sample.root_rhai_us));
    let (_, virtual_rhai_p95_us, _) =
        percentiles(samples.iter().map(|sample| sample.virtual_rhai_us));
    let (operations_p50, operations_p95, _) =
        percentiles(samples.iter().map(|sample| sample.operations));
    let (_, root_operations_p95, _) =
        percentiles(samples.iter().map(|sample| sample.root_operations));
    let (_, virtual_operations_p95, _) =
        percentiles(samples.iter().map(|sample| sample.virtual_operations));
    ScenarioReport {
        name: name.to_owned(),
        samples,
        total_p50_us,
        total_p95_us,
        total_p99_us,
        rhai_p50_us,
        rhai_p95_us,
        root_rhai_p95_us,
        virtual_rhai_p95_us,
        operations_p50,
        operations_p95,
        root_operations_p95,
        virtual_operations_p95,
    }
}

#[gpui::test]
#[ignore = "run with scripts/benchmark.sh"]
#[allow(clippy::too_many_lines)]
fn table_1000_end_to_end_baseline(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let samples = env_usize(
        "GPUI_RHAI_BENCH_SAMPLES",
        if cfg!(debug_assertions) { 3 } else { 30 },
    );
    let warmup = env_usize(
        "GPUI_RHAI_BENCH_WARMUP",
        if cfg!(debug_assertions) { 1 } else { 5 },
    );

    let prepare_started = Instant::now();
    let runtime_clock = gpui_rhai::ManualRuntimeClock::new(Instant::now());
    let prepared = table_1000_example::table_1000_view()
        .runtime_clock(runtime_clock.clock())
        .prepare()
        .unwrap();
    let prepare_us = micros(prepare_started);

    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let mount_started = Instant::now();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("table-1000-benchmark-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("table-1000-benchmark-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some(view.clone());
        BenchmarkHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(1_180.0), px(820.0)));
    let cold_snapshot = settle(cx, &mut visual, &view);
    let mount_and_first_frame_us = micros(mount_started);
    let cold_sample = sample_from(0, &cold_snapshot);
    assert_eq!(cold_sample.virtual_items, 1_000);
    assert!(cold_sample.virtual_realized < 100);
    assert!(cold_sample.virtual_realized > 0);

    let scenarios = vec![
        button_scenario(
            cx,
            &mut visual,
            &view,
            ButtonScenario {
                name: "unchanged_data_rerender",
                setup: None,
                action: "Re-render same data",
            },
            warmup,
            samples,
        ),
        button_scenario(
            cx,
            &mut visual,
            &view,
            ButtonScenario {
                name: "reverse_1000_rows",
                setup: None,
                action: "Reverse 1,000 rows",
            },
            warmup,
            samples,
        ),
        button_scenario(
            cx,
            &mut visual,
            &view,
            ButtonScenario {
                name: "select_row_500",
                setup: Some("Clear selection"),
                action: "Select row 500",
            },
            warmup,
            samples,
        ),
        resize_scenario(cx, &mut visual, &view, warmup, samples),
    ];

    let report = BenchmarkReport {
        metadata: metadata(samples, warmup),
        cold: ColdReport {
            prepare_us,
            mount_and_first_frame_us,
            recorded_rhai_us: cold_sample.recorded_rhai_us,
            root_rhai_us: cold_sample.root_rhai_us,
            virtual_rhai_us: cold_sample.virtual_rhai_us,
            operations: cold_sample.operations,
            root_operations: cold_sample.root_operations,
            virtual_operations: cold_sample.virtual_operations,
            retained_nodes: cold_sample.retained_nodes,
            virtual_items: cold_sample.virtual_items,
            virtual_realized: cold_sample.virtual_realized,
        },
        scenarios,
    };
    let json = serde_json::to_string_pretty(&report).unwrap();
    println!("{json}");
    if let Ok(path) = std::env::var("GPUI_RHAI_BENCH_OUTPUT") {
        std::fs::write(path, format!("{json}\n")).unwrap();
    }
}

#[derive(Serialize)]
struct DocumentBenchmarkReport {
    schema: &'static str,
    lines: usize,
    bytes: usize,
    direct_string_rhai_prepare_us: u64,
    native_document_rhai_prepare_us: u64,
    rust_highlight_us: u64,
    rust_diff_us: u64,
    native_ui_prepare_us: u64,
    native_ui_mount_first_frame_us: u64,
    native_ui_resize_dispatch_p95_us: u64,
    native_ui_resize_settle_p50_us: u64,
    native_ui_resize_settle_p95_us: u64,
    diff_hunks: usize,
    diff_rows: usize,
}

#[derive(Clone)]
struct BenchmarkDocumentExtension {
    document: gpui_rhai::NativeTextDocument,
}

impl ScriptViewExtension for BenchmarkDocumentExtension {
    fn configure_runtime(&self, runtime: &mut gpui_rhai::UiRuntimeState) -> Result<(), String> {
        runtime
            .native_documents
            .register("benchmark", self.document.clone())
            .map_err(|error| error.to_string())
    }
}

#[derive(Clone)]
struct BenchmarkDocumentPairExtension {
    left: gpui_rhai::NativeTextDocument,
    right: gpui_rhai::NativeTextDocument,
}

impl ScriptViewExtension for BenchmarkDocumentPairExtension {
    fn configure_runtime(&self, runtime: &mut gpui_rhai::UiRuntimeState) -> Result<(), String> {
        runtime
            .native_documents
            .register("benchmark-left", self.left.clone())
            .map_err(|error| error.to_string())?;
        runtime
            .native_documents
            .register("benchmark-right", self.right.clone())
            .map_err(|error| error.to_string())
    }
}

fn benchmark_document_source(lines: usize, changed: bool) -> String {
    use std::fmt::Write as _;

    let mut source = String::with_capacity(lines * 48);
    for index in 0..lines {
        let value = if changed && index.is_multiple_of(997) {
            index.saturating_add(1)
        } else {
            index
        };
        let _ = writeln!(
            source,
            "let server_{index} = #{{ port: {}, enabled: true }};",
            8_000 + value % 1_000
        );
    }
    source
}

fn document_component_source() -> std::collections::BTreeMap<gpui_rhai::ModuleId, String> {
    std::collections::BTreeMap::from([
        (
            gpui_rhai::ModuleId::parse("components/code_viewer").unwrap(),
            include_str!("../../../registry/components/code_viewer.rhai").to_owned(),
        ),
        (
            gpui_rhai::ModuleId::parse("components/diff_viewer").unwrap(),
            include_str!("../../../registry/components/diff_viewer.rhai").to_owned(),
        ),
    ])
}

#[gpui::test]
#[ignore = "run with scripts/benchmark.sh"]
#[allow(clippy::too_many_lines)]
fn document_end_to_end_baseline(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let lines = env_usize("GPUI_RHAI_DOCUMENT_BENCH_LINES", 20_000);
    let left = benchmark_document_source(lines, false);
    let right = benchmark_document_source(lines, true);
    let entry = gpui_rhai::ModuleId::parse("main").unwrap();
    let theme = include_str!("../../../registry/themes/default_dark.rhai");
    // Keep the direct-string/native comparison about the Rhai data boundary;
    // cold syntax-pack initialization is a separate process-level one-time cost.
    let _syntax_pack = gpui_rhai::SyntaxRegistry::new();

    let direct_main = format!(
        "import \"components/code_viewer\" as code_viewer;\nfn view(ctx) {{ code_viewer::CodeViewer(#{{ key: \"bench\", label: \"Benchmark\", language: \"rhai\", source: {} }}) }}",
        serde_json::to_string(&left).unwrap()
    );
    let mut direct_sources = document_component_source();
    direct_sources.insert(entry.clone(), direct_main);
    let direct_started = Instant::now();
    gpui_rhai::EmbeddedScriptView::new(
        entry.clone(),
        gpui_rhai::EmbeddedScriptSource::new(direct_sources),
        theme,
    )
    .prepare()
    .unwrap();
    let direct_string_rhai_prepare_us = micros(direct_started);

    let native = gpui_rhai::NativeTextDocument::new("benchmark", 1, Arc::<str>::from(left.clone()))
        .unwrap();
    let native_main = "import \"components/code_viewer\" as code_viewer;\nfn view(ctx) { code_viewer::CodeViewer(#{ key: \"bench\", label: \"Benchmark\", language: \"rhai\", source: ctx.get_native_text_document(\"benchmark\") }) }";
    let mut native_sources = document_component_source();
    native_sources.insert(entry.clone(), native_main.to_owned());
    let native_started = Instant::now();
    gpui_rhai::EmbeddedScriptView::new(
        entry,
        gpui_rhai::EmbeddedScriptSource::new(native_sources),
        theme,
    )
    .extension(BenchmarkDocumentExtension { document: native })
    .prepare()
    .unwrap();
    let native_document_rhai_prepare_us = micros(native_started);

    let syntaxes = gpui_rhai::SyntaxRegistry::new();
    let descriptor = |text: String, label: &str| gpui_rhai::DocumentDescriptor {
        source: gpui_rhai::DocumentSource::from(text),
        label: label.to_owned(),
        file_name: Some("benchmark.rhai".to_owned()),
        language: Some("rhai".to_owned()),
    };
    let left_descriptor = descriptor(left, "left");
    let right_descriptor = descriptor(right, "right");
    let highlight_started = Instant::now();
    let highlighted = gpui_rhai::prepare_document(&left_descriptor, &syntaxes).unwrap();
    let rust_highlight_us = micros(highlight_started);
    let diff_started = Instant::now();
    let diff = gpui_rhai::prepare_diff(
        &left_descriptor,
        &right_descriptor,
        gpui_rhai::DiffWhitespace::Exact,
        Some(3),
        &syntaxes,
    )
    .unwrap();
    let rust_diff_us = micros(diff_started);
    assert_eq!(highlighted.lines().len(), lines + 1);
    assert!(diff.hunk_count > 0);

    let native_ui_main = r#"
import "components/code_viewer" as code_viewer;
import "components/diff_viewer" as diff_viewer;
fn view(ctx) {
    let left = ctx.get_native_text_document("benchmark-left");
    let right = ctx.get_native_text_document("benchmark-right");
    column([
        code_viewer::CodeViewer(#{ key: "code", label: "Benchmark source",
            language: "rhai", source: left, wrap: "viewport",
            style: style().height(relative(0.42)) }),
        diff_viewer::DiffViewer(#{ key: "diff", mode: "split", wrap: "viewport",
            left: #{ source: left, label: "Left", language: "rhai" },
            right: #{ source: right, label: "Right", language: "rhai" },
            style: style().height(relative(0.58)) })
    ]).with_style(style().width(relative(1)).height(relative(1))
        .min_width(px(0)).min_height(px(0)))
}
"#;
    let mut native_ui_sources = document_component_source();
    native_ui_sources.insert(
        gpui_rhai::ModuleId::parse("document-ui-benchmark").unwrap(),
        native_ui_main.to_owned(),
    );
    let native_ui_prepare_started = Instant::now();
    let native_ui_clock = gpui_rhai::ManualRuntimeClock::new(Instant::now());
    let native_ui_prepared = gpui_rhai::EmbeddedScriptView::new(
        gpui_rhai::ModuleId::parse("document-ui-benchmark").unwrap(),
        gpui_rhai::EmbeddedScriptSource::new(native_ui_sources),
        theme,
    )
    .extension(BenchmarkDocumentPairExtension {
        left: gpui_rhai::NativeTextDocument::new(
            "benchmark-left",
            1,
            left_descriptor.source.text(),
        )
        .unwrap(),
        right: gpui_rhai::NativeTextDocument::new(
            "benchmark-right",
            1,
            right_descriptor.source.text(),
        )
        .unwrap(),
    })
    .runtime_clock(native_ui_clock.clock())
    .prepare()
    .unwrap();
    let native_ui_prepare_us = micros(native_ui_prepare_started);

    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let native_ui_mount_started = Instant::now();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("document-benchmark-window", cx).unwrap();
        let view = native_ui_prepared
            .mount(
                ScriptViewConfig::new("document-benchmark-view"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some(view.clone());
        BenchmarkHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(1_180.0), px(820.0)));
    let _ = settle(cx, &mut visual, &view);
    let native_ui_mount_first_frame_us = micros(native_ui_mount_started);

    let resize_samples = env_usize(
        "GPUI_RHAI_BENCH_SAMPLES",
        if cfg!(debug_assertions) { 3 } else { 30 },
    );
    let mut resize_dispatch_us = Vec::with_capacity(resize_samples);
    let mut resize_settle_us = Vec::with_capacity(resize_samples);
    for index in 0..resize_samples {
        let dimensions = if index.is_multiple_of(2) {
            size(px(900.0), px(700.0))
        } else {
            size(px(1_180.0), px(820.0))
        };
        let started = Instant::now();
        visual.simulate_resize(dimensions);
        resize_dispatch_us.push(micros(started));
        let snapshot = settle(cx, &mut visual, &view);
        assert_eq!(snapshot.dirty_components, 0, "document resize did not settle");
        resize_settle_us.push(micros(started));
    }
    resize_dispatch_us.sort_unstable();
    resize_settle_us.sort_unstable();
    let native_ui_resize_dispatch_p95_us = percentile(&resize_dispatch_us, 95);
    let native_ui_resize_settle_p50_us = percentile(&resize_settle_us, 50);
    let native_ui_resize_settle_p95_us = percentile(&resize_settle_us, 95);

    let report = DocumentBenchmarkReport {
        schema: "gpui-rhai-document-e2e-v2",
        lines,
        bytes: highlighted.text().len(),
        direct_string_rhai_prepare_us,
        native_document_rhai_prepare_us,
        rust_highlight_us,
        rust_diff_us,
        native_ui_prepare_us,
        native_ui_mount_first_frame_us,
        native_ui_resize_dispatch_p95_us,
        native_ui_resize_settle_p50_us,
        native_ui_resize_settle_p95_us,
        diff_hunks: diff.hunk_count,
        diff_rows: diff.rows.len(),
    };
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
