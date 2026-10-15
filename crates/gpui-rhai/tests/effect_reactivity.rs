//! Post-commit effect state uses foreground delivery, with one bounded cascade.
mod common;

use common::{compile, compile_official, mount, new_runtime, root_path};
use gpui_rhai::{InvocationOrigin, LifecycleError, UiNodeKind, UiValue};

const APP: &str = r#"
    define_component(#{metadata:#{id:"test/cascade","export":"Cascade",version:"0.2.2",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{}},
        schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},looping:#{schema:#{type:"bool"},required:true,sensitive:false},fail:#{schema:#{type:"bool"},required:true,sensitive:false}},
            state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}},events:#{},slots:#{},parts:[],effects:["step"]},render:Fn("render_cascade")});
    fn state_schema(){#{fields:#{looping:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},fail:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
    fn step(ctx,deps){
        if deps.fail {ctx.set_state("count",1000);throw "rejected start";}
        if deps.looping {ctx.set_state("count",ctx.get_state("count")+1);}
    }
    fn cleanup(ctx,deps){}
    fn render_cascade(ctx,props){
        effect("step",#{count:ctx.get_state("count"),looping:props.looping,fail:props.fail},Fn("step"),Fn("cleanup"));
        text(`${ctx.get_state("count")}`)
    }
    fn enable(ctx,payload){ctx.set_state("looping",true);}
    fn disable(ctx,payload){ctx.set_state("looping",false);ctx.set_state("fail",false);}
    fn failed_recovery(ctx,payload){ctx.set_state("looping",false);throw "rejected recovery";}
    fn reject_start(ctx,payload){ctx.set_state("fail",true);}
    fn view(ctx){render_component("test/cascade",#{key:"cascade",looping:ctx.get_state("looping"),fail:ctx.get_state("fail")})}
"#;

#[test]
fn self_exciting_effect_stops_at_the_existing_budget_and_external_input_can_recover() {
    let (mut engine, compiled) = compile_official("cascade.rhai", APP);
    let enable = engine.callback(&compiled, "enable").unwrap();
    let disable = engine.callback(&compiled, "disable").unwrap();
    let failed_recovery = engine.callback(&compiled, "failed_recovery").unwrap();
    let runtime = new_runtime();
    let mut lifecycle = mount(&mut engine, compiled, &runtime);
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &enable, UiValue::Null)
        .unwrap();
    let mut successful_renders = 0;
    loop {
        match lifecycle.render_dirty(&mut engine) {
            Ok(true) => successful_renders += 1,
            Err(LifecycleError::EffectBudget(66)) => break,
            result => panic!("unexpected cascade result: {result:?}"),
        }
        assert!(
            successful_renders <= 32,
            "a foreground yield must not reset the budget"
        );
    }
    assert_eq!(successful_renders, 32);
    let revision = lifecycle.revision();
    for _ in 0..4 {
        assert!(
            !lifecycle.render_dirty(&mut engine).unwrap(),
            "old wakes must stay inert"
        );
    }
    assert_eq!(lifecycle.revision(), revision);
    assert!(
        lifecycle
            .invoke_callback_transactional(&engine, &failed_recovery, UiValue::Null)
            .is_err()
    );
    assert!(!lifecycle.render_dirty(&mut engine).unwrap());

    let _ = lifecycle
        .invoke_callback_transactional(&engine, &disable, UiValue::Null)
        .unwrap();
    assert!(lifecycle.render_dirty(&mut engine).unwrap());
    assert!(!lifecycle.render_dirty(&mut engine).unwrap());
    assert!(matches!(lifecycle.root().unwrap().kind(), UiNodeKind::Text { text } if text == "32"));
}

#[test]
fn failed_effect_start_rolls_back_state_and_does_not_commit_a_followup() {
    let (mut engine, compiled) = compile_official("failed_effect.rhai", APP);
    let reject = engine.callback(&compiled, "reject_start").unwrap();
    let runtime = new_runtime();
    let mut lifecycle = mount(&mut engine, compiled, &runtime);
    let owner = lifecycle.root().unwrap().component_root().unwrap().clone();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &reject, UiValue::Null)
        .unwrap();
    let revision = lifecycle.revision();
    assert!(lifecycle.render_dirty(&mut engine).is_err());
    assert_eq!(lifecycle.revision(), revision);
    assert_eq!(
        runtime.borrow().component_state.get(&owner, "count"),
        Some(&UiValue::Integer(0))
    );
    assert!(matches!(lifecycle.root().unwrap().kind(), UiNodeKind::Text { text } if text == "0"));
}

const SEMANTIC: &str = r#"
    define_component(#{metadata:#{id:"test/semantic","export":"Semantic",version:"0.2.2",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{}},
        schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},count:#{schema:#{type:"integer"},required:true,sensitive:false},enabled:#{schema:#{type:"bool"},required:true,sensitive:false},on_tick:#{schema:#{type:"callback"},required:true,sensitive:false}},
            state:#{fields:#{}},events:#{tick:#{payload:#{type:"null"}}},slots:#{},parts:[],effects:["step"]},render:Fn("render_semantic")});
    fn state_schema(){#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},enabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
    fn step(ctx,deps){if deps.enabled {ctx.emit("tick",());}}
    fn cleanup(ctx,deps){}
    fn render_semantic(ctx,props){effect("step",#{count:props.count,enabled:props.enabled},Fn("step"),Fn("cleanup"));text(`${props.count}`)}
    fn tick(ctx,payload){ctx.set_state("count",ctx.get_state("count")+1);}
    fn enable(ctx,payload){ctx.set_state("enabled",true);}
    fn disable(ctx,payload){ctx.set_state("enabled",false);}
    fn view(ctx){render_component("test/semantic",#{key:"semantic",count:ctx.get_state("count"),enabled:ctx.get_state("enabled"),on_tick:Fn("tick")})}
"#;

#[test]
fn public_event_entry_keeps_origin_budget_and_rejects_clones_after_external_recovery() {
    let (mut engine, compiled) = compile_official("semantic.rhai", SEMANTIC);
    let enable = engine.callback(&compiled, "enable").unwrap();
    let disable = engine.callback(&compiled, "disable").unwrap();
    let runtime = new_runtime();
    let mut lifecycle = mount(&mut engine, compiled, &runtime);
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &enable, UiValue::Null)
        .unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    let mut retained = None;
    let mut rejected = false;
    for _ in 0..64 {
        let pending = runtime
            .borrow_mut()
            .drain_batch()
            .events
            .pop()
            .expect("effect must emit one follow-up");
        assert_eq!(pending.origin, InvocationOrigin::Effect);
        retained = Some(pending.clone());
        let result = lifecycle
            .invoke_component_event_transactional(&engine, pending)
            .and_then(|_| lifecycle.render_dirty(&mut engine));
        assert_eq!(
            runtime.borrow().invocation_origin(),
            &InvocationOrigin::Lifecycle
        );
        match result {
            Ok(true) => (),
            Err(LifecycleError::EffectBudget(_)) => {
                rejected = true;
                break;
            }
            result => panic!("unexpected public event result {result:?}"),
        }
    }
    assert!(
        rejected,
        "public delivery must not turn each effect event into external input"
    );
    assert!(!lifecycle.render_dirty(&mut engine).unwrap());
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &disable, UiValue::Null)
        .unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    let count = runtime
        .borrow()
        .component_state
        .get(&root_path(), "count")
        .cloned();
    assert!(
        lifecycle
            .invoke_component_event_transactional(&engine, retained.unwrap())
            .unwrap()
            .is_none()
    );
    assert_eq!(
        runtime
            .borrow()
            .component_state
            .get(&root_path(), "count")
            .cloned(),
        count
    );
    assert!(runtime.borrow_mut().drain_batch().events.is_empty());
}

#[test]
fn explicit_host_event_without_owner_cannot_use_effect_origin_to_reset_the_budget() {
    let (mut engine, compiled) = compile_official("host_effect_event.rhai", SEMANTIC);
    let enable = engine.callback(&compiled, "enable").unwrap();
    let runtime = new_runtime();
    let mut lifecycle = mount(&mut engine, compiled, &runtime);
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &enable, UiValue::Null)
        .unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    let mut host_event = runtime.borrow_mut().drain_batch().events.pop().unwrap();
    host_event.owner = None;
    let mut rejected = false;
    for _ in 0..65 {
        match lifecycle.invoke_component_event_transactional(&engine, host_event.clone()) {
            Ok(Some(_)) => (),
            Err(LifecycleError::EffectBudget(_)) => {
                rejected = true;
                break;
            }
            result => panic!("unexpected Host effect result {result:?}"),
        }
    }
    assert!(rejected);
    let count = runtime
        .borrow()
        .component_state
        .get(&root_path(), "count")
        .cloned();
    let _ = lifecycle
        .invoke_component_event_transactional(&engine, host_event)
        .unwrap();
    assert_eq!(
        runtime
            .borrow()
            .component_state
            .get(&root_path(), "count")
            .cloned(),
        count
    );
}

const PURE_ACTION: &str = r#"
    define_component(#{metadata:#{id:"test/pure_action","export":"PureAction",version:"0.2.2",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{}},
        schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},enabled:#{schema:#{type:"bool"},required:true,sensitive:false}},state:#{fields:#{}},events:#{},slots:#{},parts:[],effects:["dispatch"]},render:Fn("render_pure_action")});
    fn state_schema(){#{fields:#{enabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}}}}
    fn init(ctx){ctx.register_action("test.loop",Fn("loop_action"));}
    fn loop_action(ctx,payload){ctx.dispatch_action("test.loop",());}
    fn start_dispatch(ctx,enabled){if enabled {ctx.dispatch_action("test.loop",());}}
    fn cleanup_dispatch(ctx,enabled){}
    fn disable(ctx,payload){ctx.set_state("enabled",false);}
    fn render_pure_action(ctx,props){effect("dispatch",props.enabled,Fn("start_dispatch"),Fn("cleanup_dispatch"));text("stable")}
    fn view(ctx){render_component("test/pure_action",#{key:"loop",enabled:ctx.get_state("enabled")})}
"#;

#[test]
fn pure_action_rejection_prunes_rollback_queue_and_held_derivations_and_reload_recovers() {
    let (mut engine, compiled) = compile_official("pure_action.rhai", PURE_ACTION);
    let schema = engine.root_state_schema(&compiled).unwrap();
    let disable = engine.callback(&compiled, "disable").unwrap();
    let runtime = new_runtime();
    let mut lifecycle = mount(&mut engine, compiled, &runtime);
    // Same outer transaction as the native foreground pump: rollback would
    // restore the initial queued loop unless its rejection token survives.
    let before_pump = runtime.borrow_mut().begin_transaction().unwrap();
    let mut retained = None;
    let mut rejected = false;
    for _ in 0..64 {
        let pending = runtime
            .borrow_mut()
            .drain_batch()
            .actions
            .pop()
            .expect("pure action requeues itself");
        retained = Some(pending.clone());
        match lifecycle.invoke_action_transactional(&engine, pending) {
            Ok(_) => (),
            Err(LifecycleError::EffectBudget(_)) => {
                rejected = true;
                break;
            }
            result => panic!("unexpected pure dispatch result {result:?}"),
        }
    }
    assert!(
        rejected,
        "a loop without state/effect transitions must be bounded"
    );
    runtime
        .borrow_mut()
        .rollback_transaction(before_pump)
        .unwrap();
    assert!(runtime.borrow_mut().drain_batch().actions.is_empty());
    let retained = retained.unwrap();
    let failed = compile(
        &mut engine,
        "failed_reload.rhai",
        "fn view(ctx){throw \"candidate failed\";}",
    );
    assert!(lifecycle.reload(&mut engine, failed, &schema).is_err());
    let _ = lifecycle
        .invoke_action_transactional(&engine, retained.clone())
        .unwrap();
    assert!(runtime.borrow_mut().drain_batch().actions.is_empty());

    let _ = lifecycle
        .invoke_callback_transactional(&engine, &disable, UiValue::Null)
        .unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    let _ = lifecycle
        .invoke_action_transactional(&engine, retained.clone())
        .unwrap();
    assert!(
        runtime.borrow_mut().drain_batch().actions.is_empty(),
        "external recovery must not resurrect rejected derivations"
    );
    let replacement = compile(
        &mut engine,
        "normal_reload.rhai",
        "fn view(ctx){text(\"normal\")}",
    );
    lifecycle.reload(&mut engine, replacement, &schema).unwrap();
    let _ = lifecycle
        .invoke_action_transactional(&engine, retained)
        .unwrap();
    assert!(runtime.borrow_mut().drain_batch().actions.is_empty());
    assert!(!lifecycle.render_dirty(&mut engine).unwrap());
    assert!(
        matches!(lifecycle.root().unwrap().kind(), UiNodeKind::Text { text } if text == "normal")
    );
}

#[test]
fn held_event_is_cancelled_by_activation_replacement_and_successful_but_not_failed_reload() {
    let (mut engine, compiled) = compile_official("held_event.rhai", SEMANTIC);
    let schema = engine.root_state_schema(&compiled).unwrap();
    let enable = engine.callback(&compiled, "enable").unwrap();
    let disable = engine.callback(&compiled, "disable").unwrap();
    let runtime = new_runtime();
    let mut lifecycle = mount(&mut engine, compiled, &runtime);
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &enable, UiValue::Null)
        .unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    let held = runtime.borrow_mut().drain_batch().events.pop().unwrap();
    let _ = lifecycle
        .invoke_callback_transactional(&engine, &disable, UiValue::Null)
        .unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    assert!(
        lifecycle
            .invoke_component_event_transactional(&engine, held)
            .unwrap()
            .is_none()
    );

    let _ = lifecycle
        .invoke_callback_transactional(&engine, &enable, UiValue::Null)
        .unwrap();
    lifecycle.render_dirty(&mut engine).unwrap();
    let held = runtime.borrow_mut().drain_batch().events.pop().unwrap();
    let failed = compile(
        &mut engine,
        "bad_held_reload.rhai",
        "fn view(ctx){throw \"rejected\";}",
    );
    assert!(lifecycle.reload(&mut engine, failed, &schema).is_err());
    assert!(
        lifecycle
            .invoke_component_event_transactional(&engine, held.clone())
            .unwrap()
            .is_some()
    );
    let replacement = compile(&mut engine, "new_held_reload.rhai", SEMANTIC);
    lifecycle.reload(&mut engine, replacement, &schema).unwrap();
    assert!(
        lifecycle
            .invoke_component_event_transactional(&engine, held)
            .unwrap()
            .is_none()
    );
}

#[test]
fn held_message_created_in_a_rolled_back_outer_transaction_stays_invalid() {
    let (mut engine, compiled) = compile_official("rollback_message.rhai", PURE_ACTION);
    let schema = engine.root_state_schema(&compiled).unwrap();
    let runtime = new_runtime();
    let mut lifecycle = mount(&mut engine, compiled, &runtime);
    let original = runtime.borrow_mut().drain_batch().actions.pop().unwrap();
    let outer = runtime.borrow_mut().begin_transaction().unwrap();
    let _ = lifecycle
        .invoke_action_transactional(&engine, original.clone())
        .unwrap();
    let held = runtime.borrow_mut().drain_batch().actions.pop().unwrap();
    runtime.borrow_mut().rollback_transaction(outer).unwrap();
    let _ = lifecycle
        .invoke_action_transactional(&engine, held.clone())
        .unwrap();
    assert!(runtime.borrow_mut().drain_batch().actions.is_empty());
    // Failed publication does not cancel a committed ancestor from the same
    // still-active source, and later success must not revive its failed child.
    let _ = lifecycle
        .invoke_action_transactional(&engine, original)
        .unwrap();
    assert_eq!(runtime.borrow_mut().drain_batch().actions.len(), 1);
    let _ = lifecycle
        .invoke_action_transactional(&engine, held)
        .unwrap();
    assert!(runtime.borrow_mut().drain_batch().actions.is_empty());
    let replacement = compile(
        &mut engine,
        "rollback_replacement.rhai",
        "fn view(ctx){text(\"normal\")}",
    );
    lifecycle.reload(&mut engine, replacement, &schema).unwrap();
}
