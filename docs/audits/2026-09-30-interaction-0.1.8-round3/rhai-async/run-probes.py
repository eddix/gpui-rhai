#!/usr/bin/env python3
"""Copy the native dependency graph into a disposable workspace; keep product untouched."""
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

here = Path(__file__).resolve().parent
repo = here.parents[3]
native = repo / "tests/native-keyboard"
with tempfile.TemporaryDirectory(prefix="gpui-rhai-r3-async-") as temporary:
    workspace = Path(temporary)
    manifest = (native / "Cargo.toml").read_text()
    manifest = re.sub(r'path = "([^"]+)"',
                      lambda m: 'path = "' + str((native / m[1]).resolve()) + '"', manifest)
    (workspace / "Cargo.toml").write_text(manifest)
    shutil.copyfile(native / "Cargo.lock", workspace / "Cargo.lock")
    (workspace / "tests").mkdir()
    shutil.copyfile(here / "probes.rs", workspace / "tests/audit_r3_rhai_async.rs")
    environment = os.environ.copy()
    environment["CARGO_TARGET_DIR"] = str(native / "target")
    result = subprocess.run([
        "cargo", "test", "--manifest-path", str(workspace / "Cargo.toml"),
        "--locked", "--offline", "--test", "audit_r3_rhai_async", "--", "--nocapture", "--test-threads=1",
    ], cwd=repo, env=environment)
    raise SystemExit(result.returncode)
