//! Controlled task ownership and disclosure tested through actual native hits and timers.
use std::time::Duration;

use gpui::{TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{MotionPreference, ScriptViewHandle, UiValue};

use crate::support::{Mount, Step, bounds, click_center, last_error, settle, snapshot, text_of};

fn mount(
    cx: &mut TestAppContext,
    script: &str,
    motion: MotionPreference,
) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(script)
        .theme(gpui_rhai_registry::CLEAN_LIGHT_THEME)
        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
        .motion(motion)
        .mount(cx);
    visual.update(|window, _| window.activate_window());
    ready(&mut visual, &view);
    (visual, view)
}

fn ready(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    settle(visual, Step::Tick, 4);
    assert_eq!(last_error(visual, view), None);
}

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let target = bounds(visual, view, role, name);
    click_center(visual, target);
    ready(visual, view);
}

fn completed(visual: &mut VisualTestContext, view: &ScriptViewHandle, title: &str) -> UiValue {
    snapshot(visual, view)
        .find_by_role_and_name("progressbar", title)
        .next()
        .unwrap_or_else(|| panic!("missing meter {title}"))
        .value
        .clone()
        .unwrap()
}

fn task(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    title: &str,
) -> gpui_rhai::AccessibilityNode {
    snapshot(visual, view)
        .find_by_role_and_name("checkbox", title)
        .next()
        .unwrap_or_else(|| panic!("missing task {title}"))
        .clone()
}

#[gpui::test]
fn progress_tracker_visual_value_changes_only_on_host_acceptance(cx: &mut TestAppContext) {
    let app = r#"
        import "components/progress_tracker" as tracker;
        fn state_schema(){#{fields:#{
            permit:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            calls:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
            done:#{schema:#{type:"array",items:#{type:"string"}},"default":#{type:"array",value:[]}}
        }}}
        fn change(ctx,done){
            ctx.set_state("calls", ctx.get_state("calls")+1);
            if ctx.get_state("permit"){ctx.set_state("done",done);}
        }
        fn permit(ctx,payload){ctx.set_state("permit",true);}
        fn view(ctx){column([
            tracker::ProgressTracker(#{key:"tracker",title:"Checklist",value:ctx.get_state("done"),
                steps:[#{id:"a",label:"Build"},#{id:"b",label:"Test",disabled:true}],on_change:Fn("change")}),
            row([text("Allow changes")]).accessibility_role("button").on_click_value(Fn("permit"),()),
            text(`${ctx.get_state("calls")}`).test_id("calls")
        ])}
    "#;
    let (mut visual, view) = mount(cx, app, MotionPreference::None);
    click(&mut visual, &view, "checkbox", "Build");
    assert_eq!(
        completed(&mut visual, &view, "Checklist"),
        UiValue::Integer(0)
    );
    assert_eq!(text_of(&mut visual, &view, "calls"), "1");
    click(&mut visual, &view, "button", "Allow changes");
    click(&mut visual, &view, "checkbox", "Build");
    assert_eq!(
        completed(&mut visual, &view, "Checklist"),
        UiValue::Integer(1)
    );
    assert!(task(&mut visual, &view, "Test").disabled);
    click(&mut visual, &view, "checkbox", "Test");
    assert_eq!(text_of(&mut visual, &view, "calls"), "2");
}

#[gpui::test]
fn progress_tracker_playback_stops_after_interaction(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"
        import "components/progress_tracker" as tracker;
        fn view(ctx){tracker::ProgressTracker(#{key:"demo",title:"Playback",autoplay:true,interval_ms:500,
            steps:[#{id:"one",label:"First"},#{id:"two",label:"Second"},#{id:"three",label:"Third"}]})}
    "#,
        MotionPreference::None,
    );
    assert_eq!(
        completed(&mut visual, &view, "Playback"),
        UiValue::Integer(0)
    );
    visual.executor().advance_clock(Duration::from_millis(520));
    ready(&mut visual, &view);
    assert_eq!(
        completed(&mut visual, &view, "Playback"),
        UiValue::Integer(1)
    );
    click(&mut visual, &view, "checkbox", "Third");
    visual.executor().advance_clock(Duration::from_secs(3));
    ready(&mut visual, &view);
    assert_eq!(
        completed(&mut visual, &view, "Playback"),
        UiValue::Integer(2)
    );
    assert_eq!(
        task(&mut visual, &view, "Second").checked,
        Some(UiValue::Bool(false))
    );
}

const GUIDE: &str = r#"
import "components/setup_guide" as setup;
fn state_schema(){#{fields:#{
    done:#{schema:#{type:"array",items:#{type:"string"}},"default":#{type:"array",value:[]}},
    permit:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    calls:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}
}}}
fn respond(ctx,done){
    ctx.set_state("calls",ctx.get_state("calls")+1);
    if ctx.get_state("permit"){ctx.set_state("done",done);}
}
fn permit(ctx,payload){ctx.set_state("permit",true);}
fn view(ctx){column([
    setup::SetupGuide(#{key:"guide",title:"Guide",value:ctx.get_state("done"),on_change:Fn("respond"),groups:[
        #{id:"one",title:"Workspace",tasks:[#{id:"a",label:"Name workspace"},#{id:"b",label:"Invite people"}]},
        #{id:"two",title:"Publishing",tasks:[#{id:"c",label:"Publish a page"}]},
        #{id:"three",title:"Later",tasks:[#{id:"d",label:"Choose a theme"}]},
    ]}),
    row([text("Accept changes")]).accessibility_role("button").on_click_value(Fn("permit"),()),
    text(`${ctx.get_state("calls")}`).test_id("calls")
])}
"#;

#[gpui::test]
fn setup_guide_rejected_toggle_does_not_unlock_and_accepted_handoff_advances(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = mount(cx, GUIDE, MotionPreference::None);
    click(&mut visual, &view, "checkbox", "Name workspace");
    assert!(task(&mut visual, &view, "Invite people").disabled);
    assert_eq!(completed(&mut visual, &view, "Guide"), UiValue::Integer(0));
    click(&mut visual, &view, "button", "Accept changes");
    click(&mut visual, &view, "checkbox", "Name workspace");
    assert!(!task(&mut visual, &view, "Invite people").disabled);
    click(&mut visual, &view, "checkbox", "Invite people");
    assert_eq!(completed(&mut visual, &view, "Guide"), UiValue::Integer(2));
    assert!(!task(&mut visual, &view, "Publish a page").disabled);
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Invite people")
    );
    click(&mut visual, &view, "button", "Minimize setup guide");
    assert_eq!(completed(&mut visual, &view, "Guide"), UiValue::Integer(2));
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Publish a page")
    );
    click(&mut visual, &view, "button", "Expand setup guide");
    assert!(!task(&mut visual, &view, "Publish a page").disabled);
}

#[gpui::test]
fn setup_guide_manual_disclosure_supersedes_pending_handoff(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, GUIDE, MotionPreference::Normal);
    visual.simulate_resize(size(px(258.), px(650.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Accept changes");
    click(&mut visual, &view, "checkbox", "Name workspace");
    click(&mut visual, &view, "checkbox", "Invite people");
    click(&mut visual, &view, "button", "Later");
    visual.executor().advance_clock(Duration::from_millis(800));
    ready(&mut visual, &view);
    assert!(!task(&mut visual, &view, "Choose a theme").disabled);
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Publish a page")
    );
    assert_eq!(completed(&mut visual, &view, "Guide"), UiValue::Integer(2));
}
