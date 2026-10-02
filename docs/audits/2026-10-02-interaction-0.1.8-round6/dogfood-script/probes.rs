//! Bounded parser/value characterization only; no GPUI startup or disk scan.
#[cfg(test)]
use rhai::{Engine,Map,Scope,CallFnOptions};

#[test]
fn actual_skin_parser_limits_and_small_value_fixtures(){
 let root=std::env::var("DISKTREE_AUDIT_SOURCE").unwrap();
 for path in ["ui/core.rhai","ui/main.rhai","skins/minimal/core.rhai","skins/minimal/main.rhai"]{
  let source=std::fs::read_to_string(format!("{root}/{path}")).unwrap();
  for depth in [32,48,64,128]{
   let mut engine=Engine::new();engine.set_max_expr_depths(64,depth);engine.set_max_call_levels(64);
   let result=engine.compile(&source);
   println!("PARSE path={path} function_depth={depth} ok={} error={:?}",result.is_ok(),result.as_ref().err().map(|e|e.to_string()));
  }
  if path.ends_with("main.rhai") {
   let start=source.find("fn with_keys(node) {").unwrap();
   let end=start+source[start..].find("\n}\n").unwrap()+2;
   let bindings=source[start..end].lines().filter(|line|line.contains(".on_key_value(")).collect::<Vec<_>>();
   let mut replacement="fn with_keys(node) {\n".to_owned();
   for chunk in bindings.chunks(16){replacement.push_str("node = node\n");replacement.push_str(&chunk.join("\n"));replacement.push_str(";\n");}
   replacement.push_str("node\n}");
   let shorter=format!("{}{}{}",&source[..start],replacement,&source[end..]);
   let mut engine=Engine::new();engine.set_max_expr_depths(64,32);let result=engine.compile(&shorter);
   println!("PARSE split_key_chain path={path} depth=32 ok={} error={:?}",result.is_ok(),result.as_ref().err().map(|e|e.to_string()));
  }
 }
 let mut engine=Engine::new();engine.set_max_expr_depths(64,32);engine.set_max_operations(100_000);
 let cache_empty:bool=engine.eval(r#"
  let layout=#{by_key:#{},cache:[]};let tiles=[#{crumbs:[0]}];let kept_cache=[];
  kept_cache.push(["key",tiles,layout.by_key]);layout.cache=kept_cache;
  layout.by_key=#{"0":0};layout.cache[0][2].is_empty()
 "#).unwrap();assert!(cache_empty);println!("CACHE push_before_index_assignment_retains_empty_map=true");
 engine.set_max_array_size(10_000);
 let aggregate=engine.eval::<rhai::Dynamic>(r#"
  let tiles=[];let base=[];
  for i in 0..1200 {tiles.push(#{crumbs:[0,0,i]});base.push(#{t:"r"});}
  #{tiles:tiles,cache:[["key",tiles,#{}]],paint_by_mode:#{"0":#{base:base,labels:[]}}}
 "#);
 println!("CACHE 1200_depth3_tiles_plus_one_cache_and_paint={:?}",aggregate.as_ref().map(|_|()).map_err(|e|e.to_string()));assert!(aggregate.is_err());
 let extract=engine.eval::<rhai::Array>("let a=[0,1,2,3];let start=1;let end=3;a[start..end]");
 println!("RHAI computed_range_index={:?}",extract.as_ref().map(|a|a.len()).map_err(|e|e.to_string()));
 let underscore=engine.eval::<i64>("let _i=1;_i");println!("RHAI underscore_identifier={underscore:?}");
 let source=std::fs::read_to_string(format!("{root}/ui/core.rhai")).unwrap();
 let ast=engine.compile(&source).unwrap();
 let area=engine.eval::<Map>("#{x:0.0,y:0.0,w:600.0,h:400.0}").unwrap();
 let values=engine.eval::<rhai::Array>("[6.0,6.0,4.0,3.0,2.0,2.0,1.0]").unwrap();
 let rects:rhai::Array=engine.call_fn_with_options(CallFnOptions::new().eval_ast(false),&mut Scope::new(),&ast,"squarify",(values,area)).unwrap();assert_eq!(rects.len(),7);println!("SQUARIFY fixture_7_items_runs=true");
}
