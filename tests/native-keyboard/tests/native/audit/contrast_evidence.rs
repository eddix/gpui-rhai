//! Numeric contrast facts stay strict, including effective native opacity.

use gpui::TestAppContext;
use gpui_rhai::{AuditRule, MotionPreference, Rgba8};

use crate::support::{Mount, Step, by_test_id, dispatch, settle};

const PROFILE: &str = r#"fn profile(){#{name:"contrast-evidence",audit:["low-contrast-text"]}}"#;

#[gpui::test]
fn contrast_reports_painted_ink_but_not_empty_text_nodes(cx: &mut TestAppContext) {
    let (mut v, view) = Mount::new(
        r#"fn view(ctx){
            let st=style().width(px(140)).height(px(24)).text_color(rgb(0xeeeeee));
            column([
                text("").with_key("empty").with_style(st),
                text([span("")]).with_key("empty-rich").with_style(st),
                text("Visible ink").with_key("visible").accessibility_hidden(true).with_style(st)
            ]).with_style(style().width(px(300)).background(rgb(0xffffff)))
        }"#,
    )
    .with(|view| view.profile_source(PROFILE))
    .motion(MotionPreference::None)
    .mount(cx);
    settle(&mut v, Step::Frame, 3);
    assert_eq!(crate::support::last_error(&mut v, &view), None);
    let findings = v.update(|_, cx| view.composition_audit(cx).unwrap());
    let paths = findings
        .iter()
        .map(|finding| finding.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(paths, ["root/visible"], "{findings:#?}");
}

#[gpui::test]
fn loading_switch_glyphs_have_a_paired_foreground_in_every_preset(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = Mount::components(
            r#"
            import "components/switch" as sw;
            fn view(ctx){column([
                sw::Switch(#{checked:false,loading:true,label:"Pending off"}),
                sw::Switch(#{checked:true,loading:true,label:"Pending on"})
            ]).with_style(style().background(theme_color("surface_raised")))}
        "#,
        )
        .theme(theme)
        .motion(MotionPreference::None)
        .with(|view| view.profile_source(PROFILE))
        .mount(cx);
        settle(&mut visual, Step::Frame, 3);
        let findings = visual.update(|_, cx| view.composition_audit(cx).unwrap());
        assert!(
            findings.is_empty(),
            "{name}: loading paint is derived, not a source-color exception: {findings:?}"
        );
    }
}

#[gpui::test]
fn adjacent_month_dates_follow_live_subtle_ink_not_the_placeholder_role(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = Mount::components(
            r#"
            import "components/calendar" as calendar;
            fn view(ctx){calendar::Calendar(#{key:"calendar",label:"Calendar",
                value:"2026-10-14",today:"2026-10-14"})}
        "#,
        )
        .theme(theme)
        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
        .motion(MotionPreference::None)
        .with(|view| {
            view.profile_source(PROFILE)
                .theme_token_overrides(gpui_rhai::ThemeTokenOverrides {
                    colors: [
                        ("text.subtle".to_owned(), Rgba8::from_rgba_hex(0xffff_ffff)),
                        (
                            "surface_raised".to_owned(),
                            Rgba8::from_rgba_hex(0xffff_ffff),
                        ),
                        (
                            "field.placeholder".to_owned(),
                            Rgba8::from_rgba_hex(0x00ff_00ff),
                        ),
                    ]
                    .into(),
                    ..Default::default()
                })
        })
        .mount(cx);
        settle(&mut visual, Step::Frame, 3);
        let findings = visual.update(|_, cx| view.composition_audit(cx).unwrap());
        let outside = findings
            .iter()
            .find(|finding| finding.path.ends_with("/2026-09-27/0"))
            .unwrap_or_else(|| panic!("{name}: visible outside-day evidence absent: {findings:?}"));
        let evidence = outside.contrast.as_ref().unwrap();
        assert_eq!(
            evidence.foreground,
            Rgba8::from_rgba_hex(0xffff_ffff),
            "{name}"
        );
        assert_eq!(evidence.background_srgb, Some([1.0; 3]), "{name}");
        assert_eq!(evidence.ratio, 1.0, "{name}");
    }
}

fn luminance(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[gpui::test]
fn text_opacity_is_reported_separately_from_authored_color(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(
        r#"fn view(ctx){column([
        text("Faint text").with_style(style().font_size(px(13)).text_color(rgb(0)).opacity(0.2))
    ]).with_style(style().width(px(300)).height(px(80)).background(rgb(0xffffff)))}"#,
    )
    .motion(MotionPreference::None)
    .with(|v| v.profile_source(PROFILE))
    .mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let findings = visual.update(|_, cx| view.composition_audit(cx).unwrap());
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].rule, AuditRule::LowContrastText);
    let evidence = findings[0].contrast.as_ref().unwrap();
    assert_eq!(evidence.foreground, Rgba8::from_rgba_hex(0x0000_00ff));
    assert_eq!(evidence.element_opacity, 0.2);
    assert_eq!(evidence.background_srgb, Some([1.0; 3]));
    assert_eq!(evidence.minimum, 4.5);
    assert!(!evidence.conservative);
    assert!((evidence.ratio - 1.05 / (luminance(0.8) + 0.05)).abs() < 1e-9);
}

