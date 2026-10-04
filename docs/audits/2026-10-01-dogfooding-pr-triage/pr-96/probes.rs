//! Baseline characterization on f6e936a5, not a claim that PR 96 was compiled.
use gpui_rhai::{RuntimeEngine,ExecutionOperation};

#[test]
fn pr_workload_and_a_true_over_default_case(){
 for iterations in [60_000,400_000]{
  let source=format!("fn view() {{ let n = 0; for i in 0..{iterations} {{ n += 1; }} text(`${{n}}`) }}");
  let mut engine=RuntimeEngine::new();
  let compiled=engine.compile_self_contained_named("pr96-baseline",&source).unwrap();
  let result=engine.render(&compiled);
  let timing=engine.take_timings().into_iter().find(|x|x.operation==ExecutionOperation::Render).unwrap();
  println!("BASELINE iterations={iterations} ok={} operations={} elapsed_ms={:.3}",result.is_ok(),timing.operations,timing.duration.as_secs_f64()*1000.0);
  assert_eq!(result.is_ok(),iterations==60_000);
 }
}

#[test]
fn parser_expression_limit_is_distinct_from_runtime_operations(){
 for nesting in [4,64]{
  let source=format!("fn view() {{ text({}\"nested\"{}) }}","(".repeat(nesting),")".repeat(nesting));
  let mut engine=RuntimeEngine::new();
  let result=engine.compile(&source);
  let timing=engine.take_timings().into_iter().find(|x|x.operation==ExecutionOperation::Compile).unwrap();
  println!("PARSE nesting={nesting} ok={} operations={} error={:?}",result.is_ok(),timing.operations,result.as_ref().err().map(|e|e.to_string()));
  assert_eq!(result.is_ok(),nesting==4);
 }
}
