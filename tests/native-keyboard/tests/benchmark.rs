use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, px, size};
use gpui_rhai::{
    AutomationCommand, AutomationLocator, ExecutionOperation, ExecutionTiming, ScriptViewConfig,
    ScriptViewExtension, ScriptViewHandle, ScriptViewHost, ScriptViewPerformanceSnapshot, UiValue,
    VirtualCollectionId,
};
use serde::Serialize;

#[allow(dead_code)]
#[path = "../../../crates/gpui-rhai/examples/internal/performance/table_1000.rs"]
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
        .filter(|timing| matches!(timing.operation, ExecutionOperation::ComponentReuse(_)))
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
    combined.dirty_component_paths = next.dirty_component_paths;
    combined.pending_virtual_requests = next.pending_virtual_requests;
}

fn settle(
    cx: &mut TestAppContext,
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
    for _ in 0..120 {
        cx.background_executor
            .advance_clock(Duration::from_millis(16));
        visual
            .update(|window, app| {
                view.automate(AutomationCommand::AdvanceTime { millis: 16 }, window, app)
            })
            .unwrap();
        visual.run_until_parked();
        cx.refresh().unwrap();
        visual.run_until_parked();
        std::thread::yield_now();
        let next = take_snapshot(visual, view);
        let signature = virtual_signature(&next);
        if !next.pending_virtual_requests && next.dirty_components == 0 && signature == previous {
            stable_observations = stable_observations.saturating_add(1);
        } else {
            stable_observations = 0;
        }
        previous = signature;
        let final_dirty = next.dirty_components;
        let final_pending = next.pending_virtual_requests;
        merge_snapshot(&mut combined, next);
        if stable_observations >= 1 {
            assert_eq!(final_dirty, 0);
            assert!(!final_pending);
            combined.dirty_components = final_dirty;
            combined.pending_virtual_requests = final_pending;
            return combined;
        }
    }
    panic!(
        "view did not settle: dirty={} paths={:?} pending={} signature={previous:?} metrics={:?}",
        combined.dirty_components,
        combined.dirty_component_paths,
        combined.pending_virtual_requests,
        combined.virtual_collections
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
    cx: &mut TestAppContext,
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
    cx: &mut TestAppContext,
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
    .token_base(include_str!("../../../registry/tokens.rhai"))
    .prepare()
    .unwrap();
    let direct_string_rhai_prepare_us = micros(direct_started);

    let native =
        gpui_rhai::NativeTextDocument::new("benchmark", 1, Arc::<str>::from(left.clone())).unwrap();
    let native_main = "import \"components/code_viewer\" as code_viewer;\nfn view(ctx) { code_viewer::CodeViewer(#{ key: \"bench\", label: \"Benchmark\", language: \"rhai\", source: ctx.get_native_text_document(\"benchmark\") }) }";
    let mut native_sources = document_component_source();
    native_sources.insert(entry.clone(), native_main.to_owned());
    let native_started = Instant::now();
    gpui_rhai::EmbeddedScriptView::new(
        entry,
        gpui_rhai::EmbeddedScriptSource::new(native_sources),
        theme,
    )
    .token_base(include_str!("../../../registry/tokens.rhai"))
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
    .token_base(include_str!("../../../registry/tokens.rhai"))
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
        assert_eq!(
            snapshot.dirty_components, 0,
            "document resize did not settle"
        );
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

#[derive(Serialize)]
struct ChartBenchmarkReport {
    schema: &'static str,
    points: usize,
    prepare_us: u64,
    mount_first_frame_us: u64,
    resize_p50_us: u64,
    resize_p95_us: u64,
    streaming_p50_us: u64,
    streaming_p95_us: u64,
    retained_nodes: usize,
    rhai_operations_after_stream: u64,
}

fn chart_stream_chunk(start: usize, rows: usize) -> gpui_rhai::ChartDataset {
    #[allow(clippy::cast_precision_loss)]
    let values = (start..start + rows)
        .map(|index| {
            let x = index as f64;
            std::collections::BTreeMap::from([
                (
                    "id".to_owned(),
                    gpui_rhai::ChartValue::String(format!("p{index}")),
                ),
                ("x".to_owned(), gpui_rhai::ChartValue::Number(x)),
                (
                    "y".to_owned(),
                    gpui_rhai::ChartValue::Number(
                        (x / 270.0).sin() * 20.0 + (x / 67.0).cos() * 4.0,
                    ),
                ),
            ])
        })
        .collect::<Vec<_>>();
    gpui_rhai::ChartDataset::from_chart_rows(
        "main",
        &values,
        Some("id".to_owned()),
        gpui_rhai::ChartDataLimits::default(),
    )
    .unwrap()
}

fn settle_chart(
    cx: &mut TestAppContext,
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> ScriptViewPerformanceSnapshot {
    visual.run_until_parked();
    cx.refresh().unwrap();
    visual.run_until_parked();
    let mut combined = take_snapshot(visual, view);
    let mut stable_observations = 0usize;
    for _ in 0..240 {
        cx.background_executor
            .advance_clock(Duration::from_millis(16));
        visual.run_until_parked();
        cx.refresh().unwrap();
        visual.run_until_parked();
        std::thread::yield_now();
        let next = take_snapshot(visual, view);
        if next.dirty_components == 0 && !next.pending_virtual_requests {
            stable_observations = stable_observations.saturating_add(1);
        } else {
            stable_observations = 0;
        }
        let final_dirty = next.dirty_components;
        let final_pending = next.pending_virtual_requests;
        merge_snapshot(&mut combined, next);
        if stable_observations >= 2 {
            combined.dirty_components = final_dirty;
            combined.pending_virtual_requests = final_pending;
            return combined;
        }
    }
    panic!(
        "chart did not settle: dirty={} paths={:?} pending={}",
        combined.dirty_components,
        combined.dirty_component_paths,
        combined.pending_virtual_requests
    )
}

fn presented_chart_revision(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    name: &str,
) -> Option<u64> {
    visual.update(|_, cx| {
        let tree = view.accessibility_snapshot(cx).ok()?;
        let chart = tree.find_by_role_and_name("figure", name).next()?;
        let UiValue::Map(value) = chart.value.as_ref()? else {
            return None;
        };
        let UiValue::Integer(revision) = value.get("revision")? else {
            return None;
        };
        u64::try_from(*revision).ok()
    })
}

#[gpui::test]
#[ignore = "run with scripts/benchmark.sh"]
#[allow(clippy::too_many_lines)]
fn chart_end_to_end_baseline(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let samples = env_usize(
        "GPUI_RHAI_BENCH_SAMPLES",
        if cfg!(debug_assertions) { 3 } else { 30 },
    );
    let warmup = env_usize(
        "GPUI_RHAI_BENCH_WARMUP",
        if cfg!(debug_assertions) { 1 } else { 5 },
    );
    let points = env_usize("GPUI_RHAI_CHART_BENCH_POINTS", 100_000);
    let stream = gpui_rhai::NativeChartData::new(
        [chart_stream_chunk(0, points)],
        gpui_rhai::ChartDataLimits::default(),
    )
    .unwrap();
    let prepare_started = Instant::now();
    let runtime_clock = gpui_rhai::ManualRuntimeClock::new(Instant::now());
    let prepared = gpui_rhai_cli::gallery::chart_streaming_view_with_data(stream.clone())
        .unwrap()
        .runtime_clock(runtime_clock.clock())
        .motion_preference(gpui_rhai::MotionPreference::None)
        .prepare()
        .unwrap();
    let prepare_us = micros(prepare_started);

    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let mount_started = Instant::now();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("chart-benchmark-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("chart-benchmark-view"),
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
    let cold = settle_chart(cx, &mut visual, &view);
    let mount_first_frame_us = micros(mount_started);

    let mut resize = Vec::with_capacity(samples);
    for index in 0..warmup + samples {
        let dimensions = if index.is_multiple_of(2) {
            size(px(980.0), px(720.0))
        } else {
            size(px(1_180.0), px(820.0))
        };
        let started = Instant::now();
        visual.simulate_resize(dimensions);
        let _ = settle_chart(cx, &mut visual, &view);
        if index >= warmup {
            resize.push(micros(started));
        }
    }

    let mut streaming = Vec::with_capacity(samples);
    let mut next = points;
    let mut operations = 0_u64;
    for index in 0..warmup + samples {
        let chunk = chart_stream_chunk(next, 128);
        next += 128;
        let started = Instant::now();
        stream.append_sliding("main", &chunk, points).unwrap();
        let snapshot = settle_chart(cx, &mut visual, &view);
        assert_eq!(
            presented_chart_revision(&mut visual, &view, "line"),
            Some(stream.revision()),
            "benchmark stopped before the target NativeChartData revision was presented"
        );
        operations = operations.saturating_add(
            snapshot
                .timings
                .iter()
                .map(|timing| timing.operations)
                .sum::<u64>(),
        );
        if index >= warmup {
            streaming.push(micros(started));
        }
    }
    resize.sort_unstable();
    streaming.sort_unstable();
    let report = ChartBenchmarkReport {
        schema: "gpui-rhai-chart-e2e-v2",
        points,
        prepare_us,
        mount_first_frame_us,
        resize_p50_us: percentile(&resize, 50),
        resize_p95_us: percentile(&resize, 95),
        streaming_p50_us: percentile(&streaming, 50),
        streaming_p95_us: percentile(&streaming, 95),
        retained_nodes: cold.retained_nodes,
        rhai_operations_after_stream: operations,
    };
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

// Scroll benchmark: what one wheel step over a Table costs end to end, beside
// trees of growing size. Schema `gpui-rhai-scroll-e2e-v1`.

const SCROLL_TABLE: &str = include_str!("../../../registry/components/table.rhai");
const SCROLL_BADGE: &str = include_str!("../../../registry/components/badge.rhai");
const SCROLL_THEME: &str = include_str!("../../../registry/themes/default_dark.rhai");
const SCROLL_TOKENS: &str = include_str!("../../../registry/tokens.rhai");
const SCROLL_WINDOW: [f32; 2] = [1_180.0, 820.0];

/// One page the scroll benchmark mounts: a 400-row Table of 320px beside
/// `filler` plain rows of three texts (four nodes each), before or after it,
/// and optionally a second Table, which is the one scrolled.
#[derive(Clone, Copy)]
struct ScrollLayout {
    name: &'static str,
    filler: usize,
    filler_first: bool,
    two_tables: bool,
    props: &'static str,
}

const SCROLL_LAYOUTS: &[ScrollLayout] = &[
    ScrollLayout {
        name: "filler_0",
        filler: 0,
        filler_first: false,
        two_tables: false,
        props: "",
    },
    ScrollLayout {
        name: "filler_330",
        filler: 330,
        filler_first: false,
        two_tables: false,
        props: "",
    },
    ScrollLayout {
        name: "filler_1000",
        filler: 1_000,
        filler_first: false,
        two_tables: false,
        props: "",
    },
    ScrollLayout {
        name: "filler_2500",
        filler: 2_500,
        filler_first: false,
        two_tables: false,
        props: "",
    },
    ScrollLayout {
        name: "filler_1000_batch_1",
        filler: 1_000,
        filler_first: false,
        two_tables: false,
        props: "batch_rows:1,retain_rows:0,",
    },
    ScrollLayout {
        name: "filler_1000_first",
        filler: 1_000,
        filler_first: true,
        two_tables: false,
        props: "",
    },
    ScrollLayout {
        name: "two_tables_filler_1000",
        filler: 1_000,
        filler_first: false,
        two_tables: true,
        props: "",
    },
];

fn scroll_source(layout: ScrollLayout) -> String {
    let table = |key: &str, label: &str| {
        format!(
            r#"table::Table(#{{key:"{key}",label:"{label}",row_key:"id",height:320,rows:rows(),{props}
            columns:[#{{key:"label",title:"Label",width:#{{kind:"fixed",value:200}}}}]}})"#,
            props = layout.props
        )
    };
    let mut tables = Vec::new();
    if layout.two_tables {
        tables.push(table("first", "First"));
    }
    tables.push(table("rows", "Rows"));
    // Filler first sits beside the Tables, so they stay in the window while
    // the filler comes first in the tree; otherwise it follows them.
    let page = if layout.filler_first {
        format!(
            "row([column(fillers()).with_style(style().width(px(400))), column([{}]).with_style(style().width(px(400)))])",
            tables.join(", ")
        )
    } else {
        format!(
            "column([{}, column(fillers())]).with_style(style().width(px(400)))",
            tables.join(", ")
        )
    };
    format!(
        r#"import "components/table" as table;
fn rows(){{let result=[];for index in 0..400{{result.push(#{{id:`r${{index}}`,label:`Row ${{index}}`}});}}result}}
fn fillers(){{let result=[];for i in 0..{filler}{{result.push(row([text(`Filler ${{i}}`),text("a"),text("b")]).with_key(`f${{i}}`));}}result}}
fn view(ctx){{ {page} }}"#,
        filler = layout.filler,
    )
}

fn scroll_view(layout: ScrollLayout) -> gpui_rhai::EmbeddedScriptView {
    use std::collections::BTreeMap;
    let module =
        |id: &str, source: &str| (gpui_rhai::ModuleId::parse(id).unwrap(), source.to_owned());
    let scripts = gpui_rhai::EmbeddedScriptSource::new(BTreeMap::from([
        module("main", &scroll_source(layout)),
        module("components/table", SCROLL_TABLE),
        module("components/badge", SCROLL_BADGE),
    ]));
    let assets: Vec<(String, gpui_rhai::AssetData)> = [
        (
            "icons/disclosure_down",
            include_bytes!("../../../registry/assets/icons/disclosure_down.svg").as_slice(),
        ),
        (
            "icons/chevron_right",
            include_bytes!("../../../registry/assets/icons/chevron_right.svg").as_slice(),
        ),
        (
            "icons/sort_ascending",
            include_bytes!("../../../registry/assets/icons/sort_ascending.svg").as_slice(),
        ),
        (
            "icons/sort_descending",
            include_bytes!("../../../registry/assets/icons/sort_descending.svg").as_slice(),
        ),
    ]
    .into_iter()
    .map(|(name, bytes)| {
        (
            name.to_owned(),
            gpui_rhai::AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: bytes.to_vec(),
            },
        )
    })
    .collect();
    gpui_rhai::EmbeddedScriptView::new(
        gpui_rhai::ModuleId::parse("main").unwrap(),
        scripts,
        SCROLL_THEME,
    )
    .token_base(SCROLL_TOKENS)
    .asset_sources(assets)
}

#[derive(Clone, Debug, Default, Serialize)]
struct ScrollStep {
    total_us: u64,
    host_renders: u64,
    render_us: u64,
    build_us: u64,
    polls: u64,
    poll_us: u64,
    commits: u64,
    commit_phases_us: Vec<(&'static str, u64)>,
    rows_rendered: usize,
}

#[derive(Serialize)]
struct ScrollClass {
    steps: usize,
    total_p50_us: u64,
    total_p90_us: u64,
    render_p50_us: u64,
    build_p50_us: u64,
    poll_p50_us: u64,
    host_renders_p50: u64,
}

#[derive(Serialize)]
struct ScrollLayoutReport {
    name: &'static str,
    retained_nodes: usize,
    /// Steps that rendered no rows.
    quiet: ScrollClass,
    /// Steps that rendered rows: a poll, a commit and their frames.
    realizing: ScrollClass,
    /// Commit phases summed over the measured steps.
    commit_phases_us: Vec<(&'static str, u64)>,
    steps: Vec<ScrollStep>,
}

#[derive(Serialize)]
struct ReplayReport {
    rows: usize,
    mode: &'static str,
    frame_p50_us: u64,
    frame_p90_us: u64,
}

#[derive(Serialize)]
struct ScrollBenchmarkReport {
    metadata: BenchmarkMetadata,
    layouts: Vec<ScrollLayoutReport>,
    /// GPUI alone: a view rendered every frame beside a static subtree built
    /// inline, as a plain child view, or as a cached child view.
    gpui_replay: Vec<ReplayReport>,
}

fn duration_us(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn scroll_class(steps: &[&ScrollStep]) -> ScrollClass {
    let p = |values: Vec<u64>, rank: usize| {
        if values.is_empty() {
            return 0;
        }
        let mut values = values;
        values.sort_unstable();
        percentile(&values, rank)
    };
    ScrollClass {
        steps: steps.len(),
        total_p50_us: p(steps.iter().map(|step| step.total_us).collect(), 50),
        total_p90_us: p(steps.iter().map(|step| step.total_us).collect(), 90),
        render_p50_us: p(steps.iter().map(|step| step.render_us).collect(), 50),
        build_p50_us: p(steps.iter().map(|step| step.build_us).collect(), 50),
        poll_p50_us: p(steps.iter().map(|step| step.poll_us).collect(), 50),
        host_renders_p50: p(steps.iter().map(|step| step.host_renders).collect(), 50),
    }
}

/// Advances the clock, draws a frame and runs until parked, twice: what a
/// wheel step needs to realize rows, commit them and draw them.
fn scroll_settle(visual: &mut VisualTestContext) {
    for _ in 0..2 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn scroll_layout(
    cx: &mut TestAppContext,
    layout: ScrollLayout,
    warmup: usize,
    steps: usize,
) -> ScrollLayoutReport {
    let prepared = scroll_view(layout).prepare().unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("scroll-benchmark-window", cx).unwrap();
        let view = prepared
            .mount(
                ScriptViewConfig::new("scroll-benchmark-view"),
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
    visual.simulate_resize(size(px(SCROLL_WINDOW[0]), px(SCROLL_WINDOW[1])));
    scroll_settle(&mut visual);
    let table = visual
        .update(|_, cx| view.accessibility_snapshot(cx))
        .unwrap()
        .find_by_role_and_name("table", "Rows")
        .next()
        .and_then(|node| node.geometry)
        .expect("the scrolled Table has geometry")
        .visual;
    #[allow(clippy::cast_possible_truncation)]
    let at = gpui::point(
        px((table.x + table.width / 2.0) as f32),
        px((table.y + table.height / 2.0) as f32),
    );
    let mut measured = Vec::with_capacity(steps);
    for step in 0..warmup + steps {
        let _ = visual
            .update(|_, cx| view.take_render_statistics(cx))
            .unwrap();
        let _ = take_snapshot(&mut visual, &view);
        let started = Instant::now();
        visual.simulate_event(gpui::ScrollWheelEvent {
            position: at,
            delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.0), px(-32.0))),
            ..gpui::ScrollWheelEvent::default()
        });
        scroll_settle(&mut visual);
        let total_us = micros(started);
        let statistics = visual
            .update(|_, cx| view.take_render_statistics(cx))
            .unwrap();
        let snapshot = take_snapshot(&mut visual, &view);
        if step < warmup {
            continue;
        }
        measured.push(ScrollStep {
            total_us,
            host_renders: statistics.host_renders,
            render_us: duration_us(statistics.host_render_time),
            build_us: duration_us(statistics.build_time),
            polls: statistics.polls,
            poll_us: duration_us(statistics.poll_time),
            commits: statistics.commits,
            commit_phases_us: statistics
                .commit_phases
                .iter()
                .map(|(phase, elapsed)| (*phase, duration_us(*elapsed)))
                .collect(),
            rows_rendered: snapshot
                .timings
                .iter()
                .filter(|timing| {
                    matches!(timing.operation, ExecutionOperation::VirtualCollection(_))
                })
                .count(),
        });
    }
    let retained_nodes = visual
        .update(|_, cx| view.take_render_statistics(cx))
        .unwrap()
        .retained_nodes;
    let mut commit_phases_us: Vec<(&'static str, u64)> = Vec::new();
    for step in &measured {
        for (phase, elapsed) in &step.commit_phases_us {
            match commit_phases_us.iter_mut().find(|(name, _)| name == phase) {
                Some((_, total)) => *total += elapsed,
                None => commit_phases_us.push((phase, *elapsed)),
            }
        }
    }
    let quiet = measured
        .iter()
        .filter(|step| step.rows_rendered == 0)
        .collect::<Vec<_>>();
    let realizing = measured
        .iter()
        .filter(|step| step.rows_rendered > 0)
        .collect::<Vec<_>>();
    ScrollLayoutReport {
        name: layout.name,
        retained_nodes,
        quiet: scroll_class(&quiet),
        realizing: scroll_class(&realizing),
        commit_phases_us,
        steps: measured,
    }
}

struct ReplayFiller(usize);

fn replay_rows(count: usize) -> gpui::Div {
    use gpui::{ParentElement, Styled, div};
    div().flex().flex_col().children((0..count).map(|index| {
        div()
            .flex()
            .flex_row()
            .gap_2()
            .child(format!("Filler {index}"))
            .child("a")
            .child("b")
    }))
}

impl Render for ReplayFiller {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        replay_rows(self.0)
    }
}

struct ReplayRoot {
    mode: usize,
    rows: usize,
    filler: gpui::Entity<ReplayFiller>,
    frames: u64,
}

impl Render for ReplayRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use gpui::{ParentElement, Styled, div};
        self.frames += 1;
        let base = div()
            .flex()
            .flex_col()
            .w(px(400.0))
            .child(format!("frame {}", self.frames));
        match self.mode {
            0 => base.child(replay_rows(self.rows)),
            1 => base.child(self.filler.clone()),
            _ => {
                #[allow(clippy::cast_precision_loss)]
                let height = self.rows as f32 * 20.0;
                base.child(
                    gpui::AnyView::from(self.filler.clone())
                        .cached(gpui::StyleRefinement::default().w_full().h(px(height))),
                )
            }
        }
    }
}

fn gpui_replay(cx: &mut TestAppContext, frames: usize) -> Vec<ReplayReport> {
    use gpui::AppContext as _;
    let mut reports = Vec::new();
    for rows in [0_usize, 330, 1_000] {
        for (mode, name) in ["inline", "entity", "cached"].into_iter().enumerate() {
            let window = cx.add_window(|_, cx| {
                let filler = cx.new(|_| ReplayFiller(rows));
                ReplayRoot {
                    mode,
                    rows,
                    filler,
                    frames: 0,
                }
            });
            let mut visual = VisualTestContext::from_window(*window, cx);
            let mut times = Vec::with_capacity(frames);
            for frame in 0..frames + 8 {
                let started = Instant::now();
                window.update(cx, |_, _, cx| cx.notify()).unwrap();
                visual.run_until_parked();
                visual.update(|window, cx| window.simulate_next_frame(cx));
                visual.run_until_parked();
                if frame >= 8 {
                    times.push(micros(started));
                }
            }
            let drawn = window.update(cx, |root, _, _| root.frames).unwrap();
            assert!(
                drawn >= u64::try_from(frames).unwrap(),
                "{name} drew {drawn} frames"
            );
            times.sort_unstable();
            reports.push(ReplayReport {
                rows,
                mode: name,
                frame_p50_us: percentile(&times, 50),
                frame_p90_us: percentile(&times, 90),
            });
        }
    }
    reports
}

#[gpui::test]
#[ignore = "run with scripts/benchmark.sh"]
fn scroll_end_to_end_baseline(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let steps = env_usize(
        "GPUI_RHAI_BENCH_SAMPLES",
        if cfg!(debug_assertions) { 8 } else { 32 },
    );
    let warmup = env_usize(
        "GPUI_RHAI_BENCH_WARMUP",
        if cfg!(debug_assertions) { 1 } else { 4 },
    );
    // `GPUI_RHAI_SCROLL_BENCH_LAYOUTS=filler_1000,...` runs only those, and
    // not the GPUI replay comparison, as when sampling one under a profiler.
    let only = std::env::var("GPUI_RHAI_SCROLL_BENCH_LAYOUTS").ok();
    let layouts = SCROLL_LAYOUTS
        .iter()
        .filter(|layout| {
            only.as_deref()
                .is_none_or(|only| only.split(',').any(|name| name == layout.name))
        })
        .map(|layout| scroll_layout(cx, *layout, warmup, steps))
        .collect();
    let mut metadata = metadata(steps, warmup);
    metadata.schema = "gpui-rhai-scroll-e2e-v1";
    metadata.data_backend = "rhai_array";
    metadata.window = SCROLL_WINDOW.map(f64::from);
    let report = ScrollBenchmarkReport {
        metadata,
        layouts,
        gpui_replay: if only.is_some() {
            Vec::new()
        } else {
            gpui_replay(cx, steps)
        },
    };
    let json = serde_json::to_string_pretty(&report).unwrap();
    println!("{json}");
    if let Ok(path) = std::env::var("GPUI_RHAI_SCROLL_BENCH_OUTPUT") {
        std::fs::write(path, format!("{json}\n")).unwrap();
    }
}
