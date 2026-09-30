//! New boundaries only, against 815bb82b. Round 2 suite is run by the audit coordinator.
use std::{cell::RefCell,collections::BTreeMap,rc::Rc,time::{Duration,Instant}};
use gpui_rhai::{AsyncScope,CapabilityDescriptor,CapabilityId,CapabilityMethod,
    ComponentInstancePath,RuntimeEngine,ScriptLifecycle,UiRuntimeState,UiValue,ValueSchema,
    SubscriptionCapabilityHandler,SubscriptionWork,SubscriptionRegistration,SubscriptionRegistry,
    SubscriptionEmitter,SubscriptionCloseReason,CapabilityHandler};
use semver::{Version,VersionReq};

fn callbacks()->(RuntimeEngine,gpui_rhai::CompiledUi,gpui_rhai::ScriptCallback,gpui_rhai::ScriptCallback){
    let mut engine=RuntimeEngine::new();
    let compiled=engine.compile("fn view(){text(\"audit\")} fn success(ctx,value){value} fn failure(ctx,value){value}").unwrap();
    engine.render(&compiled).unwrap();
    let success=engine.callback(&compiled,"success").unwrap();
    let failure=engine.callback(&compiled,"failure").unwrap();
    (engine,compiled,success,failure)
}
fn field<'a>(value:&'a UiValue,key:&str)->&'a UiValue{
    let UiValue::Map(values)=value else{panic!("map required")};values.get(key).unwrap()
}

#[test]
fn quota_precedes_schema_and_oversize_early_rejection_is_real(){
    let (_,compiled,success,failure)=callbacks();
    for count in [10_001,900_000]{
        let mut registry=SubscriptionRegistry::new();
        let schema=ValueSchema::Array{items:Box::new(ValueSchema::integer()),max_items:None};
        let (_,emitter)=registry.subscribe(SubscriptionRegistration::new("quota-first",AsyncScope::App,
            compiled.generation(),success.clone(),failure.clone(),schema));
        emitter.emit(UiValue::Array(vec![UiValue::Null;count])).unwrap();
        let start=Instant::now();
        let delivery=registry.drain(compiled.generation()).pop().unwrap();
        let elapsed=start.elapsed();
        assert_eq!(delivery.callback.name(),"failure");
        assert_eq!(field(&delivery.payload,"kind"),&UiValue::String("async_delivery_limit".into()));
        assert_eq!(field(&delivery.payload,"resource"),&UiValue::String("array_items".into()));
        assert_eq!(field(&delivery.payload,"actual"),&UiValue::Integer(count as i64));
        println!("EARLY items={count} elapsed_ms={:.3} kind=async_delivery_limit",elapsed.as_secs_f64()*1000.0);
    }
}

#[test]
fn rhai_admissible_map_still_generates_full_schema_diagnostics(){
    let (engine,compiled,success,failure)=callbacks();
    let count=100_000;
    let value=UiValue::Map((0..count).map(|i|(format!("field_{i:06}"),UiValue::Null)).collect());
    value.validate().unwrap();
    engine.engine().ensure_data_size_within_limits(&value.clone().into_dynamic()).unwrap();
    let schema=ValueSchema::Map{values:Box::new(ValueSchema::integer())};
    let errors=schema.validate_ui_value(&value).unwrap_err();
    assert_eq!(errors.issues.len(),count);
    let formatted=errors.to_string().len();
    let mut registry=SubscriptionRegistry::new();
    let (_,emitter)=registry.subscribe(SubscriptionRegistration::new("schema-errors",AsyncScope::App,
        compiled.generation(),success,failure,schema));
    emitter.emit(value).unwrap();
    let start=Instant::now();
    let delivery=registry.drain(compiled.generation()).pop().unwrap();
    let elapsed=start.elapsed();
    assert_eq!(delivery.callback.name(),"failure");
    let UiValue::String(message)=field(&delivery.payload,"message") else{panic!()};
    assert!(message.len()<=1027);
    println!("SCHEMA admitted_map_entries={count} issues={} intermediate_bytes={formatted} final_bytes={} drain_ms={:.3}",
        errors.issues.len(),message.len(),elapsed.as_secs_f64()*1000.0);
}

#[test]
fn independent_uivalue_and_rhai_domains_are_both_enforced(){
    let (engine,compiled,success,failure)=callbacks();
    // Keys do not count against Rhai string limits, but the durable domain counts their bytes.
    let large_key=UiValue::Map(BTreeMap::from([("x".repeat(16*1024*1024+1),UiValue::Null)]));
    engine.engine().ensure_data_size_within_limits(&large_key.clone().into_dynamic()).unwrap();
    assert!(large_key.validate().is_err());
    for (name,value) in [("durable_key_bytes",large_key),("non_finite",UiValue::Float(f64::NAN))]{
        let mut registry=SubscriptionRegistry::new();
        let (_,emitter)=registry.subscribe(SubscriptionRegistration::new(name,AsyncScope::App,
            compiled.generation(),success.clone(),failure.clone(),ValueSchema::UiValue));
        emitter.emit(value).unwrap();
        let delivery=registry.drain(compiled.generation()).pop().unwrap();
        assert_eq!(delivery.callback.name(),"failure");
        engine.engine().ensure_data_size_within_limits(&delivery.payload.clone().into_dynamic()).unwrap();
        println!("DOMAIN {name} rejected_before_callback=true");
    }
}

