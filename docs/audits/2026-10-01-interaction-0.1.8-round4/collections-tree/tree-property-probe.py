"""Verify current product projection against a bounded independent forest oracle."""
from pathlib import Path
import hashlib
import subprocess
import tempfile

root=Path(__file__).resolve().parents[4]
path=root/"crates/gpui-rhai/src/collection_projection.rs"
source=path.read_text()
print(f"source={path} sha256={hashlib.sha256(source.encode()).hexdigest()}",flush=True)
decl=source[source.index("#[derive(Clone, Debug)]"):source.index("pub(crate) fn register_collection_projection_api")]
functions=source[source.index("fn outline_projection"):source.index("fn outline_message")]
main=r'''
fn random(seed:&mut u64)->usize{*seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1);(*seed>>32)as usize}
fn expected_order(parent:Option<usize>,order:&[usize],parents:&[Option<usize>],expanded:&[bool],out:&mut Vec<usize>){
  for i in order.iter().copied().filter(|i|parents[*i]==parent){out.push(i);if expanded[i]{expected_order(Some(i),order,parents,expanded,out);}}
}
fn item(i:usize,parent:Option<usize>,disabled:bool)->UiValue{UiValue::Map(BTreeMap::from([
  ("key".into(),UiValue::String(format!("k{i:05}"))), ("label".into(),UiValue::String(format!("Node {i}"))),
  ("parent".into(),parent.map_or(UiValue::Null,|p|UiValue::String(format!("k{p:05}")))),("disabled".into(),UiValue::Bool(disabled)),
]))}
fn main(){
  for case in 0..96 {
    let mut seed=case+19;
    let parents=(0..32).map(|i|if i==0||random(&mut seed)%5==0{None}else{Some(random(&mut seed)%i)}).collect::<Vec<_>>();
    let mut disabled=(0..32).map(|_|random(&mut seed)%3==0).collect::<Vec<_>>();
    let mut order=(0..32).collect::<Vec<_>>();
    for i in (1..32).rev(){let j=random(&mut seed)%(i+1);order.swap(i,j);}
    for pass in 0..2 {
      if pass==1 {disabled.iter_mut().for_each(|d|*d=!*d);order.reverse();}
      let expanded=(0..32).map(|_|pass==0||random(&mut seed)%2==0).collect::<Vec<_>>();
      let input=order.iter().map(|i|item(*i,parents[*i],disabled[*i])).collect::<Vec<_>>();
      let expanded_keys=(0..32).filter(|i|expanded[*i]).map(|i|format!("k{i:05}")).collect::<Vec<_>>();
      let output=outline_projection(&input,&expanded_keys).unwrap();
      let mut expected=vec![];expected_order(None,&order,&parents,&expanded,&mut expected);
      assert_eq!(output.len(),expected.len());
      for (value,i) in output.iter().zip(expected){
        let UiValue::Map(row)=value else{unreachable!()};
        assert_eq!(row["key"],UiValue::String(format!("k{i:05}")));
        let mut cursor=parents[i];let mut nearest=None;let mut depth=0;
        while let Some(p)=cursor{depth+=1;if nearest.is_none()&&!disabled[p]{nearest=Some(p);}cursor=parents[p];}
        assert_eq!(row["depth"],UiValue::Integer(depth));
        assert_eq!(row["eligible_parent"],nearest.map_or(UiValue::Null,|p|UiValue::String(format!("k{p:05}"))));
      }
    }
  }
  println!("192 shuffled/reversed, expanded/collapsed, disabled-toggled forest projections matched independent order/depth/nearest-parent oracle");
  for disabled in [false,true] {
    let input=(0..10000).map(|i|item(i,if i==0{None}else{Some(if i<256{i-1}else{255})},disabled&&i>0&&i<256)).collect::<Vec<_>>();
    let start=std::time::Instant::now();let output=outline_projection(&input,&[]).unwrap();
    println!("legal shared depth256 / nodes10000 / disabled={disabled}: visible={} elapsed={:?}",output.len(),start.elapsed());
  }
}
'''
code='#![allow(dead_code)]\nuse std::collections::BTreeMap;\n#[derive(Clone,Debug,PartialEq)] enum UiValue{Null,String(String),Bool(bool),Integer(i64),Map(BTreeMap<String,UiValue>)}\n'+decl+functions+main
with tempfile.TemporaryDirectory(prefix="audit-r4-tree-property-") as tmp:
    p=Path(tmp);(p/"main.rs").write_text(code)
    subprocess.run(["rustc","--edition=2024","-O",str(p/"main.rs"),"-o",str(p/"probe")],check=True,cwd=root)
    subprocess.run([str(p/"probe")],check=True)
