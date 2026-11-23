//! Full preset recipes must not hide Host palette overrides under literals.

use gpui::TestAppContext;
use gpui_rhai::{MotionPreference, Rgba8, ThemeTokenOverrides};

use crate::support::{Mount, Step, bounds, filled, last_error, settle};

#[gpui::test]
fn host_surface_override_reaches_real_preset_progress_tracks(cx: &mut TestAppContext) {
    for (name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        let override_color = 0x23bfa5ff;
        let overrides = ThemeTokenOverrides {
            colors: [(
                "surface_hover".to_owned(),
                Rgba8::from_rgba_hex(override_color),
            )]
            .into(),
            ..Default::default()
        };
        let (mut visual, view) = Mount::components(
            r#"import "components/progress" as progress;
            fn view(ctx){progress::Progress(#{key:"task",label:"Task",value:40,width:200})}"#,
        )
        .theme(*source)
        .motion(MotionPreference::None)
        .with(move |view| view.theme_token_overrides(overrides))
        .id("preset-color-overrides")
        .mount(cx);
        settle(&mut visual, Step::Frame, 3);
        let track = bounds(&mut visual, &view, "progressbar", "Task");
        assert!(
            filled(&mut visual, override_color)
                .iter()
                .any(|quad| (f64::from(quad.bounds.x) - track.x).abs() < 0.5
                    && (f64::from(quad.bounds.y) - track.y).abs() < 0.5
                    && (f64::from(quad.bounds.width) - track.width).abs() < 0.5),
            "{name}: literal preset paint shadowed the Host token"
        );
        assert!(last_error(&mut visual, &view).is_none());
    }
}
