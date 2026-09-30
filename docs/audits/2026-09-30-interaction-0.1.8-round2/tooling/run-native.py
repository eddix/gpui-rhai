#!/usr/bin/env python3
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

here=Path(__file__).resolve().parent
repo=here.parents[3]
native=repo/'tests/native-keyboard'
with tempfile.TemporaryDirectory(prefix='gpui-round2-tooling-') as directory:
    workspace=Path(directory)
    manifest=re.sub(r'path = "([^"]+)"',lambda m:'path = "'+str((native/m[1]).resolve())+'"',(native/'Cargo.toml').read_text())
    (workspace/'Cargo.toml').write_text(manifest)
    shutil.copyfile(native/'Cargo.lock',workspace/'Cargo.lock')
    (workspace/'tests').mkdir()
    shutil.copyfile(here/'probes.rs',workspace/'tests/round2_tooling.rs')
    env=os.environ.copy();env['GPUI_RHAI_AUDIT_ROOT']=str(repo);env['CARGO_TARGET_DIR']=str(native/'target')
    result=subprocess.run(['cargo','test','--manifest-path',str(workspace/'Cargo.toml'),'--locked','--offline','--test','round2_tooling','--','--nocapture','--test-threads=1'],cwd=repo,env=env)
    raise SystemExit(result.returncode)
