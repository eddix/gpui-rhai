use gpui_rhai::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

// Both collections render no batch ahead and keep no rows, so rows that scroll away release
// their reads; `a_kept_row_keeps_its_reads...` covers kept rows.
const SOURCE: &str = r#"
fn load(ctx,key){try{let value=ctx.get_native_collection(key);return text("ready");}catch(err){return text(err.to_string());}}
fn rows(){let data=[];for i in 0..80{data.push(#{key:`source-${i}`});}data}
fn row(ctx,p){
 if p.index==79{throw "candidate failed";}
 // Root and both row contributions can independently use the same name.
 let shared_value=load(ctx,"shared");
 load(ctx,p.key)
}
fn view(ctx){let root=load(ctx,"root-source");column([root,
 virtual_collection(#{key:"a",label:"A",data:rows(),height:24,estimated_height:24,overdraw_pixels:0,batch_items:1,retain_items:0},Fn("row")),
 virtual_collection(#{key:"b",label:"B",data:rows(),height:24,estimated_height:24,overdraw_pixels:0,batch_items:1,retain_items:0},Fn("row"))])}
"#;

fn setup() -> (
    RuntimeEngine,
    Rc<RefCell<UiRuntimeState>>,
    ScriptLifecycle,
    ComponentInstancePath,
) {
    let mut engine = RuntimeEngine::new();
    let compiled = engine.compile(SOURCE).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let path = ComponentInstancePath::root("App", "readers");
    let mut life = ScriptLifecycle::new(
        compiled,
        runtime.clone(),
        path.clone(),
        Some("main".into()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    life.start(&mut engine).unwrap();
    (engine, runtime, life, path)
}
fn collection<'a>(node: &'a UiNode, key: &str) -> Option<&'a VirtualCollectionNodeSpec> {
    match node.kind() {
        UiNodeKind::VirtualCollection { spec } => {
            if spec.id.key == key {
                Some(spec)
            } else {
                spec.realized
                    .values()
                    .find_map(|child| collection(child, key))
            }
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(|child| collection(child, key))
        }
        _ => None,
    }
}
fn empty_collection() -> NativeCollection {
    NativeCollection::new("id", std::iter::empty::<BTreeMap<String, UiValue>>()).unwrap()
}

#[test]
fn raw_scroll_dependencies_release_history_keep_other_contributors_and_wake_the_current_owner() {
    let (mut engine, runtime, mut life, path) = setup();
    let a = collection(life.root().unwrap(), "a").unwrap().id.clone();
    for index in 2..78 {
        runtime
            .borrow()
            .virtual_requests
            .request(a.clone(), [index]);
        life.realize_virtual_requests(&mut engine).unwrap();
    }
    let UiNodeKind::Text { text } =
        collection(life.root().unwrap(), "a").unwrap().realized[&77].kind()
    else {
        panic!("raw row must remain text")
    };
    assert!(text.as_str().contains("not registered"));
    assert!(!text.as_str().contains("limit"));
    runtime
        .borrow_mut()
        .register_native_collection_from_host(&path, "source-2", empty_collection())
        .unwrap();
    assert!(
        !life.render_dirty(&mut engine).unwrap(),
        "offscreen contribution must not schedule root"
    );
    // B still reads source-0 even though A scrolled away.
    runtime
        .borrow_mut()
        .register_native_collection_from_host(&path, "source-0", empty_collection())
        .unwrap();
    assert!(life.render_dirty(&mut engine).unwrap());
    runtime
        .borrow_mut()
        .register_native_collection_from_host(&path, "root-source", empty_collection())
        .unwrap();
    assert!(
        life.render_dirty(&mut engine).unwrap(),
        "row target must not clear root's direct contribution"
    );
    runtime
        .borrow_mut()
        .register_native_collection_from_host(&path, "shared", empty_collection())
        .unwrap();
    assert!(
        life.render_dirty(&mut engine).unwrap(),
        "shared live row contributions must survive"
    );
    runtime
        .borrow_mut()
        .register_native_collection_from_host(&path, "source-77", empty_collection())
        .unwrap();
    assert!(life.render_dirty(&mut engine).unwrap());
    assert!(
        matches!(collection(life.root().unwrap(),"a").unwrap().realized[&77].kind(),UiNodeKind::Text{text} if text=="ready")
    );
}

#[test]
fn failed_row_candidate_restores_the_previous_read_contributions() {
    let (mut engine, runtime, mut life, path) = setup();
    let a = collection(life.root().unwrap(), "a").unwrap().id.clone();
    runtime.borrow().virtual_requests.request(a, [79]);
    assert!(life.realize_virtual_requests(&mut engine).is_err());
    runtime
        .borrow_mut()
        .register_native_collection_from_host(&path, "source-0", empty_collection())
        .unwrap();
    assert!(
        life.render_dirty(&mut engine).unwrap(),
        "rollback must restore old row subscriptions"
    );
    assert!(
        matches!(collection(life.root().unwrap(),"a").unwrap().realized[&0].kind(),UiNodeKind::Text{text} if text=="ready")
    );
}

#[test]
fn a_kept_row_keeps_its_reads_so_a_change_renders_the_collection_again() {
    let mut engine = RuntimeEngine::new();
    let source = SOURCE.replacen(
        "batch_items:1,retain_items:0",
        "batch_items:1,retain_items:32",
        1,
    );
    assert_ne!(source, SOURCE);
    let compiled = engine.compile(&source).unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let path = ComponentInstancePath::root("App", "kept-readers");
    let mut life = ScriptLifecycle::new(
        compiled,
        runtime.clone(),
        path.clone(),
        Some("main".into()),
        BTreeMap::new(),
        &ComponentStateSchema::default(),
    )
    .unwrap();
    life.start(&mut engine).unwrap();
    let a = collection(life.root().unwrap(), "a").unwrap().id.clone();
    for index in [2, 10] {
        runtime
            .borrow()
            .virtual_requests
            .request(a.clone(), [index]);
        life.realize_virtual_requests(&mut engine).unwrap();
    }
    assert!(
        !collection(life.root().unwrap(), "a")
            .unwrap()
            .realized
            .contains_key(&2)
    );
    // Row 2 is kept with its read of source-2: registering it renders the collection again.
    runtime
        .borrow_mut()
        .register_native_collection_from_host(&path, "source-2", empty_collection())
        .unwrap();
    assert!(
        life.render_dirty(&mut engine).unwrap(),
        "a kept row's read must render its collection again"
    );
    runtime.borrow().virtual_requests.request(a.clone(), [2]);
    life.realize_virtual_requests(&mut engine).unwrap();
    let UiNodeKind::Text { text } =
        collection(life.root().unwrap(), "a").unwrap().realized[&2].kind()
    else {
        panic!("raw row must remain text")
    };
    assert_eq!(
        text.as_str(),
        "ready",
        "the row comes back rendered from the new data"
    );
}
