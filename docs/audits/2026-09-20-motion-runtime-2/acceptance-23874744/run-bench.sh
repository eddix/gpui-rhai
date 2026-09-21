#!/bin/bash
set -euo pipefail
audit_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${audit_dir}/../../../.." && pwd)"
probe_dir="$(mktemp -d "${TMPDIR:-/tmp}/gpui-rhai-motion-bench.XXXXXX")"
mkdir -p "${probe_dir}/src"
cp "${audit_dir}/bench.rs" "${probe_dir}/src/main.rs"
cp "${repo_root}/Cargo.lock" "${probe_dir}/Cargo.lock"
python3 - "${repo_root}" "${probe_dir}" <<'PY'
import json,pathlib,sys
root=pathlib.Path(sys.argv[1]); p=pathlib.Path(sys.argv[2])
(p/'Cargo.toml').write_text('[package]\nname="gpui-rhai-motion-bench"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[dependencies]\ngpui-rhai={path='+json.dumps(str(root/'crates/gpui-rhai'))+'}\n[profile.release]\nlto="thin"\n')
PY
export CLANG_MODULE_CACHE_PATH="${repo_root}/target/clang-module-cache"
export CARGO_TARGET_DIR="${repo_root}/target"
cargo run --manifest-path "${probe_dir}/Cargo.toml" --release --offline
