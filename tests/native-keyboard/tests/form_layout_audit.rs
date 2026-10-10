//! FormLayout's own layout passes the `spacing-not-nested` rule (#126): a horizontal
//! submit row with several nodes (its label-column spacer is a grid track, like the
//! field rows) and a vertical field with a description (one column, no nested gap).
//! One submit node and a vertical form without a description are the controls.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, BUNDLED_LAYOUT_SOURCES_BY_ID,
    BUNDLED_PATTERN_SOURCES_BY_ID, BUNDLED_PROFILES, DEFAULT_THEME,
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

const SCRIPT: &str = r#"
fn noop(ctx, payload) { () }
fn name_input() {
    input::Input(#{ key: "a", label: "A", value: "", on_change: Fn("noop") })
        .with_style(style().width(px(240)))
}
fn save() { button::Button(#{ text: "保存", variant: "primary", on_click: Fn("noop") }) }
fn cancel() { button::Button(#{ text: "取消", on_click: Fn("noop") }) }
fn form(orientation, description, submit) {
    let field = #{ label: "A", control: name_input() };
    if description { field.description = "Shown in the list."; }
    form_layout::FormLayout(#{ key: "f", label: "f", orientation: orientation,
        fields: [field], submit: submit })
}
"#;

fn audit(cx: &mut TestAppContext, view_body: &str) -> Vec<(String, String, String)> {
    cx.update(gpui_rhai::install);
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
import "components/button" as button;
import "components/input" as input;
import "patterns/form_layout" as form_layout;
{SCRIPT}
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(800))) }}
"#
    );
    let entry = ModuleId::parse("main").unwrap();
    modules.insert(entry.clone(), main);
    let profile = BUNDLED_PROFILES
        .iter()
        .find(|(name, _)| *name == "productivity")
        .unwrap()
        .1;
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .profile_source(profile)
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
        let host = ScriptViewHost::new("audit", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("audit"), host.clone(), window, cx)
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
    let findings = visual
        .update(|_, cx| view.composition_audit(cx).unwrap())
        .into_iter()
        .map(|finding| (finding.rule.id().to_owned(), finding.path, finding.message))
        .collect::<Vec<_>>();
    eprintln!("{view_body}: {findings:#?}");
    findings
}

fn spacing(findings: &[(String, String, String)]) -> Vec<String> {
    findings
        .iter()
        .filter(|(rule, _, _)| rule == "spacing-not-nested")
        .map(|(rule, path, message)| format!("{rule} at {path}: {message}"))
        .collect()
}

#[gpui::test]
fn horizontal_form_with_one_submit_node_is_clean(cx: &mut TestAppContext) {
    let found = audit(cx, r#"form("horizontal", false, [save()])"#);
    assert!(found.is_empty(), "control: {found:?}");
}

#[gpui::test]
fn horizontal_form_with_two_submit_nodes_is_clean(cx: &mut TestAppContext) {
    let found = audit(cx, r#"form("horizontal", false, [save(), cancel()])"#);
    assert!(
        spacing(&found).is_empty(),
        "FormLayout's own submit row is reported: {:?}",
        spacing(&found)
    );
}

#[gpui::test]
fn vertical_form_without_description_is_clean(cx: &mut TestAppContext) {
    let found = audit(cx, r#"form("vertical", false, [save()])"#);
    assert!(found.is_empty(), "control: {found:?}");
}

#[gpui::test]
fn vertical_form_with_description_is_clean(cx: &mut TestAppContext) {
    let found = audit(cx, r#"form("vertical", true, [save()])"#);
    assert!(
        spacing(&found).is_empty(),
        "FormLayout's own vertical field is reported: {:?}",
        spacing(&found)
    );
}
