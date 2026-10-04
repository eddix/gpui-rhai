#!/bin/bash
set -euo pipefail
cd /Users/eddix/Codes/github.com/eddix/gpui-rhai
test "$(git rev-parse HEAD)" = 43732b89c48ec6add95cc341d9e1090fe862cec4
evidence=docs/audits/2026-10-04-table-local-fixes/evidence
run() {
  name="$1"; shift
  "$@" > "$evidence/$name.log" 2>&1
  echo "$name passed"
}
run targets python3 scripts/verify-target-manifest.py
run fmt cargo fmt --all -- --check
run native-fmt cargo fmt --manifest-path tests/native-keyboard/Cargo.toml --all -- --check
run msrv cargo check --workspace --all-targets --all-features --locked
run stable cargo +stable check --workspace --all-targets --all-features --locked
run clippy cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
run workspace cargo test --workspace --all-targets --all-features --locked
run default cargo test -p gpui-rhai --lib --locked
run native env -u RUST_MIN_STACK cargo test --manifest-path tests/native-keyboard/Cargo.toml --locked -- --test-threads=1
run native-clippy cargo clippy --manifest-path tests/native-keyboard/Cargo.toml --all-targets --locked -- -D warnings
run performance-structure cargo test --manifest-path tests/performance/Cargo.toml --locked
run performance-clippy cargo clippy --manifest-path tests/performance/Cargo.toml --all-targets --locked -- -D warnings
run doc-core env RUSTDOCFLAGS=-Dwarnings cargo doc -p gpui-rhai --all-features --no-deps --locked
run doc-cli env RUSTDOCFLAGS=-Dwarnings cargo doc -p gpui-rhai-cli --lib --no-deps --locked
run package-core cargo package -p gpui-rhai --locked
run package-registry cargo package -p gpui-rhai-registry --locked
run package-cli cargo package -p gpui-rhai-cli --locked --no-verify --config 'patch.crates-io.gpui-rhai.path="crates/gpui-rhai"' --config 'patch.crates-io.gpui-rhai-registry.path="registry"'
run release-smoke bash scripts/release-smoke.sh
run release-artifacts bash scripts/audit-release-artifacts.sh
