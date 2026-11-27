//! AgentInbox flows through native input and the Gallery's controlled Host.

use std::time::{Duration, Instant};

use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ManualRuntimeClock, MotionPreference, ScriptViewHandle};

use super::conversation::{gallery, node, ready};
use crate::support::{Mount, bounds, center, click_center, snapshot};

fn press(visual: &mut VisualTestContext, view: &ScriptViewHandle, label: &str) {
    let at = bounds(visual, view, "button", label);
    visual.simulate_mouse_move(center(at), None, Modifiers::none());
    ready(visual, view);
    let at = bounds(visual, view, "button", label);
    click_center(visual, at);
    ready(visual, view);
}
fn later(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(620));
    ready(visual, view);
}
fn selected(visual: &mut VisualTestContext, view: &ScriptViewHandle, label: &str) -> bool {
    snapshot(visual, view)
        .find_by_role_and_name("button", label)
        .any(|n| n.pressed == Some(true))
}
const PLAN: &str = "Review rollout plan";

#[gpui::test]
fn gallery_agent_inbox_view_acceptance_retains_disclosure_and_real_focus(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_inbox", "modern-light");
    visual.simulate_resize(size(px(1320.), px(1140.)));
    ready(&mut visual, &view);
    let needs = bounds(&mut visual, &view, "group", "Needs you");
    let running = bounds(&mut visual, &view, "group", "Running");
    assert!(running.x > needs.x + needs.width - 1.0);
    press(&mut visual, &view, PLAN);
    node(
        &mut visual,
        &view,
        "listitem",
        "Prepared the rollback evidence",
    );
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("listitem", "Fetched draft")
            .next()
            .is_none()
    );
    let original = node(&mut visual, &view, "button", PLAN).id;
    visual.simulate_keystrokes("l");
    ready(&mut visual, &view);
    assert!(
        selected(&mut visual, &view, "Board"),
        "controlled view must not change on a proposal"
    );
    later(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted list view.");
    assert!(selected(&mut visual, &view, "List"));
    let needs = bounds(&mut visual, &view, "group", "Needs you");
    let running = bounds(&mut visual, &view, "group", "Running");
    assert!(running.y > needs.y + needs.height - 1.);
    let kept = node(&mut visual, &view, "button", PLAN);
    assert_eq!(kept.id, original);
    assert_eq!(kept.expanded, Some(true));
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "button", PLAN).expanded,
        Some(false),
        "Enter still goes to the same native task"
    );
    visual.simulate_keystrokes("b");
    later(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted board view.");
    visual.simulate_resize(size(px(650.), px(1160.)));
    ready(&mut visual, &view);
    assert!(selected(&mut visual, &view, "Board"));
    let needs = bounds(&mut visual, &view, "group", "Needs you");
    let running = bounds(&mut visual, &view, "group", "Running");
    assert!(running.y > needs.y + needs.height - 1.);
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "button", PLAN).expanded,
        Some(true)
    );
}

#[gpui::test]
fn gallery_agent_inbox_decline_then_accept_moves_only_the_target_task(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_inbox", "clean-dark");
    visual.simulate_resize(size(px(1290.), px(1090.)));
    ready(&mut visual, &view);
    press(&mut visual, &view, PLAN);
    press(&mut visual, &view, "Decline next request");
    press(&mut visual, &view, "Approve");
    node(&mut visual, &view, "status", "Waiting for the demo Host…");
    assert!(
        node(&mut visual, &view, "button", PLAN)
            .description
            .contains("Waiting for you")
    );
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined the request.",
    );
    assert_eq!(
        node(&mut visual, &view, "group", "Needs you").description,
        "2"
    );
    press(&mut visual, &view, "Approve");
    later(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted approve: gate/plan");
    assert_eq!(
        node(&mut visual, &view, "group", "Needs you").description,
        "1"
    );
    assert_eq!(
        node(&mut visual, &view, "group", "Running").description,
        "2"
    );
    assert_eq!(
        node(&mut visual, &view, "button", PLAN).expanded,
        Some(true)
    );
    node(
        &mut visual,
        &view,
        "listitem",
        "The Host approved this task.",
    );
    press(&mut visual, &view, "Open task");
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Host received open: gate/plan",
    );
}

