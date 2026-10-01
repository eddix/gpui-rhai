#!/usr/bin/env python3
"""Compile a disposable audit test with the existing native dependency cache."""
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
here=Path(__file__).resolve().parent
repo=here.parents[3]
native=repo/'tests/native-keyboard'
with tempfile.TemporaryDirectory(prefix='gpui-round4-schema-') as temporary:
    workspace=Path(temporary)
    manifest=re.sub(r'path = "([^"]+)"',lambda m:'path = "'+str((native/m[1]).resolve())+'"',(native/'Cargo.toml').read_text())
    (workspace/'Cargo.toml').write_text(manifest)
    shutil.copyfile(native/'Cargo.lock',workspace/'Cargo.lock')
    (workspace/'tests').mkdir()
    shutil.copyfile(here/'probes.rs',workspace/'tests/schema_round4.rs')
    env=os.environ.copy();env['CARGO_TARGET_DIR']=str(native/'target')
    result=subprocess.run(['cargo','test','--manifest-path',str(workspace/'Cargo.toml'),'--locked','--offline','--test','schema_round4','--','--nocapture','--test-threads=1'],cwd=repo,env=env)
    raise SystemExit(result.returncode)
