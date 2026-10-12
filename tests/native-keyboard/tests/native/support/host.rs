//! A retained-tree keyboard host and rendered-text probes shared by tests in several areas.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render,
    VisualTestContext, Window, div,
};
use gpui_rhai::{
    GpuiNodeRenderer, InteractionState, LiteralColorResolver, NodeEventDispatcher,
    PrimitiveRegistry, ScriptViewHandle, UiNode,
};

pub(crate) struct KeyboardHost {
    pub(crate) root: Rc<RefCell<UiNode>>,
    pub(crate) tree: gpui_rhai::RetainedUiTree,
    pub(crate) dispatcher: NodeEventDispatcher,
    pub(crate) host_focus: FocusHandle,
    pub(crate) primitives: PrimitiveRegistry,
}

impl Render for KeyboardHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let root = self.root.borrow().clone();
        let content = self.tree.reconcile(root).map_or_else(
            |error| div().child(error.to_string()).into_any_element(),
            |_| {
                self.primitives.retain_tree(&self.tree).map_or_else(
                    |error| div().child(error.to_string()).into_any_element(),
                    |_| {
                        GpuiNodeRenderer::render_retained_with_dispatcher(
                            &self.tree,
                            &LiteralColorResolver,
                            &InteractionState::default(),
                            &self.primitives,
                            &self.dispatcher,
                        )
                    },
                )
            },
        );
        div()
            .track_focus(&self.host_focus)
            .on_key_down(|event, window, cx| {
                if event.keystroke.key.as_str() == "tab" {
                    if event.keystroke.modifiers.shift {
                        window.focus_prev(cx);
                    } else {
                        window.focus_next(cx);
                    }
                    cx.stop_propagation();
                }
            })
            .child(content)
    }
}

pub(crate) fn wait_for_view_text(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    expected: &str,
    context: &str,
) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        visual.run_until_parked();
        let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
        let mut texts = Vec::new();
        node_texts(&root, &mut texts);
        if texts.iter().any(|text| text == expected) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{context}; expected {expected:?}, got {texts:?}"
        );
        std::thread::yield_now();
    }
}

/// Dispatches a click to the button named `label` through automation, then runs until parked.
pub(crate) fn dispatch_script_button(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    label: &str,
) {
    super::dispatch(visual, view, super::by_role("button", label), "click").unwrap();
    visual.run_until_parked();
}

pub(crate) fn node_texts(node: &UiNode, output: &mut Vec<String>) {
    match node.kind() {
        gpui_rhai::UiNodeKind::Text { text } => output.push(text.to_string()),
        gpui_rhai::UiNodeKind::Box { children } | gpui_rhai::UiNodeKind::Fragment { children } => {
            for child in children {
                node_texts(child, output);
            }
        }
        gpui_rhai::UiNodeKind::Overlay {
            trigger, content, ..
        } => {
            node_texts(trigger, output);
            node_texts(content, output);
        }
        gpui_rhai::UiNodeKind::Layer { content, .. } => node_texts(content, output),
        gpui_rhai::UiNodeKind::ErrorBoundary { child, fallback } => {
            node_texts(child, output);
            node_texts(fallback, output);
        }
        gpui_rhai::UiNodeKind::VirtualCollection { spec } => {
            for child in spec.realized.values() {
                node_texts(child, output);
            }
        }
        _ => {}
    }
}

pub(crate) fn click_palette_trigger(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    let trigger = super::bounds(visual, view, "button", "open palette");
    super::click_center(visual, trigger);
    visual.run_until_parked();
}

pub(crate) fn palette_texts(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> Vec<String> {
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut texts = Vec::new();
    node_texts(&root, &mut texts);
    texts
}
