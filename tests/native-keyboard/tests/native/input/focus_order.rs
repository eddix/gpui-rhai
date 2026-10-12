//! Tab order over a retained tree: disabled nodes are skipped and Enter activates the focused
//! node.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::KeyboardHost;
use gpui::TestAppContext;
use gpui_rhai::{
    EventPropagation, NodeEventDispatcher, OverlayDismissPolicy, OverlayId, OverlayKind,
    OverlayNodeSpec, OverlayPlacement, PrimitiveRegistry, RuntimeEngine, UiNode, UiValue,
};

#[gpui::test]
fn tab_order_skips_disabled_nodes_and_enter_activates_focus(cx: &mut TestAppContext) {
    let mut engine = RuntimeEngine::new();
    let compiled = engine
        .compile(
            r#"
                fn view(ctx) { text("keyboard test") }
                fn activated(ctx, payload) { () }
            "#,
        )
        .expect("compile callback fixture");
    let callback = engine
        .callback(&compiled, "activated")
        .expect("resolve callback fixture");
    let button = |label: &str, payload: i64| {
        UiNode::text(label)
            .with_key(label)
            .with_handler("click", callback.clone())
            .with_handler_payload("click", UiValue::Integer(payload))
    };
    let root = UiNode::column(vec![
        button("disabled", 0).with_attribute("disabled", UiValue::Bool(true)),
        UiNode::overlay(
            UiNode::text("overlay trigger"),
            UiNode::text("overlay content"),
            OverlayNodeSpec {
                initial_focus: gpui_rhai::OverlayInitialFocus::Panel,
                id: OverlayId::new("keyboard-overlay"),
                owner: None,
                parent: None,
                kind: OverlayKind::Popover,
                placement: OverlayPlacement::Bottom,
                align: gpui_rhai::OverlayAlign::Center,
                anchor: None,
                open: false,
                gap: 4.0,
                modal: false,
                dismiss: OverlayDismissPolicy {
                    escape: true,
                    outside: true,
                },
                tooltip_delays: None,
                activate_on_trigger: true,
                width_policy: gpui_rhai::OverlayWidthPolicy::Content,
            },
        )
        .with_key("overlay")
        .with_handler("open_change", callback.clone()),
        button("first", 1),
        button("second", 2),
    ]);
    let activations = Rc::new(RefCell::new(Vec::new()));
    let captured = Rc::clone(&activations);
    let dispatcher = NodeEventDispatcher::new(move |_, payload, _, _, _| {
        if let UiValue::Integer(value) = payload {
            captured.borrow_mut().push(value);
        }
        EventPropagation::Handled
    });
    let window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window, cx);
        KeyboardHost {
            root: Rc::new(RefCell::new(root)),
            tree: gpui_rhai::RetainedUiTree::new(),
            dispatcher,
            host_focus,
            primitives: PrimitiveRegistry::new(),
        }
    });
    cx.run_until_parked();

    cx.simulate_keystrokes(*window, "tab tab enter tab enter shift-tab enter");

    assert_eq!(*activations.borrow(), vec![1, 2, 1]);
}
