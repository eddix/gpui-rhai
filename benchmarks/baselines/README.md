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
