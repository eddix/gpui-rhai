#!/bin/bash
set -euo pipefail
audit_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${audit_dir}/../../.." && pwd)"
probe_dir="$(mktemp -d "${TMPDIR:-/tmp}/gpui-rhai-issue-audit.XXXXXX")"
mkdir -p "${probe_dir}/tests"
cp "${audit_dir}/native-probes.rs" "${probe_dir}/tests/edges.rs"
cp "${repo_root}/tests/native-keyboard/Cargo.lock" "${probe_dir}/Cargo.lock"
python3 - "${repo_root}" "${probe_dir}" <<'PY'
import json,pathlib,sys
root=pathlib.Path(sys.argv[1]);p=pathlib.Path(sys.argv[2])
(p/'Cargo.toml').write_text('[package]\nname="gpui-rhai-issue-audit-6373e4b3"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[dependencies]\ngpui={version="=0.2.2",default-features=false,features=["font-kit","test-support"]}\ngpui-rhai={path='+json.dumps(str(root/'crates/gpui-rhai'))+'}\n')
PY
export GPUI_RHAI_REVIEW_REPO="${repo_root}"
export CARGO_TARGET_DIR="${repo_root}/tests/native-keyboard/target"
export CLANG_MODULE_CACHE_PATH="${repo_root}/target/clang-module-cache"
cargo test --manifest-path "${probe_dir}/Cargo.toml" --offline -- --nocapture
