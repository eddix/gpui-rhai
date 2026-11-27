//! Real preset document frames preserve intrinsic and max-height contracts.

use gpui::TestAppContext;
use gpui_rhai::{Environment, Length, MotionPreference, ThemeTokenOverrides};

use crate::support::{Mount, Step, last_error, settle, snapshot};

#[gpui::test]
fn code_content_height_uses_preset_type_and_never_adds_a_layout_frame(cx: &mut TestAppContext) {
    for (name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for scale in [100, 200] {
            for (lines, maximum) in [(3, None), (30, Some(100))] {
                let source = (0..lines)
                    .map(|index| format!("let item_{index} = {index};"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let text = serde_json::to_string(&source).unwrap();
                let max = maximum.map_or_else(|| "()".to_owned(), |value| value.to_string());
                let script = format!(
                    r#"import "components/code_viewer" as code;
                    fn view(ctx) {{ column([
                        code::CodeViewer(#{{key:"source",label:"Source",source:{text},height:"content",max_height:{max}}}),
                        text("After").accessibility_role("status").accessibility_label("After")
                    ]).with_style(style().width(px(400)).height(px(800))) }}"#
                );
                let overrides = ThemeTokenOverrides {
                    environment_defaults: [("scale".to_owned(), scale.to_string())].into(),
                    ..Default::default()
                };
                let (mut visual, view) = Mount::components(script)
                    .theme(*theme)
                    .motion(MotionPreference::None)
                    .with(move |view| view.theme_token_overrides(overrides))
                    .id("preset-documents")
                    .mount(cx);
                settle(&mut visual, Step::Frame, 4);
                let tokens = visual
                    .update(|_, cx| view.theme_snapshot(cx))
                    .unwrap()
                    .variant
                    .tokens;
                let role = tokens
                    .resolve_typography("code", &Environment::EMPTY)
                    .unwrap();
                let Length::Pixels(row_height) = role.line_height else {
                    panic!("pixel type role");
                };
                let expected = maximum.map_or(f64::from(lines) * row_height, |max| {
                    (f64::from(lines) * row_height).min(f64::from(max))
                });
                let tree = snapshot(&mut visual, &view);
                let document = tree
                    .find_by_role_and_name("document", "Source")
                    .next()
                    .unwrap()
                    .geometry
                    .as_ref()
                    .unwrap()
                    .layout;
                let after = tree
                    .find_by_role_and_name("status", "After")
                    .next()
                    .unwrap()
                    .geometry
                    .as_ref()
                    .unwrap()
                    .layout;
                assert!(
                    (document.height - expected).abs() < 0.5,
                    "{name}, scale {scale}, lines {lines}: {document:?}, expected {expected}"
                );
                assert!(
                    (after.y - document.y - document.height).abs() < 0.5,
                    "the following content must start after the actual viewer box"
                );
                assert!(last_error(&mut visual, &view).is_none());
            }
        }
    }
}
