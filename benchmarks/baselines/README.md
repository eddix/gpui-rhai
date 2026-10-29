# Performance baselines

Run `bash scripts/benchmark.sh` from the repository root. Set
`GPUI_RHAI_BENCH_OUTPUT` to write the JSON report to a file.

Do not compare reports unless commit state, Rust toolchain, hardware, OS,
release profile, feature set, window size, display configuration, and power
state are controlled. Ordinary shared CI enforces structural invariants only;
wall-clock regressions require a dedicated Mac runner or repeated local runs.

Baseline files should identify their machine and configuration in the file
name. Raw per-sample values are retained so p50/p95/p99 summaries can be
recomputed instead of trusting a single aggregate.

Current reports use schema `gpui-rhai-e2e-v2`. Root/component and delayed
virtual-item Rhai durations and operation deltas are separate fields; v1 files
created before that attribution are not comparable. Compare Table reports only
when `metadata.data_backend` also matches: eager Rhai arrays and
`native_collection` intentionally exercise different ownership models.
Chart reports use `gpui-rhai-chart-e2e-v2` and additionally require the same
point count, theme, and chart feature configuration.

Scroll reports use `gpui-rhai-scroll-e2e-v1` (`scroll_end_to_end_baseline`,
written to `GPUI_RHAI_SCROLL_BENCH_OUTPUT` when set). Each layout mounts a
400-row Table beside 0 to 2500 rows of plain filler and scrolls it one row per
step; a step is `quiet` when it rendered no rows and `realizing` when it did.
Per step they record host renders, render and element-build time, polls and
poll time, commits and the time of each commit phase, all read from the
view's `take_render_statistics`. `gpui_replay` measures GPUI alone: a view
rendered every frame beside a static subtree built inline, as a plain child
view, or as a cached child view.

Shell typing reports use `gpui-rhai-shell-typing-v1`
(`app_shell_typing_baseline`, written to `GPUI_RHAI_SHELL_BENCH_OUTPUT`). They
type into a field of an `AppShell` with the draft held by the view or by a
component around the field, with cache islands off and on, and record per
keystroke the time, host renders, render and build time, and the islands
replayed and rendered in place.

`scroll-mac17-8-release-0.3.2.json`, `scroll-mac17-8-release-0.3.3.json` and
`shell-typing-mac17-8-release-0.3.3.json` were recorded on the same Mac
(Mac17,8, macOS 27.0.1, Rust 1.95.0, release, 30 samples after 5 warmup steps)
on 2026-10-29; the 0.3.2 report is that release with the benchmark added.
