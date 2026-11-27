//! Real native focus, retained workflow boundaries, and scoped submission.
use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{MotionPreference, ScriptViewHandle, UiValue};

use crate::support::{Mount, Step, bounds, click_center, last_error, settle, snapshot, text_of};

fn mount(cx: &mut TestAppContext, script: &str) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(script)
        .theme(gpui_rhai_registry::CLEAN_LIGHT_THEME)
        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
        .motion(MotionPreference::None)
        .mount(cx);
    visual.update(|window, _| window.activate_window());
    ready(&mut visual, &view);
    (visual, view)
}

fn ready(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    settle(visual, Step::Tick, 6);
    assert_eq!(last_error(visual, view), None);
}

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, label: &str) {
    let hit = bounds(visual, view, "button", label);
    click_center(visual, hit);
    ready(visual, view);
}

#[gpui::test]
fn stepper_form_focuses_the_first_invalid_control_in_the_active_pane(cx: &mut TestAppContext) {
    let script = r#"
        import "components/stepper_form" as stepper;
        import "components/input" as input;
        import "components/form_field" as field;
        fn state_schema(){#{fields:#{
            first:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
            second:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
            attempted:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}
        }}}
        fn write(key,ctx,value){ctx.set_state(key,value);}
        fn invalid(ctx,value){ctx.set_state("attempted",true);}
        fn view(ctx){
            let first=ctx.get_state("first");let second=ctx.get_state("second");
            let error=if second==""{"Enter the contact."}else{()};
            let shown=if ctx.get_state("attempted"){error}else{()};
            let pane=column([
                overlay(text("Optional popover"),
                    input::Input(#{key:"dormant-entry",label:"Dormant invalid field",value:first,error:true,
                        on_change:Fn("write").curry("first")}),
                    #{id:"dormant",kind:"popover",open:false,placement:"bottom",align:"start"}).with_key("dormant"),
                input::Input(#{key:"hidden-entry",label:"Hidden invalid field",value:first,error:true,
                    on_change:Fn("write").curry("first")}).with_style(style().hidden()),
                input::Input(#{key:"team-entry",label:"Team",value:first,on_change:Fn("write").curry("first")}),
                field::FormField(#{id:"contact",label:"Contact",error:shown,
                    control:input::Input(#{key:"contact-entry",label:"Contact",value:second,error:shown!=(),
                        on_change:Fn("write").curry("second")})})
            ]).with_style(style().gap(px(12)));
            column([
                stepper::StepperForm(#{key:"setup",steps:[
                    #{id:"details",title:"Details",content:pane,error:error},
                    #{id:"confirm",title:"Confirm",content:text("Confirm details")}
                ],on_invalid:Fn("invalid")}),
                text(`${first}|${second}`).test_id("values")
            ])
        }
    "#;
    let (mut visual, view) = mount(cx, script);
    visual.simulate_resize(size(px(346.), px(640.)));
    ready(&mut visual, &view);
    let tree = snapshot(&mut visual, &view);
    assert!(tree.nodes().any(|n| n.name == "Step 1 of 2 · Details"));
    assert!(!tree.nodes().any(|n| n.name == "Confirm details"));
    click(&mut visual, &view, "Continue");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "text_field" && n.name == "Contact" && n.invalid)
    );
    visual.simulate_input("Alex");
    ready(&mut visual, &view);
    assert_eq!(text_of(&mut visual, &view, "values"), "|Alex");
    visual.simulate_resize(size(px(730.), px(640.)));
    ready(&mut visual, &view);
    visual.simulate_input(" Rivera");
    ready(&mut visual, &view);
    assert_eq!(text_of(&mut visual, &view, "values"), "|Alex Rivera");
    let stage_one = bounds(&mut visual, &view, "listitem", "Details");
    let stage_two = bounds(&mut visual, &view, "listitem", "Confirm");
    assert!(
        stage_one.width > 160.0 && (stage_one.width - stage_two.width).abs() < 1.5,
        "{stage_one:?} {stage_two:?}"
    );
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Confirm details")
    );
}

