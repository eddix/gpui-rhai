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
with tempfile.TemporaryDirectory(prefix="gpui-rhai-pr96-semantic-") as temporary:
    workspace = Path(temporary)
    snapshot = workspace / "baseline"
    snapshot.mkdir()
    archive = workspace / "baseline.tar"
    baseline = subprocess.check_output(["git", "rev-parse", "f6e936a5"], cwd=repo, text=True).strip()
    print("PINNED_BASELINE=" + baseline, flush=True)
    with archive.open("wb") as output:
        subprocess.run(["git", "archive", baseline, "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", ".cargo", "crates", "registry", "tests/native-keyboard/Cargo.toml", "tests/native-keyboard/Cargo.lock"], cwd=repo, stdout=output, check=True)
    subprocess.run(["tar", "-xf", str(archive), "-C", str(snapshot)], check=True)
    snapshot_native = snapshot / "tests/native-keyboard"
    manifest = (snapshot_native / "Cargo.toml").read_text()
    manifest = re.sub(r'path = "([^"]+)"',
                      lambda m: 'path = "' + str((snapshot_native / m[1]).resolve()) + '"', manifest)
    (workspace / "Cargo.toml").write_text(manifest)
    shutil.copyfile(snapshot_native / "Cargo.lock", workspace / "Cargo.lock")
    (workspace / "tests").mkdir()
    shutil.copyfile(here / "probes.rs", workspace / "tests/audit_pr96_semantics.rs")
    environment = os.environ.copy()
    environment["CARGO_TARGET_DIR"] = str(native / "target")
    result = subprocess.run([
        "cargo", "test", "--manifest-path", str(workspace / "Cargo.toml"),
        "--locked", "--offline", "--test", "audit_pr96_semantics", "--", "--nocapture", "--test-threads=1",
    ], cwd=repo, env=environment)
    raise SystemExit(result.returncode)
