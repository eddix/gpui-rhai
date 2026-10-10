//! Render the Gallery acceptance baselines offscreen with the real macOS
//! renderer and write them as PNG files.
//!
//! usage: gallery_baselines <output-dir> [case-name]
//!        gallery_baselines <output-dir> --pages [density] [theme] [locale]
//!
//! `--pages` renders every Gallery page instead of the baseline cases, for a
//! visual review sweep; those images are not baselines.
//!
//! Windows are rendered at (-10000, -10000) and read back from the Metal
//! texture, so window managers, other windows and screen-recording permission
//! never enter the image.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Gallery baselines require macOS");
    std::process::exit(2);
}

#[cfg(target_os = "macos")]
fn main() {
    if let Err(error) = capture::run() {
        eprintln!("gallery_baselines: {error}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "macos")]
mod capture {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::rc::Rc;

    use gpui::{
        AnyWindowHandle, AppContext, Context, IntoElement, Render, VisualTestAppContext, Window,
        px, size,
    };
    use gpui_rhai::{MotionPreference, ScriptViewConfig, ScriptViewHandle, ScriptViewHost};
    use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

    /// Logical window size of the Gallery.
    const WIDTH: f32 = 1280.0;
    const HEIGHT: f32 = 860.0;

    /// name, page, density, theme, locale
    const CASES: &[(&str, &str, &str, &str, &str)] = &[
        (
            "button.comfortable.default-dark.en",
            "button",
            "comfortable",
            "default-dark",
            "en",
        ),
        (
            "button.comfortable.default-light.en",
            "button",
            "comfortable",
            "default-light",
            "en",
        ),
        (
            "button.compact.default-dark.en",
            "button",
            "compact",
            "default-dark",
            "en",
        ),
        (
            "button.compact.default-light.en",
            "button",
            "compact",
            "default-light",
            "en",
        ),
        (
            "scene-operations.comfortable.default-dark.en",
            "scene.operations",
            "comfortable",
            "default-dark",
            "en",
        ),
        (
            "scene-operations.comfortable.default-light.en",
            "scene.operations",
            "comfortable",
            "default-light",
            "en",
        ),
        (
            "scene-operations.compact.default-dark.en",
            "scene.operations",
            "compact",
            "default-dark",
            "en",
        ),
        (
            "scene-operations.compact.default-light.en",
            "scene.operations",
            "compact",
            "default-light",
            "en",
        ),
        (
            "scene-form.comfortable.default-dark.en",
            "scene.form",
            "comfortable",
            "default-dark",
            "en",
        ),
        (
            "scene-form.comfortable.default-light.en",
            "scene.form",
            "comfortable",
            "default-light",
            "en",
        ),
        (
            "scene-form.compact.default-dark.en",
            "scene.form",
            "compact",
            "default-dark",
            "en",
        ),
        (
            "scene-form.compact.default-light.en",
            "scene.form",
            "compact",
            "default-light",
            "en",
        ),
        (
            "scene-settings.comfortable.default-dark.en",
            "scene.settings",
            "comfortable",
            "default-dark",
            "en",
        ),
        (
            "scene-settings.comfortable.default-light.en",
            "scene.settings",
            "comfortable",
            "default-light",
            "en",
        ),
        (
            "scene-settings.compact.default-dark.en",
            "scene.settings",
            "compact",
            "default-dark",
            "en",
        ),
        (
            "scene-settings.compact.default-light.en",
            "scene.settings",
            "compact",
            "default-light",
            "en",
        ),
        (
            "scene-operations.comfortable.default-dark.ar",
            "scene.operations",
            "comfortable",
            "default-dark",
            "ar",
        ),
        (
            "table.comfortable.default-dark.zh-CN",
            "table",
            "comfortable",
            "default-dark",
            "zh-CN",
        ),
        (
            "description-list.compact.default-light.zh-CN",
            "description_list",
            "compact",
            "default-light",
            "zh-CN",
        ),
    ];

    struct Root {
        host: ScriptViewHost,
        view: ScriptViewHandle,
    }

    impl Render for Root {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.host.container(self.view.element().unwrap())
        }
    }

    pub fn run() -> Result<(), String> {
        let mut args = std::env::args().skip(1);
        let output = PathBuf::from(
            args.next()
                .ok_or("usage: gallery_baselines <output-dir> [case-name]")?,
        );
        let only = args.next();
        let cases: Vec<(String, String, String, String, String)> =
            if only.as_deref() == Some("--pages") {
                let density = args.next().unwrap_or_else(|| "comfortable".to_owned());
                let theme = args.next().unwrap_or_else(|| "default-dark".to_owned());
                let locale = args.next().unwrap_or_else(|| "en".to_owned());
                acceptance::page_ids()
                    .into_iter()
                    .map(|page| {
                        (
                            page.replace('.', "-"),
                            page,
                            density.clone(),
                            theme.clone(),
                            locale.clone(),
                        )
                    })
                    .collect()
            } else {
                CASES
                    .iter()
                    .filter(|(name, ..)| only.as_deref().is_none_or(|only| only == *name))
                    .map(|(name, page, density, theme, locale)| {
                        (
                            (*name).to_owned(),
                            (*page).to_owned(),
                            (*density).to_owned(),
                            (*theme).to_owned(),
                            (*locale).to_owned(),
                        )
                    })
                    .collect()
            };
        std::fs::create_dir_all(&output).map_err(|error| error.to_string())?;
        let mut cx = VisualTestAppContext::new(gpui_platform::current_platform(false));
        cx.update(gpui_rhai::install);
        let mut captured = 0;
        for (name, page, density, theme, locale) in &cases {
            let launch = AcceptanceLaunch {
                page: page.clone(),
                density: density.clone(),
                theme: theme.clone(),
                locale: locale.clone(),
                motion: MotionPreference::None,
            };
            let image = match capture_case(&mut cx, &launch) {
                Ok(image) => image,
                Err(error) => {
                    // Exit before the test App's leak check runs on drop.
                    eprintln!("gallery_baselines: {name}: {error}");
                    std::process::exit(1);
                }
            };
            let path = output.join(format!("{name}.png"));
            image.save(&path).map_err(|error| error.to_string())?;
            println!("captured {name} ({}x{})", image.width(), image.height());
            captured += 1;
        }
        if captured == 0 {
            return Err(format!("no case named {}", only.unwrap_or_default()));
        }
        // The test App asserts that every entity was released when it drops;
        // mounted views hold window-scoped handles, so leave it to process exit.
        std::mem::forget(cx);
        Ok(())
    }

    fn capture_case(
        cx: &mut VisualTestAppContext,
        launch: &AcceptanceLaunch,
    ) -> Result<image::RgbaImage, String> {
        let prepared = acceptance::view(launch)?
            .prepare()
            .map_err(|error| error.to_string())?;
        let bindings = prepared.key_bindings().to_vec();
        let mounted = Rc::new(RefCell::new(None));
        let capture = Rc::clone(&mounted);
        let window = cx
            .open_offscreen_window(size(px(WIDTH), px(HEIGHT)), move |window, cx| {
                let host = ScriptViewHost::new("gallery-baseline", cx).unwrap();
                host.bind_keys(bindings, cx).unwrap();
                let view = prepared
                    .mount(
                        ScriptViewConfig::new("gallery").paint_background(true),
                        host.clone(),
                        window,
                        cx,
                    )
                    .unwrap();
                *capture.borrow_mut() = Some(view.clone());
                cx.new(|_| Root { host, view })
            })
            .map_err(|error| error.to_string())?;
        let view = mounted
            .borrow()
            .clone()
            .ok_or("the Gallery did not mount")?;
        let any: AnyWindowHandle = window.into();
        // Mount, then the frames that read committed geometry (virtualized
        // rows, focus), then report the audit count the way the Gallery Host
        // does after each frame, and draw once more.
        for _ in 0..4 {
            draw(cx, any)?;
        }
        cx.update_window(any, |_, window, cx| {
            acceptance::report_audit(&view, &mut None, window, cx)
        })
        .map_err(|error| error.to_string())??;
        draw(cx, any)?;
        if let Some(error) = cx
            .update(|cx| view.last_error(cx))
            .map_err(|error| error.to_string())?
        {
            return Err(format!("{}: {error}", launch.page));
        }
        let image = cx
            .capture_screenshot(any)
            .map_err(|error| error.to_string())?;
        cx.update_window(any, |_, window, _| window.remove_window())
            .map_err(|error| error.to_string())?;
        cx.run_until_parked();
        Ok(image)
    }

    fn draw(cx: &mut VisualTestAppContext, any: AnyWindowHandle) -> Result<(), String> {
        cx.run_until_parked();
        cx.update_window(any, |_, window, cx| {
            window.refresh();
            let _ = window.draw(cx);
        })
        .map_err(|error| error.to_string())
    }
}
