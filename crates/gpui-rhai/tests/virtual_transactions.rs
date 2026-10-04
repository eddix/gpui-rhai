use gpui_rhai::*;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

const COUNTER: &str = r#"
define_component(#{metadata:#{id:"test/counter","export":"Counter",version:"0.1.8",
 runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
 schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
 state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:7}}}},
 events:#{},slots:#{},parts:[],effects:["presence"]},render:Fn("counter")});
fn noop(ctx,payload){()}
fn cleanup(ctx,payload){ctx.get_state("count");}
fn read(ctx,payload){ctx.get_state("count")}
fn counter(ctx,props){
 effect("presence",(),Fn("noop"),Fn("cleanup"));timeout("tick",1000,false,Fn("noop"),());
 let opacity=signal("opacity",1.0);
 text(props.key).with_key(props.key).bind_signal("opacity",opacity)
   .with_ref(element_ref("counter")).on_click(Fn("read"))
}
fn data(){let rows=[];for i in 0..20{rows.push(#{key:`row-${i}`});}rows}
fn row(ctx,p){if p.index==19{throw "candidate failed";}render_component("test/counter",#{key:p.key})}
fn outer_row(ctx,p){virtual_collection(#{key:`inner-${p.key}`,label:"Inner",data:data(),height:24,
 estimated_height:24,overdraw_pixels:0},Fn("row"))}
"#;

fn setup(
    view: &str,
    name: &str,
) -> (
    RuntimeEngine,
    Rc<RefCell<UiRuntimeState>>,
    ScriptLifecycle,
    ComponentInstancePath,
) {
    let mut engine = RuntimeEngine::new();
    let compiled = engine
        .compile(&format!("{COUNTER}\nfn view(ctx){{{view}}}"))
        .unwrap();
    let runtime = Rc::new(RefCell::new(UiRuntimeState::new()));
    let path = ComponentInstancePath::root("App", name);
    let mut life = ScriptLifecycle::new(
        compiled,
        runtime.clone(),
        path.clone(),
        Some(name.into()),
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
fn request(
    runtime: &Rc<RefCell<UiRuntimeState>>,
    id: VirtualCollectionId,
    indices: impl IntoIterator<Item = usize>,
) {
    runtime.borrow().virtual_requests.request(id, indices);
}
fn callback(life: &ScriptLifecycle, key: &str, index: usize) -> ScriptCallback {
    collection(life.root().unwrap(), key).unwrap().realized[&index]
        .handler("click")
        .unwrap()
        .as_script()
        .unwrap()
        .clone()
}
const SIBLINGS: &str = r#"column([
 virtual_collection(#{key:"a",label:"A",data:data(),height:24,estimated_height:24,overdraw_pixels:0},Fn("row")),
 virtual_collection(#{key:"b",label:"B",data:data(),height:24,estimated_height:24,overdraw_pixels:0},Fn("row"))])"#;

#[test]
fn disjoint_scope_targets_are_equivalent_for_both_orders_and_one_batch() {
    for mode in 0..3 {
        let (mut engine, runtime, mut life, path) = setup(SIBLINGS, "siblings");
        let a = collection(life.root().unwrap(), "a").unwrap().id.clone();
        let b = collection(life.root().unwrap(), "b").unwrap().id.clone();
        let old = callback(&life, "a", 0);
        let old_owner = path
            .child("VirtualCollection", "a")
            .child("Counter", "row-0");
        runtime
            .borrow_mut()
            .set_component_state_from_host(&old_owner, "count", UiValue::Integer(9))
            .unwrap();
        life.render_dirty(&mut engine).unwrap();
        let signal = runtime
            .borrow()
            .signals
            .resolve(&old_owner, "opacity")
            .unwrap();
        let order = if mode == 1 {
            [b.clone(), a.clone()]
        } else {
            [a.clone(), b.clone()]
        };
        for id in order {
            request(&runtime, id, [10]);
            if mode != 2 {
                life.realize_virtual_requests(&mut engine).unwrap();
            }
        }
        if mode == 2 {
            life.realize_virtual_requests(&mut engine).unwrap();
        }
        assert!(
            runtime
                .borrow()
                .component_state
                .get(&old_owner, "count")
                .is_none()
        );
        assert!(
            life.invoke_callback_transactional(&engine, &old, UiValue::Null)
                .is_err()
        );
        assert!(runtime.borrow().signals.read(&signal).is_err());
        for name in ["a", "b"] {
            let cb = callback(&life, name, 10);
            assert_eq!(
                life.invoke_callback_transactional(&engine, &cb, UiValue::Null)
                    .unwrap()
                    .as_int()
                    .unwrap(),
                7
            );
        }
        request(&runtime, a, [0]);
        life.realize_virtual_requests(&mut engine).unwrap();
        assert_eq!(
            runtime.borrow().component_state.get(&old_owner, "count"),
            Some(&UiValue::Integer(7))
        );
        assert!(runtime.borrow().component_state.paths().contains(&path));
    }
}

#[test]
fn overlapping_targets_keep_new_formal_callbacks_and_resources_live() {
    for batch in [false, true] {
        let (mut engine, runtime, mut life, _) = setup(
            r#"virtual_collection(#{key:"outer",label:"Outer",data:data(),height:24,estimated_height:24,overdraw_pixels:0},Fn("outer_row"))"#,
            "nested",
        );
        let outer = collection(life.root().unwrap(), "outer")
            .unwrap()
            .id
            .clone();
        let inner = collection(life.root().unwrap(), "inner-row-0")
            .unwrap()
            .id
            .clone();
        let old = callback(&life, "inner-row-0", 0);
        request(&runtime, outer.clone(), [0, 10]);
        if !batch {
            life.realize_virtual_requests(&mut engine).unwrap();
        }
        request(&runtime, inner.clone(), [10]);
        life.realize_virtual_requests(&mut engine).unwrap();
        let cb = callback(&life, "inner-row-0", 10);
        assert_eq!(
            life.invoke_callback_transactional(&engine, &cb, UiValue::Null)
                .unwrap()
                .as_int()
                .unwrap(),
            7
        );
        assert!(
            life.invoke_callback_transactional(&engine, &old, UiValue::Null)
                .is_err()
        );
        let owner = ComponentInstancePath::root("App", "nested")
            .child("VirtualCollection", "outer")
            .child("VirtualCollection", "inner-row-0")
            .child("Counter", "row-10");
        let signal = runtime.borrow().signals.resolve(&owner, "opacity").unwrap();
        assert!(runtime.borrow().signals.read(&signal).is_ok());
        request(&runtime, outer, [10]);
        request(&runtime, inner, [11]);
        life.realize_virtual_requests(&mut engine).unwrap();
        assert!(
            life.invoke_callback_transactional(&engine, &cb, UiValue::Null)
                .is_err()
        );
        assert!(runtime.borrow().signals.read(&signal).is_err());
    }
}

#[test]
fn batch_failure_restores_all_prior_scope_state_and_callbacks() {
    let (mut engine, runtime, mut life, path) = setup(SIBLINGS, "rollback");
    let old = callback(&life, "a", 0);
    let owner = path
        .child("VirtualCollection", "a")
        .child("Counter", "row-0");
    runtime
        .borrow_mut()
        .set_component_state_from_host(&owner, "count", UiValue::Integer(9))
        .unwrap();
    life.render_dirty(&mut engine).unwrap();
    for (name, index) in [("a", 10), ("b", 19)] {
        let id = collection(life.root().unwrap(), name).unwrap().id.clone();
        request(&runtime, id, [index]);
    }
    assert!(life.realize_virtual_requests(&mut engine).is_err());
    assert_eq!(
        life.invoke_callback_transactional(&engine, &old, UiValue::Null)
            .unwrap()
            .as_int()
            .unwrap(),
        9
    );
    assert_eq!(
        collection(life.root().unwrap(), "a")
            .unwrap()
            .realized
            .keys()
            .copied()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([0, 1])
    );
}