#[gpui::test]
fn gallery_agent_inbox_clock_retry_and_lane_filter_use_host_data(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_inbox", "clean-light");
    visual.simulate_resize(size(px(1320.), px(1130.)));
    ready(&mut visual, &view);
    let before = node(&mut visual, &view, "button", "Archive notes").description;
    assert!(before.contains("1 min 11 s"));
    assert!(
        node(&mut visual, &view, "button", "Stage artifact")
            .description
            .contains("23 s")
    );
    press(&mut visual, &view, "Advance demo clock");
    assert!(
        node(&mut visual, &view, "button", "Stage artifact")
            .description
            .contains("38 s")
    );
    assert_eq!(
        node(&mut visual, &view, "button", "Archive notes").description,
        before
    );
    press(&mut visual, &view, "Show only Needs you");
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("group", "Running")
            .next()
            .is_none()
    );
    press(&mut visual, &view, "Recover preview");
    press(&mut visual, &view, "Retry job");
    later(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted retry: recover");
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("button", "Recover preview")
            .next()
            .is_none()
    );
    press(&mut visual, &view, PLAN);
    press(&mut visual, &view, "Reject");
    later(&mut visual, &view);
    assert!(
        node(&mut visual, &view, "button", PLAN)
            .description
            .contains("Failed")
    );
    press(&mut visual, &view, "New task");
    later(&mut visual, &view);
    node(&mut visual, &view, "button", "New sample 1");
    press(&mut visual, &view, "Show all lanes");
    assert!(
        node(&mut visual, &view, "button", "Recover preview")
            .description
            .contains("Running")
    );
}

#[gpui::test]
fn agent_inbox_accepted_task_uses_shared_layout_motion_without_hiding(cx: &mut TestAppContext) {
    const SCRIPT: &str = r#"
        import "patterns/agent_inbox" as inbox;
        fn state_schema(){#{fields:#{running:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn accept(ctx,p){if p.inbox=="motion"&&p.task=="review"&&p.revision==0&&p.kind=="approve"{ctx.set_state("running",true);}}
        fn view(ctx){
            let changed=ctx.get_state("running");
            inbox::AgentInbox(#{key:"one",inbox:"motion",now_ms:2000,revision:if changed{1}else{0},
                tasks:[#{id:"review",title:"Inspect fixture",agent:"Local assistant",started_ms:1000,
                    status:if changed{"running"}else{"waiting"}}],on_action:Fn("accept")})
        }
    "#;
    for motion in [MotionPreference::Normal, MotionPreference::None] {
        let manual = ManualRuntimeClock::new(Instant::now());
        let clock = manual.clock();
        let (mut visual, view) = Mount::new(SCRIPT)
            .registry("patterns/agent_inbox")
            .motion(motion)
            .with(move |view| view.runtime_clock(clock))
            .mount(cx);
        visual.simulate_resize(size(px(1080.), px(920.)));
        ready(&mut visual, &view);
        press(&mut visual, &view, "Inspect fixture");
        let original = bounds(&mut visual, &view, "button", "Inspect fixture");
        press(&mut visual, &view, "Approve");
        let start = bounds(&mut visual, &view, "button", "Inspect fixture");
        manual.advance(Duration::from_millis(85));
        ready(&mut visual, &view);
        let middle = bounds(&mut visual, &view, "button", "Inspect fixture");
        manual.advance(Duration::from_millis(800));
        ready(&mut visual, &view);
        let end = bounds(&mut visual, &view, "button", "Inspect fixture");
        assert!(end.x > original.x + 220.);
        assert!(middle.width > 150. && middle.height > 12.);
        if motion == MotionPreference::Normal {
            assert!(
                start.x + 1. < middle.x && middle.x < end.x - 1.,
                "layout should transit actual screen positions, got {start:?}, {middle:?}, {end:?}"
            );
        } else {
            assert!((start.x - end.x).abs() < 1. && (middle.x - end.x).abs() < 1.);
        }
    }
}
