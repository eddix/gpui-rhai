//! Host callbacks and primitives without a script runtime: callbacks dispatch, explicit
//! occlusion blocks pointer hits, and a custom primitive rejects an invalid payload before
//! its callback runs.

use std::cell::RefCell;
use std::rc::Rc;

use crate::support::KeyboardHost;
use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, Modifiers, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestAppContext, VisualTestContext, Window, div, point,
};
use gpui_rhai::{
    EventPropagation, GpuiNodeRenderer, HostCallback, NodeEventDispatcher, PrimitiveContext,
    PrimitiveHandler, PrimitiveInstance, PrimitiveNode, PrimitiveProps, PrimitiveRegistry,
    PrimitiveTheme, PrimitiveValue, UiNode, UiValue, text_input_primitive_descriptor,
};

struct HostCallbackTestHost {
    root: UiNode,
    host_focus: FocusHandle,
    parent_clicks: Rc<RefCell<usize>>,
    bubbled_keys: Rc<RefCell<Vec<String>>>,
}

impl Render for HostCallbackTestHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let parent_clicks = Rc::clone(&self.parent_clicks);
        let bubbled_keys = Rc::clone(&self.bubbled_keys);
        div()
            .id("host-callback-parent")
            .size_full()
            .track_focus(&self.host_focus)
            .on_click(move |_, _, _| *parent_clicks.borrow_mut() += 1)
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key.as_str() == "tab" {
                    window.focus_next(cx);
                    cx.stop_propagation();
                } else {
                    bubbled_keys
                        .borrow_mut()
                        .push(event.keystroke.key.to_string());
                }
            })
            .child(GpuiNodeRenderer::render(&self.root))
    }
}

#[gpui::test]
fn host_callbacks_dispatch_without_a_script_runtime(cx: &mut TestAppContext) {
    let received = Rc::new(RefCell::new(Vec::new()));
    let pointer_received = Rc::clone(&received);
    let keyboard_received = Rc::clone(&received);
    let root = UiNode::text("Host action")
        .with_host_handler(
            "click",
            HostCallback::new("widget.pointer", move |payload, _, _| {
                pointer_received.borrow_mut().push(payload);
                EventPropagation::Propagate
            }),
        )
        .with_handler_payload("click", UiValue::Integer(7))
        .with_host_handler(
            "key:enter",
            HostCallback::new("widget.keyboard", move |payload, _, _| {
                keyboard_received.borrow_mut().push(payload);
                EventPropagation::Handled
            }),
        )
        .with_handler_payload("key:enter", UiValue::String("enter".to_owned()));
    let parent_clicks = Rc::new(RefCell::new(0));
    let bubbled_keys = Rc::new(RefCell::new(Vec::new()));
    let parent_clicks_for_window = Rc::clone(&parent_clicks);
    let bubbled_keys_for_window = Rc::clone(&bubbled_keys);
    let window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window, cx);
        HostCallbackTestHost {
            root,
            host_focus,
            parent_clicks: parent_clicks_for_window,
            bubbled_keys: bubbled_keys_for_window,
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    let bounds = visual.debug_bounds("root").expect("host node bounds");
    visual.simulate_click(
        point(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();
    assert_eq!(*parent_clicks.borrow(), 1);
    assert_eq!(*received.borrow(), vec![UiValue::Integer(7)]);

    visual.simulate_keystrokes("tab enter");
    visual.run_until_parked();
    assert_eq!(
        *received.borrow(),
        vec![UiValue::Integer(7), UiValue::String("enter".to_owned())]
    );
    assert!(bubbled_keys.borrow().is_empty());
}

#[gpui::test]
fn explicit_occlusion_blocks_pointer_hits_to_painted_siblings(cx: &mut TestAppContext) {
    let clicks = Rc::new(RefCell::new(0usize));
    let captured = Rc::clone(&clicks);
    let back = UiNode::text("Back")
        .with_key("back")
        .with_style(
            &gpui_rhai::Style::new()
                .width(gpui_rhai::Length::Pixels(100.0))
                .height(gpui_rhai::Length::Pixels(100.0)),
        )
        .with_host_handler(
            "click",
            HostCallback::new("back.click", move |_, _, _| {
                *captured.borrow_mut() += 1;
                EventPropagation::Handled
            }),
        );
    let front = UiNode::text("Front").with_key("front").with_style(
        &gpui_rhai::Style::new()
            .absolute()
            .top(gpui_rhai::Length::Pixels(0.0))
            .left(gpui_rhai::Length::Pixels(0.0))
            .width(gpui_rhai::Length::Pixels(100.0))
            .height(gpui_rhai::Length::Pixels(100.0))
            .hit_test(gpui_rhai::HitTestBehavior::Block),
    );
    let root = UiNode::box_node(vec![back, front]).with_style(&gpui_rhai::Style::new().relative());
    let window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window, cx);
        KeyboardHost {
            root: Rc::new(RefCell::new(root)),
            tree: gpui_rhai::RetainedUiTree::new(),
            dispatcher: NodeEventDispatcher::new(|_, _, _, _, _| EventPropagation::Handled),
            host_focus,
            primitives: PrimitiveRegistry::new(),
        }
    });
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    let bounds = visual.debug_bounds("root/front").unwrap();
    visual.simulate_click(
        point(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        ),
        Modifiers::default(),
    );
    visual.run_until_parked();
    assert_eq!(*clicks.borrow(), 0);
}

#[gpui::test]
fn custom_primitive_rejects_invalid_payload_before_host_callback(cx: &mut TestAppContext) {
    struct InvalidPayloadHandler {
        rejected: Rc<RefCell<bool>>,
    }

    impl PrimitiveHandler for InvalidPayloadHandler {
        fn render(
            &mut self,
            _: &PrimitiveInstance,
            events: &PrimitiveContext,
            _: &PrimitiveTheme,
            window: &mut Window,
            app: &mut gpui::App,
        ) -> Result<gpui::AnyElement, String> {
            *self.rejected.borrow_mut() = events
                .emit("change", UiValue::Bool(true), window, app)
                .is_err();
            Ok(div().into_any_element())
        }
    }

    let registry = PrimitiveRegistry::new();
    let descriptor = text_input_primitive_descriptor();
    let primitive = descriptor.id.clone();
    let rejected = Rc::new(RefCell::new(false));
    registry
        .register(
            descriptor,
            InvalidPayloadHandler {
                rejected: Rc::clone(&rejected),
            },
        )
        .unwrap();
    let invoked = Rc::new(RefCell::new(0));
    let invoked_by_callback = Rc::clone(&invoked);
    let props = PrimitiveProps::new().with(
        "on_change",
        PrimitiveValue::Callback(
            HostCallback::new("widget.invalid", move |_, _, _| {
                *invoked_by_callback.borrow_mut() += 1;
                EventPropagation::Handled
            })
            .into(),
        ),
    );
    let root = UiNode::custom(PrimitiveNode {
        primitive,
        key: Some("invalid-payload".to_owned()),
        props,
    });
    let dispatcher = NodeEventDispatcher::new(|_, _, _, _, _| EventPropagation::Handled);
    let _window = cx.add_window(|window, cx| {
        let host_focus = cx.focus_handle();
        host_focus.focus(window, cx);
        KeyboardHost {
            root: Rc::new(RefCell::new(root)),
            tree: gpui_rhai::RetainedUiTree::new(),
            dispatcher,
            host_focus,
            primitives: registry,
        }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    assert!(*rejected.borrow());
    assert_eq!(*invoked.borrow(), 0);
}
