#!/usr/bin/env python3
"""Accept corrected cancellation without altering the original characterization."""
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

here = Path(__file__).resolve().parent
repo = here.parents[3]
native = repo / "tests/native-keyboard"
original = (here.parent / "rhai-async/probes.rs").read_text()
old = 'assert!(result.unwrap_err().contains("unmounted incarnation"));'
assert original.count(old) == 1
corrected = original.replace(old, 'assert!(result.is_ok(), "cancelled effect must be discarded: {result:?}");')
with tempfile.TemporaryDirectory(prefix="gpui-round4-async-acceptance-") as temporary:
    workspace = Path(temporary)
    manifest = re.sub(r'path = "([^"]+)"',
                      lambda m: 'path = "' + str((native / m[1]).resolve()) + '"',
                      (native / "Cargo.toml").read_text())
    (workspace / "Cargo.toml").write_text(manifest)
    shutil.copyfile(native / "Cargo.lock", workspace / "Cargo.lock")
    (workspace / "tests").mkdir()
    (workspace / "tests/round4_async_acceptance.rs").write_text(corrected)
    environment = os.environ.copy()
    environment["CARGO_TARGET_DIR"] = str(native / "target")
    result = subprocess.run([
        "cargo", "test", "--manifest-path", str(workspace / "Cargo.toml"),
        "--locked", "--offline", "--test", "round4_async_acceptance", "--",
        "--nocapture", "--test-threads=1",
    ], cwd=repo, env=environment)
    raise SystemExit(result.returncode)
