#!/bin/bash
set -eu

examples=(
  component_gallery
  dashboard_layout
  data_table
  embedded_hello_world
  embedded_views
  extension_host
  form_showcase
  hello_world
  host_owned_tree
  multi_window
  native_overlay_smoke
  native_virtual_list_smoke
  performance_probe
  phase0_probe
  settings_panel
)
smoke_seconds="${GPUI_RHAI_SMOKE_SECONDS:-2}"
clang_cache="${TMPDIR:-/tmp}/gpui-rhai-clang-module-cache"
mkdir -p "${clang_cache}"
export CLANG_MODULE_CACHE_PATH="${clang_cache}"
export GPUI_RHAI_CLANG_CACHE="${clang_cache}"
export GPUI_RHAI_STRIP_METAL_DEBUG=1
export PATH="${PWD}/scripts/tool-wrappers:${PATH}"

cargo build --release -p gpui-rhai --examples

for example in "${examples[@]}"; do
  log="${TMPDIR:-/tmp}/gpui-rhai-${example}-smoke.log"
  "target/release/examples/${example}" >"${log}" 2>&1 &
  pid=$!
  sleep "${smoke_seconds}"
  status=0
  if kill -0 "${pid}" 2>/dev/null; then
    kill "${pid}" 2>/dev/null || true
    wait "${pid}" || status=$?
    if [[ "${status}" != "0" && "${status}" != "143" ]]; then
      echo "${example} terminated unexpectedly with status ${status}"
      sed -n '1,160p' "${log}"
      exit 1
    fi
  else
    wait "${pid}" || status=$?
    if [[ "${status}" != "0" ]]; then
      echo "${example} exited with status ${status}"
      sed -n '1,160p' "${log}"
      exit 1
    fi
  fi
  if grep -Eiq 'panicked at|thread .* panicked|failed to initialize|failed to compile' "${log}"; then
    echo "${example} reported a panic or initialization failure"
    sed -n '1,160p' "${log}"
    exit 1
  fi
  echo "release smoke passed: ${example}"
done

for state in selected loading empty; do
  log="${TMPDIR:-/tmp}/gpui-rhai-data_table-${state}-smoke.log"
  GPUI_RHAI_VISUAL_STATE="${state}" \
    "target/release/examples/data_table" >"${log}" 2>&1 &
  pid=$!
  sleep "${smoke_seconds}"
  status=0
  if kill -0 "${pid}" 2>/dev/null; then
    kill "${pid}" 2>/dev/null || true
    wait "${pid}" || status=$?
    if [[ "${status}" != "0" && "${status}" != "143" ]]; then
      echo "data_table ${state} terminated unexpectedly with status ${status}"
      sed -n '1,160p' "${log}"
      exit 1
    fi
  else
    wait "${pid}" || status=$?
    if [[ "${status}" != "0" ]]; then
      echo "data_table ${state} exited with status ${status}"
      sed -n '1,160p' "${log}"
      exit 1
    fi
  fi
  if grep -Eiq 'panicked at|thread .* panicked|failed to initialize|failed to compile' "${log}"; then
    echo "data_table ${state} reported a panic or initialization failure"
    sed -n '1,160p' "${log}"
    exit 1
  fi
  echo "release smoke passed: data_table ${state}"
done
