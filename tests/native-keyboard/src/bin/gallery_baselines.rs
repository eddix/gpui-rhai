//! Render the Gallery acceptance baselines offscreen with the real macOS
//! renderer and write them as PNG files.
//!
//! usage: gallery_baselines <output-dir> [case-name]
//!        gallery_baselines <output-dir> --pages [density] [theme] [locale]
//!        gallery_baselines <output-dir> --page <id> [density] [theme] [locale]
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
#[allow(dead_code)]
#[path = "../offscreen.rs"]
mod offscreen;

#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../baseline_cases.rs"]
mod baseline_cases;

#[cfg(target_os = "macos")]
mod capture {
    use std::path::PathBuf;

    use gpui::VisualTestAppContext;
    use gpui_rhai::MotionPreference;
    use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

    use super::baseline_cases::{GALLERY_CASES, GALLERY_WINDOW};
    use super::offscreen;

    pub fn run() -> Result<(), String> {
        let mut args = std::env::args().skip(1);
        let output = PathBuf::from(
            args.next()
                .ok_or("usage: gallery_baselines <output-dir> [case-name]")?,
        );
        let only = args.next();
        let cases: Vec<(String, String, String, String, String)> =
            if only.as_deref() == Some("--page") {
                let page = args.next().ok_or("--page requires a Gallery page id")?;
                if !acceptance::page_ids().contains(&page) {
                    return Err(format!("unknown Gallery page {page}"));
                }
                vec![(
                    page.replace('.', "-"),
                    page,
                    args.next().unwrap_or_else(|| "comfortable".to_owned()),
                    args.next().unwrap_or_else(|| "default-dark".to_owned()),
                    args.next().unwrap_or_else(|| "en".to_owned()),
                )]
            } else if only.as_deref() == Some("--pages") {
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
                GALLERY_CASES
                    .iter()
                    .filter(|case| only.as_deref().is_none_or(|only| only == case.name))
                    .map(|case| {
                        (
                            case.name.to_owned(),
                            case.page.to_owned(),
                            case.density.to_owned(),
                            case.theme.to_owned(),
                            case.locale.to_owned(),
                        )
                    })
                    .collect()
            };
        std::fs::create_dir_all(&output).map_err(|error| error.to_string())?;
        let mut cx = offscreen::context();
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
        let (width, height) = GALLERY_WINDOW;
        let (window, view) = offscreen::mount(cx, prepared, width, height, "gallery")?;
        // Mount, then the frames that read committed geometry (virtualized
        // rows, focus), then report the audit count the way the Gallery Host
        // does after each frame, and draw once more.
        offscreen::settle(cx, window)?;
        cx.update_window(window, |_, window, cx| {
            acceptance::report_audit(&view, &mut None, window, cx)
        })
        .map_err(|error| error.to_string())??;
        offscreen::draw(cx, window)?;
        if let Some(error) = cx
            .update(|cx| view.last_error(cx))
            .map_err(|error| error.to_string())?
        {
            return Err(format!("{}: {error}", launch.page));
        }
        offscreen::capture(cx, window)
    }
}
