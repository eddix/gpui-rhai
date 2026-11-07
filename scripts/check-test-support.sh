#!/bin/bash
set -eu

# GPUI's test-support stays out of every default and release dependency graph.
# Only gpui-rhai's `offscreen-pre` and `offscreen-fast` turn it on, and an
# application enables those in [dev-dependencies], for examples/shot.rs.
#
# The offscreen features double as the positive control: if a change to
# `cargo tree`'s output hid test-support, the check would fail rather than
# pass for want of matches.

cd "$(dirname "$0")/.."

enabled() {
  cargo tree --locked -e normal,build --target all --prefix none -f '{p} {f}' "$@" |
    grep -E '^gpui[-_a-z]* v[^ ]* .*test-support' | sort -u || true
}

for features in \
  "" \
  "--features gpui-rhai/charts,gpui-rhai/dev-reload" \
  "--no-default-features --features gpui-rhai/gpui-fast,gpui-rhai/charts,gpui-rhai/dev-reload"; do
  # Word splitting of ${features} is intended.
  # shellcheck disable=SC2086
  found="$(enabled --workspace ${features})"
  if [[ -n "${found}" ]]; then
    echo "GPUI test-support is enabled in the workspace graph (${features:-default features}):"
    echo "${found}"
    exit 1
  fi
done

for features in "--features offscreen-pre" "--no-default-features --features offscreen-fast"; do
  # shellcheck disable=SC2086
  if [[ -z "$(enabled -p gpui-rhai ${features})" ]]; then
    echo "gpui-rhai ${features} shows no GPUI test-support; this check no longer sees it"
    exit 1
  fi
done

echo "GPUI test-support is off in the default and release graphs."
