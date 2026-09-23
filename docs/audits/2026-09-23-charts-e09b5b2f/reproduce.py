#!/usr/bin/env python3
"""Run independent chart audit probes without editing workspace manifests."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("public", "native", "bench"))
    parser.add_argument("--out-dir", type=Path)
    args = parser.parse_args()
    audit = Path(__file__).resolve().parent
    repo = audit.parents[2]
    out = (args.out_dir or Path(tempfile.mkdtemp(prefix="chart-audit-output-"))).resolve()
    out.mkdir(parents=True, exist_ok=True)
    mode = args.mode
    name = {
        "public": "gpui-rhai-chart-audit-e09b5b2f",
        "native": "gpui-rhai-chart-native-audit-e09b5b2f",
        "bench": "gpui-rhai-chart-bench-e09b5b2f",
    }[mode]
    manifest = (
        f'[package]\nname={json.dumps(name)}\nversion="0.0.0"\nedition="2024"\n'
        '[workspace]\n[dependencies]\n'
        'gpui-rhai={path=' + json.dumps(str(repo / "crates/gpui-rhai"))
        + ',features=["charts"]}\n'
    )
    if mode == "public":
        manifest += 'serde_json="1"\n'
    elif mode == "native":
        manifest += 'gpui={version="=0.2.2",default-features=false,features=["font-kit","test-support"]}\n'
    else:
        manifest += '[profile.release]\nlto="thin"\n'
    env = os.environ.copy()
    target = repo / ("tests/native-keyboard/target" if mode == "native" else "target")
    env.update({
        "CARGO_TARGET_DIR": str(target),
        "CLANG_MODULE_CACHE_PATH": str(target / "clang-module-cache"),
        "GPUI_RHAI_REVIEW_REPO": str(repo),
        "GPUI_RHAI_CHART_AUDIT_OUTPUT": str(out),
    })
    with tempfile.TemporaryDirectory(prefix=f"chart-audit-{mode}-") as work:
        work = Path(work)
        (work / "Cargo.toml").write_text(manifest)
        # Start from the repository resolution; Cargo adds only the temporary
        # package to this copy. Offline mode prevents fetching new dependencies.
        shutil.copyfile(repo / "Cargo.lock", work / "Cargo.lock")
        source = {"public": "probes.rs", "native": "native-probes.rs", "bench": "bench.rs"}[mode]
        dest = work / ("tests/interaction.rs" if mode == "native" else "src/main.rs")
        dest.parent.mkdir(parents=True)
        shutil.copyfile(audit / source, dest)
        command = ["cargo", "test" if mode == "native" else "run"]
        if mode == "bench":
            command.append("--release")
        command += ["--manifest-path", str(work / "Cargo.toml"), "--offline"]
        if mode == "native":
            command += ["--", "--nocapture"]
        log = out / f"{mode}.log"
        print(f"Running {mode}; output: {log}", flush=True)
        with log.open("w") as stream:
            result = subprocess.run(command, cwd=repo, env=env, stdout=stream, stderr=subprocess.STDOUT)
        print(log.read_text(), end="")
        print(f"Exit code: {result.returncode}; output directory: {out}")
        return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
