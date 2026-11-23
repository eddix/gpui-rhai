//! Gradient backgrounds participate in the same audit as native paint.

use gpui::TestAppContext;
use gpui_rhai::{AuditRule, MotionPreference};

use crate::support::{Mount, Step, settle};

const PROFILE: &str = r#"fn profile(){#{name:"gradient-check",audit:["low-contrast-text","multiple-solid-actions"]}}"#;

#[gpui::test]
fn actual_preset_primary_fills_are_readable_and_still_count_as_strong_actions(
    cx: &mut TestAppContext,
) {
    for (name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for count in [1, 2] {
            let script = format!(
                r#"import "components/button" as button;
                fn view(ctx) {{
                    let controls=[];
                    for i in 0..{count} {{ controls.push(button::Button(#{{key:`action-${{i}}`,text:`Action ${{i}}`}})); }}
                    row(controls).with_style(style().gap(px(8)).width(px(600)))
                }}"#
            );
            let (mut visual, view) = Mount::components(script)
                .theme(*theme)
                .motion(MotionPreference::None)
                .with(|view| view.profile_source(PROFILE))
                .id("gradient-primary")
                .mount(cx);
            settle(&mut visual, Step::Frame, 2);
            let findings = visual.update(|_, cx| view.composition_audit(cx)).unwrap();
            assert!(
                !findings
                    .iter()
                    .any(|f| f.rule == AuditRule::LowContrastText),
                "{name}: {findings:?}"
            );
            assert_eq!(
                findings
                    .iter()
                    .any(|f| f.rule == AuditRule::MultipleSolidActions),
                count == 2,
                "{name}: {findings:?}"
            );
        }
    }
}

#[gpui::test]
fn dark_light_and_interior_gradient_contrast_have_positive_controls(cx: &mut TestAppContext) {
    for (from, to, text, broken) in [
        (0x1f45b8_u32, 0x173aac_u32, 0xffffff_u32, false),
        (0xeeeeee, 0xffffff, 0xffffff, true),
        // Both endpoints contrast with black, but their darker interior does not.
        (0xff0000, 0x00ff00, 0x000000, true),
    ] {
        let script = format!(
            r#"fn view(ctx) {{ text("Gradient text").with_style(style().width(px(300)).height(px(40))
            .linear_gradient(linear_gradient(#{{angle:180,from:rgb({from}),to:rgb({to})}}))
            .text_color(rgb({text}))) }}"#
        );
        let (mut visual, view) = Mount::new(script)
            .motion(MotionPreference::None)
            .with(|view| view.profile_source(PROFILE))
            .id("gradient-evidence")
            .mount(cx);
        settle(&mut visual, Step::Frame, 2);
        let findings = visual.update(|_, cx| view.composition_audit(cx)).unwrap();
        assert_eq!(
            findings
                .iter()
                .any(|f| f.rule == AuditRule::LowContrastText),
            broken,
            "{findings:?}"
        );
    }
}

#[gpui::test]
fn layered_contrast_uses_padding_coverage_without_ignoring_visible_edges(cx: &mut TestAppContext) {
    let source = r#"
        fn view(ctx) {
            let paint = layered_gradient([
                #{angle:180,origin:"padding",clip:"padding",
                    stops:[#{offset:0,color:rgb(0x0b1120)},#{offset:1,color:rgb(__END__)}]},
                #{angle:180,origin:"border",clip:"border",
                    stops:[#{offset:0,color:rgb(0xffffff)},#{offset:1,color:rgb(0xffffff)}]}
            ]);
            column([
                box([box([text("Interior").with_style(style().text_color(rgb(0xffffff)))])
                    .with_style(style().padding(px(6)))])
                    .with_key("inside")
                    .with_style(style().width(px(240)).height(px(68)).padding(px(10))
                        .border(px(4)).border_color(rgba(0x00000000)).radius(px(12)).layered_gradient(paint)),
                box([text("Across the border")
                    .with_style(style().absolute().left(offset_px(-3)).top(px(15)).text_color(rgb(0xffffff)))])
                    .with_key("edge")
                    .with_style(style().width(px(240)).height(px(68)).padding(px(10))
                        .border(px(4)).border_color(rgba(0x00000000)).radius(px(12)).layered_gradient(paint)),
                text("Independent sibling").with_key("outside")
                    .with_style(style().text_color(rgb(0x000000)))
            ]).with_style(style().background(rgb(0xffffff)).gap(px(8)).padding(px(24)).width(px(300)))
        }
    "#;
    for (to, interior_unreadable) in [("0x172235", false), ("0xffffff", true)] {
        let (mut visual, view) = Mount::new(source.replace("__END__", to))
            .motion(MotionPreference::None)
            .with(|view| view.profile_source(PROFILE))
            .id("layered-clip-contrast")
            .mount(cx);
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(crate::support::last_error(&mut visual, &view), None);
        let findings = visual.update(|_, cx| view.composition_audit(cx)).unwrap();
        let found = |key| {
            findings
                .iter()
                .any(|f| f.rule == AuditRule::LowContrastText && f.path.contains(key))
        };
        assert_eq!(found("/inside/"), interior_unreadable, "{findings:#?}");
        assert!(
            found("/edge/"),
            "must still detect the exposed border: {findings:#?}"
        );
        assert!(
            !found("/outside"),
            "spatial surfaces must not leak into siblings: {findings:#?}"
        );
    }
}

#[gpui::test]
fn padding_fill_covers_text_inside_the_rounded_corner_but_not_outside_it(cx: &mut TestAppContext) {
    let source = r#"
        fn tile(paint, key, inset) {
            column([text("Observed label").with_style(style().text_color(rgb(0xffffff)))])
                .with_key(key)
                .with_style(style().width(px(240)).height(px(64)).padding(px(inset))
                    .border_bottom(px(1)).border_color(rgba(0x00000000)).radius(px(16))
                    .layered_gradient(paint))
        }
        fn view(ctx) {
            let dark = #{angle:180,origin:"padding",clip:"padding",
                stops:[#{offset:0,color:rgb(0x141416)},#{offset:1,color:rgb(0x141416)}]};
            let light = #{angle:180,origin:"border",clip:"border",
                stops:[#{offset:0,color:rgb(0xffffff)},#{offset:1,color:rgb(0xffffff)}]};
            column([
                tile(layered_gradient([dark, light]), "inside_arc", 9),
                tile(layered_gradient([dark, light]), "outside_arc", 3),
                tile(layered_gradient([light]), "unreadable", 9)
            ]).with_style(style().background(rgb(0xffffff)).gap(px(8)).padding(px(24)).width(px(300)))
        }
    "#;
    let (mut visual, view) = Mount::new(source)
        .motion(MotionPreference::None)
        .with(|view| view.profile_source(PROFILE))
        .id("rounded-padding-contrast")
        .mount(cx);
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(crate::support::last_error(&mut visual, &view), None);
    let findings = visual.update(|_, cx| view.composition_audit(cx)).unwrap();
    let found = |key| {
        findings
            .iter()
            .any(|finding| finding.rule == AuditRule::LowContrastText && finding.path.contains(key))
    };
    assert!(!found("/inside_arc/"), "{findings:#?}");
    assert!(found("/outside_arc/"), "{findings:#?}");
    assert!(found("/unreadable/"), "{findings:#?}");
}
