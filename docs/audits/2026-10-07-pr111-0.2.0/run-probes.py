#!/usr/bin/env python3
"""Run the five independent PR #111 regressions on a selected checkout.

The checkout's tracked files are untouched. A uniquely named test source is
temporarily added to the existing native test workspace and removed afterward.
The original PR head is expected to fail all five assertions.
"""

import argparse
import hashlib
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--repo", required=True, type=Path)
parser.add_argument("--target-dir", type=Path)
args = parser.parse_args()
repo = args.repo.resolve()
here = Path(__file__).resolve().parent
source = (here / "probes.rs").read_bytes()
test_name = f"audit_pr111_release_{os.getpid()}"
target = repo / "tests/native-keyboard/tests" / f"{test_name}.rs"
if target.exists():
    raise SystemExit(f"Refusing to overwrite {target}")
head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()
print(f"Review target: {head}", flush=True)
print(f"Probe SHA256: {hashlib.sha256(source).hexdigest()}", flush=True)
command = [
    "cargo", "test", "--manifest-path", "tests/native-keyboard/Cargo.toml",
    "--features", "gpui-rhai/dev-reload", "--locked", "--offline",
    "--test", test_name,
]
if args.target_dir:
    command.extend(["--target-dir", str(args.target_dir.resolve())])
command.extend(["--", "--nocapture", "--test-threads=1"])
env = os.environ.copy()
env.pop("RUST_MIN_STACK", None)
try:
    target.write_bytes(source)
    result = subprocess.run(command, cwd=repo, env=env, check=False)
finally:
    if target.exists() and target.read_bytes() == source:
        target.unlink()
raise SystemExit(result.returncode)
