#!/usr/bin/env python3
"""Re-run the original 25 expected-behavior probes without changing their files."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

here = Path(__file__).resolve().parent
repo = here.parents[2]
old = repo / "docs/audits/2026-09-30-interaction-0.1.8"
out = here / "baseline"
out.mkdir(parents=True, exist_ok=True)
environment = os.environ.copy()
environment["CARGO_TARGET_DIR"] = str(repo / "tests/native-keyboard/target")
cases = [
    ("runtime-core", ["python3", str(here / "run-original-core.py")], old / "runtime-core/probes.rs"),
    ("drag-sort", ["cargo", "test", "--manifest-path", str(old / "drag-sort/probe/Cargo.toml"),
                   "--locked", "--offline", "--lib", "--", "--nocapture", "--test-threads=1"], old / "drag-sort/probe/src/lib.rs"),
    ("resize-controls", ["cargo", "test", "--manifest-path", str(old / "resize-controls/Cargo.toml"),
                         "--locked", "--offline", "--test", "probe", "--", "--nocapture", "--test-threads=1"], old / "resize-controls/probe.rs"),
    ("transforms-selection", ["cargo", "test", "--manifest-path", str(old / "transforms-selection/Cargo.toml"),
                              "--locked", "--offline", "--lib", "--", "--nocapture", "--test-threads=1"], old / "transforms-selection/src/lib.rs"),
]
results = []
for name, command, source in cases:
    started = time.monotonic()
    print(f"Running original {name} probes", flush=True)
    with (out / f"{name}.log").open("w") as log:
        completed = subprocess.run(command, cwd=repo, env=environment, stdout=log, stderr=subprocess.STDOUT)
    result = {"name": name, "exit": completed.returncode, "seconds": time.monotonic() - started,
              "source": str(source.relative_to(repo)),
              "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest()}
    results.append(result)
    print(json.dumps(result), flush=True)
report = {"head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(), "results": results}
(out / "results.json").write_text(json.dumps(report, indent=2) + "\n")
raise SystemExit(any(item["exit"] for item in results))
