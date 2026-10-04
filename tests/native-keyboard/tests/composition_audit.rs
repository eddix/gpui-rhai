use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::*;

const DEFAULT_DARK: &str = include_str!("../../../registry/themes/default_dark.rhai");
const PROFILE: &str = r#"fn profile() { #{ name: "test", audit: [
    "row-height-mismatch", "text-edge-misaligned", "spacing-not-nested",
    "multiple-solid-actions", "mixed-type-in-row", "low-contrast-text", "unresolved-font",
] } }"#;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(cx: &mut TestAppContext, script: &str) -> (WindowHandle<Host>, ScriptViewHandle) {
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(entry, script.to_owned())])),
        DEFAULT_DARK,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .profile_source(PROFILE)
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("audit", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("audit"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    (window, view)
}

fn rules(cx: &mut TestAppContext, script: &str) -> Vec<String> {
    let (window, view) = mount(cx, script);
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    let mut findings = visual
        .update(|_, cx| view.composition_audit(cx).unwrap())
        .into_iter()
        .map(|finding| finding.rule.id().to_owned())
        .collect::<Vec<_>>();
    findings.sort();
    findings.dedup();
    findings
}

#[gpui::test]
fn a_clean_composition_reports_nothing(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let found = rules(
        cx,
        r#"
fn control(label, solid) {
    let style = style().height(theme_length("metrics.control")).padding_x(px(12))
        .items_center().typography("body");
    if solid {
        style = style.background(theme_color("accent")).text_color(theme_color("on_accent"));
    } else {
        style = style.background(theme_color("surface_hover")).text_color(theme_color("text_primary"));
    }
    row([text(label)]).with_style(style).accessibility_role("button").accessibility_label(label)
}
fn view(ctx) {
    column([
        row([control("Export", false), control("Deploy", true)])
            .with_style(style().gap(theme_length("space.related"))),
        text("Hosts").with_style(style().typography("body")),
    ]).with_style(style().gap(theme_length("space.section")).padding(px(12))
        .background(theme_color("surface")))
}
"#,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[gpui::test]
fn each_broken_relationship_is_reported(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let found = rules(
        cx,
        r#"
fn control(label, height, solid) {
    let style = style().height(px(height)).padding_x(px(12)).items_center().typography("body");
    if solid {
        style = style.background(theme_color("accent")).text_color(theme_color("on_accent"));
    } else {
        style = style.background(theme_color("surface_hover"));
    }
    row([text(label)]).with_style(style).accessibility_role("button").accessibility_label(label)
}
fn view(ctx) {
    column([
        row([control("One", 28, true), control("Two", 32, true)])
            .with_style(style().gap(px(24))),
        row([
            text("Big").with_style(style().typography("title")),
            text("small").with_style(style().typography("caption")),
        ]),
        column([
            text("Aligned").with_style(style().typography("body")),
            text("Nudged").with_style(style().typography("body").margin_left(px(3))),
        ]),
        text("Faint").with_style(style().typography("body").text_color(theme_color("surface_hover"))),
        text("Missing font").with_style(style().font_family("Definitely Not A Font 9000")),
    ]).with_style(style().gap(px(16)).padding(px(12)).background(theme_color("surface")))
}
"#,
    );
    for rule in [
        "row-height-mismatch",
        "multiple-solid-actions",
        "spacing-not-nested",
        "mixed-type-in-row",
        "text-edge-misaligned",
        "low-contrast-text",
        "unresolved-font",
    ] {
        assert!(
            found.iter().any(|found| found == rule),
            "missing {rule}: {found:?}"
        );
    }
}

#[gpui::test]
fn no_profile_means_no_design_rules(cx: &mut TestAppContext) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let prepared = EmbeddedScriptView::new(
        entry.clone(),
        EmbeddedScriptSource::new(BTreeMap::from([(
            entry,
            r#"fn view(ctx) { row([text("A").with_style(style().font_size(px(30))),
                text("b").with_style(style().font_size(px(9)))]) }"#
                .to_owned(),
        )])),
        r#"fn theme() { #{ family: "Plain", name: "Dark", mode: "dark",
            tokens: #{ colors: #{ ink: 0xffffffff } } } }"#,
    )
    .prepare()
    .unwrap();
    let captured = Rc::new(RefCell::new(None));
    let capture = captured.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("plain", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("plain"), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = captured.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    let findings = visual.update(|_, cx| view.composition_audit(cx).unwrap());
    assert!(findings.is_empty(), "{findings:?}");
}