const STREAM_APP:&str=r#"
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
fn failed(ctx,error){ctx.set_state("phase","error");}
fn render_StreamProbe(ctx,props){effect("watch",ctx.get_state("epoch"),Fn("start_stream"),Fn("stop_stream"));text(ctx.get_state("phase"))}
fn view(ctx){render_component("test/stream",#{key:"stream"})}
"#;
struct Stream(std::sync::mpsc::Sender<(i64,SubscriptionEmitter)>);
impl SubscriptionCapabilityHandler for Stream{
 fn subscribe(&mut self,_:&str,input:UiValue)->Result<SubscriptionWork,String>{
  let UiValue::Integer(epoch)=input else{return Err("integer".into())};
  let sender=self.0.clone();
  Ok(SubscriptionWork::new(move|emitter|{
   sender.send((epoch,emitter.clone())).unwrap();
   while emitter.close_reason().is_none(){std::thread::sleep(Duration::from_millis(1));}
  }))
 }
}
struct Marker(Rc<RefCell<Vec<UiValue>>>);
impl CapabilityHandler for Marker{
 fn call(&mut self,_:&str,input:UiValue)->Result<UiValue,String>{self.0.borrow_mut().push(input);Ok(UiValue::Null)}
}

#[test]
fn drained_old_effect_delivery_survives_activation_cleanup_characterization(){
    let mut engine=RuntimeEngine::new();
    let compiled=engine.compile(STREAM_APP).unwrap();
    let root_schema=engine.root_state_schema(&compiled).unwrap();
    let mut state=UiRuntimeState::new();
    let (sender,receiver)=std::sync::mpsc::channel();
    let id=CapabilityId::parse("app.stream").unwrap();
    state.capabilities.register_subscription(CapabilityDescriptor{id:id.clone(),version:Version::new(1,0,0),
        methods:BTreeMap::from([("watch".into(),CapabilityMethod{input:ValueSchema::integer(),output:ValueSchema::string()})])},Stream(sender)).unwrap();
    let marker=CapabilityId::parse("app.marker").unwrap();
    let cleanups=Rc::new(RefCell::new(Vec::new()));
    state.capabilities.register(CapabilityDescriptor{id:marker.clone(),version:Version::new(1,0,0),
        methods:BTreeMap::from([("cleanup".into(),CapabilityMethod{input:ValueSchema::integer(),output:ValueSchema::Null})])},Marker(cleanups.clone())).unwrap();
    state.capabilities.activate(&BTreeMap::from([(id,VersionReq::STAR),(marker,VersionReq::STAR)])).unwrap();
    let runtime=Rc::new(RefCell::new(state));
    let root=ComponentInstancePath::root("App","root");
    let component=root.child("StreamProbe","stream");
    let mut lifecycle=ScriptLifecycle::new(compiled,runtime.clone(),root,Some("main".into()),BTreeMap::new(),&root_schema).unwrap();
    lifecycle.start(&mut engine).unwrap();
    let (epoch,old)=receiver.recv_timeout(Duration::from_secs(5)).unwrap();assert_eq!(epoch,0);
    old.emit(UiValue::String("replace".into())).unwrap();
    old.emit(UiValue::String("old-query-result".into())).unwrap();
    let batch=runtime.borrow_mut().subscriptions.drain(lifecycle.generation());
    assert_eq!(batch.len(),2);
    let mut batch=batch.into_iter();
    let first=batch.next().unwrap();
    println!("OLD_SCOPE {:?}",first.scope);
    let _=lifecycle.invoke_async_delivery_transactional(&engine,first).unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    let (epoch,new)=receiver.recv_timeout(Duration::from_secs(5)).unwrap();assert_eq!(epoch,1);
    assert_eq!(old.close_reason(),Some(SubscriptionCloseReason::ScopeDisposed));
    assert_eq!(*cleanups.borrow(),vec![UiValue::Integer(0)]);
    assert_eq!(runtime.borrow().component_state.get(&component,"phase"),Some(&UiValue::String("new-query".into())));
    let second=batch.next().unwrap();
    let result=lifecycle.invoke_async_delivery_transactional(&engine,second);
    let phase=runtime.borrow().component_state.get(&component,"phase").cloned();
    println!("AFTER_CLEANUP old_closed={:?} cleanup_calls={} callback_result={:?} phase={phase:?}",old.close_reason(),cleanups.borrow().len(),result.as_ref().map(|_|())) ;
    // Characterize a real defect: the already-drained old activation remains executable.
    assert!(result.is_ok());
    assert_eq!(phase,Some(UiValue::String("old-query-result".into())));
    new.close();
    lifecycle.dispose(&mut engine).unwrap();
}
