"""Compile unchanged product projection functions with small data enum shim."""
from pathlib import Path
import hashlib
import subprocess
import tempfile

root = Path(__file__).resolve().parents[4]
product = root / "crates/gpui-rhai/src/collection_projection.rs"
source = product.read_text()
print(f"source={product} sha256={hashlib.sha256(source.encode()).hexdigest()}", flush=True)
decl = source[source.index("#[derive(Clone, Debug)]"):source.index("pub(crate) fn register_collection_projection_api")]
funcs = source[source.index("fn outline_projection"):source.index("fn outline_message")]
main = r'''
fn main() {
  for (count, depth) in [(1000, 128), (4096, 256), (10000, 256)] {
    for disabled in [false, true] {
      let input=(0..count).map(|i|UiValue::Map(BTreeMap::from([
        ("key".into(),UiValue::String(format!("k{i:05}"))),
        ("label".into(),UiValue::String(format!("Node {i}"))),
        ("disabled".into(),UiValue::Bool(disabled && i>0 && i<depth)),
        ("parent".into(),if i==0{UiValue::Null}else{UiValue::String(format!("k{:05}",if i<depth{i-1}else{depth-1}))}),
      ]))).collect::<Vec<_>>();
      let start=std::time::Instant::now();
      let out=outline_projection(std::hint::black_box(&input),&[]).unwrap();
      println!("count={count} depth={depth} disabled={disabled} collapsed_rows={} elapsed={:?}",out.len(),start.elapsed());
    }
  }
  let make = |key:&str,parent:Option<&str>,disabled:bool| UiValue::Map(BTreeMap::from([
     ("key".into(),UiValue::String(key.into())), ("label".into(),UiValue::String(key.into())),
     ("parent".into(),parent.map_or(UiValue::Null,|p|UiValue::String(p.into()))), ("disabled".into(),UiValue::Bool(disabled)),
  ]));
  let input=vec![make("child",Some("disabled"),false),make("disabled",Some("root"),true),make("root",None,false)];
  let out=outline_projection(&input,&["root".into(),"disabled".into()]).unwrap();
  if let UiValue::Map(row)=&out[2] { println!("unsorted child eligible_parent={:?}",row["eligible_parent"]); }
}
'''
code = '#![allow(dead_code)]\nuse std::collections::BTreeMap;\n#[derive(Clone,Debug)] enum UiValue{Null,String(String),Bool(bool),Integer(i64),Map(BTreeMap<String,UiValue>)}\n'+decl+funcs+main
with tempfile.TemporaryDirectory(prefix="audit-r3-enabled-parent-") as tmp:
    folder=Path(tmp)
    (folder/"main.rs").write_text(code)
    subprocess.run(["rustc","--edition=2024","-O",str(folder/"main.rs"),"-o",str(folder/"probe")],check=True,cwd=root)
    subprocess.run([str(folder/"probe")],check=True)
