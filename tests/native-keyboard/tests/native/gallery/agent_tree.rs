use std::time::Duration;

use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{
    CanvasCommand, CanvasPathSegment, CanvasScene, PrimitiveValue, ScriptViewHandle, UiNode,
    UiNodeKind,
};

use super::conversation::{gallery, node, ready};
use crate::support::{Mount, bounds, center, click_center, last_error, snapshot};

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let rect = bounds(visual, view, role, name);
    visual.simulate_mouse_move(center(rect), None, Modifiers::none());
    ready(visual, view);
    let rect = bounds(visual, view, role, name);
    click_center(visual, rect);
    ready(visual, view);
}

fn later(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(620));
    ready(visual, view);
}

fn absent(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    assert!(
        snapshot(visual, view)
            .find_by_role_and_name(role, name)
            .next()
            .is_none(),
        "unexpected {role} {name}"
    );
}

#[gpui::test]
fn gallery_agent_tree_keyboard_folds_only_after_host_acceptance_and_keeps_totals(
    cx: &mut TestAppContext,
) {
    let (mut v, view) = gallery(cx, "agent_tree", "modern-light");
    v.simulate_resize(size(px(1450.), px(1220.)));
    later(&mut v, &view);
    let original = node(&mut v, &view, "treeitem", "Coordinator");
    assert!(
        original.description.contains("Branch: 0.43 USD"),
        "{original:?}"
    );
    node(&mut v, &view, "status", "Run total: 0.45 USD");
    click(&mut v, &view, "button", "Refuse next fold");
    click(&mut v, &view, "treeitem", "Build");
    v.simulate_keystrokes("left");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "The Host kept the old fold state.");
    node(&mut v, &view, "treeitem", "Visual checks");
    v.simulate_keystrokes("left");
    later(&mut v, &view);
    absent(&mut v, &view, "treeitem", "Visual checks");
    assert_eq!(
        node(&mut v, &view, "treeitem", "Build").expanded,
        Some(false)
    );
    let coordinator = node(&mut v, &view, "treeitem", "Coordinator");
    assert_eq!(coordinator.description, original.description);
    node(&mut v, &view, "status", "Run total: 0.45 USD");
    node(&mut v, &view, "button", "Failed: 1");
    v.simulate_keystrokes("right");
    later(&mut v, &view);
    assert_eq!(
        node(&mut v, &view, "treeitem", "Build").expanded,
        Some(true)
    );
    v.simulate_keystrokes("right enter");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Selected visual");
    assert_eq!(
        node(&mut v, &view, "treeitem", "Visual checks").selected,
        Some(true)
    );
}

#[gpui::test]
fn gallery_agent_tree_review_reaches_folded_agents_and_waits_for_host(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_tree", "clean-light");
    v.simulate_resize(size(px(1380.), px(1230.)));
    later(&mut v, &view);
    click(&mut v, &view, "button", "Switch Build fold externally");
    absent(&mut v, &view, "treeitem", "Visual checks");
    click(&mut v, &view, "button", "Refuse next resolution");
    click(&mut v, &view, "button", "Failed: 1");
    node(&mut v, &view, "dialog", "Agents that failed");
    node(
        &mut v,
        &view,
        "text",
        "The light palette snapshot has changed.",
    );
    click(&mut v, &view, "button", "Retry Visual checks");
    node(&mut v, &view, "status", "Waiting for the demo Host…");
    assert!(node(&mut v, &view, "button", "Retry Visual checks").disabled);
    later(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "The demo Host refused the resolution.",
    );
    node(&mut v, &view, "dialog", "Agents that failed");
    click(&mut v, &view, "button", "Retry Visual checks");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Accepted retry for visual");
    absent(&mut v, &view, "button", "Failed: 1");
    absent(&mut v, &view, "treeitem", "Visual checks");
    click(&mut v, &view, "button", "Needs you: 1");
    node(&mut v, &view, "dialog", "Agents that need you");
    click(&mut v, &view, "button", "Approve Build");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Accepted approve for build");
    absent(&mut v, &view, "button", "Needs you: 1");
}

fn wires(n: &UiNode) -> Option<&CanvasScene> {
    match n.kind() {
        UiNodeKind::Canvas { scene } if n.key().is_some_and(|k| k.as_str() == "agent-links") => {
            Some(scene)
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(wires)
        }
        UiNodeKind::Custom { primitive } => primitive.props.iter().find_map(|(_, p)| match p {
            PrimitiveValue::Node(child) => wires(child),
            PrimitiveValue::Nodes(children) => children.iter().find_map(wires),
            _ => None,
        }),
        _ => None,
    }
}

