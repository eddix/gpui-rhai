#!/usr/bin/env python3
"""Read-only replay of previous expected-behavior tests; save only new logs."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

here=Path(__file__).resolve().parent
repo=here.parents[2]
first=repo/'docs/audits/2026-09-30-interaction-0.1.8'
second=repo/'docs/audits/2026-09-30-interaction-0.1.8-round2'
third=repo/'docs/audits/2026-09-30-interaction-0.1.8-round3'
out=here/'baseline';out.mkdir(parents=True,exist_ok=True)
env=os.environ.copy();env['CARGO_TARGET_DIR']=str(repo/'tests/native-keyboard/target')
def cargo(manifest, target):
    return ['cargo','test','--manifest-path',str(manifest),'--locked','--offline',*target,'--','--nocapture','--test-threads=1']
cases=[
 ('r1-core',['python3',str(second/'run-original-core.py')],first/'runtime-core/probes.rs'),
 ('r1-drag',cargo(first/'drag-sort/probe/Cargo.toml',['--lib']),first/'drag-sort/probe/src/lib.rs'),
 ('r1-resize',cargo(first/'resize-controls/Cargo.toml',['--test','probe']),first/'resize-controls/probe.rs'),
 ('r1-transforms',cargo(first/'transforms-selection/Cargo.toml',['--lib']),first/'transforms-selection/src/lib.rs'),
 ('r2-core',['python3',str(second/'runtime-core/run-probes.py')],second/'runtime-core/probes.rs'),
 ('r2-drag',cargo(second/'drag-sort/probe/Cargo.toml',['--lib']),second/'drag-sort/probe/src/lib.rs'),
 ('r2-resize',cargo(second/'resize-controls/Cargo.toml',['--test','probe']),second/'resize-controls/probe.rs'),
 ('r2-transforms',cargo(second/'transforms-selection/Cargo.toml',['--lib']),second/'transforms-selection/src/lib.rs'),
 ('r2-tooling',['python3',str(second/'tooling/run-native.py')],second/'tooling/probes.rs'),
 ('r3-core',['python3',str(third/'runtime-core/run-reviewed.py')],third/'runtime-core/reviewed-probes.rs'),
 ('r3-drag',cargo(third/'drag-sort/probe/Cargo.toml',['--lib']),third/'drag-sort/probe/src/lib.rs'),
 ('r3-resize',cargo(third/'resize-controls/Cargo.toml',['--test','resize_controls_round3_probe']),third/'resize-controls/probe.rs'),
 ('r3-transforms',cargo(third/'transforms-selection/Cargo.toml',['--lib']),third/'transforms-selection/src/lib.rs'),
 ('r2-async',['python3',str(second/'rhai-async/run-probes.py')],second/'rhai-async/probes.rs'),
]
results=[]
for name,command,source in cases:
    before=hashlib.sha256(source.read_bytes()).hexdigest();start=time.monotonic()
    print('Running '+name,flush=True)
    with (out/f'{name}.log').open('w') as log:
        result=subprocess.run(command,cwd=repo,env=env,stdout=log,stderr=subprocess.STDOUT)
    after=hashlib.sha256(source.read_bytes()).hexdigest();assert before==after
    item={'name':name,'exit':result.returncode,'seconds':time.monotonic()-start,
          'source':str(source.relative_to(repo)),'source_sha256':after}
    print(json.dumps(item),flush=True);results.append(item)
    (out/'results.json').write_text(json.dumps({'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),'results':results},indent=2)+'\n')
raise SystemExit(any(r['exit'] for r in results))
