#!/usr/bin/env python3
"""Run chart acceptance probes in a disposable package; product files stay intact."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("public", "native"))
    parser.add_argument("--out-dir", type=Path)
    args = parser.parse_args()
    audit = Path(__file__).resolve().parent
    repo = next(p for p in audit.parents if (p / "crates/gpui-rhai/Cargo.toml").is_file())
    out = (args.out_dir or Path(tempfile.mkdtemp(prefix="chart-acceptance-output-"))).resolve()
    out.mkdir(parents=True, exist_ok=True)
    native = args.mode != "public"
    package = "native" if native else "public"
    manifest = (
        f'[package]\nname="chart-acceptance-{package}-a8d2ef0f"\nversion="0.0.0"\nedition="2024"\n'
        '[workspace]\n[dependencies]\n'
        'gpui-rhai={path=' + json.dumps(str(repo / "crates/gpui-rhai"))
        + ',features=["charts"]}\n'
    )
    manifest += (
        'gpui={version="=0.2.2",default-features=false,features=["font-kit","test-support"]}\n'
        if native else 'serde_json="1"\n'
    )
    env = os.environ.copy()
    target = repo / ("tests/native-keyboard/target" if native else "target")
    env.update({
        "CARGO_TARGET_DIR": str(target),
        "CLANG_MODULE_CACHE_PATH": str(target / "clang-module-cache"),
        "GPUI_RHAI_REVIEW_REPO": str(repo),
        "GPUI_RHAI_CHART_AUDIT_OUTPUT": str(out),
    })
    with tempfile.TemporaryDirectory(prefix=f"chart-acceptance-{args.mode}-") as work:
        work = Path(work)
        (work / "Cargo.toml").write_text(manifest)
        shutil.copyfile(repo / "Cargo.lock", work / "Cargo.lock")
        dest = work / ("tests/review.rs" if native else "src/main.rs")
        dest.parent.mkdir(parents=True)
        shutil.copyfile(audit / ("native-probes.rs" if native else "public-probes.rs"), dest)
        command = ["cargo", "test" if native else "run", "--manifest-path", str(work / "Cargo.toml"), "--offline"]
        if args.mode == "native":
            command += ["--", "--nocapture"]
        log = out / f"{args.mode}.log"
        print(f"Running {args.mode}; output: {log}", flush=True)
        with log.open("w") as stream:
            result = subprocess.run(command, cwd=repo, env=env, stdout=stream, stderr=subprocess.STDOUT)
        print(log.read_text(), end="")
        print(f"Exit code: {result.returncode}; output directory: {out}")
        return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
