#!/bin/bash
set -euo pipefail
capture_root=/tmp/gpui-rhai-018-native-capture.WZYwOr
case_file=/tmp/gpui-rhai-018-table-capture.8muIUl/cases.json
cleanup() {
  while IFS= read -r bundle; do
    canonical="$(cd "${bundle}/Contents/MacOS" && pwd -P)/gpui-rhai-example"
    while read -r candidate command; do
      if [[ "$command" == "$canonical" ]]; then
        kill -TERM "$candidate" 2>/dev/null || true
      fi
    done < <(ps -axo pid=,command=)
  done < <(jq -r '.[].bundle' "$case_file")
  rift-cli execute config set virtual_workspaces.app_rules \
    "$(jq -c '.virtual_workspaces.app_rules' "$capture_root/original-config.json")"
  rift-cli execute config get > "$capture_root/restored-config.json"
  jq -s '{rules_restored:(.[0].virtual_workspaces.app_rules == .[1].virtual_workspaces.app_rules),other_config_unchanged:((.[0] | del(.virtual_workspaces.app_rules)) == (.[1] | del(.virtual_workspaces.app_rules))),original_rule_count:(.[0].virtual_workspaces.app_rules|length),restored_rule_count:(.[1].virtual_workspaces.app_rules|length)}' \
    "$capture_root/original-config.json" "$capture_root/restored-config.json" > "$capture_root/restoration.json"
}
trap cleanup EXIT
mkdir -p "$capture_root/candidates"
while IFS=$'\t' read -r name bundle_id bundle; do
  canonical="$(cd "${bundle}/Contents/MacOS" && pwd -P)/gpui-rhai-example"
  pid=""
  while read -r candidate command; do
    [[ "$command" == "$canonical" ]] && pid="$candidate"
  done < <(ps -axo pid=,command=)
  if [[ -z "$pid" ]]; then
    open -n "$bundle"
    for _ in {1..40}; do
      while read -r candidate command; do
        [[ "$command" == "$canonical" ]] && pid="$candidate"
      done < <(ps -axo pid=,command=)
      [[ -n "$pid" ]] && break
      sleep 0.1
    done
  fi
  [[ -n "$pid" ]] || { echo "Missing owned process: $name" >&2; exit 1; }
  sleep 0.5
  "$capture_root/native-window" "$pid" "$bundle_id" capture "$capture_root/$name.raw.png" > "$capture_root/$name.frame.json"
  width="$(sips -g pixelWidth "$capture_root/$name.raw.png" | awk '/pixelWidth:/ {print $2}')"
  height="$(sips -g pixelHeight "$capture_root/$name.raw.png" | awk '/pixelHeight:/ {print $2}')"
  [[ "$width" == 1960 && "$height" == 1504 ]] || { echo "Nonconforming native image: $name $width x $height" >&2; exit 1; }
  sips --resampleHeightWidth 752 980 "$capture_root/$name.raw.png" --out "$capture_root/candidates/$name.png" >/dev/null
  shasum -a 256 "$canonical" "$capture_root/$name.raw.png" "$capture_root/candidates/$name.png"
  "$capture_root/native-window" "$pid" "$bundle_id" inspect > "$capture_root/$name.final-frame.json"
  command="$(ps -p "$pid" -o command=)"
  [[ "$command" == "$canonical" ]] || { echo "Owned PID changed before cleanup" >&2; exit 1; }
  kill -TERM "$pid"
  echo "captured $name"
done < <(jq -r '.[] | [.case,.bundle_id,.bundle] | @tsv' "$case_file")
