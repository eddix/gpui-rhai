use std::time::Duration;

use gpui::{Modifiers, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{
    CanvasCommand, CanvasPathSegment, CanvasScene, PrimitiveValue, ScriptViewHandle, UiNode,
    UiNodeKind,
};

use super::conversation::{gallery, node, ready};
use crate::support::{Mount, bounds, center, click_center, snapshot};

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let rect = bounds(visual, view, role, name);
    visual.simulate_mouse_move(center(rect), None, Modifiers::none());
    ready(visual, view);
    let rect = bounds(visual, view, role, name);
    click_center(visual, rect);
    ready(visual, view);
}

fn later(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    visual.executor().advance_clock(Duration::from_millis(540));
    ready(visual, view);
}

fn rail(node: &UiNode) -> Option<&CanvasScene> {
    match node.kind() {
        UiNodeKind::Canvas { scene } if node.key().is_some_and(|key| key.as_str() == "rail") => {
            Some(scene)
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(rail)
        }
        UiNodeKind::Custom { primitive } => {
            primitive.props.iter().find_map(|(_, prop)| match prop {
                PrimitiveValue::Node(node) => rail(node),
                PrimitiveValue::Nodes(nodes) => nodes.iter().find_map(rail),
                _ => None,
            })
        }
        _ => None,
    }
}

fn verify_branch(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    let stage = bounds(visual, view, "group", "Checkpoint history");
    let parent = bounds(visual, view, "listitem", "Read the brief");
    let children: Vec<_> = snapshot(visual, view)
        .nodes()
        .filter(|n| n.role == "listitem" && n.name == "Build preview")
        .filter_map(|n| n.geometry.map(|g| g.visual))
        .collect();
    assert_eq!(children.len(), 2);
    let branch = children
        .iter()
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .expect("second chronological preview");
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let scene = rail(&root).expect("presented history rail");
    let CanvasCommand::Path { segments, .. } = scene
        .commands()
        .iter()
        .find(|c| c.key() == "link-3")
        .expect("fork from Read the brief to the second preview")
    else {
        panic!("branch should be a curved path");
    };
    let [
        CanvasPathSegment::Move { x, y },
        CanvasPathSegment::Cubic { .. },
        CanvasPathSegment::Line { x: to_x, y: to_y },
    ] = segments.as_slice()
    else {
        panic!("incorrect branch segments: {segments:?}");
    };
    assert!((stage.y + y - parent.y - parent.height * 0.5).abs() < 2.0);
    assert!((stage.y + to_y - branch.y - branch.height * 0.5).abs() < 2.0);
    assert!(to_x - x > 8.0, "branch needs a different lane");
}

#[gpui::test]
fn gallery_agent_checkpoints_rail_tracks_presented_rows_across_resize(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_checkpoints", "modern-light");
    visual.simulate_resize(size(px(1250.), px(950.)));
    later(&mut visual, &view);
    verify_branch(&mut visual, &view);
    let wide = bounds(&mut visual, &view, "group", "Checkpoint history");
    visual.simulate_resize(size(px(620.), px(1090.)));
    later(&mut visual, &view);
    let narrow = bounds(&mut visual, &view, "group", "Checkpoint history");
    assert!(narrow.width < wide.width);
    verify_branch(&mut visual, &view);
}

#[gpui::test]
fn gallery_agent_checkpoints_keyboard_and_rejection_keep_exact_id(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_checkpoints", "clean-dark");
    visual.simulate_resize(size(px(1250.), px(990.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Decline next rewind");
    click(&mut visual, &view, "listitem", "Check the revised preview");
    visual.simulate_keystrokes("up enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Waiting for the demo Host…");
    assert_eq!(
        node(&mut visual, &view, "listitem", "Check the revised preview")
            .current
            .as_deref(),
        Some("step")
    );
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined the rewind.",
    );
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    later(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Started a new attempt from prototype/候補",
    );
    let head = node(&mut visual, &view, "listitem", "Resume from checkpoint");
    assert_eq!(head.current.as_deref(), Some("step"));
    assert!(head.description.contains("From Build preview"));
    node(&mut visual, &view, "listitem", "Review the first preview");
    node(&mut visual, &view, "listitem", "Check the revised preview");
}

#[gpui::test]
fn gallery_agent_checkpoints_reset_rejects_an_old_receipt(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "agent_checkpoints", "clean-light");
    visual.simulate_resize(size(px(1250.), px(990.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Rewind to Read the brief");
    node(&mut visual, &view, "status", "Waiting for the demo Host…");
    click(&mut visual, &view, "button", "Reset history");
    later(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "listitem", "Check the revised preview")
            .current
            .as_deref(),
        Some("step")
    );
    assert!(
        !snapshot(&mut visual, &view)
            .nodes()
            .any(|n| n.name == "Resume from checkpoint")
    );
}

#[gpui::test]
fn agent_checkpoints_reveals_head_in_a_bounded_history_and_home_returns_to_root(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = Mount::new(
        r#"
        import "patterns/agent_checkpoints" as p;
        fn view(ctx) {
            let rows = [#{id:"0",title:"Checkpoint 0"}];
            for i in 1..24 {
                rows.push(#{id:i.to_string(),title:"Checkpoint "+i.to_string(),parent:(i-1).to_string()});
            }
            column([text("Above history").with_key("above").accessibility_role("heading"),
                p::AgentCheckpoints(#{key:"long",run:"probe",revision:0,checkpoints:rows,height:px(144)}),
                text("Below history").with_key("below").accessibility_role("heading")])
                .with_key("root").with_style(style().height(px(650)).overflow_y_scroll())
        }
        "#,
    )
    .registry("patterns/agent_checkpoints")
    .mount(cx);
    visual.update(|window, _| window.activate_window());
    visual.simulate_resize(size(px(645.), px(710.)));
    later(&mut visual, &view);
    let above = bounds(&mut visual, &view, "heading", "Above history");
    let shell = bounds(&mut visual, &view, "group", "Checkpoints");
    assert!(shell.height < 240.0, "history must stay bounded: {shell:?}");
    let last = bounds(&mut visual, &view, "listitem", "Checkpoint 23");
    let first = bounds(&mut visual, &view, "listitem", "Checkpoint 0");
    assert!(last.y + last.height <= shell.y + shell.height + 1.0);
    assert!(first.y < shell.y, "the history should scroll to its head");
    click(&mut visual, &view, "listitem", "Checkpoint 23");
    visual.simulate_keystrokes("home");
    later(&mut visual, &view);
    let first = bounds(&mut visual, &view, "listitem", "Checkpoint 0");
    assert!(
        first.y >= shell.y,
        "Home must reveal the earliest row: first {first:?}, shell {shell:?}"
    );
    assert_eq!(
        bounds(&mut visual, &view, "heading", "Above history").y,
        above.y,
        "only the history, not the page, should move"
    );
}
