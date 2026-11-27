#!/usr/bin/env bash
# Stable Cargo builds with independent manifest/lock snapshots, without
# replacing the checkout lockfile or globally overriding another application.
set -euo pipefail

usage() {
  echo "Usage: bash scripts/with-vendor-gpui.sh sdk|native|apps pre|fast cargo-subcommand [args]" >&2
  exit 2
}
[[ $# -ge 3 ]] || usage
root="$(cd "$(dirname "$0")/.." && pwd -P)"
workspace="$1"
backend="$2"
subcommand="$3"
shift 3
[[ "$backend" == pre || "$backend" == fast ]] || usage
[[ "$(uname -s)" == Darwin ]] || { echo "Vendor GPUI rendering is currently supported on macOS only." >&2; exit 2; }
case "$workspace" in
    # The SDK and CLI expose the same two feature names. Unqualified features
    # apply to whichever member the caller selects with `-p` (or to both).
    sdk) prefix=""; features="gpui-$backend,vendor-gpui";;
    native) prefix="tests/native-keyboard/"; features="gpui-$backend,vendor-gpui";;
    apps) [[ "$backend" == pre ]] || usage; prefix="apps/"; features="vendor-gpui";;
    *) usage;;
esac
case "$subcommand" in
    build|check|clippy|doc|run|test|tree|metadata|lock) ;;
    *) usage;;
esac

scratch="$(mktemp -d "${TMPDIR:-/tmp}/gpui-rhai-vendor.XXXXXX")"
cleanup() {
    if [[ "$scratch" == */gpui-rhai-vendor.* && -d "$scratch" ]]; then
        rm -rf -- "$scratch"
    fi
}
trap cleanup EXIT
rsync -a --exclude='/vendor/gpui' --exclude='/.git' --exclude='/docs/plans' \
    --exclude='target' --exclude='.DS_Store' "$root/" "$scratch/"
for manifest in Cargo.toml apps/disktree/Cargo.toml tests/native-keyboard/Cargo.toml; do
    perl -pi -e 's/=0\.3\.14/=0.3.11/g; s/=0\.1\.19/=0.1.13/g' "$scratch/$manifest"
done
install -m 644 "$root/${prefix}Cargo.vendor.lock" "$scratch/${prefix}Cargo.lock"
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/target}/gpui-vendor/$workspace/$backend"
cd "$scratch/${prefix}"
if [[ "$subcommand" == lock ]]; then
    # Deliberate maintainer operation; other builds keep both locks untouched.
    cargo --config "$root/.cargo/vendor-gpui.toml" metadata --format-version 1 \
        --filter-platform "$(rustc -vV | sed -n 's/^host: //p')" \
        --no-default-features --features "$features" "$@" >/dev/null
    install -m 644 Cargo.lock "$root/${prefix}Cargo.vendor.lock"
    exit 0
fi
cargo --config "$root/.cargo/vendor-gpui.toml" "$subcommand" --locked --no-default-features --features "$features" "$@"
