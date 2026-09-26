use std::cell::RefCell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{Context, IntoElement, Render, TestAppContext, Window};
use gpui_rhai::{
    AutomationCommand, AutomationLocator, ScriptViewHandle, ScriptViewHost, UiNodeKind,
};
use gpui_rhai_cli::gallery::{GalleryLaunch, prepare};

struct GalleryHost {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for GalleryHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(
            gpui::div()
                .size_full()
                .child(self.view.flex_item().unwrap()),
        )
    }
}

fn node_texts(node: &gpui_rhai::UiNode, output: &mut Vec<String>) {
    match node.kind() {
        UiNodeKind::Text { text, .. } => output.push(text.to_string()),
        UiNodeKind::RichText { spans, .. } => {
            output.push(spans.iter().map(|span| span.text()).collect::<String>());
        }
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            for child in children {
                node_texts(child, output);
            }
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => {
            node_texts(trigger, output);
            node_texts(content, output);
        }
        UiNodeKind::Layer { content, .. } => node_texts(content, output),
        UiNodeKind::ErrorBoundary { child, fallback } => {
            node_texts(child, output);
            node_texts(fallback, output);
        }
        UiNodeKind::VirtualCollection { spec } => {
            for item in spec.realized.values() {
                node_texts(item, output);
            }
        }
        UiNodeKind::Custom { .. }
        | UiNodeKind::Canvas { .. }
        | UiNodeKind::Svg { .. }
        | UiNodeKind::Image { .. }
        | UiNodeKind::DirectionalImage { .. } => {}
    }
}

fn dispatch(visual: &mut gpui::VisualTestContext, view: &ScriptViewHandle, id: &str) {
    visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::TestId { id: id.to_owned() },
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

#[gpui::test]
fn operations_workbench_completes_and_cancels_the_deployment_boundary(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let prepared = prepare(&GalleryLaunch {
        story: "apps/operations".to_owned(),
        case: "config-diff".to_owned(),
        ..GalleryLaunch::default()
    })
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let captured_for_window = Rc::clone(&captured);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("acceptance-gallery", cx).unwrap();
        let view = prepared
            .mount(
                gpui_rhai::ScriptViewConfig::new("operations-workbench"),
                host.clone(),
                window,
                cx,
            )
            .unwrap();
        *captured_for_window.borrow_mut() = Some(view.clone());
        GalleryHost { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();

    let view = captured.borrow().as_ref().unwrap().clone();
    let mut visual = gpui::VisualTestContext::from_window(*window, cx);
    dispatch(&mut visual, &view, "stage-deploy");
    dispatch(&mut visual, &view, "cancel-deploy");
    let mut texts = Vec::new();
    node_texts(
        &visual.update(|_, cx| view.root(cx).unwrap().unwrap()),
        &mut texts,
    );
    assert!(
        !texts
            .iter()
            .any(|text| text.contains("Configuration verified"))
    );

    dispatch(&mut visual, &view, "stage-deploy");
    dispatch(&mut visual, &view, "confirm-deploy");
    texts.clear();
    node_texts(
        &visual.update(|_, cx| view.root(cx).unwrap().unwrap()),
        &mut texts,
    );
    assert!(
        texts
            .iter()
            .any(|text| text.contains("Configuration verified")),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|text| text == "Deployment complete"),
        "{texts:?}"
    );
    assert!(
        visual
            .update(|_, cx| view.last_error(cx).unwrap())
            .is_none()
    );
}
