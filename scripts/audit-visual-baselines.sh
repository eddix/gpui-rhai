#!/bin/bash
set -eu

settings_cases=(
  catppuccin-latte.en.ltr
  catppuccin-mocha.ar.rtl
  catppuccin-mocha.en.ltr
  default-dark.en.ltr
  default-dark.zh-cn.button-focus
  default-dark.zh-cn.switch-focus
  default-light.en.ltr
  tokyo-night.en.ltr
  tokyo-storm.en.ltr
)
dashboard_cases=(
  catppuccin-latte.en.ltr.normal
  catppuccin-latte.en.ltr.reduced
  catppuccin-mocha.ar.rtl.normal
  catppuccin-mocha.ar.rtl.reduced
  catppuccin-mocha.en.ltr.normal
  catppuccin-mocha.en.ltr.reduced
  default-dark.en.ltr.normal
  default-dark.en.ltr.reduced
  default-light.en.ltr.normal
  default-light.en.ltr.reduced
  tokyo-night.en.ltr.normal
  tokyo-night.en.ltr.reduced
  tokyo-storm.en.ltr.normal
  tokyo-storm.en.ltr.reduced
)
form_cases=(
  catppuccin-latte.en.ltr
  catppuccin-mocha.ar.rtl
  catppuccin-mocha.en.ltr
  default-dark.en.ltr
  default-light.en.dialog
  default-light.en.ltr
  default-light.en.toast
  tokyo-night.en.ltr
  tokyo-storm.en.ltr
)
gallery_cases=(
  catppuccin-mocha.ar.rtl
  default-dark.en.tooltip.reduced
  default-light.en.menu
)
embedded_view_cases=(
  default-dark.shared-host
)

check_case() {
  file="$1"
  expected_width="$2"
  expected_height="$3"
  if [[ ! -s "${file}" ]]; then
    echo "missing or empty visual baseline: ${file}" >&2
    exit 1
  fi
  dimensions="$(sips -g pixelWidth -g pixelHeight "${file}" 2>/dev/null | awk '
    /pixelWidth:/ { width = $2 }
    /pixelHeight:/ { height = $2 }
    END { print width "x" height }
  ')"
  if [[ "${dimensions}" != "${expected_width}x${expected_height}" ]]; then
    echo "unexpected visual baseline size: ${file} is ${dimensions}" >&2
    exit 1
  fi
}

for case_name in "${settings_cases[@]}"; do
  check_case "tests/visual/macos/settings_panel/${case_name}.png" 640 552
done
for case_name in "${dashboard_cases[@]}"; do
  check_case "tests/visual/macos/dashboard_layout/${case_name}.png" 760 592
done
for case_name in "${form_cases[@]}"; do
  check_case "tests/visual/macos/form_showcase/${case_name}.png" 680 712
done
for case_name in "${gallery_cases[@]}"; do
  check_case "tests/visual/macos/component_gallery/${case_name}.png" 720 552
done
for case_name in "${embedded_view_cases[@]}"; do
  check_case "tests/visual/macos/embedded_views/${case_name}.png" 900 452
done

actual_count="$(find tests/visual/macos -type f -name '*.png' | wc -l | tr -d ' ')"
if [[ "${actual_count}" != "36" ]]; then
  echo "unexpected visual baseline count: ${actual_count} (expected 36)" >&2
  exit 1
fi

echo "visual baseline audit passed: ${actual_count} PNG files"
