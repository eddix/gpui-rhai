//! AgentQueue uses accepted state, exact pressure intervals, and real controls.

use std::time::Duration;

use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{ScriptViewHandle, UiNode, UiNodeKind};

use super::conversation::{gallery, node, ready};
use crate::support::{Mount, bounds, center, click_center, snapshot};

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let hit = bounds(visual, view, role, name);
    visual.simulate_mouse_move(center(hit), None, Modifiers::none());
    ready(visual, view);
    let hit = bounds(visual, view, role, name);
    click_center(visual, hit);
    ready(visual, view);
}

fn settle_command(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(620));
    ready(visual, view);
}

fn plot(node: &UiNode) -> Option<(&gpui_rhai::CanvasScene, &UiNode)> {
    match node.kind() {
        UiNodeKind::Canvas { scene } if node.key().is_some_and(|key| key.as_str() == "plot") => {
            Some((scene, node))
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(plot)
        }
        _ => None,
    }
}

#[gpui::test]
fn gallery_agent_queue_graph_and_throttle_use_the_same_clock(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_queue", "modern-light");
    visual.simulate_resize(size(px(1280.), px(1040.)));
    ready(&mut visual, &view);
    let chart = snapshot(&mut visual, &view)
        .nodes()
        .find(|n| n.role == "image" && n.name.starts_with("Queue depth ·"))
        .expect("queue chart has a textual equivalent")
        .clone();
    assert!(
        chart.name.contains("2 waiting now") && chart.name.contains("peak 3"),
        "{}",
        chart.name
    );
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let (scene, _) = plot(&root).expect("real canvas path for the same pressure history");
    assert!(scene.commands().iter().any(|c| c.key() == "depth"));
    assert!(scene.commands().iter().any(|c| c.key() == "limit-910000"));
    click(&mut visual, &view, "button", "Limit sample provider");
    node(&mut visual, &view, "status", "Provider cooldown · 30 s");
    click(&mut visual, &view, "button", "Advance demo clock");
    node(&mut visual, &view, "status", "Provider cooldown · 15 s");
    assert!(
        node(&mut visual, &view, "button", "Write release notes")
            .description
            .contains("50%")
    );
    click(&mut visual, &view, "button", "Advance demo clock");
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.role == "status" && n.name.starts_with("Provider cooldown"))
    );
    click(&mut visual, &view, "button", "Advance demo clock");
    assert!(
        node(&mut visual, &view, "button", "Write release notes")
            .description
            .contains("88%")
    );
    visual.simulate_resize(size(px(650.), px(1200.)));
    ready(&mut visual, &view);
    let backlog = bounds(&mut visual, &view, "heading", "Waiting jobs");
    let worker = bounds(&mut visual, &view, "group", "Worker 1");
    assert!(worker.y > backlog.y + 30.);
}

#[gpui::test]
fn gallery_agent_queue_pause_rejection_and_worker_count_are_controlled(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_queue", "clean-dark");
    visual.simulate_resize(size(px(1260.), px(1040.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Decline next command");
    click(&mut visual, &view, "button", "Pause");
    node(&mut visual, &view, "button", "Pause");
    settle_command(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined the command.",
    );
    node(&mut visual, &view, "button", "Pause");

    click(&mut visual, &view, "button", "Pause");
    settle_command(&mut visual, &view);
    node(&mut visual, &view, "status", "Paused");
    click(&mut visual, &view, "button", "Start next sample");
    assert_eq!(
        node(&mut visual, &view, "group", "Waiting").description,
        "2"
    );
    click(&mut visual, &view, "button", "Increase concurrency");
    settle_command(&mut visual, &view);
    node(&mut visual, &view, "group", "Worker 3");
    click(&mut visual, &view, "button", "Resume");
    settle_command(&mut visual, &view);
    click(&mut visual, &view, "button", "Start next sample");
    node(
        &mut visual,
        &view,
        "status",
        "Started sample: Refresh preview",
    );
    assert_eq!(
        node(&mut visual, &view, "group", "Waiting").description,
        "1"
    );
}

#[gpui::test]
fn gallery_agent_queue_keyboard_retry_keeps_historical_failures(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_queue", "clean-light");
    visual.simulate_resize(size(px(1300.), px(1180.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Write release notes");
    visual.simulate_keystrokes("end enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "heading", "Publish draft");
    node(&mut visual, &view, "button", "Retry job");
    click(&mut visual, &view, "button", "Retry job");
    settle_command(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted command: retry");
    assert!(
        node(&mut visual, &view, "button", "Publish draft")
            .description
            .contains("Waiting")
    );
    assert!(
        node(&mut visual, &view, "button", "Publish draft (attempt 1)")
            .description
            .contains("Failed")
    );
    assert_eq!(node(&mut visual, &view, "group", "Failed").description, "2");
    click(&mut visual, &view, "button", "Move to front");
    settle_command(&mut visual, &view);
    click(&mut visual, &view, "button", "Start next sample");
    node(
        &mut visual,
        &view,
        "status",
        "Started sample: Publish draft",
    );
    click(&mut visual, &view, "button", "Cancel job");
    settle_command(&mut visual, &view);
    node(&mut visual, &view, "status", "Accepted command: cancel");
    assert!(
        node(&mut visual, &view, "button", "Publish draft")
            .description
            .contains("Cancelled")
    );
}

#[gpui::test]
fn agent_queue_simultaneous_events_are_coalesced_and_average_is_time_weighted(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = Mount::new(
        r#"
        import "patterns/agent_queue" as q;
        fn view(ctx){
            let now=40000;
            let data=q::pressure_snapshot([
                #{enqueued_ms:0,started_ms:20000,status:"done"},
                #{enqueued_ms:20000,status:"queued"},
                #{enqueued_ms:25000,ended_ms:30000,status:"cancelled"},
                #{enqueued_ms:20000,ended_ms:20000,status:"cancelled"},
            ],now,40000);
            let ranges=q::limit_ranges([#{from:10000,until_ms:21000},#{from:15000,until_ms:30000}]);
            let same=data.peak==2&&data.depth==1&&data.points.len==4&&data.points[1].ms==25000&&data.points[2].ms==30000
                &&(data.average-1.125).abs()<0.0001&&ranges.len==1&&q::duration_without_limits(5000,35000,ranges)==10000;
            text(if same{"Exact queue history"}else{data.to_string()}).accessibility_role("status")
        }
    "#,
    )
    .registry("patterns/agent_queue")
    .mount(cx);
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Exact queue history");
}
