#!/usr/bin/env python3
"""Replay review probes against a checkout without changing tracked files.

The assertions state required behavior. They intentionally fail at the reviewed
head and should pass after remediation. Requires the repository Rust toolchain.
"""
import argparse, hashlib, os, subprocess
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--repo',type=Path,required=True)
p.add_argument('--target-dir',type=Path)
a=p.parse_args(); repo=a.repo.resolve(); here=Path(__file__).resolve().parent
print('Review target:',subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),flush=True)
cmd=['cargo','test','--manifest-path','tests/native-keyboard/Cargo.toml','--locked','--offline','--no-fail-fast']
if a.target_dir: cmd+=['--target-dir',str(a.target_dir.resolve())]
files=[]
try:
    for name in ['probes','issue131']:
        data=(here/(name+'.rs')).read_bytes()
        target='audit_pr111_round4_'+name
        dest=repo/'tests/native-keyboard/tests'/(target+'.rs')
        if dest.exists(): raise SystemExit('Refusing to overwrite '+str(dest))
        dest.write_bytes(data);files.append((dest,data));cmd+=['--test',target]
        print(name,hashlib.sha256(data).hexdigest(),flush=True)
    env=os.environ.copy();env.pop('RUST_MIN_STACK',None)
    result=subprocess.run(cmd+['--','--nocapture','--test-threads=1'],cwd=repo,env=env)
finally:
    for path,data in files:
        if path.exists() and path.read_bytes()==data:path.unlink()
raise SystemExit(result.returncode)
