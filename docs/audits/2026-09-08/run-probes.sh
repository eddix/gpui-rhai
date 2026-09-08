#!/bin/bash
set -euo pipefail

audit_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${audit_dir}/../../.." && pwd)"
probe_dir="$(mktemp -d "${TMPDIR:-/tmp}/gpui-rhai-audit.XXXXXX")"
mkdir -p "${probe_dir}/src"
cp "${1:-${audit_dir}/probes.rs}" "${probe_dir}/src/main.rs"
cp "${repo_root}/Cargo.lock" "${probe_dir}/Cargo.lock"

python3 - "${repo_root}" "${probe_dir}" <<'PY'
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
destination = pathlib.Path(sys.argv[2])
core = json.dumps(str(root / "crates/gpui-rhai"))
cli = json.dumps(str(root / "crates/gpui-rhai-cli"))
(destination / "Cargo.toml").write_text(f'''[package]
name = "gpui-rhai-audit-probes"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
gpui-rhai = {{ path = {core}, features = ["dev-reload", "grain-backend"] }}
gpui-rhai-cli = {{ path = {cli} }}
rhai = "=1.26.0"
serde_json = "1"
''')
PY

echo "Probe project: ${probe_dir}"
export CLANG_MODULE_CACHE_PATH="${repo_root}/target/clang-module-cache"
export CARGO_TARGET_DIR="${repo_root}/target"
cargo run --manifest-path "${probe_dir}/Cargo.toml" --offline
