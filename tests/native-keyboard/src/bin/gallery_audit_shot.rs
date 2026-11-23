//! Capture the real Gallery audit dialog after publishing actual Host evidence.
//! Usage: gallery_audit_shot <output-dir>; generated artifacts stay outside git.
//! To inspect selected pages without writing images:
//! gallery_audit_shot --inspect <theme> <density> <scale> <page> [<page> ...]

#[cfg(feature = "gpui-fast")]
extern crate gpui_fast as gpui;
#[cfg(feature = "gpui-fast")]
extern crate gpui_fast_platform as gpui_platform;

#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../offscreen.rs"]
mod offscreen;

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Gallery audit capture requires macOS");
    std::process::exit(2);
}

#[cfg(target_os = "macos")]
fn main() {
    let mut args = std::env::args().skip(1);
    let result = match args.next() {
        Some(command) if command == "--inspect" => inspect(args),
        Some(output) => run(std::path::PathBuf::from(output)),
        None => Err("output directory or --inspect required".to_owned()),
    };
    if let Err(error) = result {
        eprintln!("gallery_audit_shot: {error}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "macos")]
fn run(output: std::path::PathBuf) -> Result<(), String> {
    use gpui_rhai::{AutomationCommand, AutomationLocator, MotionPreference};
    use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};
    std::fs::create_dir_all(&output).map_err(|e| e.to_string())?;
    let mut cx = offscreen::context();
    for theme in ["modern-light", "modern-dark", "clean-light", "clean-dark"] {
        let prepared = acceptance::view(&AcceptanceLaunch {
            page: "select".to_owned(),
            theme: theme.to_owned(),
            ..Default::default()
        })?
        .motion_preference(MotionPreference::None)
        .prepare()
        .map_err(|e| e.to_string())?;
        let (window, view) = offscreen::mount(&mut cx, prepared, 1280.0, 860.0, theme)?;
        offscreen::settle(&mut cx, window)?;
        let mut report = None;
        cx.update_window(window, |_, window, cx| {
            acceptance::report_audit(&view, &mut report, window, cx)
        })
        .map_err(|e| e.to_string())??;
        offscreen::settle(&mut cx, window)?;
        let report = report.ok_or("the Host did not publish evidence")?;
        assert!(
            !report.findings.is_empty(),
            "source palette risk must not disappear"
        );
        std::fs::write(output.join(format!("{theme}.txt")), report.report_text())
            .map_err(|e| e.to_string())?;
        cx.update_window(window, |_, window, cx| {
            view.automate(
                AutomationCommand::Dispatch {
                    locator: AutomationLocator::RoleName {
                        role: "button".to_owned(),
                        name: "Open audit report".to_owned(),
                    },
                    event: "click".to_owned(),
                    payload: None,
                },
                window,
                cx,
            )
        })
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
        let mut ready = false;
        for _ in 0..30 {
            offscreen::draw(&mut cx, window)?;
            ready = cx
                .update(|cx| {
                    view.accessibility_snapshot(cx).map(|tree| {
                        tree.find_by_role_and_name("document", "Audit evidence")
                            .any(|node| node.geometry.is_some_and(|g| g.visual.height > 100.0))
                    })
                })
                .map_err(|e| e.to_string())?;
            if ready {
                break;
            }
        }
        if !ready {
            return Err(format!(
                "{theme}: report viewer did not prepare its actual content"
            ));
        }
        offscreen::settle(&mut cx, window)?;
        offscreen::capture(&mut cx, window)?
            .save(output.join(format!("{theme}.png")))
            .map_err(|e| e.to_string())?;
        println!(
            "{theme}: {} actual findings captured",
            report.findings.len()
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn inspect(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    use gpui_rhai::{AutomationCommand, MotionPreference, SCALE_STEPS, UiValue};
    use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch, GalleryAuditSnapshot};

    let theme = args.next().ok_or("missing theme")?;
    let density = args.next().ok_or("missing density")?;
    let scale = args.next().ok_or("missing scale")?;
    let pages = args.collect::<Vec<_>>();
    if !SCALE_STEPS.contains(&scale.as_str()) || pages.is_empty() {
        return Err(format!(
            "expected a reading scale ({}) and one or more page IDs",
            SCALE_STEPS.join(", ")
        ));
    }
    let mut cx = offscreen::context();
    for page in pages {
        let prepared = acceptance::view(&AcceptanceLaunch {
            page: page.clone(),
            theme: theme.clone(),
            density: density.clone(),
            motion: MotionPreference::None,
            ..Default::default()
        })?
        .prepare()
        .map_err(|error| error.to_string())?;
        // The page name is a catalog selector, not a ScriptView ID:
        // names such as "foundations.affine" legitimately contain dots.
        let (window, view) = offscreen::mount(&mut cx, prepared, 1280.0, 860.0, "gallery-inspect")?;
        settle_audit_geometry(&mut cx, window)?;
        cx.update_window(window, |_, window, cx| {
            view.automate(
                AutomationCommand::Action {
                    id: "gallery.set_scale".to_owned(),
                    payload: Some(UiValue::String(scale.clone())),
                },
                window,
                cx,
            )
        })
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
        settle_audit_geometry(&mut cx, window)?;
        println!("=== {page}/{theme}/{density}/{scale}; settled Metal frame, no entry motion ===");
        cx.update_window(window, |_, window, cx| -> Result<(), String> {
            if let Some(error) = view.last_error(cx).map_err(|error| error.to_string())? {
                return Err(format!("{page}: {error}"));
            }
            let variant = view
                .theme_snapshot(cx)
                .map_err(|error| error.to_string())?
                .variant;
            let findings = view
                .composition_audit(cx)
                .map_err(|error| error.to_string())?;
            let report = GalleryAuditSnapshot {
                theme_family: variant.family,
                theme_name: variant.name,
                findings,
            };
            println!("{}", report.report_text());
            for finding in view
                .debug_unreachable_controls(cx)
                .map_err(|error| error.to_string())?
            {
                println!("{finding}");
            }
            window.remove_window();
            Ok(())
        })
        .map_err(|error| error.to_string())??;
        cx.run_until_parked();
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn settle_audit_geometry(
    cx: &mut gpui::VisualTestAppContext,
    window: gpui::AnyWindowHandle,
) -> Result<(), String> {
    // Layout observers are published through the Host's timed controller;
    // rendering another frame alone will not deliver a new width class.
    for _ in 0..4 {
        cx.advance_clock(std::time::Duration::from_millis(32));
        offscreen::draw(cx, window)?;
    }
    Ok(())
}
