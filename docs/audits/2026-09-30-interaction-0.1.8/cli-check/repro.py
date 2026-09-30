#!/usr/bin/env python3
"""Characterize issue #90 with the current CLI in a disposable project."""
import json
from pathlib import Path
import subprocess
import tempfile

repo = Path(__file__).resolve().parents[4]
subprocess.run([
    "cargo", "build", "--quiet", "--locked", "--offline", "-p", "gpui-rhai-cli"
], cwd=repo, check=True)
binary = repo / "target/debug/gpui-rhai"
results = []
with tempfile.TemporaryDirectory(prefix="gpui-check-90-") as temporary:
    root = Path(temporary)
    (root / "src").mkdir()
    (root / "src/main.rs").write_text("fn main() {}\n")
    (root / "Cargo.toml").write_text(
        '[package]\nname = "audit-check-90"\nversion = "0.0.0"\nedition = "2024"\n\n[dependencies]\n'
    )
    def run(command, name):
        result = subprocess.run([str(binary), "--root", str(root), command],
                                capture_output=True, text=True)
        record = {"case": name, "exit": result.returncode,
                  "stdout": result.stdout, "stderr": result.stderr}
        results.append(record)
        return record
    assert run("init", "initialize")["exit"] == 0
    assert run("check", "generated_baseline")["exit"] == 0
    (root / "ui/app.toml").write_text(
        'entry = "main"\nruntime_api = 2\n\n[capabilities]\n"app.demo" = "*"\n'
    )
    (root / "ui/main.rhai").write_text('''
fn init(ctx) { let result = ctx.call_capability("app.demo", "ping", ()); }
fn view(ctx) { column([text("hello")]) }
''')
    failure = run("check", "manifest_declared_capability_in_init")
    assert failure["exit"] != 0 and "was not declared" in failure["stderr"]
    (root / "ui/main.rhai").write_text('''
fn view(ctx) { throw "intentional invalid initial view"; }
''')
    assert run("check", "invalid_initial_view_control")["exit"] != 0
print(json.dumps({"head": subprocess.check_output(["git", "rev-parse", "HEAD"],
    cwd=repo, text=True).strip(), "results": results}, ensure_ascii=False, indent=2))
