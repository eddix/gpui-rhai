//! Independent cross-boundary policy probes; no native windows or full suite.
use gpui_rhai::*;
use std::{cell::RefCell,collections::BTreeMap,rc::Rc,time::{Duration,Instant}};
use semver::{Version,VersionReq};

struct Done;
impl AsyncCapabilityHandler for Done{
 fn start(&mut self,_:&str,_:UiValue)->Result<TaskWork,String>{Ok(TaskWork::new(||Ok(UiValue::Null)))}
}
const CHAIN:&str=r#"
fn state_schema(){#{fields:#{hops:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}}}
fn init(ctx){hop(ctx);}
fn hop(ctx){ctx.start_task("audit.task","get",(),Fn("completed"),Fn("failed"));}
fn failed(ctx,error){throw error.message;}
fn completed(ctx,out){let cost=0;for i in 0..2000{cost+=i;}
 let n=ctx.get_state("hops")+1;ctx.set_state("hops",n);if n<200{hop(ctx);}n}
fn view(ctx){text(ctx.get_state("hops").to_string())}
"#;

#[test]
fn actual_task_capture_chain_obeys_fresh_quota_and_reports_later_host_change(){
 let mut engine=RuntimeEngine::new();engine.set_operation_limit(10_000);
 let compiled=engine.compile_named("policy/task-chain.rhai",CHAIN).unwrap();let schema=engine.root_state_schema(&compiled).unwrap();
 let mut state=UiRuntimeState::new();let id=CapabilityId::parse("audit.task").unwrap();
 state.capabilities.register_async(CapabilityDescriptor{id:id.clone(),version:Version::new(1,0,0),methods:BTreeMap::from([("get".into(),CapabilityMethod{input:ValueSchema::Null,output:ValueSchema::Null})])},Done).unwrap();
 state.capabilities.activate(&BTreeMap::from([(id,VersionReq::STAR)])).unwrap();
 let state=Rc::new(RefCell::new(state));let root=ComponentInstancePath::root("App","policy-chain");
 let mut life=ScriptLifecycle::new(compiled,state.clone(),root.clone(),None,BTreeMap::new(),&schema).unwrap();life.start(&mut engine).unwrap();let _=engine.take_timings();
 let until=Instant::now()+Duration::from_secs(15);let mut first=None;let mut aggregate=0;
 for hop in 1..=200{
  let delivery=loop{if let Some(delivery)=state.borrow_mut().tasks.drain(life.generation()).pop(){break delivery;}assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(1));};
  if first.is_none(){first=Some(delivery.clone());}
  assert_eq!(life.invoke_async_delivery_transactional(&engine,delivery).unwrap().as_int().unwrap(),hop);
  let timing=engine.take_timings().into_iter().find(|t|matches!(t.operation,ExecutionOperation::Callback(_))).unwrap();
  assert_eq!(timing.operation_limit,10_000);assert!(timing.round_operations<10_000);aggregate+=timing.operations;
 }
 assert!(aggregate>DEFAULT_SCRIPT_OPERATION_LIMIT);assert_eq!(state.borrow().component_state.get(&root,"hops"),Some(&UiValue::Integer(200)));
 engine.set_operation_limit(100);let error=life.invoke_async_delivery_transactional(&engine,first.unwrap()).unwrap_err();
 let failed=engine.last_failed_timing().unwrap();assert_eq!(failed.operation_limit,100);assert_eq!(failed.round_operations,101);assert_eq!(failed.source,"policy/task-chain.rhai");
 assert_eq!(state.borrow().component_state.get(&root,"hops"),Some(&UiValue::Integer(200)));
 engine.set_operation_limit(500_000);let LifecycleError::Runtime(error)=error else{panic!("runtime")};
 let context=UiContext::new(state.clone(),root,None,ExecutionPhase::Event,BTreeMap::new()).with_generation(life.generation());
 let diagnostic=Diagnostic::from_runtime(&error,&DiagnosticContext::capture(&engine,&context,Some("policy/task-chain.rhai".into()),None).unwrap());let budget=diagnostic.operation_budget.unwrap();assert_eq!(budget.maximum,100);assert_eq!(budget.consumed,101);assert_eq!(diagnostic.source,Some("policy/task-chain.rhai".into()));
 println!("ACTUAL_TASK_CHAIN hops=200 aggregate_ops={aggregate} per_round_limit=10000 later_limit100_rejected_at101=true rollback_and_diagnostic_snapshot=true");
}

#[test]
fn parser_policy_is_independent_of_quota_and_named_failure_keeps_its_source(){
 let source=format!("fn view(){{text({}\"ok\"{})}}","(".repeat(24),")".repeat(24));
 let mut engine=RuntimeEngine::new();engine.set_operation_limit(17);
 let error=engine.compile_named("policy/deep-view.rhai",&source).err().expect("default parser rejects depth");let failed=engine.last_failed_timing().unwrap();
 assert_eq!(failed.operation,ExecutionOperation::Compile);assert_eq!(failed.source,"policy/deep-view.rhai");assert_eq!(failed.operation_limit,17);assert_eq!(failed.round_operations,0);
 let diagnostic=Diagnostic::from_runtime(&error,&DiagnosticContext{source:Some(failed.source.clone()),execution_timing:Some(failed),..Default::default()});assert_eq!(diagnostic.code,DiagnosticCode::ScriptCompile);assert_eq!(diagnostic.source,Some("policy/deep-view.rhai".into()));assert!(diagnostic.line.is_some());
 engine.set_expression_depth_limits(64,64);let compiled=engine.compile_named("policy/deep-view.rhai",&source).unwrap();engine.set_operation_limit(1_000);engine.render(&compiled).unwrap();
 let data_limits=(engine.engine().max_call_levels(),engine.engine().max_array_size(),engine.engine().max_map_size(),engine.engine().max_string_size());assert_eq!(data_limits,(64,10_000,100_000,1_048_576));
 println!("PARSE source_preserved=true runtime_ops=0 expression64_admits_same_source=true other_limits_unchanged=true");
}
