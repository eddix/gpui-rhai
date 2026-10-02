use std::{cell::RefCell,collections::BTreeMap,rc::Rc};
use gpui_rhai::{ComponentInstancePath,ComponentStateSchema,ExecutionPhase,RuntimeEngine,ScriptLifecycle,StoreId,UiContext,UiNode,UiNodeKind,UiRuntimeState,UiValue};

fn callback(root:&UiNode,index:usize)->gpui_rhai::ScriptCallback{
    let UiNodeKind::VirtualCollection{spec}=root.kind()else{panic!("root must be virtual")};
    fn find(node:&UiNode)->Option<gpui_rhai::ScriptCallback>{
        if let Some(handler)=node.handler("click"){return handler.as_script().cloned();}
        if let UiNodeKind::Box{children}|UiNodeKind::Fragment{children}=node.kind(){return children.iter().find_map(find);}
        None
    }
    find(&spec.realized[&index]).unwrap()
}
fn wrapper_probe(mode:&str){
    let mut engine=RuntimeEngine::new();
    let script=r#"
    define_component(#{metadata:#{id:"audit/counter","export":"Counter",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
      schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:7}}}},events:#{},slots:#{},parts:[]},render:Fn("render_counter")});
    define_component(#{metadata:#{id:"audit/wrapper","export":"Wrapper",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
      schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:[]},render:Fn("render_wrapper")});
    fn Counter(props){render_component("audit/counter",props)}
    fn Wrapper(props){render_component("audit/wrapper",props)}
    fn read_count(ctx,payload){ctx.get_state("count")}
    fn render_counter(ctx,props){text(props.key).on_click(Fn("read_count"))}
    fn render_wrapper(ctx,props){WRAPPER_BODY}
    fn row(ctx,payload){if payload.index==19{throw "candidate row rejected";}ROW_BODY}
    fn view(ctx){let data=[];for i in 0..20{data.push(#{key:`row-${i}`});}
      virtual_collection(#{key:"rows",label:"Rows",data:data,height:24,estimated_height:24,overdraw_pixels:0},Fn("row"))}
    "#.replace("WRAPPER_BODY",if mode=="boxed"{r#"column([Counter(#{key:"inner"})])"#}else{r#"Counter(#{key:"inner"})"#})
       .replace("ROW_BODY",if mode=="direct"{"Counter(#{key:payload.key})"}else{"Wrapper(#{key:payload.key})"});
    let compiled=engine.compile_named("audit/wrapper.rhai",&script).unwrap();
    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));
    let root=ComponentInstancePath::root("App",mode);
    let mut lifecycle=ScriptLifecycle::new(compiled,Rc::clone(&runtime),root.clone(),Some("main".into()),BTreeMap::new(),&ComponentStateSchema::default()).unwrap();
    lifecycle.start(&mut engine).unwrap();
    let cb=callback(lifecycle.root().unwrap(),0);
    let id=if let UiNodeKind::VirtualCollection{spec}=lifecycle.root().unwrap().kind(){spec.id.clone()}else{unreachable!()};
    let child=if mode=="direct"{root.child("VirtualCollection","rows").child("Counter","row-0")}else{root.child("VirtualCollection","rows").child("Wrapper","row-0").child("Counter","inner")};
    println!("wrapper mode={mode} initial callback={:?}",lifecycle.invoke_callback_transactional(&engine,&cb,UiValue::Null));
    runtime.borrow().virtual_requests.request(id.clone(),[0,10]);
    println!("wrapper mode={mode} realize={:?}",lifecycle.realize_virtual_requests(&mut engine));
    let state=runtime.borrow().component_state.get(&child,"count").cloned();
    let result=lifecycle.invoke_callback_transactional(&engine,&cb,UiValue::Null);
    println!("wrapper mode={mode} retained row0 inner state={state:?}, callback={result:?}");
    if mode=="direct"{
        runtime.borrow().virtual_requests.request(id,[0,10,11,19]);
        let error=lifecycle.realize_virtual_requests(&mut engine).unwrap_err();
        let UiNodeKind::VirtualCollection{spec}=lifecycle.root().unwrap().kind()else{unreachable!()};
        assert_eq!(spec.realized.keys().copied().collect::<Vec<_>>(),vec![0,10]);
        let candidate=root.child("VirtualCollection","rows").child("Counter","row-11");
        assert!(runtime.borrow().component_state.get(&candidate,"count").is_none());
        assert_eq!(lifecycle.invoke_callback_transactional(&engine,&cb,UiValue::Null).unwrap().as_int().unwrap(),7);
        println!("failed candidate add11/19 rollback preserves target0/10 and callback, removes candidate11 state: passed ({error})");
    }
}

fn visible_text(lifecycle:&ScriptLifecycle)->String{
    let UiNodeKind::VirtualCollection{spec}=lifecycle.root().unwrap().kind()else{unreachable!()};
    format!("{:?}",spec.realized[&0].kind())
}
fn store_probe(kind:&str,path:bool){
    let read=match(kind,path){
        ("app",false)=>"ctx.get_app_store(\"data\",\"model\").label",
        ("app",true)=>"ctx.get_app_store_path(\"data\",\"model\",[\"label\"])",
        ("window",false)=>"ctx.get_window_store(\"data\",\"model\").label",
        ("window",true)=>"ctx.get_window_store_path(\"data\",\"model\",[\"label\"])",
        ("local",false)=>"ctx.get_state(\"model\").label",
        ("local",true)=>"ctx.get_state_path(\"model\",[\"label\"])",
        _=>unreachable!(),
    };
    let script=format!(r#"
    fn state_schema(){{#{{fields:#{{model:#{{schema:#{{type:"map",values:#{{type:"string"}}}},"default":#{{type:"map",value:#{{label:#{{type:"string",value:"before"}}}}}}}}}}}}}}
    fn row(ctx,payload){{text({read})}}
    fn view(ctx){{virtual_collection(#{{key:"rows",label:"Rows",data:[#{{key:"a"}}],height:24,estimated_height:24,overdraw_pixels:0}},Fn("row"))}}
    "#);
    let mut engine=RuntimeEngine::new();let compiled=engine.compile_named("audit/store.rhai",&script).unwrap();
    let schema=engine.root_state_schema(&compiled).unwrap();let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));
    runtime.borrow_mut().stores.declare(StoreId::app("data"),schema.clone()).unwrap();
    runtime.borrow_mut().stores.declare(StoreId::window("main","data"),schema.clone()).unwrap();
    let root=ComponentInstancePath::root("App",format!("{kind}-{path}"));
    let mut lifecycle=ScriptLifecycle::new(compiled,Rc::clone(&runtime),root.clone(),Some("main".into()),BTreeMap::new(),&schema).unwrap();
    lifecycle.start(&mut engine).unwrap();let before=visible_text(&lifecycle);
    let event=UiContext::new(Rc::clone(&runtime),root,Some("main".into()),ExecutionPhase::Event,BTreeMap::new());
    let value=UiValue::Map(BTreeMap::from([("label".into(),UiValue::String("after".into()))])).into_dynamic();
    match kind{"app"=>event.set_app_store("data","model",value).unwrap(),"window"=>event.set_window_store("data","model",value).unwrap(),"local"=>event.set_state("model",value).unwrap(),_=>unreachable!()};
    let rendered=lifecycle.render_dirty(&mut engine);
    println!("store kind={kind} path={path} before={before}, render_dirty={rendered:?}, after={}",visible_text(&lifecycle));
}

fn nested_probe(nested:bool){
    let view=if nested{r#"virtual_collection(#{key:"outer",label:"Outer",data:[#{key:"a"}],height:48,estimated_height:48,overdraw_pixels:0},Fn("outer_row"))"#}else{r#"outer_row(ctx,#{key:"a"})"#};
    let script=format!(r#"
      fn inner_row(ctx,payload){{text(payload.key)}}
      fn outer_row(ctx,payload){{let data=[];for i in 0..20{{data.push(#{{key:`inner-${{i}}`}});}}
        virtual_collection(#{{key:`inner-${{payload.key}}`,label:"Inner",data:data,height:24,estimated_height:24,overdraw_pixels:0}},Fn("inner_row"))}}
      fn view(ctx){{{view}}}
    "#);
    let mut engine=RuntimeEngine::new();let compiled=engine.compile_named("audit/nested.rhai",&script).unwrap();
    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut lifecycle=ScriptLifecycle::new(compiled,Rc::clone(&runtime),ComponentInstancePath::root("App",format!("nested-{nested}")),Some("main".into()),BTreeMap::new(),&ComponentStateSchema::default()).unwrap();
    lifecycle.start(&mut engine).unwrap();
    let UiNodeKind::VirtualCollection{spec}=lifecycle.root().unwrap().kind()else{unreachable!()};
    let spec=if nested{let UiNodeKind::VirtualCollection{spec}=spec.realized[&0].kind()else{unreachable!()};spec}else{spec};
    assert!(!spec.realized.contains_key(&10));let id=spec.id.clone();
    runtime.borrow().virtual_requests.request(id,[10]);
    println!("nested={nested} request inner row10 realization={:?}",lifecycle.realize_virtual_requests(&mut engine));
}
fn main(){for kind in ["app","window","local"]{for path in [false,true]{store_probe(kind,path);}}for mode in ["direct","bare","boxed"]{wrapper_probe(mode);}nested_probe(false);nested_probe(true);}
