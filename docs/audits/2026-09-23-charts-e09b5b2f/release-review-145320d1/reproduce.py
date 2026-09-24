#!/usr/bin/env python3
"""Run the release-readiness native probes without modifying workspace manifests."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("native", "previous-native", "previous-public"))
    parser.add_argument("--out-dir", type=Path)
    args = parser.parse_args()
    audit = Path(__file__).resolve().parent
    repo = next(p for p in audit.parents if (p / "crates/gpui-rhai/Cargo.toml").is_file())
    out = (args.out_dir or Path(tempfile.mkdtemp(prefix="chart-core-output-"))).resolve()
    out.mkdir(parents=True, exist_ok=True)
    if args.mode.startswith("previous-"):
        previous = audit.parent / "acceptance-30c29929/reproduce.py"
        return subprocess.run([
            sys.executable, str(previous), args.mode.removeprefix("previous-"),
            "--out-dir", str(out),
        ], cwd=repo).returncode

    manifest = (
        '[package]\nname="chart-release-native-145320d1"\nversion="0.0.0"\nedition="2024"\n'
        '[workspace]\n[dependencies]\n'
        'gpui-rhai={path=' + json.dumps(str(repo / "crates/gpui-rhai"))
        + ',features=["charts"]}\n'
        'gpui={version="=0.2.2",default-features=false,features=["font-kit","test-support"]}\n'
    )
    env = os.environ.copy()
    target = repo / "tests/native-keyboard/target"
    env.update({
        "CARGO_TARGET_DIR": str(target),
        "CLANG_MODULE_CACHE_PATH": str(target / "clang-module-cache"),
        "GPUI_RHAI_REVIEW_REPO": str(repo),
    })
    with tempfile.TemporaryDirectory(prefix="chart-core-native-") as work:
        work = Path(work)
        (work / "Cargo.toml").write_text(manifest)
        shutil.copyfile(repo / "Cargo.lock", work / "Cargo.lock")
        (work / "tests").mkdir()
        shutil.copyfile(audit / "native-probes.rs", work / "tests/review.rs")
        log = out / "native.log"
        print(f"Running release-review probes; output: {log}", flush=True)
        with log.open("w") as stream:
            result = subprocess.run([
                "cargo", "test", "--manifest-path", str(work / "Cargo.toml"),
                "--offline", "--", "--nocapture",
            ], cwd=repo, env=env, stdout=stream, stderr=subprocess.STDOUT)
        print(log.read_text(), end="")
        print(f"Exit code: {result.returncode}; output directory: {out}")
        return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
