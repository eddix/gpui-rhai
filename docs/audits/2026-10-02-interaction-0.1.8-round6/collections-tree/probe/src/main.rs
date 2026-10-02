use std::{cell::RefCell,collections::BTreeMap,rc::Rc};
use gpui_rhai::{ComponentInstancePath,ComponentStateSchema,RuntimeEngine,ScriptLifecycle,UiNode,UiNodeKind,UiRuntimeState,UiValue,VirtualCollectionNodeSpec};
const COUNTER:&str=r#"
define_component(#{metadata:#{id:"audit/counter","export":"Counter",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:7}}}},events:#{},slots:#{},parts:[]},render:Fn("render_counter")});
fn Counter(props){render_component("audit/counter",props)}
fn read_count(ctx,payload){ctx.get_state("count")}
fn render_counter(ctx,props){text(props.key).on_click(Fn("read_count"))}
fn inner_row(ctx,payload){Counter(#{key:payload.key})}
fn data(){let values=[];for i in 0..20{values.push(#{key:`row-${i}`});}values}
fn outer_row(ctx,payload){virtual_collection(#{key:`inner-${payload.key}`,label:"Inner",data:data(),height:24,estimated_height:24,overdraw_pixels:0},Fn("inner_row"))}
"#;
fn collection<'a>(node:&'a UiNode,key:&str)->Option<&'a VirtualCollectionNodeSpec>{
    match node.kind(){
        UiNodeKind::VirtualCollection{spec}=>if spec.id.key==key{Some(spec)}else{spec.realized.values().find_map(|n|collection(n,key))},
        UiNodeKind::Box{children}|UiNodeKind::Fragment{children}=>children.iter().find_map(|n|collection(n,key)),
        _=>None,
    }
}
fn setup(script:&str,key:&str)->(RuntimeEngine,Rc<RefCell<UiRuntimeState>>,ScriptLifecycle,ComponentInstancePath){
    let mut engine=RuntimeEngine::new();let compiled=engine.compile_named("audit/round6.rhai",script).unwrap();
    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));let root=ComponentInstancePath::root("App",key);
    let mut lifecycle=ScriptLifecycle::new(compiled,Rc::clone(&runtime),root.clone(),Some("main".into()),BTreeMap::new(),&ComponentStateSchema::default()).unwrap();
    lifecycle.start(&mut engine).unwrap();(engine,runtime,lifecycle,root)
}
fn nested_batch(batch:bool){
    let script=format!(r#"{COUNTER}
      fn view(ctx){{virtual_collection(#{{key:"outer",label:"Outer",data:data(),height:24,estimated_height:24,overdraw_pixels:0}},Fn("outer_row"))}}
    "#);
    let (mut engine,runtime,mut lifecycle,root)=setup(&script,if batch{"nested-batch"}else{"nested-sequential"});
    let outer=collection(lifecycle.root().unwrap(),"outer").unwrap().id.clone();
    let inner=collection(lifecycle.root().unwrap(),"inner-row-0").unwrap().id.clone();
    runtime.borrow().virtual_requests.request(outer,[0,10]);
    if !batch{lifecycle.realize_virtual_requests(&mut engine).unwrap();}
    runtime.borrow().virtual_requests.request(inner,[10]);
    let result=lifecycle.realize_virtual_requests(&mut engine);
    let spec=collection(lifecycle.root().unwrap(),"inner-row-0").unwrap();
    let cb=spec.realized[&10].handler("click").unwrap().as_script().unwrap().clone();
    let path=root.child("VirtualCollection","outer").child("VirtualCollection","inner-row-0").child("Counter","row-10");
    let state=runtime.borrow().component_state.get(&path,"count").cloned();
    println!("nested batch={batch}: realization={result:?}, new inner state={state:?}, self callback={:?}",lifecycle.invoke_callback_transactional(&engine,&cb,UiValue::Null));
}
fn sibling_batch(batch:bool){
    let script=format!(r#"{COUNTER}
      fn view(ctx){{column([
        virtual_collection(#{{key:"a",label:"A",data:data(),height:24,estimated_height:24,overdraw_pixels:0}},Fn("inner_row")),
        virtual_collection(#{{key:"b",label:"B",data:data(),height:24,estimated_height:24,overdraw_pixels:0}},Fn("inner_row"))
      ])}}
    "#);
    let (mut engine,runtime,mut lifecycle,root)=setup(&script,if batch{"sibling-batch"}else{"sibling-sequential"});
    let a=collection(lifecycle.root().unwrap(),"a").unwrap().id.clone();
    let b=collection(lifecycle.root().unwrap(),"b").unwrap().id.clone();
    let path=root.child("VirtualCollection","a").child("Counter","row-0");
    runtime.borrow_mut().set_component_state_from_host(&path,"count",UiValue::Integer(9)).unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    runtime.borrow().virtual_requests.request(a.clone(),[10]);
    if !batch{lifecycle.realize_virtual_requests(&mut engine).unwrap();}
    runtime.borrow().virtual_requests.request(b,[10]);
    let result=lifecycle.realize_virtual_requests(&mut engine);
    let after_prune=runtime.borrow().component_state.get(&path,"count").cloned();
    runtime.borrow().virtual_requests.request(a,[0]);
    lifecycle.realize_virtual_requests(&mut engine).unwrap();
    let after_remount=runtime.borrow().component_state.get(&path,"count").cloned();
    println!("siblings batch={batch}: realization={result:?}, removed row0 state={after_prune:?}, remounted row0 state={after_remount:?}");
}
fn row_dependency_retention(formal:bool){
    let script=r#"
      define_component(#{metadata:#{id:"audit/loader","export":"Loader",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
        schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:[]},render:Fn("render_loader")});
      fn load(ctx,key){try{let value=ctx.get_native_collection(key);return text("ready");}catch(err){return text(err.to_string());}}
      fn render_loader(ctx,props){load(ctx,props.key)}
      fn row(ctx,payload){ROW_BODY}
      fn view(ctx){let values=[];for i in 0..66{values.push(#{key:`source-${i}`});}
        virtual_collection(#{key:"rows",label:"Rows",data:values,height:24,estimated_height:24,overdraw_pixels:0},Fn("row"))}
    "#.replace("ROW_BODY",if formal{r#"render_component("audit/loader",#{key:payload.key})"#}else{r#"load(ctx,payload.key)"#});
    let (mut engine,runtime,mut lifecycle,root)=setup(&script,"missing-scroll");
    let id=collection(lifecycle.root().unwrap(),"rows").unwrap().id.clone();
    for i in 2..66{
        runtime.borrow().virtual_requests.request(id.clone(),[i]);
        lifecycle.realize_virtual_requests(&mut engine).unwrap();
        if i==2||i==63||i==65{
            let spec=collection(lifecycle.root().unwrap(),"rows").unwrap();
            let registry=format!("{:?}",runtime.borrow().native_collections);
            let names=(0..66).filter(|key|registry.contains(&format!("\"source-{key}\": {{"))).count();
            println!("row formal={formal} scroll index={i}, realized={}, registered negative names={names}, display={:?}",spec.realized.len(),spec.realized[&i].kind());
        }
    }
    let ready=gpui_rhai::NativeCollection::new("id",std::iter::empty::<BTreeMap<String,UiValue>>()).unwrap();
    runtime.borrow_mut().register_native_collection_from_host(&root,"source-65",ready).unwrap();
    let rendered=lifecycle.render_dirty(&mut engine);
    let spec=collection(lifecycle.root().unwrap(),"rows").unwrap();
    println!("row formal={formal} register currently visible source-65: render_dirty={rendered:?}, display={:?}",spec.realized[&65].kind());
}
fn main(){nested_batch(false);nested_batch(true);sibling_batch(false);sibling_batch(true);row_dependency_retention(false);row_dependency_retention(true);}
