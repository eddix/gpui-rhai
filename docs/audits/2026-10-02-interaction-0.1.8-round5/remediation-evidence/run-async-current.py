#!/usr/bin/env python3
"""Replay the unchanged round-five async probes against the current checkout."""
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

here = Path(__file__).resolve().parent
repo = here.parents[3]
native = repo / "tests/native-keyboard"
with tempfile.TemporaryDirectory(prefix="gpui-round5-async-current-") as temporary:
    workspace = Path(temporary)
    manifest = re.sub(
        r'path = "([^"]+)"',
        lambda match: 'path = "' + str((native / match[1]).resolve()) + '"',
        (native / "Cargo.toml").read_text(),
    )
    (workspace / "Cargo.toml").write_text(manifest)
    shutil.copyfile(native / "Cargo.lock", workspace / "Cargo.lock")
    (workspace / "tests").mkdir()
    shutil.copyfile(here.parent / "rhai-async/probes.rs", workspace / "tests/round5_async_current.rs")
    environment = os.environ.copy()
    environment["CARGO_TARGET_DIR"] = str(native / "target")
    result = subprocess.run([
        "cargo", "test", "--manifest-path", str(workspace / "Cargo.toml"),
        "--locked", "--offline", "--test", "round5_async_current", "--",
        "--nocapture", "--test-threads=1",
    ], cwd=repo, env=environment)
    raise SystemExit(result.returncode)
