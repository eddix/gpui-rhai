#!/bin/bash
set -eu

if [[ $# -gt 4 ]]; then
  echo "usage: $0 [visual-theme] [visual-locale] [visual-state] [normal|reduced]" >&2
  exit 2
fi

visual_theme="${1:-}"
visual_locale="${2:-}"
visual_state="${3:-}"
visual_motion="${4:-normal}"

binary="target/release/gpui-rhai"
if [[ ! -x "${binary}" ]]; then
  echo "missing release CLI: ${binary}" >&2
  echo "run: cargo build --release -p gpui-rhai-cli" >&2
  exit 1
fi

bundle_root="$(mktemp -d "${TMPDIR:-/tmp}/gpui-rhai-theme-studio.XXXXXX")"
bundle="${bundle_root}/GPUI Rhai Theme Studio.app"
mkdir -p "${bundle}/Contents/MacOS"
cp scripts/macos-app/Info.plist "${bundle}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c \
  "Set :CFBundleIdentifier com.eddix.gpui-rhai.theme-studio.${bundle_root##*.}" \
  "${bundle}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c \
  "Set :CFBundleName GPUI Rhai Theme Studio" \
  "${bundle}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :LSEnvironment dict" "${bundle}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c \
  "Add :LSEnvironment:GPUI_RHAI_THEME_STUDIO string 1" \
  "${bundle}/Contents/Info.plist"
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
if [[ -n "${visual_state}" ]]; then
  /usr/libexec/PlistBuddy -c \
    "Add :LSEnvironment:GPUI_RHAI_VISUAL_STATE string ${visual_state}" \
    "${bundle}/Contents/Info.plist"
fi
if [[ "${visual_motion}" == "reduced" ]]; then
  /usr/libexec/PlistBuddy -c \
    "Add :LSEnvironment:GPUI_RHAI_REDUCED_MOTION string 1" \
    "${bundle}/Contents/Info.plist"
fi
cp "${binary}" "${bundle}/Contents/MacOS/gpui-rhai-example"
chmod 755 "${bundle}/Contents/MacOS/gpui-rhai-example"

echo "${bundle}"
