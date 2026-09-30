use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Instant};
use gpui_rhai::{ComponentInstancePath, ComponentStateSchema, EmbeddedScriptSource, ModuleId, RestrictedModuleResolver, RuntimeEngine, ScriptLifecycle, UiRuntimeState, UiValue};

fn item(key: String, parent: Option<String>) -> rhai::Dynamic {
    UiValue::Map(BTreeMap::from([
        ("label".to_owned(), UiValue::String(key.clone())),
        ("key".to_owned(), UiValue::String(key)),
        ("parent".to_owned(), parent.map_or(UiValue::Null, UiValue::String)),
    ])).into_dynamic()
}

fn tree_probe(label: &str, items: &str, expanded: &str, active: &str) {
    let module = ModuleId::parse("components/tree").unwrap();
    let source = EmbeddedScriptSource::new(BTreeMap::from([(module, include_str!("../../../../../../registry/components/tree.rhai").to_owned())]));
    let mut engine = RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let script = format!(r#"
      import "components/tree" as tree;
      fn view(ctx) {{ tree::Tree(#{{key:"outline",label:"Outline",items:{items},expanded:{expanded},active_key:{active},selected_keys:[],height:300}}) }}
    "#);
    let compiled = engine.compile_self_contained_named("audit/tree.rhai", &script).unwrap();
    let mut lifecycle = ScriptLifecycle::new(compiled, Rc::new(RefCell::new(UiRuntimeState::new())), ComponentInstancePath::root("App", "root"), Some("main".to_owned()), BTreeMap::new(), &ComponentStateSchema::default()).unwrap();
    match lifecycle.start(&mut engine) {
        Ok(root) => {
            println!("tree {label} mounted");
            for key in ["up", "down", "left", "right", "home", "end", "enter"] {
                println!(" key {key}: {:?}", root.handler_payload(&format!("key:{key}")));
            }
            println!(" handlers: {:?}", root.handlers().keys().collect::<Vec<_>>());
        }
        Err(err) => println!("tree {label} ERROR: {err}; timing={:?}",engine.last_failed_timing()),
    }
}

fn main() {
    projection_probe();
    live_register_probe();
    tree_probe("no active", r#"[#{key:"a",label:"A"},#{key:"b",label:"B"}]"#, "[]", "()");
    tree_probe("empty", "[]", "[]", "()");
    tree_probe("collapsed active child", r#"[#{key:"parent",label:"Parent"},#{key:"child",label:"Child",parent:"parent"},#{key:"next",label:"Next"}]"#, "[]", "\"child\"");
    tree_probe("disabled parent", r#"[#{key:"parent",label:"Parent",disabled:true},#{key:"child",label:"Child",parent:"parent"},#{key:"next",label:"Next"}]"#, "[\"parent\"]", "\"child\"");
    tree_probe("after accepting disabled parent", r#"[#{key:"parent",label:"Parent",disabled:true},#{key:"child",label:"Child",parent:"parent"},#{key:"next",label:"Next"}]"#, "[\"parent\"]", "\"parent\"");
    tree_probe("deleted expanded", r#"[#{key:"a",label:"A"}]"#, "[\"deleted\"]", "()");
    disabled_chain_probe();
}

fn live_register_probe() {
    let source = EmbeddedScriptSource::new(BTreeMap::from([(
        ModuleId::parse("components/live_reader").unwrap(),
        r#"/* gpui-rhai
        {"id":"components/live_reader","export":"LiveReader","version":"0.1.8","runtime_api":{"min_inclusive":2,"max_exclusive":3},"dependencies":[],"capabilities":{}}
        */
        define_component(#{
          metadata:#{id:"components/live_reader","export":"LiveReader",version:"0.1.8",runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
          schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:[]},
          render:Fn("render_reader")
        });
        fn LiveReader(props){render_component("components/live_reader",props)}
        fn render_reader(ctx,props){
          try { let data=ctx.get_native_collection("late"); return text(`ready:${data.len}`); }
          catch (err) { return text("missing"); }
        }
        "#.to_owned(),
    )]));
    let mut engine=RuntimeEngine::new();
    engine.set_module_resolver(RestrictedModuleResolver::from_source(&source).unwrap());
    let compiled=engine.compile_self_contained_named("audit/live.rhai", r#"import "components/live_reader" as reader; fn view(ctx){reader::LiveReader(#{key:"child"})}"#).unwrap();
    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));
    let root_path=ComponentInstancePath::root("App","live");
    let mut lifecycle=ScriptLifecycle::new(compiled,Rc::clone(&runtime),root_path.clone(),Some("main".to_owned()),BTreeMap::new(),&ComponentStateSchema::default()).unwrap();
    lifecycle.start(&mut engine).unwrap();
    println!("live register initial root: {:?}",lifecycle.root().unwrap().kind());
    let collection=gpui_rhai::NativeCollection::new("key",[BTreeMap::from([("key".to_owned(),UiValue::String("row1".to_owned()))])]).unwrap();
    runtime.borrow_mut().register_native_collection_from_host(&root_path,"late",collection).unwrap();
    println!("live register render_dirty: {:?}",lifecycle.render_dirty(&mut engine));
    println!("live register after root invalidation: {:?}",lifecycle.root().unwrap().kind());
    lifecycle.render(&mut engine).unwrap();
    println!("live register after forced full render: {:?}",lifecycle.root().unwrap().kind());
}

fn disabled_chain_probe() {
    for disabled in [false,true] {
    let items=(0..1000).map(|i|if i<=128{
        format!("#{{key:\"k{i}\",label:\"Node {i}\",parent:{},disabled:{}}}",if i==0{"()".to_owned()}else{format!("\"k{}\"",i-1)},disabled && i<128)
    }else{format!("#{{key:\"k{i}\",label:\"Node {i}\"}}")}).collect::<Vec<_>>();
    let expanded=(0..128).map(|i|format!("\"k{i}\"")).collect::<Vec<_>>().join(",");
    tree_probe(&format!("1000 rows depth128 disabled ancestry={disabled}"),&format!("[{}]",items.join(",")),&format!("[{expanded}]"),"\"k128\"");
    }
}

fn projection_probe() {
    let runtime=RuntimeEngine::new();
    for (count, reverse, expand) in [(257,false,false),(257,true,true),(258,false,false),(258,true,true),(1000,true,false)] {
        let mut items: rhai::Array=(0..count).map(|i|item(format!("k{i:05}"),if i==0{None}else{Some(format!("k{:05}",i-1))})).collect();
        if reverse { items.reverse(); }
        let expanded: rhai::Array=if expand{(0..count).map(|i|rhai::Dynamic::from(format!("k{i:05}"))).collect()}else{vec![]};
        let mut scope=rhai::Scope::new();scope.push("items",items);scope.push("expanded",expanded);
        let now=Instant::now();
        let result=runtime.engine().eval_with_scope::<rhai::Array>(&mut scope,"outline_projection(items, expanded)");
        println!("projection count={count} reversed={reverse} expanded={expand}, elapsed={:?}, result={:?}",now.elapsed(),result.as_ref().map(|r|r.len()).map_err(|e|e.to_string()));
        assert_eq!(result.is_ok(),count<=257);
    }
}
