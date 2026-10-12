//! FormLayout's own layout passes the `spacing-not-nested` rule (#126): a horizontal
//! submit row with several nodes (its label-column spacer is a grid track, like the
//! field rows) and a vertical field with a description (one column, no nested gap).
//! One submit node and a vertical form without a description are the controls.

use gpui::TestAppContext;
use gpui_rhai::MotionPreference;
use gpui_rhai_registry::{
    BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID, BUNDLED_PROFILES,
};

use crate::support::{Mount, Step, settle};

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
    let main = format!(
        r#"
import "components/button" as button;
import "components/input" as input;
import "patterns/form_layout" as form_layout;
{SCRIPT}
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(800))) }}
"#
    );
    let profile = BUNDLED_PROFILES
        .iter()
        .find(|(name, _)| *name == "productivity")
        .unwrap()
        .1;
    let (mut visual, view) = Mount::components(main)
        .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .with(move |view| view.profile_source(profile))
        .id("audit")
        .mount(cx);
    visual.run_until_parked();
    settle(&mut visual, Step::Refresh, 1);
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