#[gpui::test]
fn semantically_hidden_decoration_stays_painted_and_contrast_audited(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(r#"fn view(ctx){
        text("Visible decoration").accessibility_hidden(true)
            .with_style(style().font_size(px(13)).text_color(rgb(0xffffff)).background(rgb(0xffffff)))
    }"#).with(|view|view.profile_source(PROFILE)).mount(cx);
    settle(&mut visual, Step::Frame, 3);
    assert!(
        !crate::support::snapshot(&mut visual, &view)
            .nodes()
            .any(|node| node.name == "Visible decoration")
    );
    let findings = visual.update(|_, cx| view.composition_audit(cx).unwrap());
    assert_eq!(
        findings.len(),
        1,
        "semantic hiding is not a visual-audit exemption: {findings:?}"
    );
    assert_eq!(findings[0].contrast.as_ref().unwrap().ratio, 1.0);
}

#[gpui::test]
fn elided_fragments_do_not_hide_their_visible_text_from_the_audit(cx: &mut TestAppContext) {
    let (mut visual,view)=Mount::new(r#"fn view(ctx){column([
        fragment([fragment([text("Still visible").with_style(style().font_size(px(13)).text_color(rgb(0xffffff)).background(rgb(0xffffff)))])])
    ]).with_style(style().width(px(300)).height(px(80)))}"#)
        .motion(MotionPreference::None).with(|v|v.profile_source(PROFILE)).mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let findings = visual.update(|_, cx| view.composition_audit(cx).unwrap());
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].contrast.as_ref().unwrap().ratio, 1.0);
}

#[gpui::test]
fn nested_opacity_matches_per_primitive_native_compositing_not_group_isolation(
    cx: &mut TestAppContext,
) {
    let (mut visual, view) = Mount::new(
        r#"fn view(ctx){column([
        row([text("Nested").with_style(style().font_size(px(13)).text_color(rgb(0)).opacity(0.5))])
            .with_style(style().background(rgb(0xffffff)).opacity(0.5))
    ]).with_style(style().width(px(300)).height(px(80)).background(rgb(0)))}"#,
    )
    .motion(MotionPreference::None)
    .with(|v| v.profile_source(PROFILE))
    .mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let findings = visual.update(|_, cx| view.composition_audit(cx).unwrap());
    assert_eq!(findings.len(), 1, "{findings:?}");
    let evidence = findings[0].contrast.as_ref().unwrap();
    assert_eq!(evidence.element_opacity, 0.25);
    assert_eq!(evidence.background_srgb, Some([0.5; 3]));
    let expected = (luminance(0.5) + 0.05) / (luminance(0.375) + 0.05);
    assert!((evidence.ratio - expected).abs() < 1e-9);
}

#[gpui::test]
fn closed_overlay_and_hidden_measurement_text_are_not_visible_contrast_failures(
    cx: &mut TestAppContext,
) {
    let (mut visual,view)=Mount::components(r#"
        import "components/dialog" as dialog;
        fn state_schema(){#{fields:#{open:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}}}
        fn open(ctx,p){ctx.set_state("open",true);}
        fn view(ctx){column([
            text("Hidden measurement").with_style(style().text_color(rgb(0xffffff)).background(rgb(0xffffff)).opacity(0.0)),
            text("Invisible measurement").with_style(style().text_color(rgb(0xffffff)).background(rgb(0xffffff)).invisible()),
            text("Open").test_id("open").on_click(Fn("open")),
            dialog::Dialog(#{key:"contrast",open:ctx.get_state("open"),title:"Contrast probe",
                content:text("Visible failure").with_style(style().font_size(px(13)).text_color(rgb(0xffffff)).background(rgb(0xffffff)))})
        ]).with_style(style().width(px(400)).height(px(300)))}
    "#).motion(MotionPreference::None).with(|v|v.profile_source(PROFILE)).mount(cx);
    settle(&mut visual, Step::Frame, 3);
    assert!(
        visual
            .update(|_, cx| view.composition_audit(cx).unwrap())
            .is_empty()
    );
    dispatch(&mut visual, &view, by_test_id("open"), "click").unwrap();
    settle(&mut visual, Step::Frame, 3);
    let findings = visual.update(|_, cx| view.composition_audit(cx).unwrap());
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].contrast.as_ref().unwrap().ratio, 1.0);
}
