//! The visual examples are compositions too: each passes the productivity
//! composition audit in its baseline states, like the Gallery pages.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;
use gpui_rhai_registry::BUNDLED_PROFILES;

use crate::support::{Host, Step, last_error, settle};

#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/settings_panel.rs"]
mod settings_panel;

#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/dashboard_layout.rs"]
mod dashboard_layout;

#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/form_showcase.rs"]
mod form_showcase;

#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/data_table.rs"]
mod data_table;

#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/embedded_views.rs"]
pub(super) mod embedded_views;

/// The bundled productivity profile.
fn productivity() -> &'static str {
    BUNDLED_PROFILES
        .iter()
        .find(|(id, _)| *id == "productivity")
        .unwrap()
        .1
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

fn mounted(
    cx: &mut TestAppContext,
    prepared: PreparedScriptView,
    (width, height): (f32, f32),
) -> (VisualTestContext, ScriptViewHandle) {
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
    settle(&mut visual, Step::Refresh, 3);
    (visual, view)
}

fn findings(
    cx: &mut TestAppContext,
    prepared: PreparedScriptView,
    size: (f32, f32),
) -> Vec<String> {
    let (mut visual, view) = mounted(cx, prepared, size);
    if let Some(error) = last_error(&mut visual, &view) {
        return vec![format!("render error: {error}")];
    }
    visual
        .update(|_, cx| view.composition_audit_with(&rules(), cx).unwrap())
        .into_iter()
        .map(|finding| finding.to_string())
        .collect()
}

fn audited(view: ScriptView) -> PreparedScriptView {
    view.profile_source(productivity())
        .motion_preference(MotionPreference::None)
        .prepare()
        .unwrap()
}

#[gpui::test]
fn reference_examples_mount_complete_designs_and_keep_all_four_available(cx: &mut TestAppContext) {
    type Factory = fn(&str) -> ScriptView;
    let examples: [(&str, Factory, (f32, f32)); 4] = [
        (
            "settings",
            |theme| settings_panel::view(theme, "en"),
            settings_panel::WINDOW,
        ),
        (
            "dashboard",
            |theme| dashboard_layout::view(theme, "en"),
            dashboard_layout::WINDOW,
        ),
        (
            "form",
            |theme| form_showcase::view(theme, "en", "default"),
            form_showcase::WINDOW,
        ),
        (
            "table",
            |theme| data_table::view(theme, "en", "default"),
            data_table::WINDOW,
        ),
    ];
    let presets = [
        ("modern-light", "Modern", "Light"),
        ("modern-dark", "Modern", "Dark"),
        ("clean-light", "Clean", "Light"),
        ("clean-dark", "Clean", "Dark"),
        ("", "Modern", "Dark"),
    ];
    for (name, factory, size) in examples {
        for (slug, family, variant) in presets {
            let (mut visual, view) = mounted(
                cx,
                factory(slug)
                    .motion_preference(MotionPreference::None)
                    .prepare()
                    .unwrap(),
                size,
            );
            let theme = visual.update(|_, cx| view.theme_snapshot(cx).unwrap());
            assert_eq!(
                (theme.variant.family.as_str(), theme.variant.name.as_str()),
                (family, variant),
                "{name}/{slug}: init must select the full design, not fall back to a legacy palette"
            );
            for (_, family, variant) in &presets[..4] {
                visual
                    .update(|_, cx| view.select_theme(family, variant, cx))
                    .unwrap();
                settle(&mut visual, Step::Frame, 2);
                let theme = visual.update(|_, cx| view.theme_snapshot(cx).unwrap());
                assert_eq!(
                    (theme.variant.family.as_str(), theme.variant.name.as_str()),
                    (*family, *variant)
                );
                assert!(
                    last_error(&mut visual, &view).is_none(),
                    "{name}/{slug} -> {family}/{variant}"
                );
            }
        }
    }
}

#[gpui::test]
fn examples_pass_the_productivity_audit(cx: &mut TestAppContext) {
    let mut failures = Vec::new();
    // These example states exercise readable alternate palettes and the
    // embedded widget. Require a raw clean report. Positive controls for
    // preserved, source-exact palette risks live in the Gallery matrix.
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
