#!/bin/bash
set -eu

embedded_examples=(
  component_gallery
  dashboard_layout
  embedded_hello_world
  extension_host
  form_showcase
  multi_window
  settings_panel
)

for example in "${embedded_examples[@]}"; do
  binary="target/release/examples/${example}"
  if [[ ! -x "${binary}" ]]; then
    echo "missing release binary: ${binary}"
    exit 1
  fi
  if strings "${binary}" | grep -Fq "${PWD}"; then
    echo "absolute workspace path leaked into ${binary}"
    exit 1
  fi
  if strings "${binary}" | grep -Eq 'cmd-alt-i|gpui_rhai_devtools|ToggleInspector'; then
    echo "development inspector leaked into ${binary}"
    exit 1
  fi
  echo "release artifact passed: ${example}"
done
