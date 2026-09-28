#!/bin/bash
set -eu

examples=($(
  python3 scripts/verify-target-manifest.py --examples
))
smoke_seconds="${GPUI_RHAI_SMOKE_SECONDS:-2}"
clang_cache="${TMPDIR:-/tmp}/gpui-rhai-clang-module-cache"
mkdir -p "${clang_cache}"
export CLANG_MODULE_CACHE_PATH="${clang_cache}"
export GPUI_RHAI_CLANG_CACHE="${clang_cache}"
export GPUI_RHAI_STRIP_METAL_DEBUG=1
export PATH="${PWD}/scripts/tool-wrappers:${PATH}"

cargo build --release -p gpui-rhai --examples
cargo build --release -p gpui-rhai-cli

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
    if [[ "${example}" != "performance_probe" || "${status}" != "0" ]]; then
      echo "${example} exited before the smoke window with status ${status}"
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

theme_studio_log="${TMPDIR:-/tmp}/gpui-rhai-theme-studio-smoke.log"
GPUI_RHAI_THEME_STUDIO=1 target/release/gpui-rhai >"${theme_studio_log}" 2>&1 &
theme_studio_pid=$!
sleep "${smoke_seconds}"
theme_studio_status=0
if kill -0 "${theme_studio_pid}" 2>/dev/null; then
  kill "${theme_studio_pid}" 2>/dev/null || true
  wait "${theme_studio_pid}" || theme_studio_status=$?
else
  wait "${theme_studio_pid}" || theme_studio_status=$?
  echo "theme studio exited before the smoke window with status ${theme_studio_status}"
  sed -n '1,160p' "${theme_studio_log}"
  exit 1
fi
if [[ "${theme_studio_status}" != "0" && "${theme_studio_status}" != "143" ]]; then
  echo "theme studio exited unexpectedly with status ${theme_studio_status}"
  sed -n '1,160p' "${theme_studio_log}"
  exit 1
fi
if grep -Eiq 'panicked at|thread .* panicked|failed to initialize|failed to compile' "${theme_studio_log}"; then
  echo "theme studio reported a panic or initialization failure"
  sed -n '1,160p' "${theme_studio_log}"
  exit 1
fi
echo "release smoke passed: theme studio"

gallery_binary="${PWD}/target/release/gpui-rhai"
gallery_root="$(mktemp -d "${TMPDIR:-/tmp}/gpui-rhai-gallery-smoke.XXXXXX")"
gallery_list="$(cd "${gallery_root}" && "${gallery_binary}" gallery --list)"
if ! grep -q '^components/catalog' <<<"${gallery_list}" \
  || ! grep -q '^motion/catalog' <<<"${gallery_list}" \
  || ! grep -q '^charts/catalog' <<<"${gallery_list}"; then
  echo "gallery --list omitted a required catalog"
  exit 1
fi
gallery_log="${TMPDIR:-/tmp}/gpui-rhai-gallery-smoke.log"
(
  cd "${gallery_root}"
  exec "${gallery_binary}" gallery --story apps/operations --case large
) >"${gallery_log}" 2>&1 &
gallery_pid=$!
sleep "${smoke_seconds}"
gallery_status=0
if kill -0 "${gallery_pid}" 2>/dev/null; then
  kill "${gallery_pid}" 2>/dev/null || true
  wait "${gallery_pid}" || gallery_status=$?
else
  wait "${gallery_pid}" || gallery_status=$?
  echo "gallery exited before the smoke window with status ${gallery_status}"
  sed -n '1,160p' "${gallery_log}"
  exit 1
fi
if [[ "${gallery_status}" != "0" && "${gallery_status}" != "143" ]]; then
  echo "gallery exited unexpectedly with status ${gallery_status}"
  sed -n '1,160p' "${gallery_log}"
  exit 1
fi
if grep -Eiq 'panicked at|thread .* panicked|failed to initialize|failed to compile' "${gallery_log}"; then
  echo "gallery reported a panic or initialization failure"
  sed -n '1,160p' "${gallery_log}"
  exit 1
fi
if find "${gallery_root}" -mindepth 1 -print -quit | grep -q .; then
  echo "gallery wrote files into its launch directory"
  find "${gallery_root}" -mindepth 1 -maxdepth 2 -print
  exit 1
fi
echo "release smoke passed: gallery"

for state in selected loading empty grouped; do
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
    echo "data_table ${state} exited before the smoke window with status ${status}"
    sed -n '1,160p' "${log}"
    exit 1
  fi
  if grep -Eiq 'panicked at|thread .* panicked|failed to initialize|failed to compile' "${log}"; then
    echo "data_table ${state} reported a panic or initialization failure"
    sed -n '1,160p' "${log}"
    exit 1
  fi
  echo "release smoke passed: data_table ${state}"
done
