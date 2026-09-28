#!/usr/bin/env python3
"""Use the existing native-test graph in a disposable workspace, without product edits."""
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

here = Path(__file__).resolve().parent
repo = here.parents[2]
native = repo / "tests/native-keyboard"
with tempfile.TemporaryDirectory(prefix="gpui-gallery-audit-") as temporary:
    workspace = Path(temporary)
    manifest = (native / "Cargo.toml").read_text()
    manifest = re.sub(r'path = "([^"]+)"',
                      lambda match: 'path = "' + str((native / match[1]).resolve()) + '"', manifest)
    (workspace / "Cargo.toml").write_text(manifest)
    shutil.copyfile(native / "Cargo.lock", workspace / "Cargo.lock")
    (workspace / "tests").mkdir()
    shutil.copyfile(here / "probes.rs", workspace / "tests/audit.rs")
    environment = os.environ.copy()
    environment["GPUI_RHAI_AUDIT_ROOT"] = str(repo)
    environment["CARGO_TARGET_DIR"] = str(native / "target")
    result = subprocess.run([
        "cargo", "test", "--manifest-path", str(workspace / "Cargo.toml"),
        "--locked", "--offline", "--test", "audit", "--", "--nocapture",
    ], cwd=repo, env=environment)
    raise SystemExit(result.returncode)
