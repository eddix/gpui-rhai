#!/bin/bash
set -eu

for package_dir in crates/gpui-rhai crates/gpui-rhai-cli registry; do
  for license in LICENSE-MIT LICENSE-APACHE; do
    if ! cmp -s "${license}" "${package_dir}/${license}"; then
      echo "published license drift: ${package_dir}/${license}"
      exit 1
    fi
  done
done

embedded_examples=(
  dashboard_layout
  data_table
  embedded_hello_world
  embedded_views
  extension_host
  form_showcase
  host_owned_tree
  multi_window
  settings_panel
  table_1000
)

for example in "${embedded_examples[@]}"; do
  binary="target/release/examples/${example}"
  if [[ ! -x "${binary}" ]]; then
    echo "missing release binary: ${binary}"
    exit 1
  fi
  # GPUI 0.2.2 asks Apple's Metal compiler for line tables; the resulting
  # metallib records its Cargo OUT_DIR even in release mode. Ignore only that
  # dependency-owned build directory while continuing to reject application,
  # Rhai, asset, or source paths from this workspace.
  if strings "${binary}" \
    | grep -F "${PWD}" \
    | grep -Fv "${PWD}/target/release/build/gpui-" \
    | grep -q .; then
    echo "absolute workspace path leaked into ${binary}"
    exit 1
  fi
  if strings "${binary}" | grep -Eq 'cmd-alt-i|gpui_rhai_devtools|ToggleInspector'; then
    echo "development inspector leaked into ${binary}"
    exit 1
  fi
  echo "release artifact passed: ${example}"
done

cli_binary="target/release/gpui-rhai"
if [[ ! -x "${cli_binary}" ]]; then
  echo "missing release binary: ${cli_binary}"
  exit 1
fi
if strings "${cli_binary}" \
  | grep -F "${PWD}" \
  | grep -Fv "${PWD}/target/release/build/gpui-" \
  | grep -q .; then
  echo "absolute workspace path leaked into ${cli_binary}"
  exit 1
fi
if strings "${cli_binary}" | grep -Eq 'cmd-alt-i|gpui_rhai_devtools|ToggleInspector'; then
  echo "development inspector leaked into ${cli_binary}"
  exit 1
fi
echo "release artifact passed: gpui-rhai CLI and Gallery"
