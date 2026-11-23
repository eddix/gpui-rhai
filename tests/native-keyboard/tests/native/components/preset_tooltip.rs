//! Tooltip polarity, alpha, shortcut treatment and logical optical insets.

use crate::support::{Mount, Step, bounds, center, filled, last_error, settle};
use gpui::{Modifiers, TestAppContext, VisualTestContext};
use gpui_rhai::{MotionPreference, ScriptViewHandle, Style, UiNode, UiNodeKind};
use gpui_rhai_registry::{AR_LOCALE, EN_LOCALE};

fn mount(
    cx: &mut TestAppContext,
    theme: &str,
    scale: i32,
    density: &str,
    rtl: bool,
    shortcut: bool,
) -> (VisualTestContext, ScriptViewHandle) {
    let main = format!(
        r#"
        import "components/button" as button;
        import "components/tooltip" as tooltip;
        fn init(ctx){{ctx.set_locale("{}");}}
        fn noop(ctx,value){{}}
        fn view(ctx){{column([
            tooltip::Tooltip(#{{key:"hint",label:"Hint",placement:"bottom",show_delay_ms:0,
                trigger:button::Button(#{{text:"Hover me",on_click:Fn("noop")}}),
                content:text("Tooltip body"),shortcut:{} }})
        ]).with_style(style().width(px(700)).padding(px(20)))
            .env(#{{scale:"{scale}",density:"{density}"}})}}
    "#,
        if rtl { "ar" } else { "en" },
        if shortcut { "\"⌘K\"" } else { "()" }
    );
    let (mut visual, view) = Mount::components(main)
        .theme(theme)
        .motion(MotionPreference::None)
        .locale("en.rhai", EN_LOCALE)
        .locale("ar.rhai", AR_LOCALE)
        .id("preset-tip")
        .mount(cx);
    settle(&mut visual, Step::Frame, 2);
    let trigger = bounds(&mut visual, &view, "button", "Hover me");
    visual.simulate_mouse_move(center(trigger), None, Modifiers::none());
    settle(&mut visual, Step::Refresh, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

fn shortcut_style(node: &UiNode) -> Option<&Style> {
    match node.kind() {
        UiNodeKind::Text { text } if text == "⌘K" => Some(node.style()),
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(shortcut_style)
        }
        UiNodeKind::Overlay {
            trigger, content, ..
        } => shortcut_style(trigger).or_else(|| shortcut_style(content)),
        UiNodeKind::Layer { content, .. } => shortcut_style(content),
        UiNodeKind::ErrorBoundary { child, fallback } => {
            shortcut_style(child).or_else(|| shortcut_style(fallback))
        }
        _ => None,
    }
}

fn assert_shortcut_alpha(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    clean: bool,
    dark: bool,
) {
    visual.update(|_, cx| {
        let root = view.root(cx).unwrap().unwrap();
        assert_eq!(shortcut_style(&root).unwrap().base.opacity, Some(1.0));
        let expected = match (clean, dark) {
            (false, false) => 0xffff_ffb8,
            (false, true) => 0x0000_0099,
            (true, false) => 0xffff_ff99,
            (true, true) => 0x0000_0080,
        };
        assert_eq!(
            view.theme_snapshot(cx)
                .unwrap()
                .variant
                .tokens
                .color("tooltip.muted")
                .unwrap()
                .as_rgba_hex(),
            expected
        );
    });
}

#[gpui::test]
fn tooltip_presets_keep_inverse_polarity_and_single_authored_shortcut_alpha(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for scale in [100, 200] {
            for density in ["comfortable", "compact"] {
                for rtl in [false, true] {
                    for shortcut in [false, true] {
                        let (mut visual, view) = mount(cx, theme, scale, density, rtl, shortcut);
                        let clean = name.starts_with("clean");
                        let dark = name.contains("dark");
                        let factor = f64::from(scale) / 100.0;
                        let space = if density == "compact" { 0.75 } else { 1.0 } * factor;
                        let panel = bounds(&mut visual, &view, "tooltip", "Hint");
                        let body = bounds(&mut visual, &view, "text", "Tooltip body");
                        visual.update(|window, _| window.visual_probe_with_painted_scene(|scene| {
                            assert_eq!(scene.has_backdrop_blur(), clean, "{name}: only Clean filters the tooltip backdrop");
                            if clean {
                                let scale = window.scale_factor();
                                assert!(scene.backdrop_blurs.iter().any(|blur| {
                                    (blur.sigma - 8. * scale).abs() < 0.01
                                        && (f64::from(blur.bounds.origin.x.0 / scale) - panel.x).abs() < 1.
                                        && (f64::from(blur.bounds.origin.y.0 / scale) - panel.y).abs() < 1.
                                        && blur.corner_radii.top_left.0 > 0.
                                }), "{name}: missing live filter on the actual tooltip border box");
                            }
                        }));
                        assert!(
                            (body.height - if clean { 16.0 } else { 18.0 } * factor).abs() < 0.5
                        );
                        let surface = visual.update(|_, cx| {
                            view.theme_snapshot(cx)
                                .unwrap()
                                .variant
                                .tokens
                                .color("tooltip.surface")
                                .unwrap()
                        });
                        let expected = match (clean, dark) {
                            (false, false) => 0x1717_17ff,
                            (false, true) => 0xfafa_faff,
                            (true, false) => 0x1818_1beb,
                            (true, true) => 0xfafa_faf0,
                        };
                        assert_eq!(
                            surface.as_rgba_hex(),
                            expected,
                            "{name}: inverse tint/alpha is a source contract"
                        );
                        assert!(
                            filled(&mut visual, expected).iter().any(|quad| (f64::from(
                                quad.bounds.x
                            ) - panel.x)
                                .abs()
                                < 0.5
                                && (f64::from(quad.bounds.y) - panel.y).abs() < 0.5)
                        );
                        let start = if clean { 8.0 } else { 9.0 } * space;
                        let distance = if rtl {
                            panel.x + panel.width - body.x - body.width
                        } else {
                            body.x - panel.x
                        };
                        assert!(
                            (distance - start).abs() < 0.5,
                            "{name}/{scale}/{density}/{rtl}: text inset {distance}, expected {start}"
                        );
                        if shortcut {
                            assert_shortcut_alpha(&mut visual, &view, clean, dark);
                            let key = bounds(&mut visual, &view, "text", "⌘K");
                            assert!((key.height - 16.0 * factor).abs() < 0.5);
                            let end = if clean { 8.0 } else { 6.0 } * space;
                            let distance = if rtl {
                                key.x - panel.x
                            } else {
                                panel.x + panel.width - key.x - key.width
                            };
                            assert!(
                                (distance - end).abs() < 0.5,
                                "{name}: optical inset {distance}, expected {end}"
                            );
                        }
                    }
                }
            }
        }
    }
}
