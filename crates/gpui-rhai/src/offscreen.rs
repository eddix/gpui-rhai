//! Render a script view offscreen and save PNGs, for an application's
//! `examples/shot.rs` (feature `offscreen-pre` or `offscreen-fast`, the one
//! that matches the backend).
//!
//! ```no_run
//! // examples/shot.rs
//! fn main() {
//!     if let Err(error) = gpui_rhai::offscreen::run(|| gpui_rhai::script_view!().prepare()) {
//!         eprintln!("shot: {error}");
//!         std::process::exit(1);
//!     }
//! }
//! ```
//!
//! `gpui-rhai shot --size 1280x800 --theme Nord/Dark --click Settings --out a.png`
//! runs it: the steps run in order on the real macOS renderer, in a window off
//! the screen whose frames are read back from the GPU, so nothing appears and
//! no screen-recording permission is involved.
//! The application mounts its view the way it ships, with its own extensions
//! (fake services where the real ones need a network).
//!
//! Steps: `--size WxH` (before the view mounts; 1280x800 by default),
//! `--theme Family/Variant` or `--theme Variant` (in the current family),
//! `--locale ID`, `--click NAME` (the first node whose accessible name is
//! NAME, or else starts with it), `--key CHORD` (GPUI syntax, such as
//! `cmd-k`), `--wait MS` (advances the view's clock), `--out PATH` (saves the
//! frame) and `--steps FILE`. A steps file holds one JSON command per line:
//! the automation protocol's requests ([`crate::AutomationRequest`], such as
//! `{"id":"q1","command":"query","locator":{"kind":"text","text":"Save"}}`)
//! and the shot commands `click` (`name`, or `locator` as in automation),
//! `keys`, `wait` (`millis`), `capture` (`path`), `theme` and `locale`. Each
//! line prints one [`crate::AutomationResponse`]; the first failing line stops
//! the run.
//!
//! The feature turns on GPUI's `test-support`, so enable it only in
//! `[dev-dependencies]`: a `cargo build` never contains it.

// The example is an application's `examples/shot.rs`, whose `main` is the point.
#![allow(clippy::needless_doctest_main)]

use std::path::PathBuf;

use thiserror::Error;

use crate::{PreparedScriptView, ScriptViewError};

/// Why a shot failed.
#[derive(Debug, Error)]
pub enum ShotError {
    #[error("{0}")]
    Arguments(String),
    #[error(transparent)]
    View(#[from] ScriptViewError),
    #[error("{0}")]
    Step(String),
    #[error("failed to write {path}: {message}")]
    Write { path: PathBuf, message: String },
    #[error("offscreen shots need macOS")]
    Unsupported,
}

/// One step of a shot, in command-line order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShotStep {
    Theme(String),
    Locale(String),
    Click(String),
    Key(String),
    Wait(u64),
    Out(PathBuf),
    Steps(PathBuf),
}

/// The window size and the steps of one shot run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShotPlan {
    pub width: u16,
    pub height: u16,
    pub steps: Vec<ShotStep>,
}

