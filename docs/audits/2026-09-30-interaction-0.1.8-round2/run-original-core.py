#!/usr/bin/env python3
"""Run immutable first-round core probes under a unique second-round target."""
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

here = Path(__file__).resolve().parent
repo = here.parents[2]
native = repo / "tests/native-keyboard"
source = repo / "docs/audits/2026-09-30-interaction-0.1.8/runtime-core/probes.rs"
original = source.read_bytes()
with tempfile.TemporaryDirectory(prefix="gpui-round2-original-core-") as temporary:
    workspace = Path(temporary)
    manifest = re.sub(r'path = "([^"]+)"', lambda m: 'path = "' + str((native / m[1]).resolve()) + '"',
                      (native / "Cargo.toml").read_text())
    (workspace / "Cargo.toml").write_text(manifest)
    shutil.copyfile(native / "Cargo.lock", workspace / "Cargo.lock")
    (workspace / "tests").mkdir()
    copied = workspace / "tests/round2_original_core.rs"
    copied.write_bytes(original)
    env = os.environ.copy()
    env["GPUI_RHAI_AUDIT_ROOT"] = str(repo)
    env["CARGO_TARGET_DIR"] = str(native / "target")
    print("Original probe SHA256:", hashlib.sha256(original).hexdigest(), flush=True)
    result = subprocess.run(["cargo", "test", "--manifest-path", str(workspace / "Cargo.toml"),
        "--locked", "--offline", "--test", "round2_original_core", "--", "--nocapture", "--test-threads=1"], cwd=repo, env=env)
    assert copied.read_bytes() == original and source.read_bytes() == original
    raise SystemExit(result.returncode)
