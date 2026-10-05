//! Render the example baselines (`tests/visual/macos/<example>/<case>.png`)
//! offscreen with the real macOS renderer.
//!
//! usage: example_baselines <visual-root> [example] [case]
//!
//! The examples are compiled in as modules, so the capture uses exactly the
//! view each example's `main` runs.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("example baselines require macOS");
    std::process::exit(2);
}

#[cfg(target_os = "macos")]
fn main() {
    if let Err(error) = capture::run() {
        eprintln!("example_baselines: {error}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "macos")]
#[path = "../offscreen.rs"]
mod offscreen;

#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../../../../crates/gpui-rhai/examples/settings_panel.rs"]
mod settings_panel;

#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../../../../crates/gpui-rhai/examples/dashboard_layout.rs"]
mod dashboard_layout;

#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../../../../crates/gpui-rhai/examples/form_showcase.rs"]
mod form_showcase;

#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../../../../crates/gpui-rhai/examples/data_table.rs"]
mod data_table;

#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../../../../crates/gpui-rhai/examples/embedded_views.rs"]
mod embedded_views;

#[cfg(target_os = "macos")]
mod capture {
    use std::path::PathBuf;
    use std::time::Duration;

    use gpui::{AppContext, VisualTestAppContext, px, size};
    use gpui_rhai::{EmbeddedScriptView, MotionPreference};

    use super::{
        dashboard_layout, data_table, embedded_views, form_showcase, offscreen, settings_panel,
    };

    /// One baseline: the example, the file name, how to build it, and the
    /// keystrokes that bring it into the recorded state.
    struct Case {
        example: &'static str,
        name: &'static str,
        theme: &'static str,
        locale: &'static str,
        state: &'static str,
        motion: MotionPreference,
        keys: &'static str,
    }

    const fn case(
        example: &'static str,
        name: &'static str,
        theme: &'static str,
        locale: &'static str,
        state: &'static str,
    ) -> Case {
        Case {
            example,
            name,
            theme,
            locale,
            state,
            motion: MotionPreference::None,
            keys: "",
        }
    }

    fn cases() -> Vec<Case> {
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
            let name = leak(format!("{theme}.{locale}.{direction}"));
            cases.push(case("settings_panel", name, theme, locale, ""));
            cases.push(case("form_showcase", name, theme, locale, ""));
            for motion in ["normal", "reduced"] {
                cases.push(Case {
                    motion: if motion == "normal" {
                        MotionPreference::Normal
                    } else {
                        MotionPreference::Reduced
                    },
                    ..case(
                        "dashboard_layout",
                        leak(format!("{theme}.{locale}.{direction}.{motion}")),
                        theme,
                        locale,
                        "",
                    )
                });
            }
        }
        // Focus states come from real keyboard traversal: the language switch
        // pressed with the keyboard, and the notification Switch toggled.
        cases.push(Case {
            keys: SETTINGS_LANGUAGE_FOCUS,
            ..case(
                "settings_panel",
                "default-dark.zh-cn.button-focus",
                "default-dark",
                "en",
                "",
            )
        });
        cases.push(Case {
            keys: SETTINGS_SWITCH_FOCUS,
            ..case(
                "settings_panel",
                "default-dark.zh-cn.switch-focus",
                "default-dark",
                "zh-CN",
                "",
            )
        });
        cases.push(case(
            "form_showcase",
            "default-light.en.dialog",
            "default-light",
            "en",
            "dialog",
        ));
        cases.push(case(
            "form_showcase",
            "default-light.en.toast",
            "default-light",
            "en",
            "toast",
        ));
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

    /// Tab into the language ToggleGroup, move to 简体中文 and press it.
    const SETTINGS_LANGUAGE_FOCUS: &str = "tab tab tab right space";
    /// Tab to the notification Switch and toggle it.
    const SETTINGS_SWITCH_FOCUS: &str = "tab tab tab tab tab space";

    fn leak(value: String) -> &'static str {
        Box::leak(value.into_boxed_str())
    }

    fn view(case: &Case) -> Option<(EmbeddedScriptView, (f32, f32))> {
        Some(match case.example {
            "settings_panel" => (
                settings_panel::view(case.theme, case.locale),
                settings_panel::WINDOW,
            ),
            "dashboard_layout" => (
                dashboard_layout::view(case.theme, case.locale),
                dashboard_layout::WINDOW,
            ),
            "form_showcase" => (
                form_showcase::view(case.theme, case.locale, case.state),
                form_showcase::WINDOW,
            ),
            "data_table" => (
                data_table::view(case.theme, case.locale, case.state),
                data_table::WINDOW,
            ),
            _ => return None,
        })
    }

    pub fn run() -> Result<(), String> {
        let mut args = std::env::args().skip(1);
        let root = PathBuf::from(
            args.next()
                .ok_or("usage: example_baselines <visual-root> [example] [case]")?,
        );
        let only_example = args.next();
        let only_case = args.next();
        let mut cx = offscreen::context();
        let mut captured = 0;
        for case in cases() {
            if only_example
                .as_deref()
                .is_some_and(|only| only != case.example)
                || only_case.as_deref().is_some_and(|only| only != case.name)
            {
                continue;
            }
            let image = match capture_case(&mut cx, &case) {
                Ok(image) => image,
                Err(error) => {
                    // Exit before the test App's leak check runs on drop.
                    eprintln!("example_baselines: {}/{}: {error}", case.example, case.name);
                    std::process::exit(1);
                }
            };
            let directory = root.join(case.example);
            std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
            let path = directory.join(format!("{}.png", case.name));
            image.save(&path).map_err(|error| error.to_string())?;
            println!(
                "captured {}/{} ({}x{})",
                case.example,
                case.name,
                image.width(),
                image.height()
            );
            captured += 1;
        }
        if captured == 0 {
            return Err("no case matched".to_owned());
        }
        // The test App checks for leaked entities on drop; leave it to exit.
        std::mem::forget(cx);
        Ok(())
    }

    fn capture_case(
        cx: &mut VisualTestAppContext,
        case: &Case,
    ) -> Result<image::RgbaImage, String> {
        if case.example == "embedded_views" {
            return capture_embedded(cx);
        }
        let (view, (width, height)) = view(case).ok_or("unknown example")?;
        // Motion runs on a manual clock, so a normal-motion capture is a fixed
        // moment rather than whenever the frame happened to be drawn.
        let clock = gpui_rhai::ManualRuntimeClock::new(std::time::Instant::now());
        let prepared = view
            .motion_preference(case.motion)
            .runtime_clock(clock.clock())
            .prepare()
            .map_err(|error| error.to_string())?;
        let (window, handle) = offscreen::mount(cx, prepared, width, height, case.example)?;
        offscreen::settle(cx, window)?;
        if std::env::var_os("EXAMPLE_BASELINES_STEPS").is_some() {
            // Debug aid: one image per keystroke, to find the focus path.
            for (step, keystroke) in case.keys.split_whitespace().enumerate() {
                offscreen::keys(cx, window, keystroke)?;
                let image = cx
                    .capture_screenshot(window)
                    .map_err(|error| error.to_string())?;
                image
                    .save(format!(
                        "/tmp/{}-{}-{step}-{keystroke}.png",
                        case.example, case.name
                    ))
                    .map_err(|error| error.to_string())?;
            }
        } else {
            offscreen::keys(cx, window, case.keys)?;
        }
        if case.motion == MotionPreference::Normal {
            // Entrance motion has finished; looping motion (the indeterminate
            // Progress) is part-way through its cycle.
            clock.advance(Duration::from_millis(1200));
            cx.advance_clock(Duration::from_millis(1200));
            offscreen::settle(cx, window)?;
        }
        if let Some(error) = cx
            .update(|cx| handle.last_error(cx))
            .map_err(|error| error.to_string())?
        {
            return Err(error);
        }
        offscreen::capture(cx, window)
    }

    fn capture_embedded(cx: &mut VisualTestAppContext) -> Result<image::RgbaImage, String> {
        let (width, height) = embedded_views::WINDOW;
        let window = cx
            .open_offscreen_window(size(px(width), px(height)), |window, cx| {
                cx.new(|cx| embedded_views::EmbeddedViewsDemo::new(window, cx))
            })
            .map_err(|error| error.to_string())?;
        let window = window.into();
        offscreen::settle(cx, window)?;
        offscreen::capture(cx, window)
    }
}
