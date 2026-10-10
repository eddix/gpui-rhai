#!/usr/bin/env python3
"""Replay the original five PR #111 probes and the second-round regressions.

Only temporary test files are added to the selected checkout. Tracked files
are untouched, and the copied tests are removed after Cargo returns.
At 2c5e8a07: previous=5 pass; new=5 pass. All ten assertions stay unchanged.
"""

import argparse
import hashlib
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--repo", required=True, type=Path)
parser.add_argument("--target-dir", type=Path)
parser.add_argument("--suite", choices=["all", "previous", "new"], default="all")
args = parser.parse_args()
repo = args.repo.resolve()
here = Path(__file__).resolve().parent
sources = {"previous": "previous-probes.rs", "new": "probes.rs"}
selected = list(sources) if args.suite == "all" else [args.suite]
head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()
print(f"Review target: {head}", flush=True)
command = [
    "cargo", "test", "--manifest-path", "tests/native-keyboard/Cargo.toml",
    "--features", "gpui-rhai/dev-reload", "--locked", "--offline", "--no-fail-fast",
]
if args.target_dir:
    command.extend(["--target-dir", str(args.target_dir.resolve())])
copied = []
try:
    for suite in selected:
        source = (here / sources[suite]).read_bytes()
        test = f"audit_pr111_round2_{suite}_{os.getpid()}"
        path = repo / "tests/native-keyboard/tests" / f"{test}.rs"
        if path.exists():
            raise SystemExit(f"Refusing to overwrite {path}")
        path.write_bytes(source)
        copied.append((path, source))
        command.extend(["--test", test])
        print(f"{suite} SHA256: {hashlib.sha256(source).hexdigest()}", flush=True)
    command.extend(["--", "--nocapture", "--test-threads=1"])
    env = os.environ.copy()
    env.pop("RUST_MIN_STACK", None)
    result = subprocess.run(command, cwd=repo, env=env, check=False)
finally:
    for path, source in copied:
        if path.exists() and path.read_bytes() == source:
            path.unlink()
raise SystemExit(result.returncode)
