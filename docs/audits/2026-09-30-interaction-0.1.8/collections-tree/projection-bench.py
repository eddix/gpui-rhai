"""Benchmark the exact product projection source without GPUI link/build overhead.

This extracts unchanged functions from collection_projection.rs and supplies
only its UiValue data enum; it does not reimplement the algorithm.
"""
from pathlib import Path
import hashlib
import subprocess
import tempfile

root = Path(__file__).resolve().parents[4]
source_path = root / "crates/gpui-rhai/src/collection_projection.rs"
source = source_path.read_text()
print(f"source={source_path} sha256={hashlib.sha256(source.encode()).hexdigest()}", flush=True)
decl = source[source.index("#[derive(Clone, Debug)]"):source.index("pub(crate) fn register_collection_projection_api")]
funcs = source[source.index("fn outline_projection"):source.index("fn outline_message")]
main = r'''
fn main() {
    for count in [100, 1000, 3000, 6000, 10000] {
        let input = (0..count).map(|i| UiValue::Map(BTreeMap::from([
            ("key".into(), UiValue::String(format!("k{i:05}"))),
            ("label".into(), UiValue::String(format!("k{i:05}"))),
            ("parent".into(), if i == 0 { UiValue::Null } else { UiValue::String(format!("k{:05}", i-1)) }),
        ]))).collect::<Vec<_>>();
        let start = std::time::Instant::now();
        let output = outline_projection(&input, &[]);
        println!("collapsed chain count={count} elapsed={:?} result={:?}", start.elapsed(), output.as_ref().map(|v| v.len()));
    }
    let input = (0..258).map(|i| UiValue::Map(BTreeMap::from([
        ("key".into(), UiValue::String(format!("k{i:05}"))),
        ("label".into(), UiValue::String(format!("k{i:05}"))),
        ("parent".into(), if i == 0 { UiValue::Null } else { UiValue::String(format!("k{:05}", i-1)) }),
    ]))).collect::<Vec<_>>();
    println!("depth257 collapsed: {:?}", outline_projection(&input, &[]).as_ref().map(|v| v.len()));
    println!("depth257 expanded: {:?}", outline_projection(&input, &(0..258).map(|i|format!("k{i:05}")).collect::<Vec<_>>()).as_ref().map(|v|v.len()));
}
'''
code = '#![allow(dead_code)]\nuse std::collections::BTreeMap;\n#[derive(Clone,Debug)] enum UiValue { Null, String(String), Bool(bool), Integer(i64), Map(BTreeMap<String,UiValue>) }\n' + decl + funcs + main
with tempfile.TemporaryDirectory(prefix="audit-tree-projection-") as tmp:
    p = Path(tmp)
    (p / "main.rs").write_text(code)
    subprocess.run(["rustc", "--edition=2024", "-O", str(p / "main.rs"), "-o", str(p / "probe")], check=True, cwd=root)
    subprocess.run([str(p / "probe")], check=True)
