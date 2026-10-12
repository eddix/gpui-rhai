use gpui::{TestAppContext, VisualTestContext, WindowHandle};
use gpui_rhai::{MotionPreference, ScriptViewHandle};

use crate::support::{Host, Mount};

const PROFILE: &str = r#"fn profile() { #{ name: "test", audit: [
    "row-height-mismatch", "text-edge-misaligned", "spacing-not-nested",
    "multiple-solid-actions", "mixed-type-in-row", "low-contrast-text", "unresolved-font",
] } }"#;

fn mount(cx: &mut TestAppContext, script: &str) -> (WindowHandle<Host>, ScriptViewHandle) {
    Mount::new(script)
        .motion(MotionPreference::None)
        .with(|view| view.profile_source(PROFILE))
        .id("audit")
        .mount_window(cx)
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
            .with_style(style().gap(px(8))),
        // Proximity competes along one axis: a column inside a column.
        column([text("Loose").with_style(style().typography("body")),
            text("Pair").with_style(style().typography("body"))])
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
    let (window, view) = Mount::new(
        r#"fn view(ctx) { row([text("A").with_style(style().font_size(px(30))),
                text("b").with_style(style().font_size(px(9)))]) }"#,
    )
    .theme(
        r#"fn theme() { #{ family: "Plain", name: "Dark", mode: "dark",
            tokens: #{ colors: #{ ink: 0xffffffff } } } }"#,
    )
    .no_token_base()
    .id("plain")
    .mount_window(cx);
    let mut visual = VisualTestContext::from_window(*window, cx);
    let findings = visual.update(|_, cx| view.composition_audit(cx).unwrap());
    assert!(findings.is_empty(), "{findings:?}");
}
