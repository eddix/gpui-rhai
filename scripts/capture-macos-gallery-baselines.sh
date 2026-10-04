#!/bin/bash
# Capture the Gallery acceptance baselines: representative pages in both
# densities and both default modes, plus one RTL and two CJK cases.
#
# usage: scripts/capture-macos-gallery-baselines.sh [case-name]
#
# The Gallery is rendered offscreen by the real macOS renderer and read back
# from the GPU texture (tests/native-keyboard/src/bin/gallery_baselines.rs),
# so window managers, other windows and screen-recording permission never
# affect the result. Images are stored in device pixels (2x on Retina).
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Gallery baselines require macOS" >&2
  exit 1
fi
root="$(cd "$(dirname "$0")/.." && pwd)"
output="${root}/tests/visual/macos/gallery"
(
  cd "${root}/tests/native-keyboard"
  cargo build --release --bin gallery_baselines
  ./target/release/gallery_baselines "${output}" "$@"
)
