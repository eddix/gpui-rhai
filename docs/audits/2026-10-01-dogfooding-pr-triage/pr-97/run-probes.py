#!/usr/bin/env python3
"""Run independent public-API probes without changing product/test workspaces."""
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

here = Path(__file__).resolve().parent
repo = here.parents[3]
native = repo / "tests/native-keyboard"
with tempfile.TemporaryDirectory(prefix="gpui-pr97-key-dispatch-audit-") as temporary:
    workspace = Path(temporary)
    manifest = (native / "Cargo.toml").read_text()
    manifest = re.sub(r'path = "([^"]+)"',
                      lambda match: 'path = "' + str((native / match[1]).resolve()) + '"', manifest)
    (workspace / "Cargo.toml").write_text(manifest)
    shutil.copyfile(native / "Cargo.lock", workspace / "Cargo.lock")
    (workspace / "tests").mkdir()
    shutil.copyfile(here / "probes.rs", workspace / "tests/pr97_key_dispatch.rs")
    shutil.copyfile(here / "pr97_validation.rs", workspace / "tests/pr97_validation.rs")
    environment = os.environ.copy()
    environment["GPUI_RHAI_AUDIT_ROOT"] = str(repo)
    environment["CARGO_TARGET_DIR"] = str(native / "target")
    result = subprocess.run([
        "cargo", "test", "--manifest-path", str(workspace / "Cargo.toml"),
        "--locked", "--offline", "--test", "pr97_key_dispatch", "--", "--nocapture",
    ], cwd=repo, env=environment)
    raise SystemExit(result.returncode)