impl ShotPlan {
    /// Parse shot arguments, such as those `gpui-rhai shot` passes on.
    ///
    /// # Errors
    ///
    /// Returns [`ShotError::Arguments`] for an unknown option, a missing or
    /// malformed value, or no `--out` or `--steps`.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, ShotError> {
        let mut plan = Self {
            width: 1280,
            height: 800,
            steps: Vec::new(),
        };
        let mut args = args.into_iter();
        while let Some(option) = args.next() {
            let mut value = || {
                args.next()
                    .ok_or_else(|| ShotError::Arguments(format!("{option} needs a value")))
            };
            match option.as_str() {
                "--size" => {
                    let size = value()?;
                    let (width, height) = size
                        .split_once('x')
                        .and_then(|(width, height)| {
                            Some((width.parse::<u16>().ok()?, height.parse::<u16>().ok()?))
                        })
                        .filter(|(width, height)| *width > 0 && *height > 0)
                        .ok_or_else(|| {
                            ShotError::Arguments(format!("--size {size}: expected WIDTHxHEIGHT"))
                        })?;
                    plan.width = width;
                    plan.height = height;
                }
                "--theme" => plan.steps.push(ShotStep::Theme(value()?)),
                "--locale" => plan.steps.push(ShotStep::Locale(value()?)),
                "--click" => plan.steps.push(ShotStep::Click(value()?)),
                "--key" => plan.steps.push(ShotStep::Key(value()?)),
                "--wait" => {
                    let millis = value()?;
                    plan.steps.push(ShotStep::Wait(millis.parse().map_err(|_| {
                        ShotError::Arguments(format!("--wait {millis}: expected milliseconds"))
                    })?));
                }
                "--out" => plan.steps.push(ShotStep::Out(PathBuf::from(value()?))),
                "--steps" => plan.steps.push(ShotStep::Steps(PathBuf::from(value()?))),
                other => {
                    return Err(ShotError::Arguments(format!(
                        "unknown option {other}; shot takes --size --theme --locale --click --key --wait --out --steps"
                    )));
                }
            }
        }
        if !plan
            .steps
            .iter()
            .any(|step| matches!(step, ShotStep::Out(_) | ShotStep::Steps(_)))
        {
            return Err(ShotError::Arguments(
                "nothing to save: pass --out PATH or --steps FILE".to_owned(),
            ));
        }
        Ok(plan)
    }
}

/// Run the shot the process arguments describe on the view `prepare`
/// returns.
///
/// # Errors
///
/// Returns [`ShotError`] for bad arguments, a view that fails to prepare or
/// mount, a failing step, or a platform other than macOS.
pub fn run(
    prepare: impl FnOnce() -> Result<PreparedScriptView, ScriptViewError>,
) -> Result<(), ShotError> {
    let plan = ShotPlan::parse(std::env::args().skip(1))?;
    run_plan(&plan, prepare)
}

/// Run `plan` on the view `prepare` returns.
///
/// # Errors
///
/// As [`run`].
#[cfg(not(target_os = "macos"))]
pub fn run_plan(
    _plan: &ShotPlan,
    _prepare: impl FnOnce() -> Result<PreparedScriptView, ScriptViewError>,
) -> Result<(), ShotError> {
    Err(ShotError::Unsupported)
}

#[cfg(target_os = "macos")]
pub use macos::run_plan;

#[cfg(target_os = "macos")]
mod macos {
    use std::cell::RefCell;
    use std::io::{BufRead, Write as _};
    use std::rc::Rc;
    use std::time::Duration;

    use gpui::{
        AnyWindowHandle, AppContext, Context, IntoElement, Modifiers, ParentElement, Render,
        VisualTestAppContext, Window, point, px, size,
    };
    use serde::Deserialize;

    use super::{ShotError, ShotPlan, ShotStep};
    use crate::{
        AccessibilityNode, AutomationLocator, AutomationRequest, AutomationResponse,
        PreparedScriptView, ScriptViewConfig, ScriptViewError, ScriptViewHandle, ScriptViewHost,
    };

    /// The window root: the mounted view, or nothing when mounting failed
    /// (the error goes back to [`run_plan`]).
    struct Root(Option<(ScriptViewHost, ScriptViewHandle)>);

