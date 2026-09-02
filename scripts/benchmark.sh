#!/bin/bash
set -eu

export GPUI_RHAI_BENCH_SAMPLES="${GPUI_RHAI_BENCH_SAMPLES:-30}"
export GPUI_RHAI_BENCH_WARMUP="${GPUI_RHAI_BENCH_WARMUP:-5}"

exec cargo test \
  --release \
  --offline \
  --locked \
  --manifest-path tests/performance/Cargo.toml \
  --test e2e \
  table_1000_end_to_end_baseline \
  -- \
  --ignored \
  --nocapture \
  --test-threads=1
