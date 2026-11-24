//! Native interaction and reflow for the maintained decision-tree example.

use std::time::Duration;

use gpui::{ClipboardItem, Modifiers, TestAppContext, VisualTestContext, point, px, size};
use gpui_rhai::{
    CanvasCommand, CanvasPathSegment, CanvasScene, PrimitiveValue, ScriptViewHandle, UiNode,
    UiNodeKind, UiValue,
};

use super::conversation::{gallery, node, ready};
use crate::support::{bounds, center, click_center, snapshot};

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, label: &str) {
    let rect = bounds(visual, view, role, label);
    visual.simulate_mouse_move(center(rect), None, Modifiers::none());
    ready(visual, view);
    let rect = bounds(visual, view, role, label);
    click_center(visual, rect);
    ready(visual, view);
}

fn connectors(node: &UiNode) -> Option<&CanvasScene> {
    match node.kind() {
        UiNodeKind::Canvas { scene } if node.key().is_some_and(|key| key.as_str() == "lines") => {
            Some(scene)
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(connectors)
        }
        UiNodeKind::Custom { primitive } => {
            primitive.props.iter().find_map(|(_, prop)| match prop {
                PrimitiveValue::Node(child) => connectors(child),
                PrimitiveValue::Nodes(children) => children.iter().find_map(connectors),
                _ => None,
            })
        }
        _ => None,
    }
}

fn first_connector(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> Vec<CanvasPathSegment> {
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let scene = connectors(&root).expect("a native layout has published the connector geometry");
    assert!(scene.view_box().is_some());
    let result = scene
        .commands()
        .iter()
        .find(|draw| draw.key() == "branch-0");
    let Some(CanvasCommand::Path { segments, .. }) = result else {
        panic!("the first offered branch has no connector")
    };
    segments.clone()
}

#[gpui::test]
fn gallery_decision_tree_only_leaves_propose_and_rejections_keep_the_panel(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "decision_tree", "modern-light");
    visual.simulate_resize(size(px(1200.), px(900.)));
    ready(&mut visual, &view);
    click(&mut visual, &view, "button", "Decline next proposal");
    click(&mut visual, &view, "button", "Contact customers first");
    node(&mut visual, &view, "button", "Email subscribers");
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("status", "Waiting for the demo Host…")
            .next()
            .is_none()
    );
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    node(&mut visual, &view, "status", "Waiting for the demo Host…");
    visual.executor().advance_clock(Duration::from_millis(660));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host declined this proposal.",
    );
    click(&mut visual, &view, "button", "Queue in support");
    visual.executor().advance_clock(Duration::from_millis(660));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "The demo Host recorded the selected branch.",
    );
    assert_eq!(
        node(&mut visual, &view, "button", "Queue in support")
            .current
            .as_deref(),
        Some("true")
    );
    click(&mut visual, &view, "button", "Next decision");
    node(
        &mut visual,
        &view,
        "heading",
        "Who should see the release summary first?",
    );
}

#[gpui::test]
fn gallery_decision_tree_draft_reflows_and_submits_as_a_custom_instruction(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = gallery(cx, "decision_tree", "clean-dark");
    visual.simulate_resize(size(px(1260.), px(1000.)));
    ready(&mut visual, &view);
    let first = bounds(&mut visual, &view, "button", "Cancel the newer ones");
    let question = bounds(
        &mut visual,
        &view,
        "heading",
        "How should we handle three duplicate subscriptions?",
    );
    assert!(first.x > question.x + 120.);
    assert!(matches!(
        first_connector(&mut visual, &view).as_slice(),
        [CanvasPathSegment::Move { x, .. }, CanvasPathSegment::Cubic { x: end, .. }]
            if end > x
    ));
    visual.simulate_resize(size(px(570.), px(1250.)));
    visual.executor().advance_clock(Duration::from_millis(140));
    ready(&mut visual, &view);
    let question = bounds(
        &mut visual,
        &view,
        "heading",
        "How should we handle three duplicate subscriptions?",
    );
    let first = bounds(&mut visual, &view, "button", "Cancel the newer ones");
    assert!(first.y > question.y + 30.);
    assert!(matches!(
        first_connector(&mut visual, &view).as_slice(),
        [CanvasPathSegment::Move { x, y }, CanvasPathSegment::Line { x: rail, y: down }, CanvasPathSegment::Line { x: end, y: target }]
            if (*rail - *x).abs() < 0.01 && down > y && (*down - *target).abs() < 0.01 && end > rail
    ));

    click(&mut visual, &view, "button", "Other…");
    node(&mut visual, &view, "text_field", "Custom instruction");
    visual.update(|_, cx| {
        cx.write_to_clipboard(ClipboardItem::new_string(
            "  Check the ledger first  ".into(),
        ));
    });
    visual.simulate_keystrokes(&crate::support::primary_keys("secondary-v"));
    ready(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "text_field", "Custom instruction").value,
        Some(UiValue::String("  Check the ledger first  ".into()))
    );
    visual.simulate_keystrokes("enter");
    ready(&mut visual, &view);
    visual.executor().advance_clock(Duration::from_millis(660));
    ready(&mut visual, &view);
    node(
        &mut visual,
        &view,
        "status",
        "Instruction recorded: Check the ledger first",
    );
}

#[gpui::test]
fn gallery_decision_tree_flag_is_inspectable_by_pointer_and_keyboard(cx: &mut TestAppContext) {
    let (mut visual, view) = gallery(cx, "decision_tree", "clean-light");
    visual.simulate_resize(size(px(1260.), px(960.)));
    ready(&mut visual, &view);
    let mark = bounds(&mut visual, &view, "group", "about $147");
    let note = "Two renewals are still processing; the total can change. · Billing ledger · draft";
    let open = |visual: &mut VisualTestContext| {
        snapshot(visual, &view)
            .find_by_role_and_name("group", note)
            .next()
            .and_then(|n| n.geometry)
            .is_some()
    };
    assert!(!open(&mut visual), "closed preview is not presented");
    visual.simulate_mouse_move(center(mark), None, Modifiers::none());
    visual.executor().advance_clock(Duration::from_millis(300));
    ready(&mut visual, &view);
    assert!(open(&mut visual), "hover opens the evidence preview");
    visual.simulate_mouse_move(point(px(5.), px(5.)), None, Modifiers::none());
    visual.executor().advance_clock(Duration::from_millis(400));
    ready(&mut visual, &view);
    assert!(!open(&mut visual));
    click(&mut visual, &view, "button", "Contact customers first");
    visual.simulate_mouse_move(point(px(5.), px(5.)), None, Modifiers::none());
    visual.simulate_keystrokes("escape");
    ready(&mut visual, &view);
    visual.simulate_keystrokes("shift-tab");
    visual.executor().advance_clock(Duration::from_millis(300));
    ready(&mut visual, &view);
    assert!(open(&mut visual), "keyboard focus opens the same evidence");
}
