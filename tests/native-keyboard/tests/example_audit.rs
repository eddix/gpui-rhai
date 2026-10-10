//! The visual examples are compositions too: each passes the productivity
//! composition audit in its baseline states, like the Gallery pages.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
use gpui_rhai::*;

#[allow(dead_code)]
#[path = "../../../crates/gpui-rhai/examples/settings_panel.rs"]
mod settings_panel;

#[allow(dead_code)]
#[path = "../../../crates/gpui-rhai/examples/dashboard_layout.rs"]
mod dashboard_layout;

#[allow(dead_code)]
#[path = "../../../crates/gpui-rhai/examples/form_showcase.rs"]
mod form_showcase;

#[allow(dead_code)]
#[path = "../../../crates/gpui-rhai/examples/data_table.rs"]
mod data_table;

#[allow(dead_code)]
#[path = "../../../crates/gpui-rhai/examples/embedded_views.rs"]
mod embedded_views;

const PRODUCTIVITY: &str = include_str!("../../../registry/profiles/productivity.rhai");

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

/// Every rule except font resolution: the test platform ships no system fonts.
fn rules() -> AuditRules {
    AuditRules::from_ids(
        AuditRule::ALL
            .into_iter()
            .filter(|rule| *rule != AuditRule::UnresolvedFont)
            .map(AuditRule::id),
    )
    .unwrap()
}

fn findings(
    cx: &mut TestAppContext,
    prepared: PreparedScriptView,
    (width, height): (f32, f32),
) -> Vec<String> {
    cx.update(gpui_rhai::install);
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("example-audit", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("example"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(gpui::size(gpui::px(width), gpui::px(height)));
    for _ in 0..3 {
        visual.update(|window, _| window.refresh());
        visual.run_until_parked();
    }
    if let Some(error) = visual.update(|_, cx| view.last_error(cx).unwrap()) {
        return vec![format!("render error: {error}")];
    }
    visual
        .update(|_, cx| view.composition_audit_with(&rules(), cx).unwrap())
        .into_iter()
        .map(|finding| finding.to_string())
        .collect()
}

fn audited(view: EmbeddedScriptView) -> PreparedScriptView {
    view.profile_source(PRODUCTIVITY)
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap()
}

#[gpui::test]
fn examples_pass_the_productivity_audit(cx: &mut TestAppContext) {
    let mut failures = Vec::new();
    let mut check = |name: &str, prepared: PreparedScriptView, size: (f32, f32)| {
        for finding in findings(cx, prepared, size) {
            failures.push(format!("{name}: {finding}"));
        }
    };
    for locale in ["en", "ar"] {
        check(
            &format!("settings_panel/{locale}"),
            audited(settings_panel::view("default-dark", locale)),
            settings_panel::WINDOW,
        );
        check(
            &format!("dashboard_layout/{locale}"),
            audited(dashboard_layout::view("default-dark", locale)),
            dashboard_layout::WINDOW,
        );
    }
    for state in ["default", "dialog", "toast", "date-picker", "textarea"] {
        check(
            &format!("form_showcase/{state}"),
            audited(form_showcase::view("default-light", "en", state)),
            form_showcase::WINDOW,
        );
    }
    for state in ["default", "selected", "loading", "empty", "grouped"] {
        check(
            &format!("data_table/{state}"),
            audited(data_table::view("default-light", "en", state)),
            data_table::WINDOW,
        );
    }
    check(
        "embedded_views/widget",
        embedded_views::prepared_widget(false, false),
        (280.0, 300.0),
    );
    assert!(
        failures.is_empty(),
        "{} findings:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
