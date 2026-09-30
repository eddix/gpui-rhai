#!/usr/bin/env python3
"""Verify old #90 is fixed, then test a real initialization dependency."""
import json
from pathlib import Path
import subprocess
import tempfile

repo = Path(__file__).resolve().parents[4]
subprocess.run(["cargo", "build", "--quiet", "--locked", "--offline", "-p", "gpui-rhai-cli"], cwd=repo, check=True)
binary = repo / "target/debug/gpui-rhai"
results = []
with tempfile.TemporaryDirectory(prefix="gpui-round2-check-") as temporary:
    root = Path(temporary)
    (root / "src").mkdir()
    (root / "src/main.rs").write_text("fn main() {}\n")
    (root / "Cargo.toml").write_text('[package]\nname="audit-check-round2"\nversion="0.0.0"\nedition="2024"\n[dependencies]\n')
    def call(command, case):
        p = subprocess.run([str(binary), "--root", str(root), command], capture_output=True, text=True)
        r = {"case": case, "exit": p.returncode, "stdout": p.stdout, "stderr": p.stderr}
        results.append(r)
        return r
    assert call("init", "initialize")["exit"] == 0
    manifest = 'entry="main"\nruntime_api=2\n'
    capabilities = '\n[capabilities]\n"app.demo"="*"\n'
    (root / "ui/app.toml").write_text(manifest + capabilities)
    (root / "ui/main.rhai").write_text('fn init(ctx){ctx.call_capability("app.demo","ping",());}\nfn view(ctx){text("hello")}\n')
    assert call("check", "old_issue90_control")["exit"] == 0
    source = '''
fn state_schema(){#{fields:#{ready:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
fn init(ctx){ctx.set_state("ready",true);}
fn view(ctx){if !ctx.get_state("ready"){throw "init must prepare view state";} text("ready")}
'''
    (root / "ui/main.rhai").write_text(source)
    (root / "ui/app.toml").write_text(manifest)
    assert call("check", "valid_init_dependency_without_capability")["exit"] == 0
    (root / "ui/app.toml").write_text(manifest + capabilities)
    # The exact same legal script should not become invalid just because the
    # application declares a Host capability. This is a characterization.
    call("check", "same_valid_script_with_declared_capability")
print(json.dumps({"head": subprocess.check_output(["git","rev-parse","HEAD"],cwd=repo,text=True).strip(),
                  "results": results}, ensure_ascii=False, indent=2))
