#!/bin/bash
set -euo pipefail
audit_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${audit_dir}/../../../.." && pwd)"
probe_dir="$(mktemp -d "${TMPDIR:-/tmp}/gpui-rhai-svg-drain.XXXXXX")"
mkdir -p "${probe_dir}/src"
cp "${audit_dir}/drain-bench.rs" "${probe_dir}/src/main.rs"
cp "${repo_root}/Cargo.lock" "${probe_dir}/Cargo.lock"
python3 - "${repo_root}" "${probe_dir}" <<'PY'
import json,pathlib,sys
root=pathlib.Path(sys.argv[1]);p=pathlib.Path(sys.argv[2])
(p/'Cargo.toml').write_text('[package]\nname="gpui-rhai-svg-drain-bench-050c7fa2"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[dependencies]\ngpui-rhai={path='+json.dumps(str(root/'crates/gpui-rhai'))+'}\nrhai="=1.26.0"\n[profile.release]\nlto="thin"\n')
PY
export CARGO_TARGET_DIR="${repo_root}/target"
export CLANG_MODULE_CACHE_PATH="${repo_root}/target/clang-module-cache"
cargo run --manifest-path "${probe_dir}/Cargo.toml" --release --offline
