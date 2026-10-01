#!/usr/bin/env python3
"""Compile exact production numeric functions with a finite property harness.
Only this audit directory and a new unique TemporaryDirectory are written.
"""
from pathlib import Path
import hashlib,re,subprocess,tempfile
root=Path(__file__).resolve().parents[4]
out=Path(__file__).resolve().parent
slider=root/'crates/gpui-rhai/src/range_slider.rs'
axis=root/'crates/gpui-rhai/src/range_input.rs'
def extract(path,name):
    s=path.read_text();m=re.search(r'(?:pub\(crate\) )?fn '+name+r'\(',s);start=m.start();brace=s.index('{',m.end());depth=1;end=brace+1
    while depth:
        depth+= (s[end]=='{')-(s[end]=='}');end+=1
    return s[start:end]
functions=[extract(axis,'normalize_value')]+[extract(slider,name) for name in ['normalize_pair_values','first_step_at_or_above','last_step_at_or_below','pair_distance','normalize_constrained_value']]
program='#[derive(Clone,Copy,Debug,PartialEq)] struct RangePair{low:f64,high:f64}\n'+'\n'.join(functions)+r'''
fn main(){
 let mut count=0;
 for min in [0.0_f64,0.1,3.5,-20.0] {for span in [1.0_f64,3.7,97.0,100.0]{let max=min+span;
  for step in [0.1_f64,0.3,1.0,3.0,10.0] {if step>span{continue}
   for gap in [0.0,span/10.0,span/3.0,span/2.0,span] {
    for li in 0..=20{for hi in li..=20 {let low=min+span*f64::from(li)/20.0;let high=min+span*f64::from(hi)/20.0;
     if low+gap>high{continue}
     let source=RangePair{low,high};let pair=normalize_pair_values(source,min,max,step,gap);
     assert!(pair.low.is_finite()&&pair.high.is_finite());
     assert!(pair.low>=min-1e-9&&pair.high<=max+1e-9&&pair.high-pair.low+1e-9>=gap,"bounds/gap source={source:?} pair={pair:?} min={min} max={max} step={step} gap={gap}");
     let second=normalize_pair_values(pair,min,max,step,gap);
     assert!((second.low-pair.low).abs()<1e-8&&(second.high-pair.high).abs()<1e-8,"not idempotent source={source:?} pair={pair:?} second={second:?} min={min} max={max} step={step} gap={gap}");
     count+=1;
    }}
   }
  }
 }}
 println!("PASS {count} finite legal source/constraint combinations: bounds, gap and idempotence (1e-9/1e-8 tolerance)");
}
'''
(out/'math-probe.rs').write_text(program)
print('range_slider sha256='+hashlib.sha256(slider.read_bytes()).hexdigest(),flush=True)
print('range_input sha256='+hashlib.sha256(axis.read_bytes()).hexdigest(),flush=True)
with tempfile.TemporaryDirectory(prefix='resize-controls-round3-815bb82b-math-') as tmp:
    binary=Path(tmp)/'math_probe'
    subprocess.run(['rustc','--edition=2024','-Awarnings',str(out/'math-probe.rs'),'-o',str(binary)],cwd=root,check=True)
    subprocess.run([str(binary)],check=True)
