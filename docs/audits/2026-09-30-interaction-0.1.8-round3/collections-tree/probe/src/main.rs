use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use gpui_rhai::{ComponentInstancePath, ComponentStateSchema, ExecutionPhase, NativeCollection, RuntimeEngine, ScriptLifecycle, UiContext, UiRuntimeState, UiValue};

fn empty() -> NativeCollection {
    NativeCollection::new("key", std::iter::empty::<BTreeMap<String, UiValue>>()).unwrap()
}

fn main() {
    let mut engine=RuntimeEngine::new();
    let suffix="x".repeat(512);
    let script=format!(r#"
        fn view(ctx) {{ text("idle") }}
        fn touch(ctx, batch) {{
            for i in 0..64 {{
                let name=`probe-name-${{batch}}-${{i}}-{suffix}`;
                try {{ ctx.get_native_collection(name); }} catch (err) {{ }}
            }}
        }}
        fn touch_valid(ctx, batch) {{
            for i in 0..64 {{
                try {{ ctx.get_native_collection(`probe-valid-${{batch}}-${{i}}`); }} catch (err) {{ }}
            }}
        }}
    "#);
    let compiled=engine.compile_named("audit/missing-budget.rhai",&script).unwrap();
    let callback=engine.callback(&compiled,"touch").unwrap();
    let valid_callback=engine.callback(&compiled,"touch_valid").unwrap();
    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));
    let root=ComponentInstancePath::root("App","audit-r3");
    let mut lifecycle=ScriptLifecycle::new(compiled,Rc::clone(&runtime),root.clone(),Some("main".to_owned()),BTreeMap::new(),&ComponentStateSchema::default()).unwrap();
    lifecycle.start(&mut engine).unwrap();
    for batch in 0..3 {
        let _=lifecycle.invoke_callback_transactional(&engine,&callback,UiValue::Integer(batch)).unwrap();
        let dirty=lifecycle.render_dirty(&mut engine).unwrap();
        let registry=format!("{:?}",runtime.borrow().native_collections);
        println!("caught-miss callback batch={batch}, retained_invalid_names={}, registry_debug_bytes={}, render_dirty={dirty}",registry.matches("probe-name-").count(),registry.len());
    }
    for batch in 0..2 {
        let _=lifecycle.invoke_callback_transactional(&engine,&valid_callback,UiValue::Integer(batch)).unwrap();
        let dirty=lifecycle.render_dirty(&mut engine).unwrap();
        let registry=format!("{:?}",runtime.borrow().native_collections);
        println!("valid missing callback batch={batch}, retained_valid_names={}, render_dirty={dirty}",registry.matches("probe-valid-").count());
    }
    let impossible=format!("probe-name-0-0-{suffix}");
    println!("host register corresponding name (length={}): {:?}",impossible.len(),runtime.borrow_mut().register_native_collection_from_host(&root,&impossible,empty()).map_err(|e|format!("{:?}",e).split('(').next().unwrap().to_owned()));

    // Use the actual runtime transaction checkpoint, not a mirror registry.
    let ctx=UiContext::new(Rc::clone(&runtime),root.clone(),Some("main".into()),ExecutionPhase::Event,BTreeMap::new());
    assert!(ctx.get_native_collection("before").is_err());
    let snapshot=runtime.borrow_mut().begin_transaction().unwrap();
    assert!(ctx.get_native_collection("rolled_back").is_err());
    runtime.borrow_mut().rollback_transaction(snapshot).unwrap();
    runtime.borrow_mut().register_native_collection_from_host(&root,"rolled_back",empty()).unwrap();
    println!("register rolled-back dependency render_dirty={}",lifecycle.render_dirty(&mut engine).unwrap());
    runtime.borrow_mut().register_native_collection_from_host(&root,"before",empty()).unwrap();
    println!("register prior dependency render_dirty={}",lifecycle.render_dirty(&mut engine).unwrap());

    assert!(ctx.get_native_collection("unmounted").is_err());
    runtime.borrow_mut().release_window("main",&root).unwrap();
    let registry=format!("{:?}",runtime.borrow().native_collections);
    println!("release_window missing_readers_cleared={}",registry.ends_with("missing_readers: {} }"));
    isolation_probe();
}

fn isolation_probe() {
    let runtime=Rc::new(RefCell::new(UiRuntimeState::new()));
    let a=ComponentInstancePath::root("App","a");
    let b=ComponentInstancePath::root("App","b");
    let ca=UiContext::new(Rc::clone(&runtime),a.clone(),Some("a".into()),ExecutionPhase::Render,BTreeMap::new());
    let cb=UiContext::new(Rc::clone(&runtime),b.clone(),Some("b".into()),ExecutionPhase::Render,BTreeMap::new());
    assert!(ca.get_native_collection("late_a").is_err());
    assert!(cb.get_native_collection("late_b").is_err());
    runtime.borrow_mut().register_native_collection_from_host(&b,"late_a",empty()).unwrap();
    assert_eq!(runtime.borrow().dirty_components(),&std::collections::BTreeSet::from([a.clone()]));
    let _=ca.get_native_collection("late_a").unwrap();
    runtime.borrow_mut().release_window("a",&a).unwrap();
    let changed=runtime.borrow_mut().replace_native_collection_from_host("late_a",empty()).unwrap();
    assert!(!changed);
    runtime.borrow_mut().register_native_collection_from_host(&a,"late_b",empty()).unwrap();
    assert_eq!(runtime.borrow().dirty_components(),&std::collections::BTreeSet::from([b]));
    println!("cross-root exact-reader invalidation and positive-reader cleanup: passed");
}
