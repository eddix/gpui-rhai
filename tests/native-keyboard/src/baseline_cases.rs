//! The committed macOS visual baselines, `tests/visual/macos/<directory>/<name>.png`: every
//! case the capture tools write and the window it is captured in. `gallery_baselines` and
//! `example_baselines` capture these cases; the native test `audit::visual_baselines` checks
//! the committed files against them. Plain data, so it compiles on every platform.

use gpui_rhai::MotionPreference;

/// Baselines are offscreen captures stored in device pixels: twice the window size.
pub const SCALE: f32 = 2.0;

/// The Gallery's logical window size.
pub const GALLERY_WINDOW: (f32, f32) = (1280.0, 860.0);

/// The directory of the Gallery baselines.
pub const GALLERY_DIRECTORY: &str = "gallery";

/// One Gallery baseline: the file name and the page, density, theme and locale it captures.
pub struct GalleryCase {
    pub name: &'static str,
    pub page: &'static str,
    pub density: &'static str,
    pub theme: &'static str,
    pub locale: &'static str,
}

const fn gallery(
    name: &'static str,
    page: &'static str,
    density: &'static str,
    theme: &'static str,
    locale: &'static str,
) -> GalleryCase {
    GalleryCase {
        name,
        page,
        density,
        theme,
        locale,
    }
}

/// In capture order.
#[rustfmt::skip]
pub const GALLERY_CASES: &[GalleryCase] = &[
    gallery("button.comfortable.default-dark.en", "button", "comfortable", "default-dark", "en"),
    gallery("button.comfortable.default-light.en", "button", "comfortable", "default-light", "en"),
    gallery("button.compact.default-dark.en", "button", "compact", "default-dark", "en"),
    gallery("button.compact.default-light.en", "button", "compact", "default-light", "en"),
    gallery("scene-operations.comfortable.default-dark.en", "scene.operations", "comfortable", "default-dark", "en"),
    gallery("scene-operations.comfortable.default-light.en", "scene.operations", "comfortable", "default-light", "en"),
    gallery("scene-operations.compact.default-dark.en", "scene.operations", "compact", "default-dark", "en"),
    gallery("scene-operations.compact.default-light.en", "scene.operations", "compact", "default-light", "en"),
    gallery("scene-form.comfortable.default-dark.en", "scene.form", "comfortable", "default-dark", "en"),
    gallery("scene-form.comfortable.default-light.en", "scene.form", "comfortable", "default-light", "en"),
    gallery("scene-form.compact.default-dark.en", "scene.form", "compact", "default-dark", "en"),
    gallery("scene-form.compact.default-light.en", "scene.form", "compact", "default-light", "en"),
    gallery("scene-settings.comfortable.default-dark.en", "scene.settings", "comfortable", "default-dark", "en"),
    gallery("scene-settings.comfortable.default-light.en", "scene.settings", "comfortable", "default-light", "en"),
    gallery("scene-settings.compact.default-dark.en", "scene.settings", "compact", "default-dark", "en"),
    gallery("scene-settings.compact.default-light.en", "scene.settings", "compact", "default-light", "en"),
    gallery("scene-operations.comfortable.default-dark.ar", "scene.operations", "comfortable", "default-dark", "ar"),
    gallery("table.comfortable.default-dark.zh-CN", "table", "comfortable", "default-dark", "zh-CN"),
    gallery("description-list.compact.default-light.zh-CN", "description_list", "compact", "default-light", "zh-CN"),
];

/// One example baseline: the example (also its directory), the file name, how to build the
/// example's view and the keystrokes that bring it into the recorded state.
pub struct ExampleCase {
    pub example: &'static str,
    pub name: String,
    pub theme: &'static str,
    pub locale: &'static str,
    pub state: &'static str,
    pub motion: MotionPreference,
    pub keys: &'static str,
}

/// Each example's logical window size: its `WINDOW` constant, which `example_baselines`
/// checks against this.
pub fn example_window(example: &str) -> Option<(f32, f32)> {
    Some(match example {
        "settings_panel" => (640.0, 520.0),
        "dashboard_layout" => (760.0, 560.0),
        "form_showcase" => (760.0, 720.0),
        "data_table" => (980.0, 720.0),
        "embedded_views" => (900.0, 420.0),
        _ => return None,
    })
}

