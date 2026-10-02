use std::{cell::RefCell,collections::BTreeMap,rc::Rc};
use gpui_rhai::{ComponentInstancePath,RuntimeEngine,ScriptLifecycle,UiNode,UiNodeKind,UiRuntimeState,UiValue,VirtualCollectionNodeSpec};
const COUNTER:&str=r#"
define_component(#{metadata:#{id:"audit/counter","export":"Counter",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:7}}}},events:#{},slots:#{},parts:[]},render:Fn("render_counter")});
fn Counter(props){render_component("audit/counter",props)}
fn read_count(ctx,payload){ctx.get_state("count")}
fn render_counter(ctx,props){text(props.key).on_click(Fn("read_count"))}
fn state_schema(){#{fields:#{fail:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
fn inner_row(ctx,payload){if payload.index==19&&ctx.get_state("fail"){throw "candidate rejected";}Counter(#{key:payload.key})}
fn data(){let values=[];for i in 0..20{values.push(#{key:`row-${i}`});}values}
fn outer_row(ctx,payload){virtual_collection(#{key:`inner-${payload.key}`,label:"Inner",data:data(),height:24,estimated_height:24,overdraw_pixels:0},Fn("inner_row"))}
"#;
const NESTED_VIEW:&str=r#"fn view(ctx){virtual_collection(#{key:"outer",label:"Outer",data:data(),height:24,estimated_height:24,overdraw_pixels:0},Fn("outer_row"))}"#;
fn collection<'a>(node:&'a UiNode,key:&str)->Option<&'a VirtualCollectionNodeSpec>{
    match node.kind(){
        UiNodeKind::VirtualCollection{spec}=>if spec.id.key==key{Some(spec)}else{spec.realized.values().find_map(|n|collection(n,key))},
        UiNodeKind::Box{children}|UiNodeKind::Fragment{children}=>children.iter().find_map(|n|collection(n,key)),
        _=>None,
    }
}
fn setup(script:&str,key:&str)->(RuntimeEngine,Rc<RefCell<UiRuntimeState>>,ScriptLifecycle,ComponentInstancePath){
    let mut engine=RuntimeEngine::new();let compiled=engine.compile_named("audit/round7.rhai",script).unwrap();
    let schema=engine.root_state_schema(&compiled).unwrap();
    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));let root=ComponentInstancePath::root("App",key);
    let mut lifecycle=ScriptLifecycle::new(compiled,Rc::clone(&runtime),root.clone(),Some("main".into()),BTreeMap::new(),&schema).unwrap();
    lifecycle.start(&mut engine).unwrap();(engine,runtime,lifecycle,root)
}
fn callback(lifecycle:&ScriptLifecycle,key:&str,index:usize)->gpui_rhai::ScriptCallback{
    collection(lifecycle.root().unwrap(),key).unwrap().realized[&index].handler("click").unwrap().as_script().unwrap().clone()
}
fn assert_live(lifecycle:&ScriptLifecycle,engine:&RuntimeEngine,key:&str,index:usize){
    let cb=callback(lifecycle,key,index);
    assert_eq!(lifecycle.invoke_callback_transactional(engine,&cb,UiValue::Null).unwrap().as_int().unwrap(),7);
}
fn nested_order(mode:u8){
    let (mut engine,runtime,mut lifecycle,_root)=setup(&format!("{COUNTER}{NESTED_VIEW}"),&format!("nested-{mode}"));
    let outer=collection(lifecycle.root().unwrap(),"outer").unwrap().id.clone();
    let inner=collection(lifecycle.root().unwrap(),"inner-row-0").unwrap().id.clone();
    if mode==1{
        runtime.borrow().virtual_requests.request(inner.clone(),[10]);
        lifecycle.realize_virtual_requests(&mut engine).unwrap();
        runtime.borrow().virtual_requests.request(outer,[0,10]);
    }else{
        runtime.borrow().virtual_requests.request(outer,[0,10]);
        if mode==0{lifecycle.realize_virtual_requests(&mut engine).unwrap();}
        runtime.borrow().virtual_requests.request(inner,[10]);
    }
    lifecycle.realize_virtual_requests(&mut engine).unwrap();
    assert_live(&lifecycle,&engine,"inner-row-0",10);
    println!("nested mode={mode} (0 parent-first, 1 child-first, 2 batched): live self callback=7");
}
fn sibling_order(mode:u8){
    let script=format!(r#"{COUNTER} fn view(ctx){{column([
      virtual_collection(#{{key:"a",label:"A",data:data(),height:24,estimated_height:24,overdraw_pixels:0}},Fn("inner_row")),
      virtual_collection(#{{key:"b",label:"B",data:data(),height:24,estimated_height:24,overdraw_pixels:0}},Fn("inner_row"))])}}"#);
    let (mut engine,runtime,mut lifecycle,root)=setup(&script,&format!("siblings-{mode}"));
    let a=collection(lifecycle.root().unwrap(),"a").unwrap().id.clone();let b=collection(lifecycle.root().unwrap(),"b").unwrap().id.clone();
    let pa=root.child("VirtualCollection","a").child("Counter","row-0");let pb=root.child("VirtualCollection","b").child("Counter","row-0");
    runtime.borrow_mut().set_component_state_from_host(&pa,"count",UiValue::Integer(9)).unwrap();
    runtime.borrow_mut().set_component_state_from_host(&pb,"count",UiValue::Integer(11)).unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    let (first,second)=if mode==1{(b.clone(),a.clone())}else{(a.clone(),b.clone())};
    runtime.borrow().virtual_requests.request(first,[10]);if mode!=2{lifecycle.realize_virtual_requests(&mut engine).unwrap();}
    runtime.borrow().virtual_requests.request(second,[10]);lifecycle.realize_virtual_requests(&mut engine).unwrap();
    assert!(runtime.borrow().component_state.get(&pa,"count").is_none());assert!(runtime.borrow().component_state.get(&pb,"count").is_none());
    runtime.borrow().virtual_requests.request(a,[0]);runtime.borrow().virtual_requests.request(b,[0]);lifecycle.realize_virtual_requests(&mut engine).unwrap();
    assert_eq!(runtime.borrow().component_state.get(&pa,"count"),Some(&UiValue::Integer(7)));assert_eq!(runtime.borrow().component_state.get(&pb,"count"),Some(&UiValue::Integer(7)));
    assert_live(&lifecycle,&engine,"a",0);assert_live(&lifecycle,&engine,"b",0);
    println!("siblings mode={mode}: both pruned states absent, both remount defaults=7");
}
fn repeated_nested_and_rollback(){
    let (mut engine,runtime,mut lifecycle,root)=setup(&format!("{COUNTER}{NESTED_VIEW}"),"nested-transitions");
    let outer=collection(lifecycle.root().unwrap(),"outer").unwrap().id.clone();let inner=collection(lifecycle.root().unwrap(),"inner-row-0").unwrap().id.clone();
    for (sibling,children) in [(10,vec![10]),(11,vec![10,12]),(12,vec![12])]{
        runtime.borrow().virtual_requests.request(outer.clone(),[0,sibling]);runtime.borrow().virtual_requests.request(inner.clone(),children.clone());lifecycle.realize_virtual_requests(&mut engine).unwrap();
        for index in children{assert_live(&lifecycle,&engine,"inner-row-0",index);}
    }
    let surviving=callback(&lifecycle,"inner-row-0",12);
    runtime.borrow_mut().set_component_state_from_host(&root,"fail",UiValue::Bool(true)).unwrap();lifecycle.render_dirty(&mut engine).unwrap();
    // A root rerender may add the reported viewport's seed rows. Compare
    // rollback with this actual last-good snapshot, not a guessed target.
    let outer_before=collection(lifecycle.root().unwrap(),"outer").unwrap().realized.keys().copied().collect::<Vec<_>>();
    let inner_before=collection(lifecycle.root().unwrap(),"inner-row-0").unwrap().realized.keys().copied().collect::<Vec<_>>();
    runtime.borrow().virtual_requests.request(outer.clone(),[0,13]);runtime.borrow().virtual_requests.request(inner.clone(),[12,19]);
    let error=lifecycle.realize_virtual_requests(&mut engine).unwrap_err();
    let outer_now=collection(lifecycle.root().unwrap(),"outer").unwrap().realized.keys().copied().collect::<Vec<_>>();let inner_now=collection(lifecycle.root().unwrap(),"inner-row-0").unwrap().realized.keys().copied().collect::<Vec<_>>();
    assert_eq!(outer_now,outer_before);assert_eq!(inner_now,inner_before);
    assert_eq!(lifecycle.invoke_callback_transactional(&engine,&surviving,UiValue::Null).unwrap().as_int().unwrap(),7);
    let candidate=root.child("VirtualCollection","outer").child("VirtualCollection","inner-row-13");
    assert!(!runtime.borrow().component_state.paths().iter().any(|p|p.is_within(&candidate)));
    println!("repeated overlapping batches + candidate rollback: old target/callback retained, candidate branch13 state absent ({error})");
    runtime.borrow_mut().set_component_state_from_host(&root,"fail",UiValue::Bool(false)).unwrap();lifecycle.render_dirty(&mut engine).unwrap();
    lifecycle.realize_virtual_requests(&mut engine).unwrap();assert_live(&lifecycle,&engine,"inner-row-0",19);
    let doomed=callback(&lifecycle,"inner-row-0",19);
    runtime.borrow().virtual_requests.request(outer,[13]);runtime.borrow().virtual_requests.request(inner,[18]);lifecycle.realize_virtual_requests(&mut engine).unwrap();
    assert!(collection(lifecycle.root().unwrap(),"inner-row-0").is_none());
    assert!(lifecycle.invoke_callback_transactional(&engine,&doomed,UiValue::Null).is_err());
    println!("retry after candidate failure and simultaneous parent prune/obsolete child request: passed");
}
fn main(){for mode in 0..3{nested_order(mode);sibling_order(mode);}repeated_nested_and_rollback();}
