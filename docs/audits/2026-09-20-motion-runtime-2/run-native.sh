#!/bin/bash
set -euo pipefail

audit_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${audit_dir}/../../.." && pwd)"
probe_dir="$(mktemp -d "${TMPDIR:-/tmp}/gpui-rhai-motion-native.XXXXXX")"
mkdir -p "${probe_dir}/tests"
cp "${audit_dir}/native-policy.rs" "${probe_dir}/tests/policy.rs"
cp "${repo_root}/tests/native-keyboard/Cargo.lock" "${probe_dir}/Cargo.lock"
python3 - "${repo_root}" "${probe_dir}" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
destination = pathlib.Path(sys.argv[2])
core = json.dumps(str(root / "crates/gpui-rhai"))
(destination / "Cargo.toml").write_text(f'''[package]
name = "gpui-rhai-motion-review-native"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
gpui = {{ version = "=0.2.2", default-features = false, features = ["font-kit", "test-support"] }}
gpui-rhai = {{ path = {core} }}
''')
PY

echo "Native probe project: ${probe_dir}"
export GPUI_RHAI_REVIEW_REPO="${repo_root}"
export CLANG_MODULE_CACHE_PATH="${repo_root}/target/clang-module-cache"
export CARGO_TARGET_DIR="${repo_root}/tests/native-keyboard/target"
cargo test --manifest-path "${probe_dir}/Cargo.toml" --offline -- --nocapture
