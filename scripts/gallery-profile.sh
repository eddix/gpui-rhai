#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd -P)"
mode="${1:-public}"
backend="${2:-pre}"
if [[ $# -ge 2 ]]; then shift 2; else set --; fi
[[ "$backend" == pre || "$backend" == fast ]] || { echo "Backend must be pre or fast." >&2; exit 2; }
if [[ "$backend" == fast ]]; then export GPUI_SCROLL_LAYERS=0; fi
case "$mode" in
    public) exec cargo run --manifest-path "$root/Cargo.toml" --locked --release -p gpui-rhai-cli \
        --no-default-features --features "gpui-$backend" -- gallery "$@";;
    vendor) exec bash "$root/scripts/with-vendor-gpui.sh" sdk "$backend" run --release -p gpui-rhai-cli -- gallery "$@";;
    *) echo "Usage: bash scripts/gallery-profile.sh public|vendor [pre|fast] [gallery args]" >&2; exit 2;;
esac