fn assert_wires(v: &mut VisualTestContext, view: &ScriptViewHandle, wide: bool) {
    let stage = bounds(v, view, "group", "Dispatch hierarchy");
    let root = bounds(v, view, "treeitem", "Coordinator");
    let child = bounds(v, view, "treeitem", "Build");
    let peer = bounds(v, view, "treeitem", "Supervisor");
    let node = v.update(|_, cx| view.root(cx).unwrap().unwrap());
    let scene = wires(&node).expect("presented Canvas");
    let CanvasCommand::Path { segments, .. } = scene
        .commands()
        .iter()
        .find(|c| c.key() == "edge-3")
        .expect("the Build agent's link")
    else {
        panic!("Expected a branch path");
    };
    if wide {
        let [
            CanvasPathSegment::Move { x, y },
            CanvasPathSegment::Cubic { x: tx, y: ty, .. },
        ] = segments.as_slice()
        else {
            panic!("Wide layout must connect centers with a curve: {segments:?}");
        };
        assert!((stage.x + x - root.x - root.width / 2.0).abs() < 2.0);
        assert!((stage.y + y - root.y - root.height).abs() < 2.0);
        assert!((stage.x + tx - child.x - child.width / 2.0).abs() < 2.0);
        assert!((stage.y + ty - child.y).abs() < 2.0);
        assert!((peer.y - root.y).abs() < 4.0);
    } else {
        let [
            CanvasPathSegment::Move { x, y },
            CanvasPathSegment::Line { .. },
            CanvasPathSegment::Line { .. },
            CanvasPathSegment::Line { x: tx, y: ty },
        ] = segments.as_slice()
        else {
            panic!("Narrow layout must connect side edges: {segments:?}");
        };
        assert!((stage.x + x - root.x).abs() < 2.0);
        assert!((stage.y + y - root.y - root.height / 2.0).abs() < 2.0);
        assert!((stage.x + tx - child.x).abs() < 2.0);
        assert!((stage.y + ty - child.y - child.height / 2.0).abs() < 2.0);
        assert!(peer.y >= root.y + root.height);
    }
    assert!(
        scene
            .commands()
            .iter()
            .any(|c| c.key().starts_with("watch-")),
        "supervisor has a dashed relation, not a child edge"
    );
    assert!(scene.commands().iter().any(|c| c.key() == "joint-0"));
}

#[gpui::test]
fn gallery_agent_tree_switches_real_geometry_and_preserves_card_focus(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_tree", "clean-dark");
    v.simulate_resize(size(px(1410.), px(1240.)));
    later(&mut v, &view);
    assert_wires(&mut v, &view, true);
    click(&mut v, &view, "treeitem", "Build");
    v.simulate_resize(size(px(645.), px(1250.)));
    later(&mut v, &view);
    assert_wires(&mut v, &view, false);
    v.simulate_keystrokes("right enter");
    later(&mut v, &view);
    node(&mut v, &view, "status", "Selected visual");
}

#[gpui::test]
fn gallery_agent_tree_async_fold_recovers_focus_to_visible_parent(cx: &mut TestAppContext) {
    let (mut v, view) = gallery(cx, "agent_tree", "clean-dark");
    v.simulate_resize(size(px(1440.), px(1230.)));
    later(&mut v, &view);
    click(&mut v, &view, "button", "Schedule Build fold");
    click(&mut v, &view, "treeitem", "Visual checks");
    later(&mut v, &view);
    node(
        &mut v,
        &view,
        "status",
        "The Host folded Build asynchronously.",
    );
    absent(&mut v, &view, "treeitem", "Visual checks");
    v.simulate_keystrokes("enter");
    ready(&mut v, &view);
    node(&mut v, &view, "status", "Selected build");
}

#[gpui::test]
fn agent_tree_rejects_disconnected_cycles_without_replacing_last_good_ui(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::new(
        r#"
        import "patterns/agent_tree" as tree;
        fn state_schema(){#{fields:#{cycle:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn corrupt(ctx,event){ctx.set_state("cycle",true);handled()}
        fn view(ctx){
            let items=[#{id:"root",name:"Lead",task:"Run preview",state:"running"},
                #{id:"worker",parent:if ctx.get_state("cycle"){"review"}else{"root"},name:"Worker",task:"Build preview",state:"queued"},
                #{id:"review",parent:"worker",name:"Review",task:"Check preview",state:"queued"}];
            column([text("Propose cycle").with_key("corrupt").accessibility_role("button").on_click(Fn("corrupt")),
                tree::AgentTree(#{key:"org",run:"acceptance",revision:0,now_ms:0,agents:items})])
        }
        "#,
    )
    .registry("patterns/agent_tree")
    .mount(cx);
    v.simulate_resize(size(px(780.), px(900.)));
    later(&mut v, &view);
    let good = node(&mut v, &view, "treeitem", "Worker");
    let target = bounds(&mut v, &view, "button", "Propose cycle");
    click_center(&mut v, target);
    assert!(
        last_error(&mut v, &view)
            .expect("cycle must be rejected")
            .contains("must all reach the dispatch root")
    );
    assert_eq!(node(&mut v, &view, "treeitem", "Worker").id, good.id);
}
