use std::{cell::RefCell,collections::BTreeMap,rc::Rc};
use gpui_rhai::{ComponentInstancePath,ComponentStateSchema,ExecutionPhase,NativeCollection,RuntimeEngine,ScriptLifecycle,UiContext,UiRuntimeState,UiValue};

fn collection(count:usize)->NativeCollection{
    NativeCollection::new("key",(0..count).map(|i|BTreeMap::from([("key".to_owned(),UiValue::String(format!("item-{i}")))]))).unwrap()
}
fn context(runtime:&Rc<RefCell<UiRuntimeState>>,key:&str,phase:ExecutionPhase)->UiContext{
    UiContext::new(Rc::clone(runtime),ComponentInstancePath::root("App",key),Some("main".into()),phase,BTreeMap::new())
}
fn missing_count(runtime:&Rc<RefCell<UiRuntimeState>>)->String{
    format!("{:?}",runtime.borrow().native_collections)
}
fn budget_probes(){
    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));
    let event=context(&runtime,"event",ExecutionPhase::Event);
    for i in 0..256{assert!(event.get_native_collection(&format!("event-{i}")).is_err());}
    assert!(event.get_native_collection(&"x".repeat(257)).unwrap_err().to_string().contains("invalid"));
    assert!(missing_count(&runtime).ends_with("missing_readers: {} }"));
    let render=context(&runtime,"render",ExecutionPhase::Render);
    for i in 0..64{assert!(render.get_native_collection(&format!("render-{i}")).unwrap_err().to_string().contains("not registered"));}
    let before=missing_count(&runtime);
    assert!(render.get_native_collection("render-0").unwrap_err().to_string().contains("not registered"));
    assert!(render.get_native_collection("render-overflow").unwrap_err().to_string().contains("names per component"));
    assert_eq!(before,missing_count(&runtime));
    runtime.borrow_mut().release_window("main",&ComponentInstancePath::root("App","render")).unwrap();
    let render=context(&runtime,"render",ExecutionPhase::Render);
    assert!(render.get_native_collection("after-release").unwrap_err().to_string().contains("not registered"));
    println!("event-only no tracking; name validation; per-reader quota/dedup; release reclaim: passed");

    let pairs=Rc::new(RefCell::new(UiRuntimeState::new()));
    for reader in 0..64{
        let ctx=context(&pairs,&format!("reader-{reader}"),ExecutionPhase::Render);
        for key in 0..64{assert!(ctx.get_native_collection(&format!("shared-{key}")).unwrap_err().to_string().contains("not registered"));}
    }
    let before=missing_count(&pairs);
    let extra=context(&pairs,"reader-extra",ExecutionPhase::Render);
    assert!(extra.get_native_collection("shared-0").unwrap_err().to_string().contains("component/name pairs"));
    assert_eq!(before,missing_count(&pairs));
    println!("global 4096 pair quota and no mutation on rejection: passed");

    let bytes=Rc::new(RefCell::new(UiRuntimeState::new()));
    for reader in 0..4{
        let ctx=context(&bytes,&format!("bytes-{reader}"),ExecutionPhase::Render);
        for key in 0..64{
            let prefix=format!("b{reader:02}-{key:02}-");let name=format!("{prefix}{}","x".repeat(256-prefix.len()));
            assert!(ctx.get_native_collection(&name).unwrap_err().to_string().contains("not registered"));
        }
    }
    let before=missing_count(&bytes);
    let extra=context(&bytes,"bytes-extra",ExecutionPhase::Render);
    assert!(extra.get_native_collection("next-byte").unwrap_err().to_string().contains("name bytes"));
    assert_eq!(before,missing_count(&bytes));
    println!("global 64KiB name bytes quota and no mutation on rejection: passed");

    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));
    let ctx=context(&runtime,"rollback",ExecutionPhase::Render);
    assert!(ctx.get_native_collection("prior").is_err());
    let snap=runtime.borrow_mut().begin_transaction().unwrap();
    assert!(ctx.get_native_collection("transient").is_err());
    runtime.borrow_mut().rollback_transaction(snap).unwrap();
    let root=ComponentInstancePath::root("App","rollback");
    runtime.borrow_mut().register_native_collection_from_host(&root,"transient",collection(0)).unwrap();
    assert!(runtime.borrow().dirty_components().is_empty());
    runtime.borrow_mut().register_native_collection_from_host(&root,"prior",collection(0)).unwrap();
    assert_eq!(runtime.borrow().dirty_components(),&std::collections::BTreeSet::from([root]));
    println!("negative-read transaction rollback and exact wakeup: passed");
}
fn shown(lifecycle:&ScriptLifecycle)->String{
    let root=lifecycle.root().unwrap();
    let kind=if let gpui_rhai::UiNodeKind::VirtualCollection{spec}=root.kind(){spec.realized[&0].kind()}else{root.kind()};
    format!("{kind:?}")
}
fn row_dependency_probe(virtualized:bool){
    let mut engine=RuntimeEngine::new();
    let view=if virtualized{r#"fn view(ctx){virtual_collection(#{key:"rows",label:"Rows",data:[#{key:"a"}],height:24,estimated_height:24,overdraw_pixels:0},Fn("render_row"))}"#}else{r#"fn view(ctx){render_row(ctx,())}"#};
    let script=format!(r#"fn render_row(ctx,payload){{try{{let data=ctx.get_native_collection("late");return text(`ready:${{data.len}}`);}}catch(err){{return text("missing");}}}} {view}"#);
    let compiled=engine.compile_named("audit/row-dependency.rhai",&script).unwrap();
    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));
    let root=ComponentInstancePath::root("App",if virtualized{"virtual"}else{"direct"});
    let mut lifecycle=ScriptLifecycle::new(compiled,Rc::clone(&runtime),root.clone(),Some("main".into()),BTreeMap::new(),&ComponentStateSchema::default()).unwrap();
    lifecycle.start(&mut engine).unwrap();
    println!("virtualized={virtualized} initial={}",shown(&lifecycle));
    runtime.borrow_mut().register_native_collection_from_host(&root,"late",collection(1)).unwrap();
    let rendered=lifecycle.render_dirty(&mut engine).unwrap();
    println!("virtualized={virtualized} register render_dirty={rendered} shown={}",shown(&lifecycle));
    lifecycle.render(&mut engine).unwrap();
    println!("virtualized={virtualized} forced-render shown={}",shown(&lifecycle));
    let changed=runtime.borrow_mut().replace_native_collection_from_host("late",collection(2)).unwrap();
    let rendered=lifecycle.render_dirty(&mut engine).unwrap();
    println!("virtualized={virtualized} replace changed={changed} render_dirty={rendered} shown={}",shown(&lifecycle));
}
fn manifest_probe(add_row:bool){
    let mut engine=RuntimeEngine::new();
    let source=r#"
    define_component(#{
      metadata:#{id:"audit/counter","export":"Counter",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
      schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:7}}}},events:#{},slots:#{},parts:[]},
      render:Fn("render_counter")
    });
    fn Counter(props){render_component("audit/counter",props)}
    fn read_count(ctx,payload){ctx.get_state("count")}
    fn render_counter(ctx,props){text(props.key).on_click(Fn("read_count"))}
    fn row(ctx,payload){Counter(#{key:payload.key})}
    fn view(ctx){let data=[];for i in 0..20{data.push(#{key:`row-${i}`});}
      virtual_collection(#{key:"rows",label:"Rows",data:data,height:24,estimated_height:24,overdraw_pixels:0},Fn("row"))}
    "#;
    let compiled=engine.compile_named("audit/manifest.rhai",source).unwrap();
    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));
    let root=ComponentInstancePath::root("App",if add_row{"add-row"}else{"prune-only"});
    let mut lifecycle=ScriptLifecycle::new(compiled,Rc::clone(&runtime),root.clone(),Some("main".into()),BTreeMap::new(),&ComponentStateSchema::default()).unwrap();
    lifecycle.start(&mut engine).unwrap();
    let (id,callback,observed)=if let gpui_rhai::UiNodeKind::VirtualCollection{spec}=lifecycle.root().unwrap().kind(){
      let observed=if add_row{0}else{1};
      (spec.id.clone(),spec.realized[&observed].handler("click").unwrap().as_script().unwrap().clone(),observed)
    }else{unreachable!()};
    let child=root.child("VirtualCollection","rows").child("Counter",format!("row-{observed}"));
    println!("manifest add_row={add_row} before callback={:?}, state={:?}",lifecycle.invoke_callback_transactional(&engine,&callback,UiValue::Null),runtime.borrow().component_state.get(&child,"count"));
    let target=if add_row{vec![0,10]}else{vec![0]};
    runtime.borrow().virtual_requests.request(id,target);
    lifecycle.realize_virtual_requests(&mut engine).unwrap();
    let indices=if let gpui_rhai::UiNodeKind::VirtualCollection{spec}=lifecycle.root().unwrap().kind(){spec.realized.keys().copied().collect::<Vec<_>>()}else{unreachable!()};
    println!("manifest add_row={add_row} target={indices:?}, observed_state={:?}",runtime.borrow().component_state.get(&child,"count"));
    println!("manifest add_row={add_row} after callback={:?}",lifecycle.invoke_callback_transactional(&engine,&callback,UiValue::Null));
}
fn main(){budget_probes();row_dependency_probe(false);row_dependency_probe(true);manifest_probe(true);manifest_probe(false);}
