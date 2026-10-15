//! Post-commit effect state uses foreground delivery, with one bounded cascade.
mod common;

use common::{compile_official, mount, new_runtime};
use gpui_rhai::{LifecycleError, UiNodeKind, UiValue};

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
