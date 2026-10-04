use std::{cell::RefCell,collections::{BTreeMap,BTreeSet},rc::Rc};
use gpui::{AnyElement,App,IntoElement,Window,div};
use gpui_rhai::{ComponentInstancePath,ComponentStateSchema,ExecutionPhase,ObjectField,PrimitiveContext,PrimitiveDescriptor,PrimitiveHandler,PrimitiveId,PrimitiveInstance,PrimitiveTheme,PrimitiveValue,RuntimeEngine,ScriptLifecycle,ThemeManager,ThemeSelection,UiContext,UiNode,UiNodeKind,UiRuntimeState,UiValue,ValueSchema,VirtualCollectionNodeSpec};

struct Slot;
impl PrimitiveHandler for Slot{
    fn render(&mut self,_:&PrimitiveInstance,_:&PrimitiveContext,_:&PrimitiveTheme,_:&mut Window,_:&mut App)->Result<AnyElement,String>{Ok(div().into_any_element())}
}
fn collection(node:&UiNode)->Option<&VirtualCollectionNodeSpec>{
    match node.kind(){
        UiNodeKind::VirtualCollection{spec}=>Some(spec),
        UiNodeKind::Custom{primitive}=>primitive.props.iter().find_map(|(_,v)|match v{
            PrimitiveValue::Node(n)=>collection(n),PrimitiveValue::Nodes(ns)=>ns.iter().find_map(collection),_=>None,
        }),
        UiNodeKind::Box{children}|UiNodeKind::Fragment{children}=>children.iter().find_map(collection),
        _=>None,
    }
}
fn case(nodes:bool,formal:bool){
    let mut engine=RuntimeEngine::new();
    engine.register_primitive(PrimitiveDescriptor{
        id:PrimitiveId::parse("audit.slot").unwrap(),export:"Slot".into(),
        props:BTreeMap::from([
            ("child".into(),ObjectField::required(ValueSchema::Node)),
            ("children".into(),ObjectField::required(ValueSchema::Array{items:Box::new(ValueSchema::Node),max_items:Some(4)})),
        ]),events:BTreeMap::new(),state:ComponentStateSchema::default(),lifecycle:false,effect:None,
    },Slot).unwrap();
    let script=r#"
      define_component(#{metadata:#{id:"audit/counter","export":"Counter",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
        schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:7}}}},events:#{},slots:#{},parts:[]},render:Fn("render_counter")});
      fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:7}}}}}
      fn read_count(ctx,value){ctx.get_state("count")}
      fn content(ctx,key){let info=ctx.theme_variant();text(info.name).with_key(key).on_click(Fn("read_count"))}
      fn render_counter(ctx,props){content(ctx,props.key)}
      fn row(ctx,payload){if payload.index==19{let ignored=ctx.theme_variant();throw "audit rejected row";}ROW_BODY}
      fn view(ctx){let data=[];for i in 0..20{data.push(#{key:`row-${i}`});}
        let list=virtual_collection(#{key:"rows",label:"Rows",data:data,height:24,estimated_height:24,overdraw_pixels:0},Fn("row"));
        audit::Slot(#{child:CHILD,children:CHILDREN}).with_key("slot")}
    "#.replace("ROW_BODY",if formal{r#"render_component("audit/counter",#{key:payload.key})"#}else{r#"content(ctx,payload.key)"#})
      .replace("CHILDREN",if nodes{"[list]"}else{"[]"}).replace("CHILD",if nodes{r#"text("prefix")"#}else{"list"});
    let compiled=engine.compile_named("audit/slot-theme.rhai",&script).unwrap();
    let schema=engine.root_state_schema(&compiled).unwrap();
    let dark=gpui_rhai::load_theme_source(engine.engine(),"dark",include_str!("../../../../../../registry/themes/default_dark.rhai")).unwrap();
    let light=gpui_rhai::load_theme_source(engine.engine(),"light",include_str!("../../../../../../registry/themes/default_light.rhai")).unwrap();
    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));
    runtime.borrow_mut().theme=Some(ThemeManager::from_variants([dark,light],ThemeSelection::new("Default","Dark")).unwrap());
    let root=ComponentInstancePath::root("App",format!("slot-{nodes}-{formal}"));
    let mut life=ScriptLifecycle::new(compiled,Rc::clone(&runtime),root.clone(),Some("main".into()),BTreeMap::new(),&schema).unwrap();
    life.start(&mut engine).unwrap();
    let id=collection(life.root().unwrap()).unwrap().id.clone();
    runtime.borrow().virtual_requests.request(id.clone(),[10]);life.realize_virtual_requests(&mut engine).unwrap();
    let cb=collection(life.root().unwrap()).unwrap().realized[&10].handler("click").unwrap().as_script().unwrap().clone();
    assert_eq!(life.invoke_callback_transactional(&engine,&cb,UiValue::Null).unwrap().as_int().unwrap(),7);
    assert!(matches!(collection(life.root().unwrap()).unwrap().realized[&10].kind(),UiNodeKind::Text{text} if text=="Dark"));
    // An Event read must not add a direct root contribution alongside the row.
    let event=UiContext::new(Rc::clone(&runtime),root.clone(),Some("main".into()),ExecutionPhase::Event,BTreeMap::new());
    assert_eq!(event.resolved_theme_variant().unwrap().name,"Dark");
    runtime.borrow_mut().select_theme_from_host("Default","Light").unwrap();
    let expected=if formal{root.child("VirtualCollection","rows").child("Counter","row-10")}else{root.clone()};
    assert_eq!(runtime.borrow().dirty_components(),&BTreeSet::from([expected]));
    assert!(life.render_dirty(&mut engine).unwrap());
    assert!(matches!(collection(life.root().unwrap()).unwrap().realized[&10].kind(),UiNodeKind::Text{text} if text=="Light"));
    runtime.borrow().virtual_requests.request(id.clone(),[]);life.realize_virtual_requests(&mut engine).unwrap();
    assert!(collection(life.root().unwrap()).unwrap().realized.is_empty());
    if formal{assert!(life.invoke_callback_transactional(&engine,&cb,UiValue::Null).is_err());}
    assert_eq!(event.resolved_theme_selection(),Some(ThemeSelection::new("Default","Light")));
    runtime.borrow_mut().select_theme_from_host("Default","Dark").unwrap();
    assert!(runtime.borrow().dirty_components().is_empty());assert!(!life.render_dirty(&mut engine).unwrap());
    runtime.borrow().virtual_requests.request(id,[19]);assert!(life.realize_virtual_requests(&mut engine).is_err());
    assert!(collection(life.root().unwrap()).unwrap().realized.is_empty());
    runtime.borrow_mut().select_theme_from_host("Default","Light").unwrap();
    assert!(runtime.borrow().dirty_components().is_empty());
    println!("slot_nodes={nodes} formal_row={formal}: delayed row/state/callback, exact live theme owner, Event read isolation, prune and failed-read rollback passed");
}
fn retained_raw_ref(formal:bool,retain_old:bool){
    let mut engine=RuntimeEngine::new();
    let script=r#"
      define_component(#{metadata:#{id:"audit/refrow","export":"RefRow",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
        schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:[]},render:Fn("render_refrow")});
      fn content(key){text(key).with_key(key).with_ref(element_ref(key))}
      fn render_refrow(ctx,props){content(props.key)}
      fn row(ctx,payload){ROW_BODY}
      fn view(ctx){let data=[];for i in 0..20{data.push(#{key:`row-${i}`});}
        virtual_collection(#{key:"refs",label:"Refs",data:data,height:24,estimated_height:24,overdraw_pixels:0},Fn("row"))}
    "#.replace("ROW_BODY",if formal{r#"render_component("audit/refrow",#{key:payload.key})"#}else{r#"content(payload.key)"#});
    let compiled=engine.compile_named("audit/retained-raw-ref.rhai",&script).unwrap();
    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));
    let mut life=ScriptLifecycle::new(compiled,Rc::clone(&runtime),ComponentInstancePath::root("App",format!("ref-{formal}-{retain_old}")),Some("main".into()),BTreeMap::new(),&ComponentStateSchema::default()).unwrap();
    life.start(&mut engine).unwrap();
    let id=collection(life.root().unwrap()).unwrap().id.clone();
    let target=if retain_old{vec![0,10]}else{vec![10]};
    runtime.borrow().virtual_requests.request(id,target);
    let result=life.realize_virtual_requests(&mut engine);
    println!("refs formal={formal} retain_old={retain_old}: realization={result:?}, committed_indices={:?}",collection(life.root().unwrap()).unwrap().realized.keys().collect::<Vec<_>>());
}
fn main(){for nodes in [false,true]{for formal in [false,true]{case(nodes,formal);}}retained_raw_ref(false,false);retained_raw_ref(false,true);retained_raw_ref(true,true);}
