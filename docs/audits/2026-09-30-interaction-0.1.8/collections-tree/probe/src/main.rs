use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Instant};
use gpui_rhai::{ComponentInstancePath, ComponentStateSchema, EmbeddedScriptSource, ModuleId, RestrictedModuleResolver, RuntimeEngine, ScriptLifecycle, UiRuntimeState, UiValue};

fn item(key: String, parent: Option<String>) -> rhai::Dynamic {
    UiValue::Map(BTreeMap::from([
        ("label".to_owned(), UiValue::String(key.clone())),
        ("key".to_owned(), UiValue::String(key)),
        ("parent".to_owned(), parent.map_or(UiValue::Null, UiValue::String)),
    ])).into_dynamic()
}

fn projection_probe() {
    let runtime = RuntimeEngine::new();
    for count in [100, 1_000, 3_000, 6_000, 10_000] {
        let items: rhai::Array = (0..count).map(|i| item(format!("k{i:05}"), if i == 0 { None } else { Some(format!("k{:05}", i - 1)) })).collect();
        let mut scope = rhai::Scope::new();
        scope.push("items", items);
        let now = Instant::now();
        let result = runtime.engine().eval_with_scope::<rhai::Array>(&mut scope, "outline_projection(items, [])");
        println!("collapsed chain count={count} elapsed={:?} result={:?}", now.elapsed(), result.as_ref().map(|r| r.len()).map_err(|e| e.to_string()));
    }
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
        Err(err) => println!("tree {label} ERROR: {err}"),
    }
}

fn main() {
    if std::env::args().any(|arg| arg == "projection") { projection_probe(); return; }
    tree_probe("no active", r#"[#{key:"a",label:"A"},#{key:"b",label:"B"}]"#, "[]", "()");
    tree_probe("empty", "[]", "[]", "()");
    tree_probe("collapsed active child", r#"[#{key:"parent",label:"Parent"},#{key:"child",label:"Child",parent:"parent"},#{key:"next",label:"Next"}]"#, "[]", "\"child\"");
    tree_probe("disabled parent", r#"[#{key:"parent",label:"Parent",disabled:true},#{key:"child",label:"Child",parent:"parent"},#{key:"next",label:"Next"}]"#, "[\"parent\"]", "\"child\"");
    tree_probe("after accepting disabled parent", r#"[#{key:"parent",label:"Parent",disabled:true},#{key:"child",label:"Child",parent:"parent"},#{key:"next",label:"Next"}]"#, "[\"parent\"]", "\"parent\"");
    tree_probe("deleted expanded", r#"[#{key:"a",label:"A"}]"#, "[\"deleted\"]", "()");
}
