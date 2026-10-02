from pathlib import Path
import io, tarfile, tempfile, subprocess, re
out=Path(__file__).resolve().parent
root=out.parents[3]
commit='3209039e2d792dde9f65daa7eb2600cd90a3e3f6'
print('PR snapshot commit='+commit,flush=True)
with tempfile.TemporaryDirectory(prefix='pr99-snapshot-',dir=out) as tmp:
    tmp=Path(tmp); snap=tmp/'source'; snap.mkdir(); harness=tmp/'harness';harness.mkdir()
    archive=subprocess.check_output(['git','archive',commit,'Cargo.toml','Cargo.lock','rust-toolchain.toml','.cargo/config.toml','crates/gpui-rhai','registry'],cwd=root)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        for member in tar.getmembers():
            assert not member.name.startswith('/') and '..' not in Path(member.name).parts
            assert not member.issym() and not member.islnk()
        tar.extractall(snap)
    p=snap/'Cargo.toml';p.write_text(re.sub(r'members = \[.*?\]','members = ["crates/gpui-rhai", "registry"]',p.read_text(),count=1,flags=re.S))
    manifest=(out/'Cargo.toml').read_text().replace('path = "../../../../crates/gpui-rhai"','path = "../source/crates/gpui-rhai"')
    (harness/'Cargo.toml').write_text(manifest);(harness/'Cargo.lock').write_text((out/'Cargo.lock').read_text());(harness/'probe.rs').write_text((out/'probe.rs').read_text());(harness/'snapshot-theme.rhai').write_text((snap/'registry/themes/default_dark.rhai').read_text())
    result=subprocess.run(['cargo','test','--offline','--locked','--manifest-path',str(harness/'Cargo.toml'),'--target-dir',str(root/'tests/native-keyboard/target'),'--test','pr99_theme_characterization','--','--nocapture','--test-threads=1'],cwd=root)
    raise SystemExit(result.returncode)
