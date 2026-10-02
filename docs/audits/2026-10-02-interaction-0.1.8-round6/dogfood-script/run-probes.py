#!/usr/bin/env python3
"""Tiny Rhai-only probe, reusing gpui-rhai dependency cache; never starts the app."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

here=Path(__file__).resolve().parent
repo=here.parents[3]
dogfood=repo.parent/"disktree-rhai"
with tempfile.TemporaryDirectory(prefix="gpui-r6-dogfood-script-") as temporary:
    workspace=Path(temporary)
    (workspace/"src").mkdir()
    (workspace/"Cargo.toml").write_text('''[package]
name="dogfood-script-audit"
version="0.0.0"
edition="2024"
[workspace]
[dependencies]
rhai={version="=1.26.0",features=["internals","metadata","serde"]}
''')
    shutil.copyfile(here/"probes.rs",workspace/"src/lib.rs")
    environment=os.environ.copy()
    environment["CARGO_TARGET_DIR"]=str(repo/"tests/native-keyboard/target")
    environment["DISKTREE_AUDIT_SOURCE"]=str(dogfood)
    result=subprocess.run(["cargo","test","--offline","--manifest-path",str(workspace/"Cargo.toml"),"--","--nocapture"],cwd=repo,env=environment)
    raise SystemExit(result.returncode)