#[gpui::test]
fn stepper_form_uses_only_the_committed_controlled_step(cx: &mut TestAppContext) {
    let script = r#"
        import "components/stepper_form" as stepper;
        fn state_schema(){#{fields:#{
            step:#{schema:#{type:"string"},"default":#{type:"string",value:"start"}},
            allowed:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            proposals:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
        }}}
        fn permit(ctx,payload){ctx.set_state("allowed",true);}
        fn change(ctx,p){
            ctx.set_state("proposals",ctx.get_state("proposals")+1);
            if ctx.get_state("allowed"){ctx.set_state("step",p.step);}
        }
        fn view(ctx){column([
            stepper::StepperForm(#{key:"control",step:ctx.get_state("step"),steps:[
                #{id:"start",title:"Start",content:text("Draft contents")},
                #{id:"end",title:"End",content:text("Confirmed contents")}
            ],on_step_change:Fn("change")}),
            row([text("Accept navigation")]).accessibility_role("button").on_click_value(Fn("permit"),()),
            text(`${ctx.get_state("step")}:${ctx.get_state("proposals")}`).test_id("state")
        ])}
    "#;
    let (mut visual, view) = mount(cx, script);
    click(&mut visual, &view, "Continue");
    assert_eq!(text_of(&mut visual, &view, "state"), "start:1");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Confirmed contents")
    );
    click(&mut visual, &view, "Accept navigation");
    click(&mut visual, &view, "Continue");
    assert_eq!(text_of(&mut visual, &view, "state"), "end:2");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Confirmed contents")
    );
}

#[gpui::test]
fn stepper_form_locks_pending_submit_and_can_retry_a_host_failure(cx: &mut TestAppContext) {
    let script = r#"
        import "components/stepper_form" as stepper;
        fn state_schema(){#{fields:#{
            status:#{schema:#{type:"string"},"default":#{type:"string",value:"editing"}},
            count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
            request:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
        }}}
        fn submit(ctx,p){
            ctx.set_state("status","submitting");ctx.set_state("request",p.request);
            ctx.set_state("count",ctx.get_state("count")+1);
        }
        fn result(ctx,status){ctx.set_state("status",status);}
        fn view(ctx){column([
            stepper::StepperForm(#{key:"save",status:ctx.get_state("status"),submission_error:"Server rejected this draft",
                steps:[#{id:"check",title:"Check",content:text("Ready to save")}],on_submit:Fn("submit")}),
            row([text("Reject operation")]).accessibility_role("button").on_click_value(Fn("result"),"failed"),
            row([text("Complete operation")]).accessibility_role("button").on_click_value(Fn("result"),"succeeded"),
            text(`${ctx.get_state("count")}:${ctx.get_state("request")}`).test_id("calls")
        ])}
    "#;
    let (mut visual, view) = mount(cx, script);
    click(&mut visual, &view, "Finish");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Submitting…" && n.disabled)
    );
    click(&mut visual, &view, "Submitting…");
    visual.simulate_keystrokes("enter enter");
    ready(&mut visual, &view);
    assert_eq!(text_of(&mut visual, &view, "calls"), "1:1");
    click(&mut visual, &view, "Reject operation");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "alert" && n.name == "Server rejected this draft")
    );
    click(&mut visual, &view, "Finish");
    assert_eq!(text_of(&mut visual, &view, "calls"), "2:2");
    click(&mut visual, &view, "Complete operation");
    let tree = snapshot(&mut visual, &view);
    assert!(tree.nodes().any(|n| n.name == "All set"));
    assert!(!tree.nodes().any(|n| n.name == "Ready to save"));
    assert!(
        !tree
            .nodes()
            .any(|n| n.role == "button" && n.name == "Finish")
    );
    assert!(
        !tree
            .nodes()
            .any(|n| n.value == Some(UiValue::String("Server rejected this draft".into())))
    );
}

#[gpui::test]
fn stepper_form_moves_the_new_pane_and_footer_on_the_shared_clock(cx: &mut TestAppContext) {
    let script = r#"
        import "components/stepper_form" as stepper;
        fn finish(ctx,payload){}
        fn view(ctx){
            stepper::StepperForm(#{key:"motion",on_submit:Fn("finish"),steps:[
                #{id:"short",title:"Short",content:row([text("Short content")]).with_style(style().height(px(32)))},
                #{id:"tall",title:"Tall",heading:"Taller pane",content:row([text("Tall content")]).with_style(style().height(px(190)))},
            ]})
        }
    "#;
    let (mut visual, view) = Mount::components(script)
        .theme(gpui_rhai_registry::MODERN_DARK_THEME)
        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
        .motion(MotionPreference::Normal)
        .mount(cx);
    visual.update(|window, _| window.activate_window());
    ready(&mut visual, &view);
    let before = bounds(&mut visual, &view, "button", "Continue");
    click_center(&mut visual, before);
    settle(&mut visual, Step::Tick, 2);
    let mid = bounds(&mut visual, &view, "button", "Finish");
    let entering = bounds(&mut visual, &view, "heading", "Taller pane");
    settle(&mut visual, Step::Tick, 16);
    let settled = bounds(&mut visual, &view, "button", "Finish");
    let heading = bounds(&mut visual, &view, "heading", "Taller pane");
    assert!(
        settled.y > before.y + 140.0 && mid.y < settled.y - 1.0,
        "from {before:?}, mid {mid:?}, to {settled:?}"
    );
    assert!(
        entering.x > heading.x + 0.5,
        "entering {entering:?}, settled {heading:?}"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}
