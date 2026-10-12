//! Hosts and tree helpers shared by tests in several areas: a retained-tree keyboard
//! Host, an embedded-view Host, rendered-text probes and the official icon assets.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, Modifiers, ParentElement, Render,
    VisualTestContext, Window, div, point, px,
};
use gpui_rhai::{
    AssetData, GpuiNodeRenderer, InteractionState, LiteralColorResolver, NodeEventDispatcher,
    PrimitiveRegistry, ScriptViewHandle, ScriptViewHost, UiNode,
};

pub(crate) fn official_icon_assets() -> Vec<(String, AssetData)> {
    [
        (
            "check",
            include_bytes!("../../../../../registry/assets/icons/check.svg").as_slice(),
        ),
        (
            "close",
            include_bytes!("../../../../../registry/assets/icons/close.svg").as_slice(),
        ),
        (
            "chevron_left",
            include_bytes!("../../../../../registry/assets/icons/chevron_left.svg").as_slice(),
        ),
        (
            "chevron_right",
            include_bytes!("../../../../../registry/assets/icons/chevron_right.svg").as_slice(),
        ),
        (
            "chevron_down",
            include_bytes!("../../../../../registry/assets/icons/chevron_down.svg").as_slice(),
        ),
        (
            "chevron_up",
            include_bytes!("../../../../../registry/assets/icons/chevron_up.svg").as_slice(),
        ),
        (
            "calendar",
            include_bytes!("../../../../../registry/assets/icons/calendar.svg").as_slice(),
        ),
        (
            "date_previous",
            include_bytes!("../../../../../registry/assets/icons/date_previous.svg").as_slice(),
        ),
        (
            "date_next",
            include_bytes!("../../../../../registry/assets/icons/date_next.svg").as_slice(),
        ),
        (
            "disclosure_down",
            include_bytes!("../../../../../registry/assets/icons/disclosure_down.svg").as_slice(),
        ),
        (
            "sort_ascending",
            include_bytes!("../../../../../registry/assets/icons/sort_ascending.svg").as_slice(),
        ),
        (
            "sort_descending",
            include_bytes!("../../../../../registry/assets/icons/sort_descending.svg").as_slice(),
        ),
        (
            "minus",
            include_bytes!("../../../../../registry/assets/icons/minus.svg").as_slice(),
        ),
        (
            "plus",
            include_bytes!("../../../../../registry/assets/icons/plus.svg").as_slice(),
        ),
        (
            "search",
            include_bytes!("../../../../../registry/assets/icons/search.svg").as_slice(),
        ),
        (
            "info",
            include_bytes!("../../../../../registry/assets/icons/info.svg").as_slice(),
        ),
        (
            "warning",
            include_bytes!("../../../../../registry/assets/icons/warning.svg").as_slice(),
        ),
    ]
    .into_iter()
    .map(|(name, bytes)| {
        (
            format!("icons/{name}"),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: bytes.to_vec(),
            },
        )
    })
    .collect()
}

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

pub(crate) struct SingleEmbeddedHost {
    pub(crate) host: ScriptViewHost,
    pub(crate) view: ScriptViewHandle,
}

impl Render for SingleEmbeddedHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
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

pub(crate) fn dispatch_script_button(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    label: &str,
) {
    visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Dispatch {
                    locator: gpui_rhai::AutomationLocator::RoleName {
                        role: "button".to_owned(),
                        name: label.to_owned(),
                    },
                    event: "click".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .unwrap();
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
    let trigger = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("button", "open palette")
            .next()
            .unwrap()
            .geometry
            .unwrap()
    });
    visual.simulate_click(
        point(
            px((trigger.visual.x + trigger.visual.width / 2.0) as f32),
            px((trigger.visual.y + trigger.visual.height / 2.0) as f32),
        ),
        Modifiers::default(),
    );
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