fn case(
    example: &'static str,
    name: impl Into<String>,
    theme: &'static str,
    locale: &'static str,
    state: &'static str,
) -> ExampleCase {
    ExampleCase {
        example,
        name: name.into(),
        theme,
        locale,
        state,
        motion: MotionPreference::None,
        keys: "",
    }
}

/// Tab into the language ToggleGroup, move to 简体中文 and press it.
const SETTINGS_LANGUAGE_FOCUS: &str = "tab tab tab right space";
/// Tab to the notification Switch and toggle it.
const SETTINGS_SWITCH_FOCUS: &str = "tab tab tab tab tab space";

/// Every example baseline, in capture order.
pub fn example_cases() -> Vec<ExampleCase> {
    let mut cases = Vec::new();
    for (theme, locale, direction) in [
        ("catppuccin-latte", "en", "ltr"),
        ("catppuccin-mocha", "ar", "rtl"),
        ("catppuccin-mocha", "en", "ltr"),
        ("default-dark", "en", "ltr"),
        ("default-light", "en", "ltr"),
        ("tokyo-night", "en", "ltr"),
        ("tokyo-storm", "en", "ltr"),
    ] {
        let name = format!("{theme}.{locale}.{direction}");
        cases.push(case("settings_panel", &*name, theme, locale, ""));
        cases.push(case("form_showcase", &*name, theme, locale, ""));
        for (motion, preference) in [
            ("normal", MotionPreference::Normal),
            ("reduced", MotionPreference::Reduced),
        ] {
            cases.push(ExampleCase {
                motion: preference,
                ..case(
                    "dashboard_layout",
                    format!("{name}.{motion}"),
                    theme,
                    locale,
                    "",
                )
            });
        }
    }
    // Focus states come from real keyboard traversal: the language switch pressed with the
    // keyboard, and the notification Switch toggled.
    cases.push(ExampleCase {
        keys: SETTINGS_LANGUAGE_FOCUS,
        ..case(
            "settings_panel",
            "default-dark.zh-cn.button-focus",
            "default-dark",
            "en",
            "",
        )
    });
    cases.push(ExampleCase {
        keys: SETTINGS_SWITCH_FOCUS,
        ..case(
            "settings_panel",
            "default-dark.zh-cn.switch-focus",
            "default-dark",
            "zh-CN",
            "",
        )
    });
    for state in ["dialog", "toast"] {
        cases.push(case(
            "form_showcase",
            format!("default-light.en.{state}"),
            "default-light",
            "en",
            state,
        ));
    }
    for (name, theme, locale, state) in [
        (
            "catppuccin-mocha.ar.rtl",
            "catppuccin-mocha",
            "ar",
            "default",
        ),
        ("default-dark.en.selected", "default-dark", "en", "selected"),
        ("default-light.en.empty", "default-light", "en", "empty"),
        ("default-light.en.ltr", "default-light", "en", "default"),
        ("default-light.en.loading", "default-light", "en", "loading"),
    ] {
        cases.push(case("data_table", name, theme, locale, state));
    }
    cases.push(case(
        "embedded_views",
        "default-dark.shared-host",
        "default-dark",
        "en",
        "",
    ));
    cases
}

/// One committed baseline file.
pub struct Baseline {
    /// `<directory>/<name>.png` under `tests/visual/macos`.
    pub path: String,
    /// The image size in device pixels.
    pub size: (u32, u32),
}

/// Every committed baseline: the example cases, then the Gallery cases.
pub fn baselines() -> Vec<Baseline> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pixels = |(width, height): (f32, f32)| ((width * SCALE) as u32, (height * SCALE) as u32);
    let examples = example_cases().into_iter().map(|case| Baseline {
        path: format!("{}/{}.png", case.example, case.name),
        size: pixels(example_window(case.example).expect("every example has a window")),
    });
    let gallery = GALLERY_CASES.iter().map(|case| Baseline {
        path: format!("{GALLERY_DIRECTORY}/{}.png", case.name),
        size: pixels(GALLERY_WINDOW),
    });
    examples.chain(gallery).collect()
}
