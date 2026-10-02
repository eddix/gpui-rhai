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
with tempfile.TemporaryDirectory(prefix="gpui-rhai-r7-reads-") as temporary:
    workspace = Path(temporary)
    manifest = (native / "Cargo.toml").read_text()
    manifest = re.sub(r'path = "([^"]+)"',
                      lambda m: 'path = "' + str((native / m[1]).resolve()) + '"', manifest)
    (workspace / "Cargo.toml").write_text(manifest)
    shutil.copyfile(native / "Cargo.lock", workspace / "Cargo.lock")
    (workspace / "tests").mkdir()
    shutil.copyfile(here / "probes.rs", workspace / "tests/audit_r7_read_contributions.rs")
    environment = os.environ.copy()
    environment["CARGO_TARGET_DIR"] = str(native / "target")
    environment["GPUI_RHAI_AUDIT_ROOT"] = str(repo)
    baseline = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()
    assert baseline.startswith("cda11ce7"), baseline
    subprocess.run(["git", "diff", "--exit-code", "HEAD", "--", "crates", "registry", "Cargo.toml", "Cargo.lock"], cwd=repo, check=True)
    print("BASELINE=" + baseline, flush=True)
    result = subprocess.run([
        "cargo", "test", "--manifest-path", str(workspace / "Cargo.toml"),
        "--locked", "--offline", "--test", "audit_r7_read_contributions", "--", "--nocapture", "--test-threads=1",
    ], cwd=repo, env=environment)
    subprocess.run(["git", "diff", "--exit-code", "HEAD", "--", "crates", "registry", "Cargo.toml", "Cargo.lock"], cwd=repo, check=True)
    assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip() == baseline
    raise SystemExit(result.returncode)
