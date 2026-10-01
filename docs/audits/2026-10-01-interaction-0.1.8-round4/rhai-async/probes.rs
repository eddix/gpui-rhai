//! Local bounded acceptance of effect-activation delivery at f6e936a5.
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
fn replaced_effect_discards_old_success_and_error_while_new_activation_delivers(){
 let mut h=Harness::new(false,false);let(epoch,old)=h.producer();assert_eq!(epoch,0);
 old.emit(UiValue::String("replace".into())).unwrap();old.emit(UiValue::String("old-success".into())).unwrap();old.emit_error("old-error").unwrap();
 let mut batch=h.drain().into_iter();let first=batch.next().unwrap();let old_scope=first.scope.clone();h.deliver(first).unwrap();h.render().unwrap();
 let(epoch,new)=h.producer();assert_eq!(epoch,1);assert_eq!(old.close_reason(),Some(SubscriptionCloseReason::ScopeDisposed));
 for d in batch{assert_eq!(d.scope,old_scope);h.deliver(d).unwrap();h.expect_phase("new-query");}
 new.emit(UiValue::String("fresh-success".into())).unwrap();let fresh=h.drain().pop().unwrap();assert_ne!(fresh.scope,old_scope);h.deliver(fresh).unwrap();h.expect_phase("fresh-success");
 println!("REPLACEMENT old_success_and_error_ignored=true fresh_delivery_accepted=true");
}

#[test]
fn normal_work_return_and_same_activation_rerender_do_not_discard_valid_messages(){
 let h=Harness::new(false,true);let(_,emitter)=h.producer();
 let until=Instant::now()+Duration::from_secs(5);while emitter.close_reason().is_none(){assert!(Instant::now()<until);std::thread::sleep(Duration::from_millis(1));}
 assert_eq!(emitter.close_reason(),Some(SubscriptionCloseReason::WorkReturned));let batch=h.drain();assert_eq!(batch.len(),1);assert_eq!(h.runtime.borrow().subscriptions.active_count(),0);
 h.deliver(batch.into_iter().next().unwrap()).unwrap();h.expect_phase("finite-value");
 drop(h);
 let mut h=Harness::new(false,false);let(_,emitter)=h.producer();emitter.emit(UiValue::String("first".into())).unwrap();emitter.emit(UiValue::String("second".into())).unwrap();let mut batch=h.drain().into_iter();
 h.deliver(batch.next().unwrap()).unwrap();h.render().unwrap();assert!(h.cleanups.borrow().is_empty());h.deliver(batch.next().unwrap()).unwrap();h.expect_phase("second");
 println!("VALID normal_work_return_and_same_activation_rerender_accepted=true");
}

#[test]
fn failed_replacement_restores_old_activation_delivery_eligibility(){
 let mut h=Harness::new(true,false);let(_,old)=h.producer();old.emit(UiValue::String("replace".into())).unwrap();old.emit(UiValue::String("old-still-valid".into())).unwrap();let mut batch=h.drain().into_iter();
 h.deliver(batch.next().unwrap()).unwrap();let error=h.render().unwrap_err();assert!(error.contains("startup rejected"));assert_eq!(old.close_reason(),None);
 h.deliver(batch.next().unwrap()).unwrap();h.expect_phase("old-still-valid");
 println!("ROLLBACK replacement_failure_retains_old_scope=true");
}

#[test]
fn failed_suspend_keeps_eligibility_successful_suspend_and_resume_replace_it(){
 let mut h=Harness::new(false,false);let(_,old)=h.producer();old.emit(UiValue::String("after-failed-suspend".into())).unwrap();let first=h.drain().pop().unwrap();
 h.fail_cleanup.set(true);assert!(h.lifecycle.suspend(&mut h.engine).is_err());assert_eq!(old.close_reason(),None);h.deliver(first).unwrap();h.expect_phase("after-failed-suspend");
 h.fail_cleanup.set(false);old.emit(UiValue::String("must-not-run".into())).unwrap();let late=h.drain().pop().unwrap();let old_scope=late.scope.clone();assert!(h.lifecycle.suspend(&mut h.engine).unwrap());assert_eq!(old.close_reason(),Some(SubscriptionCloseReason::ScopeDisposed));
 h.deliver(late).unwrap();h.expect_phase("after-failed-suspend");assert!(h.lifecycle.resume(&mut h.engine).unwrap());let(_,new)=h.producer();new.emit(UiValue::String("after-resume".into())).unwrap();let delivery=h.drain().pop().unwrap();assert_ne!(delivery.scope,old_scope);h.deliver(delivery).unwrap();h.expect_phase("after-resume");
 println!("SUSPEND failed_cleanup_compensated=true successful_suspend_discards_old=true resume_fresh_accepted=true");
}

#[test]
fn unmounted_effect_delivery_reports_owner_error_before_scope_drop_characterization(){
 let mut h=Harness::new(false,false);let(_,old)=h.producer();old.emit(UiValue::String("late".into())).unwrap();let late=h.drain().pop().unwrap();
 let explicit_callback=late.callback.clone();
 let gpui_rhai::UiNodeKind::Box {children}=h.lifecycle.root().unwrap().kind() else{panic!("column box")};
 let hide=children[0].handler("click").unwrap().as_script().unwrap().clone();let _=h.lifecycle.invoke_callback_transactional(&h.engine,&hide,UiValue::Null).unwrap();h.render().unwrap();
 assert_eq!(old.close_reason(),Some(SubscriptionCloseReason::ScopeDisposed));assert!(h.phase().is_none());let result=h.deliver(late);
 println!("UNMOUNT old_scope_closed={:?} delivery_result={result:?}",old.close_reason());
 // Characterization, not acceptance: expected behavior is a normal silent discard.
 assert!(result.unwrap_err().contains("unmounted incarnation"));
 let explicit=h.lifecycle.invoke_callback_transactional(&h.engine,&explicit_callback,UiValue::String("explicit-old-callback".into()));
 println!("CONTROL explicit_old_callback={:?}",explicit.as_ref().map(|_|()));
 assert!(explicit.unwrap_err().to_string().contains("unmounted incarnation"));
}