    impl Render for Root {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let Some((host, view)) = &self.0 else {
                return gpui::div().into_any_element();
            };
            match view.element() {
                Ok(element) => host.container(element).into_any_element(),
                Err(error) => gpui::div().child(error.to_string()).into_any_element(),
            }
        }
    }

    /// The `command` values of [`ShotCommand`]; any other line of a steps
    /// file is an automation request.
    const SHOT_COMMANDS: [&str; 6] = ["click", "keys", "wait", "capture", "theme", "locale"];

    /// A shot command of a steps file.
    #[derive(Debug, Deserialize)]
    #[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
    enum ShotCommand {
        Click {
            name: Option<String>,
            locator: Option<AutomationLocator>,
        },
        Keys {
            keys: String,
        },
        Wait {
            millis: u64,
        },
        Capture {
            path: std::path::PathBuf,
        },
        Theme {
            theme: String,
        },
        Locale {
            locale: String,
        },
    }

    struct Session {
        cx: VisualTestAppContext,
        window: AnyWindowHandle,
        view: ScriptViewHandle,
    }

    /// Run `plan` on the view `prepare` returns.
    ///
    /// # Errors
    ///
    /// As [`super::run`].
    pub fn run_plan(
        plan: &ShotPlan,
        prepare: impl FnOnce() -> Result<PreparedScriptView, ScriptViewError>,
    ) -> Result<(), ShotError> {
        let mut cx = VisualTestAppContext::new(crate::gpui_platform::current_platform(false));
        cx.update(crate::install);
        let prepared = prepare()?;
        let bindings = prepared.key_bindings().to_vec();
        let mounted = Rc::new(RefCell::new(None));
        let capture = Rc::clone(&mounted);
        let window_size = size(px(f32::from(plan.width)), px(f32::from(plan.height)));
        let window = cx
            .open_offscreen_window(window_size, move |window, cx| {
                let result = ScriptViewHost::new("shot-host", cx).and_then(|host| {
                    host.bind_keys(bindings, cx)?;
                    let view = prepared.mount(
                        ScriptViewConfig::new("shot").paint_background(true),
                        host.clone(),
                        window,
                        cx,
                    )?;
                    let _ = view.focus(window, cx);
                    Ok((host, view))
                });
                let root = result.as_ref().ok().cloned();
                *capture.borrow_mut() = Some(result.map(|(_, view)| view));
                cx.new(|_| Root(root))
            })
            .map_err(|error| ShotError::Step(error.to_string()))?;
        let view = mounted
            .borrow_mut()
            .take()
            .ok_or_else(|| ShotError::Step("the view did not mount".to_owned()))??;
        let mut session = Session {
            cx,
            window: window.into(),
            view,
        };
        session.settle()?;
        for step in &plan.steps {
            session.step(step)?;
        }
        Ok(())
    }

    impl Session {
        fn draw(&mut self) -> Result<(), ShotError> {
            self.cx.run_until_parked();
            self.cx
                .update_window(self.window, |_, window, cx| {
                    window.refresh();
                    let _ = window.draw(cx);
                })
                .map_err(|error| ShotError::Step(error.to_string()))
        }

        /// A few frames, so geometry-dependent work (virtual rows, overlays,
        /// focus) settles.
        fn settle(&mut self) -> Result<(), ShotError> {
            for _ in 0..4 {
                self.draw()?;
            }
            Ok(())
        }

        fn step(&mut self, step: &ShotStep) -> Result<(), ShotError> {
            match step {
                ShotStep::Theme(theme) => self.theme(theme),
                ShotStep::Locale(locale) => self.locale(locale),
                ShotStep::Click(name) => self.click_name(name),
                ShotStep::Key(keys) => self.keys(keys),
                ShotStep::Wait(millis) => self.wait(*millis),
                ShotStep::Out(path) => self.capture(path),
                ShotStep::Steps(path) => self.steps_file(path),
            }
        }

        fn theme(&mut self, theme: &str) -> Result<(), ShotError> {
            let (family, variant) = if let Some((family, variant)) = theme.split_once('/') {
                (family.to_owned(), variant.to_owned())
            } else {
                let family = self
                    .cx
                    .update(|cx| self.view.theme_snapshot(cx))?
                    .variant
                    .family;
                (family, theme.to_owned())
            };
            let view = self.view.clone();
            self.cx
                .update(|cx| view.select_theme(&family, &variant, cx))?;
            self.settle()
        }

        fn locale(&mut self, locale: &str) -> Result<(), ShotError> {
            let view = self.view.clone();
            self.cx.update(|cx| view.select_locale(locale, cx))?;
            self.settle()
        }

        /// Click the node whose accessible name is `name`, or else the first
        /// whose name starts with it.
        fn click_name(&mut self, name: &str) -> Result<(), ShotError> {
            let snapshot = self.cx.update(|cx| self.view.accessibility_snapshot(cx))?;
            let node = snapshot
                .nodes()
                .find(|node| node.name == name && node.geometry.is_some())
                .or_else(|| {
                    snapshot
                        .nodes()
                        .find(|node| node.name.starts_with(name) && node.geometry.is_some())
                })
                .ok_or_else(|| ShotError::Step(format!("no node named `{name}` to click")))?;
            self.click_node(node)
        }

        /// Click the node `locator` finds, as automation would resolve it.
        fn click_locator(&mut self, locator: &AutomationLocator) -> Result<(), ShotError> {
            let snapshot = self.cx.update(|cx| self.view.accessibility_snapshot(cx))?;
            let id = crate::automation::resolve_locator(&snapshot, locator)
                .map_err(|error| ShotError::Step(error.to_string()))?;
            let node = snapshot
                .node(id)
                .ok_or_else(|| ShotError::Step(format!("no node to click for {locator:?}")))?;
            self.click_node(node)
        }

        /// Click the center of the visible part of `node`.
        fn click_node(&mut self, node: &AccessibilityNode) -> Result<(), ShotError> {
            let hidden = || ShotError::Step(format!("`{}` is not on screen", node.name));
            let geometry = node.geometry.ok_or_else(hidden)?;
            let bounds = match geometry.clip {
                Some(clip) => {
                    let left = geometry.visual.x.max(clip.x);
                    let top = geometry.visual.y.max(clip.y);
                    let right =
                        (geometry.visual.x + geometry.visual.width).min(clip.x + clip.width);
                    let bottom =
                        (geometry.visual.y + geometry.visual.height).min(clip.y + clip.height);
                    if right <= left || bottom <= top {
                        return Err(hidden());
                    }
                    (left, top, right, bottom)
                }
                None => (
                    geometry.visual.x,
                    geometry.visual.y,
                    geometry.visual.x + geometry.visual.width,
                    geometry.visual.y + geometry.visual.height,
                ),
            };
            #[allow(clippy::cast_possible_truncation)]
            let center = point(
                px(f64::midpoint(bounds.0, bounds.2) as f32),
                px(f64::midpoint(bounds.1, bounds.3) as f32),
            );
            self.cx
                .simulate_click(self.window, center, Modifiers::default());
            self.settle()
        }

        fn keys(&mut self, keys: &str) -> Result<(), ShotError> {
            for keystroke in keys.split_whitespace() {
                self.cx.simulate_keystrokes(self.window, keystroke);
                self.settle()?;
            }
            Ok(())
        }

        fn wait(&mut self, millis: u64) -> Result<(), ShotError> {
            self.cx
                .background_executor
                .advance_clock(Duration::from_millis(millis));
            self.settle()
        }

        fn capture(&mut self, path: &std::path::Path) -> Result<(), ShotError> {
            self.settle()?;
            let image = self
                .cx
                .capture_screenshot(self.window)
                .map_err(|error| ShotError::Step(error.to_string()))?;
            image.save(path).map_err(|error| ShotError::Write {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
            Ok(())
        }

        /// Run a steps file, printing one JSON response per line.
        fn steps_file(&mut self, path: &std::path::Path) -> Result<(), ShotError> {
            let file = std::fs::File::open(path).map_err(|error| {
                ShotError::Arguments(format!("--steps {}: {error}", path.display()))
            })?;
            let mut stdout = std::io::stdout();
            for (index, line) in std::io::BufReader::new(file).lines().enumerate() {
                let line = line.map_err(|error| ShotError::Step(error.to_string()))?;
                if line.trim().is_empty() {
                    continue;
                }
                let response = self.steps_line(&line).map_err(|error| {
                    ShotError::Step(format!("{}:{}: {error}", path.display(), index + 1))
                })?;
                let response = serde_json::to_string(&response)
                    .map_err(|error| ShotError::Step(error.to_string()))?;
                let _ = writeln!(stdout, "{response}");
            }
            Ok(())
        }

        /// One line of a steps file: a shot command, or else an automation
        /// request; either may carry an `id`, which the response repeats.
        fn steps_line(&mut self, line: &str) -> Result<AutomationResponse, ShotError> {
            let mut request: serde_json::Map<String, serde_json::Value> =
                serde_json::from_str(line).map_err(|error| ShotError::Step(error.to_string()))?;
            let command = request
                .get("command")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if !SHOT_COMMANDS.contains(&command) {
                let request: AutomationRequest =
                    serde_json::from_value(serde_json::Value::Object(request))
                        .map_err(|error| ShotError::Step(error.to_string()))?;
                let view = self.view.clone();
                let result = self
                    .cx
                    .update_window(self.window, |_, window, cx| {
                        view.automate(request.command, window, cx)
                    })
                    .map_err(|error| ShotError::Step(error.to_string()))??;
                self.settle()?;
                return Ok(AutomationResponse {
                    id: request.id,
                    ok: true,
                    value: Some(result),
                    error: None,
                });
            }
            let id = request.remove("id").unwrap_or_default();
            let command: ShotCommand = serde_json::from_value(serde_json::Value::Object(request))
                .map_err(|error| ShotError::Step(error.to_string()))?;
            match command {
                ShotCommand::Click { name, locator } => match (name, locator) {
                    (Some(name), None) => self.click_name(&name)?,
                    (None, Some(locator)) => self.click_locator(&locator)?,
                    _ => {
                        return Err(ShotError::Step(
                            "click takes `name` or `locator`".to_owned(),
                        ));
                    }
                },
                ShotCommand::Keys { keys } => self.keys(&keys)?,
                ShotCommand::Wait { millis } => self.wait(millis)?,
                ShotCommand::Capture { path } => self.capture(&path)?,
                ShotCommand::Theme { theme } => self.theme(&theme)?,
                ShotCommand::Locale { locale } => self.locale(&locale)?,
            }
            Ok(AutomationResponse {
                id,
                ok: true,
                value: None,
                error: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(line: &str) -> Vec<String> {
        line.split_whitespace().map(ToOwned::to_owned).collect()
    }

    #[test]
    fn shot_arguments_keep_their_order() {
        let plan = ShotPlan::parse(args(
            "--theme Nord/Dark --size 800x600 --click Settings --wait 250 --out a.png --key cmd-k --out b.png",
        ))
        .unwrap();
        assert_eq!((plan.width, plan.height), (800, 600));
        assert_eq!(
            plan.steps,
            [
                ShotStep::Theme("Nord/Dark".to_owned()),
                ShotStep::Click("Settings".to_owned()),
                ShotStep::Wait(250),
                ShotStep::Out(PathBuf::from("a.png")),
                ShotStep::Key("cmd-k".to_owned()),
                ShotStep::Out(PathBuf::from("b.png")),
            ]
        );
        for (line, expected) in [
            ("--click Settings", "nothing to save"),
            ("--size 800 --out a.png", "expected WIDTHxHEIGHT"),
            ("--wait soon --out a.png", "expected milliseconds"),
            ("--out", "--out needs a value"),
            ("--zoom 2 --out a.png", "unknown option --zoom"),
        ] {
            let error = ShotPlan::parse(args(line)).unwrap_err().to_string();
            assert!(error.contains(expected), "{line}: {error}");
        }
    }
}
