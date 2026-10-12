//! InlineState draws `detail` (the raw error, selectable) in the error, stale and
//! refreshing states (#122): stale is where a refresh failed while the data stays on
//! screen. The error state is the control.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, BUNDLED_LAYOUT_SOURCES_BY_ID,
    BUNDLED_PATTERN_SOURCES_BY_ID, DEFAULT_THEME,
};

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(cx: &mut TestAppContext, view_body: &str) -> (VisualTestContext, ScriptViewHandle) {
    let mut modules = BTreeMap::new();
    let tables = [
        BUNDLED_COMPONENT_SOURCES_BY_ID,
        BUNDLED_LAYOUT_SOURCES_BY_ID,
        BUNDLED_PATTERN_SOURCES_BY_ID,
    ];
    for (id, source) in tables.iter().flat_map(|t| t.iter()) {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let main = format!(
        r#"
import "patterns/inline_state" as inline_state;
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(400)).height(px(400))) }}
"#
    );
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(entry.clone(), main);
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .motion_preference(MotionPreference::None)
            .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
                (
                    path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".to_owned(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }))
            .prepare()
            .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window: WindowHandle<Host> = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("state", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("state"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    (visual, view)
}

/// Every text node in the accessibility tree, in tree order.
fn texts(cx: &mut TestAppContext, state: &str) -> Vec<String> {
    cx.update(gpui_rhai::install);
    let body = format!(
        r#"inline_state::InlineState(#{{ key: "s", state: "{state}", title: "刷新失败", detail: "原文" }})"#
    );
    let (mut visual, view) = mount(cx, &body);
    let found = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .filter(|node| node.role == "text")
            .map(|node| node.name.clone())
            .collect::<Vec<_>>()
    });
    eprintln!("{state}: text nodes {found:?}");
    found
}

#[gpui::test]
fn error_state_draws_detail(cx: &mut TestAppContext) {
    let found = texts(cx, "error");
    assert!(
        found.iter().any(|text| text == "刷新失败"),
        "error: the title is drawn (precondition): {found:?}"
    );
    assert!(
        found.iter().any(|text| text == "原文"),
        "error: detail should be drawn (control): {found:?}"
    );
}

#[gpui::test]
fn stale_state_draws_detail(cx: &mut TestAppContext) {
    let found = texts(cx, "stale");
    assert!(
        found.iter().any(|text| text == "刷新失败"),
        "stale: the title is drawn (precondition): {found:?}"
    );
    assert!(
        found.iter().any(|text| text == "原文"),
        "stale: the schema accepts `detail` but it is not drawn: {found:?}"
    );
}

#[gpui::test]
fn refreshing_state_draws_detail(cx: &mut TestAppContext) {
    let found = texts(cx, "refreshing");
    assert!(
        found.iter().any(|text| text == "刷新失败"),
        "refreshing: the title is drawn (precondition): {found:?}"
    );
    assert!(
        found.iter().any(|text| text == "原文"),
        "refreshing: the schema accepts `detail` but it is not drawn: {found:?}"
    );
}
