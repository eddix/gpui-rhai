//! Inline legends are bare metadata, not nested keycaps or an audit exemption for body text.

use crate::support::{Mount, Step, bounds, filled, last_error, nodes, painted_quads, settle};
use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{AuditRule, AuditRules, MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::{AR_LOCALE, EN_LOCALE};

struct KeycapCase<'a> {
    name: &'a str,
    theme: &'a str,
    scale: i32,
    density: &'a str,
    size: &'a str,
    platform: &'a str,
    rtl: bool,
}

impl KeycapCase<'_> {
    fn mount(&self, cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
        let source = format!(
            r#"
            import "components/kbd" as kbd;
            fn init(ctx){{ctx.set_locale("{}");}}
            fn view(ctx){{column([
                kbd::Kbd(#{{keys:["mod","shift","p"],size:"{}",platform:"{}"}}),
                text("Outside"),kbd::Kbd(#{{keys:[" ",""],text:""}})
            ]).with_style(style().width(px(600)).items_start().padding(px(20)).gap(px(16)))
                .env(#{{scale:"{}",density:"{}"}})}}
        "#,
            if self.rtl { "ar" } else { "en" },
            self.size,
            self.platform,
            self.scale,
            self.density,
        );
        let (mut visual, view) = Mount::components(source)
            .theme(self.theme)
            .motion(MotionPreference::None)
            .locale("en.rhai", EN_LOCALE)
            .locale("ar.rhai", AR_LOCALE)
            .id("keycap-geometry")
            .mount(cx);
        settle(&mut visual, Step::Frame, 2);
        assert_eq!(last_error(&mut visual, &view), None);
        (visual, view)
    }

    fn assert_layout(&self, visual: &mut VisualTestContext, view: &ScriptViewHandle) {
        let clean = self.name.starts_with("clean");
        let factor = f64::from(self.scale) / 100.0;
        let name = if self.platform == "mac" {
            "Command Shift P"
        } else {
            "Control Shift P"
        };
        let cap = bounds(visual, view, "text", name);
        let height = match (clean, self.size) {
            (true, "sm") => 16.0,
            (true, _) => 20.0,
            (false, "sm") => 18.0,
            (false, _) => 22.0,
        } * factor;
        assert!(
            (cap.height - height).abs() < 0.5,
            "{}/{}/{}/{}/{}: {cap:?}",
            self.name,
            self.size,
            self.platform,
            self.scale,
            self.density
        );
        let labels = if self.platform == "mac" {
            ["⌘", "⇧", "P"]
        } else {
            ["Ctrl", "Shift", "P"]
        };
        let keys = labels.map(|key| bounds(visual, view, "presentation", key));
        if self.platform == "other" {
            assert!(
                keys[0].width > keys[2].width * 1.5 && keys[1].width > keys[2].width * 1.5,
                "long names must not allocate equal-width slots to a single letter: {keys:?}"
            );
        }
        let gap = if self.platform == "mac" { 1.0 } else { 3.0 } * factor;
        for pair in keys.windows(2) {
            assert!(
                (pair[1].x - pair[0].x - pair[0].width - gap).abs() < 0.5,
                "{}: order/gap must survive RTL and compact density: {keys:?}",
                self.name
            );
        }
        let text_size = if self.size == "md" {
            12.0
        } else if clean {
            10.5
        } else {
            11.0
        };
        assert!((keys[2].height - text_size * factor).abs() < 0.5);
        if self.platform == "mac" {
            assert!((keys[0].height - text_size * 1.1 * factor).abs() < 0.5);
        }
        let tallest = keys
            .iter()
            .max_by(|a, b| a.height.total_cmp(&b.height))
            .unwrap();
        let optical = if clean { 0.0 } else { -0.5 * factor };
        assert!(
            (tallest.y + tallest.height / 2.0 - cap.y - cap.height / 2.0 - optical).abs() < 0.5,
            "{}: optical centering: {cap:?}/{keys:?}",
            self.name
        );
        let ink: gpui::Rgba = visual.update(|_, cx| {
            let theme = view.theme_snapshot(cx).unwrap().variant;
            if clean {
                let foreground = theme.tokens.color("text_primary").unwrap();
                let alpha = if theme.mode == gpui_rhai::ThemeMode::Dark {
                    0.10
                } else {
                    0.07
                };
                let mut ink = gpui::rgba(foreground.as_rgba_hex());
                ink.a *= alpha;
                ink
            } else {
                gpui::rgba(theme.tokens.color("kbd.surface").unwrap().as_rgba_hex())
            }
        });
        let matching: Vec<_> = painted_quads(visual)
            .into_iter()
            .filter(|quad| {
                quad.background.as_solid().is_some_and(|fill| {
                    let actual: gpui::Rgba = fill.into();
                    [
                        actual.r - ink.r,
                        actual.g - ink.g,
                        actual.b - ink.b,
                        actual.a - ink.a,
                    ]
                    .into_iter()
                    .all(|delta| delta.abs() < 0.00001)
                })
            })
            .filter(|quad| {
                (f64::from(quad.bounds.x) - cap.x).abs() < 0.5
                    && (f64::from(quad.bounds.y) - cap.y).abs() < 0.5
                    && (f64::from(quad.bounds.width) - cap.width).abs() < 0.5
            })
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "{}: exactly one keycap surface",
            self.name
        );
        let radius = if self.size == "md" {
            if clean { 5.0 } else { 6.0 }
        } else if clean {
            4.0
        } else {
            5.0
        } * factor;
        assert!(
            matching[0]
                .corner_radii
                .iter()
                .all(|r| (f64::from(*r) - radius).abs() < 0.5)
        );
        let text_names: Vec<_> = nodes(visual, view)
            .into_iter()
            .filter(|node| node.role == "text")
            .map(|node| node.name)
            .collect();
        assert_eq!(
            text_names,
            [name, "Outside"],
            "glyphs must not duplicate the spoken legend or render an empty chip"
        );
    }
}

#[gpui::test]
fn keycap_combinations_follow_the_source_sizes_and_keep_ltr_order_in_rtl(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for scale in [100, 200] {
            for density in ["comfortable", "compact"] {
                for size in ["sm", "md"] {
                    for platform in ["mac", "other"] {
                        for rtl in [false, true] {
                            let case = KeycapCase {
                                name,
                                theme,
                                scale,
                                density,
                                size,
                                platform,
                                rtl,
                            };
                            let (mut visual, view) = case.mount(cx);
                            case.assert_layout(&mut visual, &view);
                        }
                    }
                }
            }
        }
    }
}

#[gpui::test]
fn inline_key_legends_clear_keycap_geometry_without_erasing_app_overrides(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for scale in [100, 200] {
            for custom in [false, true] {
                let sheet = if custom {
                    r#"fn component_styles(){#{"components/kbd":#{inline:style().background(rgba(0x123456ff))}}}"#
                } else {
                    "fn component_styles(){#{}}"
                };
                let (mut visual, view) = Mount::components(format!(
                    r#"
                    import "components/kbd" as kbd;
                    fn view(ctx){{column([
                        row([text("Action").with_style(style().typography("body")),
                            kbd::Kbd(#{{text:"Ctrl K",label:"Inline legend",appearance:"inline"}})])
                            .with_style(style().items_baseline().gap(px(8))),
                        kbd::Kbd(#{{text:"Esc",label:"Framed key"}})
                    ]).with_style(style().items_start().padding(px(20)).gap(px(20)))
                        .env(#{{scale:"{scale}"}})}}
                "#
                ))
                .theme(theme)
                .motion(MotionPreference::None)
                .with(move |view| view.component_styles(sheet))
                .id("inline-key-legend")
                .mount(cx);
                settle(&mut visual, Step::Frame, 2);
                assert_eq!(last_error(&mut visual, &view), None);
                let inline = bounds(&mut visual, &view, "text", "Inline legend");
                assert_eq!(
                    inline.height,
                    16.0 * f64::from(scale) / 100.0,
                    "{name}: bare caption line, {inline:?}"
                );
                let at = |quad: &crate::support::PaintedQuad| {
                    (f64::from(quad.bounds.x) - inline.x).abs() < 0.5
                        && (f64::from(quad.bounds.y) - inline.y).abs() < 0.5
                };
                if custom {
                    assert!(
                        filled(&mut visual, 0x1234_56ff).iter().any(at),
                        "{name}: inline part is still public"
                    );
                } else {
                    assert!(
                        !painted_quads(&mut visual).iter().any(|quad| at(quad)
                            && (!quad.background.is_transparent() || quad.has_border())),
                        "{name}: keycap paint leaked into inline legend"
                    );
                }
                let cap = bounds(&mut visual, &view, "text", "Framed key");
                assert!(
                    painted_quads(&mut visual)
                        .iter()
                        .any(|quad| (f64::from(quad.bounds.x) - cap.x).abs() < 0.5
                            && (f64::from(quad.bounds.y) - cap.y).abs() < 0.5
                            && !quad.background.is_transparent()),
                    "{name}: normal appearance keeps its surface"
                );
                let rules = AuditRules::from_ids(["mixed-type-in-row"]).unwrap();
                let findings =
                    visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
                assert!(
                    findings.is_empty(),
                    "{name}: a named key legend is metadata, {findings:?}"
                );
            }
        }
    }
}

#[gpui::test]
fn contextual_menu_caps_keep_twenty_pixel_geometry_and_inherit_ink_below_host_parts(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for scale in [100, 200] {
            for custom in [false, true] {
                let sheet = if custom {
                    r#"fn component_styles(){#{"components/kbd":#{root:style().background(rgba(0xff00ffff))}}}"#
                } else {
                    "fn component_styles(){#{}}"
                };
                let inherited_ink = if !custom && name == "modern_dark.rhai" {
                    0x232326ff_u32
                } else {
                    0xffffffff_u32
                };
                let (mut visual,view) = Mount::components(format!(r#"
                    import "components/kbd" as kbd;
                    fn view(ctx){{row([
                        kbd::Kbd(#{{keys:"p",label:"Menu P",appearance:"menu",size:"sm"}}),
                        kbd::Kbd(#{{keys:"k",label:"Menu K",appearance:"menu",size:"md"}})
                    ]).with_style(style().padding(px(20)).gap(px(8)).items_center()
                        .background(rgb(0xffffff)).text_color(rgba(0x{inherited_ink:08x}))).env(#{{scale:"{scale}"}})}}
                "#)).theme(theme).motion(MotionPreference::None)
                    .with(move |view|view.component_styles(sheet))
                    .id("menu-keycaps").mount(cx);
                settle(&mut visual, Step::Frame, 2);
                let factor = f64::from(scale) / 100.0;
                for label in ["Menu P", "Menu K"] {
                    let cap = bounds(&mut visual, &view, "text", label);
                    assert!(
                        (cap.height - 20.0 * factor).abs() < 0.5
                            && (cap.width - 20.0 * factor).abs() < 0.5,
                        "{name}/{scale}/{label}: contextual caps are independent of standalone size: {cap:?}"
                    );
                    if custom {
                        assert!(
                            filled(&mut visual, 0xff00ffff)
                                .iter()
                                .any(|quad| (f64::from(quad.bounds.x) - cap.x).abs() < 0.5
                                    && (f64::from(quad.bounds.y) - cap.y).abs() < 0.5),
                            "{name}: Host root style must win over the preset menu alias"
                        );
                    }
                }
                let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
                let findings =
                    visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
                assert!(
                    findings.len() >= 2,
                    "{name}: deliberate low-contrast inherited text must still be audited: {findings:?}"
                );
                assert!(
                    findings.iter().all(|finding| finding
                        .contrast
                        .as_ref()
                        .is_some_and(|evidence| evidence.foreground
                            == gpui_rhai::Rgba8::from_rgba_hex(inherited_ink))),
                    "{name}: contextual cap reapplied a standalone ink: {findings:?}"
                );
                assert!(last_error(&mut visual, &view).is_none());
            }
        }
    }
}

#[gpui::test]
fn ordinary_caption_body_type_errors_still_report_beside_a_key_legend(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(
        r#"
        import "components/kbd" as kbd;
        fn view(ctx){row([
            text("Body").with_style(style().typography("body")),
            text("Unrelated small prose").with_style(style().typography("caption")),
            kbd::Kbd(#{text:"Ctrl K",appearance:"inline"})
        ]).with_style(style().items_baseline().gap(px(8)))}
    "#,
    )
    .motion(MotionPreference::None)
    .id("key-legend-type-control")
    .mount(cx);
    settle(&mut visual, Step::Frame, 2);
    let rules = AuditRules::from_ids(["mixed-type-in-row"]).unwrap();
    let findings = visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule == AuditRule::MixedTypeInRow),
        "{findings:?}"
    );
}
