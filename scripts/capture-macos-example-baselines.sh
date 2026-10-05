#!/bin/bash
# Capture the example baselines (settings_panel, dashboard_layout, form_showcase,
# data_table, embedded_views) offscreen with the real macOS renderer.
#
# usage: scripts/capture-macos-example-baselines.sh [example] [case]
#
# The examples are compiled into tests/native-keyboard/src/bin/example_baselines.rs
# as modules, so each capture uses exactly the view the example runs. Images are
# stored in device pixels (2x on Retina).
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "example baselines require macOS" >&2
  exit 1
fi
root="$(cd "$(dirname "$0")/.." && pwd)"
(
  cd "${root}/tests/native-keyboard"
  cargo build --release --bin example_baselines
  ./target/release/example_baselines "${root}/tests/visual/macos" "$@"
)
