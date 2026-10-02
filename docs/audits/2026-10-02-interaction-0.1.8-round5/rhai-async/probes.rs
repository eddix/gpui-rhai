//! Local bounded acceptance of effect-activation delivery at 23fce238.
use std::{cell::{Cell,RefCell},collections::BTreeMap,rc::Rc,time::{Duration,Instant}};
use gpui_rhai::{CapabilityDescriptor,CapabilityId,CapabilityMethod,ComponentInstancePath,
 RuntimeEngine,ScriptLifecycle,UiRuntimeState,UiValue,ValueSchema,SubscriptionCapabilityHandler,
 SubscriptionWork,SubscriptionEmitter,SubscriptionCloseReason,CapabilityHandler,AsyncDelivery};
use semver::{Version,VersionReq};

const APP:&str=r#"
define_component(#{
 metadata:#{id:"test/stream", "export":"StreamProbe",version:"0.1.0",
 runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{"app.stream":"*","app.marker":"*"}},
 schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
 state:#{fields:#{phase:#{schema:#{type:"string"},"default":#{type:"string",value:"idle"}},
 epoch:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}},
 events:#{},slots:#{},parts:[],effects:["watch"]},render:Fn("render_StreamProbe")});
fn start_stream(ctx,deps){ctx.start_subscription("app.stream","watch",deps,Fn("received"),Fn("failed"),#{delivery:"all"});}
fn stop_stream(ctx,deps){ctx.call_capability("app.marker","cleanup",deps);}
fn received(ctx,value){
 if value=="replace" {ctx.set_state("epoch",1);ctx.set_state("phase","new-query");}
 else {ctx.set_state("phase",value);}
}
fn failed(ctx,error){ctx.set_state("phase","error:"+error.message);}
fn render_StreamProbe(ctx,props){effect("watch",ctx.get_state("epoch"),Fn("start_stream"),Fn("stop_stream"));text(ctx.get_state("phase"))}
fn state_schema(){#{fields:#{visible:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}}}}
fn hide(ctx,event){ctx.set_state("visible",false);}
fn view(ctx){column([text("hide").on_click(Fn("hide")),if ctx.get_state("visible"){render_component("test/stream",#{key:"stream"})}else{text("gone")}])}
"#;
struct Stream{send:std::sync::mpsc::Sender<(i64,SubscriptionEmitter)>,fail_new:bool,finite:bool}
impl SubscriptionCapabilityHandler for Stream{
 fn subscribe(&mut self,_:&str,input:UiValue)->Result<SubscriptionWork,String>{
  let UiValue::Integer(epoch)=input else{return Err("integer".into())};
  if self.fail_new && epoch==1{return Err("new subscription startup rejected".into());}
  let send=self.send.clone();let finite=self.finite;
  Ok(SubscriptionWork::new(move|emitter|{
   if finite{emitter.emit(UiValue::String("finite-value".into())).unwrap();}
   send.send((epoch,emitter.clone())).unwrap();
   if !finite{
    let until=Instant::now()+Duration::from_secs(10);
    while emitter.close_reason().is_none() && Instant::now()<until{std::thread::sleep(Duration::from_millis(1));}
   }
  }))
 }
}
struct Marker{calls:Rc<RefCell<Vec<UiValue>>>,fail:Rc<Cell<bool>>}
impl CapabilityHandler for Marker{
 fn call(&mut self,_:&str,input:UiValue)->Result<UiValue,String>{
  self.calls.borrow_mut().push(input);
  if self.fail.get(){Err("cleanup-rejected".into())}else{Ok(UiValue::Null)}
 }
}
struct Harness{
 engine:RuntimeEngine,lifecycle:ScriptLifecycle,runtime:Rc<RefCell<UiRuntimeState>>,
 component:ComponentInstancePath,receive:std::sync::mpsc::Receiver<(i64,SubscriptionEmitter)>,
 cleanups:Rc<RefCell<Vec<UiValue>>>,fail_cleanup:Rc<Cell<bool>>,
}
impl Harness{
 fn new(fail_new:bool,finite:bool)->Self{
  let mut engine=RuntimeEngine::new();let compiled=engine.compile(APP).unwrap();
  let root_schema=engine.root_state_schema(&compiled).unwrap();let mut state=UiRuntimeState::new();
  let(send,receive)=std::sync::mpsc::channel();let id=CapabilityId::parse("app.stream").unwrap();
  state.capabilities.register_subscription(CapabilityDescriptor{id:id.clone(),version:Version::new(1,0,0),
   methods:BTreeMap::from([("watch".into(),CapabilityMethod{input:ValueSchema::integer(),output:ValueSchema::string()})])},Stream{send,fail_new,finite}).unwrap();
  let marker=CapabilityId::parse("app.marker").unwrap();let cleanups=Rc::new(RefCell::new(Vec::new()));let fail_cleanup=Rc::new(Cell::new(false));
  state.capabilities.register(CapabilityDescriptor{id:marker.clone(),version:Version::new(1,0,0),
   methods:BTreeMap::from([("cleanup".into(),CapabilityMethod{input:ValueSchema::integer(),output:ValueSchema::Null})])},Marker{calls:cleanups.clone(),fail:fail_cleanup.clone()}).unwrap();
  state.capabilities.activate(&BTreeMap::from([(id,VersionReq::STAR),(marker,VersionReq::STAR)])).unwrap();
  let runtime=Rc::new(RefCell::new(state));let root=ComponentInstancePath::root("App","root");let component=root.child("StreamProbe","stream");
  let mut lifecycle=ScriptLifecycle::new(compiled,runtime.clone(),root,Some("main".into()),BTreeMap::new(),&root_schema).unwrap();
  lifecycle.start(&mut engine).unwrap();Self{engine,lifecycle,runtime,component,receive,cleanups,fail_cleanup}
 }
 fn producer(&self)->(i64,SubscriptionEmitter){self.receive.recv_timeout(Duration::from_secs(5)).unwrap()}
 fn drain(&self)->Vec<AsyncDelivery>{self.runtime.borrow_mut().subscriptions.drain(self.lifecycle.generation())}
 fn deliver(&self,d:AsyncDelivery)->Result<(),String>{self.lifecycle.invoke_async_delivery_transactional(&self.engine,d).map(|_|()).map_err(|e|e.to_string())}
 fn render(&mut self)->Result<bool,String>{self.lifecycle.render_dirty(&mut self.engine).map_err(|e|e.to_string())}
 fn phase(&self)->Option<UiValue>{self.runtime.borrow().component_state.get(&self.component,"phase").cloned()}
 fn expect_phase(&self,s:&str){assert_eq!(self.phase(),Some(UiValue::String(s.into())));}
}
impl Drop for Harness{fn drop(&mut self){self.fail_cleanup.set(false);let _=self.lifecycle.dispose(&mut self.engine);}}

#[test]
fn old_effect_delivery_is_dropped_after_same_path_remount_but_explicit_owner_errors_remain(){
 let mut h=Harness::new(false,false);let(_,old)=h.producer();old.emit(UiValue::String("old-late".into())).unwrap();let late=h.drain().pop().unwrap();let old_callback=late.callback.clone();
 let root=h.component.parent().unwrap();h.runtime.borrow_mut().set_component_state_from_host(&root,"visible",UiValue::Bool(false)).unwrap();h.render().unwrap();
 assert_eq!(old.close_reason(),Some(SubscriptionCloseReason::ScopeDisposed));assert!(h.phase().is_none());assert_eq!(*h.cleanups.borrow(),vec![UiValue::Integer(0)]);
 h.runtime.borrow_mut().set_component_state_from_host(&root,"visible",UiValue::Bool(true)).unwrap();h.render().unwrap();let(_,new)=h.producer();h.expect_phase("idle");
 h.deliver(late).unwrap();h.expect_phase("idle");
 let explicit=h.lifecycle.invoke_callback_transactional(&h.engine,&old_callback,UiValue::String("explicit".into()));assert!(explicit.unwrap_err().to_string().contains("unmounted incarnation"));
 new.emit(UiValue::String("new-active".into())).unwrap();let fresh=h.drain().pop().unwrap();
 let mut invalid_owner=fresh.clone();invalid_owner.callback=old_callback;assert!(h.deliver(invalid_owner).unwrap_err().contains("unmounted incarnation"));h.expect_phase("idle");
 h.deliver(fresh).unwrap();h.expect_phase("new-active");
 println!("ASYNC remount_old_scope_silent_drop=true explicit_old_callback_error=true active_scope_wrong_owner_error=true new_scope_delivery=true");
}

fn root_source(default:&str)->String{
 format!(r#"fn state_schema(){{#{{fields:#{{data:#{{schema:#{{type:"ui_value"}},"default":{default}}}}}}}}} fn view(ctx){{text("root")}}"#)
}
fn formal_definition(default:&str)->String{
 format!(r#"define_component(#{{
 metadata:#{{id:"test/defaults","export":"Defaults",version:"0.1.8",runtime_api:#{{min_inclusive:2,max_exclusive:3}},dependencies:[],capabilities:#{{}}}},
 schema:#{{props:#{{key:#{{schema:#{{type:"string"}},required:true,sensitive:false}}}},state:#{{fields:#{{data:#{{schema:#{{type:"ui_value"}},"default":{default}}}}}}},events:#{{}},slots:#{{}},parts:[]}},render:Fn("render_Defaults")}});
 fn render_Defaults(ctx,props){{text("formal")}}
 "#)
}

#[test]
fn invalid_recursive_defaults_name_root_and_formal_fields(){
 for (label,default) in [
  ("map_plain_item",r##"#{type:"map",value:#{name:"ordinary"}}"##),
  ("array_plain_item",r##"#{type:"array",value:["ordinary"]}"##),
  ("nested_array_map",r##"#{type:"map",value:#{rows:#{type:"array",value:[#{type:"map",value:#{name:"ordinary"}}]}}}"##),
 ]{
  let mut engine=RuntimeEngine::new();let compiled=engine.compile(&root_source(default)).unwrap();let error=engine.root_state_schema(&compiled).unwrap_err().to_string();
  assert!(error.contains("state_schema.fields.data.default"),"{error}");assert!(error.contains("recursively tagged UiValue"),"{error}");println!("DEFAULT root {label}: {error}");
  let mut engine=RuntimeEngine::new();let source=formal_definition(default)+"fn view(){render_component(\"test/defaults\",#{key:\"one\"})}";
  let compiled=engine.compile(&source).unwrap();let error=engine.render(&compiled).unwrap_err().to_string();
  assert!(error.contains("component.schema.state.fields.data.default"),"{error}");assert!(error.contains("recursively tagged UiValue"),"{error}");println!("DEFAULT formal {label}: {error}");
 }
}

#[test]
fn valid_recursive_defaults_remain_accepted_and_preserve_values(){
 let cases=[
  (r##"#{type:"null"}"##,UiValue::Null),
  (r##"#{type:"map",value:#{}}"##,UiValue::Map(BTreeMap::new())),
  (r##"#{type:"array",value:[]}"##,UiValue::Array(vec![])),
  (r##"#{type:"array",value:[#{type:"map",value:#{name:#{type:"string",value:"accepted"},count:#{type:"integer",value:7}}}]}"##,
   UiValue::Array(vec![UiValue::Map(BTreeMap::from([("name".into(),UiValue::String("accepted".into())),("count".into(),UiValue::Integer(7))]))])),
 ];
 for (default,expected) in cases{
  let mut engine=RuntimeEngine::new();let compiled=engine.compile(&root_source(default)).unwrap();let schema=engine.root_state_schema(&compiled).unwrap();assert_eq!(schema.field("data").unwrap().default,expected);
  let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));let root=ComponentInstancePath::root("App","root");let mut lifecycle=ScriptLifecycle::new(compiled,runtime.clone(),root.clone(),None,BTreeMap::new(),&schema).unwrap();lifecycle.start(&mut engine).unwrap();assert_eq!(runtime.borrow().component_state.get(&root,"data"),Some(&expected));
  let mut engine=RuntimeEngine::new();let source=formal_definition(default)+"fn view(ctx){render_component(\"test/defaults\",#{key:\"one\"})}";let compiled=engine.compile(&source).unwrap();let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));let schema=engine.root_state_schema(&compiled).unwrap();let mut lifecycle=ScriptLifecycle::new(compiled,runtime.clone(),root.clone(),None,BTreeMap::new(),&schema).unwrap();lifecycle.start(&mut engine).unwrap();assert_eq!(runtime.borrow().component_state.get(&root.child("Defaults","one"),"data"),Some(&expected));
 }
 println!("DEFAULT tagged_null_empty_map_empty_array_nested_array_map accepted_at_root_and_formal=true");
}

#[test]
fn rejected_formal_default_rolls_back_candidate_init_and_keeps_last_good(){
 let old=r##"fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:7}}}}}
 fn inc(ctx,payload){ctx.set_state("count",ctx.get_state("count")+1);}
 fn view(ctx){text(ctx.get_state("count").to_string()).on_click(Fn("inc"))}"##;
 let mut engine=RuntimeEngine::new();let compiled=engine.compile(old).unwrap();let schema=engine.root_state_schema(&compiled).unwrap();let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));let root=ComponentInstancePath::root("App","root");let mut lifecycle=ScriptLifecycle::new(compiled,runtime.clone(),root.clone(),None,BTreeMap::new(),&schema).unwrap();lifecycle.start(&mut engine).unwrap();
 let callback=lifecycle.root().unwrap().handler("click").unwrap().as_script().unwrap().clone();let generation=lifecycle.generation();
 let mut rejected_root_engine=RuntimeEngine::new();let rejected_root=rejected_root_engine.compile(&root_source(r##"#{type:"array",value:["bad"]}"##)).unwrap();assert!(rejected_root_engine.root_state_schema(&rejected_root).unwrap_err().to_string().contains("state_schema.fields.data.default"));assert_eq!(lifecycle.generation(),generation);assert_eq!(runtime.borrow().component_state.get(&root,"count"),Some(&UiValue::Integer(7)));
 let definition=formal_definition(r##"#{type:"array",value:["bad"]}"##);let (definition,render)=definition.split_once("fn render_Defaults").unwrap();
 let candidate_source=format!("fn init(ctx){{ctx.set_state(\"count\",99);}} fn install_bad(){{{definition}}} fn render_Defaults{render} fn view(ctx){{install_bad();text(\"candidate\")}}");
 let mut candidate_engine=RuntimeEngine::new();let candidate=candidate_engine.compile(&candidate_source).unwrap();let error=lifecycle.reload(&mut candidate_engine,candidate,&schema).unwrap_err().to_string();
 assert!(error.contains("component.schema.state.fields.data.default"),"{error}");assert_eq!(lifecycle.generation(),generation);assert_eq!(runtime.borrow().component_state.get(&root,"count"),Some(&UiValue::Integer(7)));assert!(matches!(lifecycle.root().unwrap().kind(),gpui_rhai::UiNodeKind::Text{text} if text=="7"));
 let _=lifecycle.invoke_callback_transactional(&engine,&callback,UiValue::Null).unwrap();assert_eq!(runtime.borrow().component_state.get(&root,"count"),Some(&UiValue::Integer(8)));
 println!("ROLLBACK invalid_formal_default_restores_init_state_root_generation_and_old_callback=true");
}
