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

#[cfg(target_os = "macos")]
#[gpui::test]
fn isolated_group_contrast_matches_metal_instead_of_double_fading_ink(cx: &mut TestAppContext) {
    use crate::support::{Step, node_by_test_id, settle};
    #[cfg(feature = "gpui-fast")]
    use apple_fast::metal_renderer::MetalRenderer;
    #[cfg(feature = "gpui-pre")]
    use apple_pre::metal_renderer::MetalRenderer;

    cx.update(gpui_rhai::install);
    let source = r#"
fn witness(key,paper,ink,ink_opacity,isolated,grayscale){
    let square=box([]).with_style(style().absolute().left(px(70)).top(px(8)).width(px(20))
        .height(px(20)).background(ink).opacity(ink_opacity));
    box([text("Ink").with_key(key).with_style(style().font_size(px(12)).text_color(ink).opacity(ink_opacity)),square])
        .accessibility_role("group").accessibility_label(key).test_id(key)
        .with_style(style().relative().width(px(100)).height(px(42))
             .background(paper).isolated_opacity(isolated).opacity(0.5)
             .foreground_grayscale(grayscale))
}
fn view(ctx){
    let nested=box([witness("nested",rgba(0x33333380),rgb(0xffffff),0.8,true,0)])
        .with_style(style().width(px(100)).height(px(42))
            .background(rgba(0x666666a0)).isolated_opacity(true).opacity(0.7));
    column([witness("legible",rgb(0xffffff),rgb(0),1.0,true,0),
            witness("faint",rgb(0x4d4d4d),rgb(0xffffff),1.0,true,0),nested,
            witness("ordinary",rgb(0xffffff),rgb(0),1.0,false,0),
            witness("grayscale",rgba(0x00ea0080),rgb(0xff0000),0.7,false,1)])
        .with_style(style().width(px(100)).gap(px(6)).background(rgb(0)))
}
"#;
    let (window, view) = mount(cx, source);
    let mut v = VisualTestContext::from_window(*window, cx);
    v.simulate_resize(gpui::size(gpui::px(200.), gpui::px(281.)));
    settle(&mut v, Step::Tick, 4);
    let labels = ["legible", "faint", "nested", "ordinary", "grayscale"];
    let areas = labels.map(|id| node_by_test_id(&mut v, &view, id).geometry.unwrap().visual);
    let pixels = v.update(|window, _| {
        let scale = window.scale_factor();
        let viewport = window.viewport_size();
        let extent = gpui::size(
            gpui::DevicePixels((f32::from(viewport.width) * scale).ceil() as i32),
            gpui::DevicePixels((f32::from(viewport.height) * scale).ceil() as i32),
        );
        window.visual_probe_with_painted_scene(|scene| {
            let mut renderer = MetalRenderer::new_headless(Default::default()).unwrap();
            let image = renderer.render_scene_to_image(scene, extent).unwrap();
            areas.map(|area| {
                let at = |dx: f64, dy: f64| {
                    image
                        .get_pixel(
                            ((area.x + dx) * f64::from(scale)).floor() as u32,
                            ((area.y + dy) * f64::from(scale)).floor() as u32,
                        )
                        .0
                };
                (at(50., 30.), at(78., 18.))
            })
        })
    });
    let ratio = pixels.map(|(paper, ink)| {
        let luminance = |v: u8| {
            let s = f64::from(v) / 255.;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        let [a, b] = [luminance(paper[0]), luminance(ink[0])];
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    });
    assert!(
        ratio[0] > 5.0
            && (3.6..4.0).contains(&ratio[1])
            && (1.2..3.5).contains(&ratio[2])
            && (2.5..2.8).contains(&ratio[3]),
        "{pixels:?}, {ratio:?}"
    );
    assert_eq!(pixels[4].0[0], pixels[4].0[1]);
    assert_eq!(pixels[4].1[0], pixels[4].1[1]);
    let rules = gpui_rhai::AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let findings = v.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
    assert_eq!(findings.len(), 4, "Metal {ratio:?}, audit {findings:#?}");
    for (index, label) in labels.iter().enumerate().skip(1) {
        let finding = findings
            .iter()
            .find(|finding| finding.path.rsplit('/').next() == Some(*label))
            .unwrap_or_else(|| panic!("Metal {ratio:?}, audit {findings:#?}"));
        let evidence = finding.contrast.as_ref().unwrap();
        assert!(!evidence.conservative);
        assert!(
            (evidence.ratio - ratio[index]).abs() < 0.08,
            "{evidence:?}, Metal {ratio:?}"
        );
        let backdrop = evidence.background_srgb.unwrap();
        for (channel, expected) in backdrop.into_iter().enumerate() {
            assert!((expected * 255. - f64::from(pixels[index].0[channel])).abs() < 2.);
        }
    }
}

#[gpui::test]
fn equal_font_counts_in_independent_apps_do_not_share_inventory(cx: &mut TestAppContext) {
    const MAIN: &str = r#"fn view(ctx) {
        text("Font inventory").with_style(style().font_family("Inter"))
    }"#;
    let inspect = |cx: &mut TestAppContext, sources: Vec<gpui_rhai::FontSource>| {
        let (mut visual, view) = Mount::new(MAIN)
            .no_token_base()
            .with(move |view| view.font_sources(sources).profile_source(PROFILE))
            .id("font-inventory")
            .mount(cx);
        visual.run_until_parked();
        visual.update(|_, cx| view.composition_audit(cx)).unwrap()
    };
    let mut first = TestAppContext::build(cx.dispatcher.clone(), None);
    let fonts = gpui_rhai_registry::BUNDLED_FONT_SOURCES
        .iter()
        .map(|&(name, bytes)| gpui_rhai::FontSource::new(name, bytes.to_vec()).unwrap())
        .collect();
    let findings = inspect(&mut first, fonts);
    assert!(
        !findings
            .iter()
            .any(|finding| finding.rule == gpui_rhai::AuditRule::UnresolvedFont),
        "{findings:?}"
    );

    // The no-op platform accepts these three distinct header-only payloads.
    // Neither equal counts nor the Host label are evidence of an Inter face.
    let mut second = TestAppContext::build(cx.dispatcher.clone(), None);
    let fonts = (0..3_u8)
        .map(|index| {
            gpui_rhai::FontSource::new("Inter", [b"OTTO".as_slice(), &[index]].concat()).unwrap()
        })
        .collect();
    let findings = inspect(&mut second, fonts);
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == gpui_rhai::AuditRule::UnresolvedFont),
        "{findings:?}"
    );
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

#[cfg(not(feature = "vendor-gpui"))]
use crate::support::paint::PublicSceneWitness as _;
