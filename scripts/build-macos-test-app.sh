#!/bin/bash
set -eu

if [[ $# -lt 1 || $# -gt 5 ]]; then
  echo "usage: $0 <release-example-name> [visual-theme] [visual-locale] [visual-state] [normal|reduced]" >&2
  exit 2
fi

example="$1"
visual_theme="${2:-}"
visual_locale="${3:-}"
visual_state="${4:-}"
visual_motion="${5:-normal}"
if [[ ! "${example}" =~ ^[a-zA-Z0-9_-]+$ ]]; then
  echo "invalid example name: ${example}" >&2
  exit 2
fi
if [[ -n "${visual_theme}" && ! "${visual_theme}" =~ ^[a-zA-Z0-9-]+$ ]]; then
  echo "invalid visual theme: ${visual_theme}" >&2
  exit 2
fi
if [[ -n "${visual_locale}" && ! "${visual_locale}" =~ ^[a-zA-Z0-9-]+$ ]]; then
  echo "invalid visual locale: ${visual_locale}" >&2
  exit 2
fi
if [[ -n "${visual_state}" && ! "${visual_state}" =~ ^[a-zA-Z0-9-]+$ ]]; then
  echo "invalid visual state: ${visual_state}" >&2
  exit 2
fi
if [[ "${visual_motion}" != "normal" && "${visual_motion}" != "reduced" ]]; then
  echo "invalid visual motion: ${visual_motion}" >&2
  exit 2
fi

binary="target/release/examples/${example}"
if [[ ! -x "${binary}" ]]; then
  echo "missing release example: ${binary}" >&2
  echo "run: cargo build --release -p gpui-rhai --example ${example}" >&2
  exit 1
fi

bundle_root="$(mktemp -d "${TMPDIR:-/tmp}/gpui-rhai-visual.XXXXXX")"
bundle="${bundle_root}/GPUI Rhai Visual Test.app"
mkdir -p "${bundle}/Contents/MacOS"
cp scripts/macos-app/Info.plist "${bundle}/Contents/Info.plist"
identifier_example="${example//_/-}"
identifier_suffix="${bundle_root##*.}"
/usr/libexec/PlistBuddy -c \
  "Set :CFBundleIdentifier com.eddix.gpui-rhai.visual-test.${identifier_example}.${identifier_suffix}" \
  "${bundle}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c \
  "Set :CFBundleName GPUI Rhai ${example}" \
  "${bundle}/Contents/Info.plist"
if [[ -n "${visual_theme}" || -n "${visual_locale}" || -n "${visual_state}" || "${visual_motion}" == "reduced" ]]; then
  /usr/libexec/PlistBuddy -c "Add :LSEnvironment dict" "${bundle}/Contents/Info.plist"
fi
if [[ "${visual_motion}" == "reduced" ]]; then
  /usr/libexec/PlistBuddy -c \
    "Add :LSEnvironment:GPUI_RHAI_REDUCED_MOTION string 1" \
    "${bundle}/Contents/Info.plist"
fi
if [[ -n "${visual_state}" ]]; then
  /usr/libexec/PlistBuddy -c \
    "Add :LSEnvironment:GPUI_RHAI_VISUAL_STATE string ${visual_state}" \
    "${bundle}/Contents/Info.plist"
fi
if [[ -n "${visual_theme}" ]]; then
  /usr/libexec/PlistBuddy -c \
    "Add :LSEnvironment:GPUI_RHAI_VISUAL_THEME string ${visual_theme}" \
    "${bundle}/Contents/Info.plist"
fi
if [[ -n "${visual_locale}" ]]; then
  /usr/libexec/PlistBuddy -c \
    "Add :LSEnvironment:GPUI_RHAI_VISUAL_LOCALE string ${visual_locale}" \
    "${bundle}/Contents/Info.plist"
fi
cp "${binary}" "${bundle}/Contents/MacOS/gpui-rhai-example"
chmod 755 "${bundle}/Contents/MacOS/gpui-rhai-example"

echo "${bundle}"
