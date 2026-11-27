//! Derived chips keep paired tone paint, native close behavior and live theme overrides.

use crate::support::{
    Mount, Step, bounds, click_center, filled, last_error, nodes, painted_quads, settle, text_of,
};
use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{
    AuditRules, FocusIndicators, GeometryBounds, MotionPreference, Rgba8, ScriptViewHandle,
    ThemeTokenOverrides,
};

const MAIN: &str = r#"
import "components/tag" as tag;
fn state_schema(){#{fields:#{log:#{schema:#{type:"string"},"default":#{type:"string",value:""}}}}}
fn closed(ctx,value){ctx.set_state("log",value);}
fn view(ctx){
    let chips=[];
    for tone in ["neutral","accent","success","warning","danger"] {
        chips.push(tag::Tag(#{text:`${tone} plain`,variant:tone,closable:true,on_close:Fn("closed")}));
        chips.push(tag::Tag(#{text:`${tone} facet`,facet:"kind",variant:tone,closable:true,on_close:Fn("closed")}));
    }
    chips.push(tag::Tag(#{text:"locked",closable:true,disabled:true,on_close:Fn("closed")}));
    chips.push(tag::Tag(#{text:"empty",facet:""}));
    chips.push(text(ctx.get_state("log")).test_id("log"));
    column(chips).with_style(style().items_start().width(px(600)).padding(px(20)).gap(px(6))
        .background(theme_color("surface_raised"))).env(#{density:"__DENSITY__",scale:"__SCALE__"})
}
"#;

fn mount(
    cx: &mut TestAppContext,
    theme: &str,
    scale: i32,
    density: &str,
    styles: &str,
) -> (VisualTestContext, ScriptViewHandle) {
    let source = MAIN
        .replace("__DENSITY__", density)
        .replace("__SCALE__", &scale.to_string());
    let sheet = styles.to_owned();
    let (mut visual, view) = Mount::components(source)
        .theme(theme)
        .motion(MotionPreference::None)
        .config(|config| config.focus_indicators(FocusIndicators::Always))
        .with(move |view| {
            view.component_styles(sheet)
                .theme_token_overrides(ThemeTokenOverrides {
                    colors: [("focus_ring".to_owned(), Rgba8::from_rgba_hex(0x00ff_00ff))].into(),
                    ..Default::default()
                })
        })
        .id("preset-chips")
        .mount(cx);
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

fn assert_chip_geometry(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    name: &str,
    scale: i32,
) {
    let factor = f64::from(scale) / 100.0;
    let chip = bounds(visual, view, "status", "neutral plain");
    assert_eq!(chip.height, 24.0 * factor, "{name}/{scale}: {chip:?}");
    let value = bounds(visual, view, "text", "neutral plain");
    assert_eq!(value.height, 20.0 * factor);
    assert!((value.y - chip.y - 2.0 * factor).abs() < 0.5);
    let color = visual.update(|_, cx| {
        view.theme_snapshot(cx)
            .unwrap()
            .variant
            .tokens
            .color("tag.chip")
            .unwrap()
    });
    let quads = filled(visual, color.as_rgba_hex());
    let painted = quads
        .iter()
        .find(|quad| {
            (f64::from(quad.bounds.x) - chip.x).abs() < 0.5
                && (f64::from(quad.bounds.y) - chip.y).abs() < 0.5
        })
        .unwrap();
    let radius = if name.starts_with("clean") { 5.0 } else { 7.0 } * factor;
    assert!(
        painted
            .corner_radii
            .iter()
            .all(|r| (f64::from(*r) - radius).abs() < 0.5)
    );
    let close = bounds(visual, view, "button", "Remove neutral plain");
    assert_eq!((close.width, close.height), (18.0 * factor, 18.0 * factor));
    assert!(
        (chip.x + chip.width - close.x - close.width - 3.0 * factor).abs() < 0.5,
        "{name}/{scale}: close inset, {chip:?}, {close:?}"
    );
}

#[gpui::test]
fn tag_tones_and_facets_are_readable_and_follow_chip_geometry(cx: &mut TestAppContext) {
    let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for scale in [100, 200] {
            for density in ["comfortable", "compact"] {
                let (mut visual, view) =
                    mount(cx, theme, scale, density, "fn component_styles(){#{}}");
                assert_chip_geometry(&mut visual, &view, name, scale);
                let findings =
                    visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
                assert!(
                    findings.is_empty(),
                    "{name}/{scale}/{density}: {findings:?}"
                );
                let locked = nodes(&mut visual, &view)
                    .into_iter()
                    .find(|node| node.role == "button" && node.name == "Remove locked")
                    .unwrap();
                assert!(locked.disabled);
                click_center(&mut visual, locked.geometry.unwrap().visual);
                settle(&mut visual, Step::Frame, 2);
                assert_eq!(text_of(&mut visual, &view, "log"), "");
                let close = bounds(&mut visual, &view, "button", "Remove accent plain");
                click_center(&mut visual, close);
                settle(&mut visual, Step::Frame, 2);
                assert_eq!(text_of(&mut visual, &view, "log"), "accent plain");
            }
        }
    }
}

fn covers(quad: &crate::support::PaintedQuad, target: GeometryBounds) -> bool {
    (f64::from(quad.bounds.x) - target.x).abs() < 0.5
        && (f64::from(quad.bounds.y) - target.y).abs() < 0.5
        && (f64::from(quad.bounds.width) - target.width).abs() < 0.5
        && (f64::from(quad.bounds.height) - target.height).abs() < 0.5
}

#[gpui::test]
fn a_broken_application_tag_override_still_reports_low_contrast(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        gpui_rhai_registry::MODERN_LIGHT_THEME,
        100,
        "comfortable",
        r#"
        fn component_styles(){#{"components/tag":#{root:style().background(rgba(0x123456ff)).text_color(rgba(0x123456ff))}}}
    "#,
    );
    let rules = AuditRules::from_ids(["low-contrast-text"]).unwrap();
    let findings = visual.update(|_, cx| view.composition_audit_with(&rules, cx).unwrap());
    assert!(
        !findings.is_empty(),
        "readable default recipes must not waive a bad Host override"
    );
}

#[gpui::test]
fn tag_close_focus_and_instance_layers_keep_their_public_contract(cx: &mut TestAppContext) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let (mut visual, view) = mount(
            cx,
            theme,
            100,
            "comfortable",
            r#"fn component_styles(){#{
            "components/tag":#{facet:style().background(rgba(0x123456ff)).text_color(rgba(0xfefefeff))}
        }}"#,
        );
        assert!(
            !filled(&mut visual, 0x1234_56ff).is_empty(),
            "{name}: a preset must not erase application facet paint"
        );
        visual.update(|window, cx| view.focus(window, cx)).unwrap();
        visual.simulate_keystrokes("tab");
        settle(&mut visual, Step::Frame, 2);
        let close = bounds(&mut visual, &view, "button", "Remove neutral plain");
        let ring: gpui::Hsla = gpui::rgba(0x00ff_00ff).into();
        assert!(
            painted_quads(&mut visual)
                .iter()
                .any(|quad| quad.border_color == ring && quad.has_border() && covers(quad, close)),
            "{name}: close owns its inset cue"
        );
        visual.simulate_keystrokes("enter");
        settle(&mut visual, Step::Frame, 2);
        assert_eq!(text_of(&mut visual, &view, "log"), "neutral plain");
    }
}
