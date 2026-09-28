#!/bin/bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Gallery baselines require macOS" >&2
  exit 1
fi
if [[ ! -x target/release/gpui-rhai ]]; then
  echo "missing target/release/gpui-rhai; build gpui-rhai-cli --release first" >&2
  exit 1
fi

requested="${1:-}"
output_root="tests/visual/macos/gallery"
mkdir -p "${output_root}"

window_geometry() {
  local pid="$1"
  GPUI_GALLERY_PID="${pid}" swift -e 'import CoreGraphics
import Foundation
let pid = Int(ProcessInfo.processInfo.environment["GPUI_GALLERY_PID"] ?? "0") ?? 0
let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
for window in windows {
    guard (window[kCGWindowOwnerPID as String] as? NSNumber)?.intValue == pid,
          let number = (window[kCGWindowNumber as String] as? NSNumber)?.intValue else { continue }
    let bounds = window[kCGWindowBounds as String] as! NSDictionary
    let x = (bounds["X"] as! NSNumber).intValue
    let y = (bounds["Y"] as! NSNumber).intValue
    let width = (bounds["Width"] as! NSNumber).intValue
    let height = (bounds["Height"] as! NSNumber).intValue
    guard
          height >= 820 else { continue }
    print("\(number)\t\(x)\t\(y)\t\(width)\t\(height)")
    break
}'
}

capture_case() {
  local filename="$1" story="$2" case_id="$3" theme="$4" locale="$5" motion="$6"
  if [[ -n "${requested}" && "${requested}" != "${filename}" ]]; then
    return
  fi
  local bundle pid geometry number x y width height content_y content_height temporary
  bundle="$(GPUI_RHAI_GALLERY_AUTO_QUIT_MS=4000 scripts/build-macos-gallery-app.sh \
    "${story}" "${case_id}" "${theme}" "${locale}" "${motion}")"
  bundle="$(cd "$(dirname "${bundle}")" && pwd -P)/$(basename "${bundle}")"
  open -n "${bundle}"
  for _ in {1..40}; do
    pid=""
    while IFS= read -r candidate; do
      command="$(ps -p "${candidate}" -o command= 2>/dev/null || true)"
      if [[ "${command}" == "${bundle}/Contents/MacOS/gpui-rhai-example" ]]; then
        pid="${candidate}"
        break
      fi
    done < <(pgrep -f gpui-rhai-example || true)
    [[ -n "${pid}" ]] && break
    sleep 0.1
  done
  if [[ -z "${pid:-}" ]]; then
    echo "Gallery process did not start for ${filename}" >&2
    exit 1
  fi
  geometry=""
  for _ in {1..20}; do
    sleep 0.2
    geometry="$(window_geometry "${pid}")"
    [[ -n "${geometry}" ]] && break
  done
  if [[ -z "${geometry}" ]]; then
    kill -TERM "${pid}" 2>/dev/null || true
    echo "Gallery window did not appear for ${filename}" >&2
    exit 1
  fi
  IFS=$'\t' read -r number x y width height <<<"${geometry}"
  sleep 0.8
  content_y=$((y + height - 820))
  content_height=820
  temporary="$(mktemp "${TMPDIR:-/tmp}/gpui-gallery-baseline.XXXXXX.png")"
  screencapture -x -R"${x},${content_y},${width},${content_height}" "${temporary}"
  sips --resampleHeightWidth 820 1181 "${temporary}" \
    --out "${output_root}/${filename}.png" >/dev/null
  rm -f "${temporary}"
  for _ in {1..50}; do
    kill -0 "${pid}" 2>/dev/null || break
    sleep 0.1
  done
  kill -TERM "${pid}" 2>/dev/null || true
  echo "captured ${filename} (${story}/${case_id}, ${theme}, ${locale}, ${motion})"
}

while IFS='|' read -r filename story case_id theme locale motion; do
  [[ -z "${filename}" ]] && continue
  capture_case "${filename}" "${story}" "${case_id}" "${theme}" "${locale}" "${motion}"
done <<'CASES'
charts-catalog.catppuccin-mocha.ar.normal|charts/catalog|basic|catppuccin-mocha|ar|normal
charts-catalog.catppuccin-mocha.ar.reduced|charts/catalog|basic|catppuccin-mocha|ar|reduced
charts-catalog.default-dark.en.none|charts/catalog|basic|default-dark|en|none
charts-catalog.default-dark.en.normal|charts/catalog|basic|default-dark|en|normal
charts-catalog.default-light.en.normal|charts/catalog|basic|default-light|en|normal
charts-interaction.diagnostics.default-dark.en.normal|charts/interaction|diagnostics|default-dark|en|normal
components-catalog.documents.default-dark.en.normal|components/catalog|documents|default-dark|en|normal
components-catalog.documents.default-light.en.normal|components/catalog|documents|default-light|en|normal
components-catalog.forms.catppuccin-mocha.ar.normal|components/catalog|forms|catppuccin-mocha|ar|normal
components-catalog.forms.default-dark.en.normal|components/catalog|forms|default-dark|en|normal
components-catalog.forms.default-light.en.normal|components/catalog|forms|default-light|en|normal
components-catalog.foundations.default-dark.en.normal|components/catalog|basic|default-dark|en|normal
components-catalog.foundations.default-light.en.normal|components/catalog|basic|default-light|en|normal
components-catalog.navigation.default-dark.en.normal|components/catalog|navigation|default-dark|en|normal
components-catalog.navigation.default-light.en.normal|components/catalog|navigation|default-light|en|normal
components-catalog.overlays.default-dark.en.normal|components/catalog|overlays|default-dark|en|normal
components-catalog.overlays.default-light.en.normal|components/catalog|overlays|default-light|en|normal
components-catalog.overlays.tokyo-night.en.reduced|components/catalog|overlays|tokyo-night|en|reduced
host-embedding.default-dark.en.normal|apps/host-embedding|basic|default-dark|en|normal
motion-catalog.default-dark.en.none|motion/catalog|basic|default-dark|en|none
resizable.default-dark.en.normal|components/resizable|basic|default-dark|en|normal
split-pane.default-dark.en.normal|components/split-pane|basic|default-dark|en|normal
operations.command-dialog.default-dark.en.normal|apps/operations|command-dialog|default-dark|en|normal
operations.config-diff.default-light.en.normal|apps/operations|config-diff|default-light|en|normal
operations.dashboard.default-dark.en.normal|apps/operations|basic|default-dark|en|normal
operations.dashboard.default-light.en.normal|apps/operations|basic|default-light|en|normal
operations.empty.default-dark.en.normal|apps/operations|empty|default-dark|en|normal
operations.failure-terminal.default-dark.en.normal|apps/operations|failure-terminal|default-dark|en|normal
operations.failure.default-dark.en.normal|apps/operations|failure|default-dark|en|normal
operations.large.default-dark.en.normal|apps/operations|large|default-dark|en|normal
operations.loading.default-light.en.normal|apps/operations|loading|default-light|en|normal
CASES
