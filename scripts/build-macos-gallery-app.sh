#!/bin/bash
set -eu

if [[ $# -gt 4 ]]; then
  echo "usage: $0 [story-id] [case-id] [theme] [locale]" >&2
  exit 2
fi

story="${1:-}"
case_id="${2:-basic}"
theme="${3:-default-dark}"
locale="${4:-en}"
if [[ -n "${story}" && ! "${story}" =~ ^[a-zA-Z0-9_/-]+$ ]]; then
  echo "invalid story ID: ${story}" >&2
  exit 2
fi
for value in "${case_id}" "${theme}" "${locale}"; do
  if [[ ! "${value}" =~ ^[a-zA-Z0-9_-]+$ ]]; then
    echo "invalid Gallery argument: ${value}" >&2
    exit 2
  fi
done

binary="target/release/gpui-rhai"
if [[ ! -x "${binary}" ]]; then
  echo "missing release CLI: ${binary}" >&2
  echo "run: cargo build --release -p gpui-rhai-cli" >&2
  exit 1
fi

bundle_root="$(mktemp -d "${TMPDIR:-/tmp}/gpui-rhai-gallery.XXXXXX")"
bundle="${bundle_root}/GPUI Rhai Gallery.app"
mkdir -p "${bundle}/Contents/MacOS"
cp scripts/macos-app/Info.plist "${bundle}/Contents/Info.plist"
identifier_suffix="${bundle_root##*.}"
/usr/libexec/PlistBuddy -c \
  "Set :CFBundleIdentifier com.eddix.gpui-rhai.gallery.${identifier_suffix}" \
  "${bundle}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleName GPUI Rhai Gallery ${identifier_suffix}" \
  "${bundle}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :LSEnvironment dict" "${bundle}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :LSEnvironment:GPUI_RHAI_GALLERY string 1" \
  "${bundle}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :LSEnvironment:GPUI_RHAI_GALLERY_CASE string ${case_id}" \
  "${bundle}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :LSEnvironment:GPUI_RHAI_GALLERY_THEME string ${theme}" \
  "${bundle}/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :LSEnvironment:GPUI_RHAI_GALLERY_LOCALE string ${locale}" \
  "${bundle}/Contents/Info.plist"
if [[ -n "${story}" ]]; then
  /usr/libexec/PlistBuddy -c "Add :LSEnvironment:GPUI_RHAI_GALLERY_STORY string ${story}" \
    "${bundle}/Contents/Info.plist"
fi
cp "${binary}" "${bundle}/Contents/MacOS/gpui-rhai-example"
chmod 755 "${bundle}/Contents/MacOS/gpui-rhai-example"

echo "${bundle}"
