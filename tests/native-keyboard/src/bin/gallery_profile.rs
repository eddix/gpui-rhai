//! Profile the Gallery with the real macOS renderer (offscreen).
//!
//! usage: cargo run --release --bin gallery_profile [iterations]
//!
//! Reports, per interaction, the Rhai work (render and callback timings from the
//! view's performance snapshot), the CPU frame time (`Window::draw`: layout,
//! prepaint and paint), and the composition audit the Gallery Host runs after
//! frames. Numbers are wall time on this machine; compare runs, not machines.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("the Gallery profile requires macOS");
    std::process::exit(2);
}

#[cfg(target_os = "macos")]
fn main() {
    if let Err(error) = profile::run() {
        eprintln!("gallery_profile: {error}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "macos")]
#[allow(dead_code)]
#[path = "../offscreen.rs"]
mod offscreen;

#[cfg(target_os = "macos")]
mod profile {
    use std::time::{Duration, Instant};

    use gpui::{
        AnyWindowHandle, Modifiers, MouseMoveEvent, ScrollDelta, ScrollWheelEvent,
        VisualTestAppContext, point, px,
    };
    use gpui_rhai::{
        AutomationCommand, ExecutionOperation, MotionPreference, ScriptViewHandle, UiValue,
    };
    use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

    use super::offscreen;

    #[derive(Default)]
    struct Sample {
        rhai: Duration,
        rhai_renders: usize,
        draw: Duration,
        audit: Duration,
    }

    struct Stats {
        name: String,
        samples: Vec<Sample>,
    }

    fn millis(duration: Duration) -> f64 {
        duration.as_secs_f64() * 1000.0
    }

    fn percentile(values: &mut [f64], p: f64) -> f64 {
        values.sort_by(f64::total_cmp);
        let index = ((values.len() as f64 - 1.0) * p).round() as usize;
        values[index.min(values.len() - 1)]
    }

    impl Stats {
        fn line(&self) -> String {
            let pick = |f: &dyn Fn(&Sample) -> f64| {
                let mut values = self.samples.iter().map(f).collect::<Vec<_>>();
                (percentile(&mut values, 0.5), percentile(&mut values, 0.95))
            };
            let (rhai50, rhai95) = pick(&|s| millis(s.rhai));
            let (draw50, draw95) = pick(&|s| millis(s.draw));
            let (audit50, audit95) = pick(&|s| millis(s.audit));
            let renders = self
                .samples
                .iter()
                .map(|s| s.rhai_renders)
                .max()
                .unwrap_or(0);
            format!(
                "{:<28} rhai {rhai50:>7.2} / {rhai95:>7.2}  draw {draw50:>7.2} / {draw95:>7.2}  audit {audit50:>7.2} / {audit95:>7.2}  renders {renders}",
                self.name
            )
        }
    }

    struct Session {
        cx: VisualTestAppContext,
        window: AnyWindowHandle,
        view: ScriptViewHandle,
    }

    impl Session {
        fn new(page: &str) -> Result<Self, String> {
            let mut cx = offscreen::context();
            let launch = AcceptanceLaunch {
                page: page.to_owned(),
                motion: MotionPreference::None,
                ..AcceptanceLaunch::default()
            };
            let started = Instant::now();
            let prepared = acceptance::prepare(&launch)?;
            let prepared_in = started.elapsed();
            let started = Instant::now();
            let (window, view) = offscreen::mount(&mut cx, prepared, 1280.0, 860.0, "gallery")?;
            offscreen::draw(&mut cx, window)?;
            println!(
                "prepare {:.1} ms, mount + first frame {:.1} ms",
                millis(prepared_in),
                millis(started.elapsed())
            );
            offscreen::settle(&mut cx, window)?;
            let mut session = Self { cx, window, view };
            session.take_rhai();
            Ok(session)
        }

        fn take_rhai(&mut self) -> (Duration, usize) {
            let view = self.view.clone();
            let snapshot = self
                .cx
                .update(|cx| view.take_performance_snapshot(cx))
                .expect("view is live");
            let renders = snapshot
                .timings
                .iter()
                .filter(|timing| timing.operation == ExecutionOperation::Render)
                .count();
            (
                snapshot.timings.iter().map(|timing| timing.duration).sum(),
                renders,
            )
        }

        /// Run `act`, then measure the Rhai work it caused, one frame, and the
        /// audit the Host would run after that frame.
        fn measure(&mut self, act: impl FnOnce(&mut Self)) -> Sample {
            act(self);
            self.cx.run_until_parked();
            let window = self.window;
            let started = Instant::now();
            self.cx
                .update_window(window, |_, window, cx| {
                    window.refresh();
                    let _ = window.draw(cx);
                })
                .expect("window is live");
            let draw = started.elapsed();
            let (rhai, rhai_renders) = self.take_rhai();
            let view = self.view.clone();
            let started = Instant::now();
            if std::env::var_os("GALLERY_PROFILE_LOOP").is_none()
                && std::env::var_os("GALLERY_PROFILE_LOOP_NAV").is_none()
            {
                let _ = self.cx.update(|cx| view.composition_audit(cx));
            }
            let audit = started.elapsed();
            Sample {
                rhai,
                rhai_renders,
                draw,
                audit,
            }
        }

        fn action(&mut self, id: &str, payload: UiValue) {
            let view = self.view.clone();
            let window = self.window;
            let _ = self.cx.update_window(window, |_, window, cx| {
                view.automate(
                    AutomationCommand::Action {
                        id: id.to_owned(),
                        payload: Some(payload),
                    },
                    window,
                    cx,
                )
            });
        }
    }

    pub fn run() -> Result<(), String> {
        let iterations = std::env::args()
            .nth(1)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(20);
        println!("Gallery profile, release build, offscreen Metal, {iterations} iterations");
        println!("columns: p50 / p95 in ms");
        if let Ok(path) = std::env::var("GALLERY_PROFILE_RHAI") {
            // Draw an arbitrary Rhai view (all registry modules available).
            let source = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
            let mut cx = offscreen::context();
            let mut modules = std::collections::BTreeMap::new();
            for (id, module) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID
                .iter()
                .chain(gpui_rhai_registry::BUNDLED_LAYOUT_SOURCES_BY_ID)
                .chain(gpui_rhai_registry::BUNDLED_PATTERN_SOURCES_BY_ID)
            {
                modules.insert(
                    gpui_rhai::ModuleId::parse(*id).unwrap(),
                    (*module).to_owned(),
                );
            }
            modules.insert(gpui_rhai::ModuleId::parse("main").unwrap(), source);
            let prepared = gpui_rhai::EmbeddedScriptView::new(
                gpui_rhai::ModuleId::parse("main").unwrap(),
                gpui_rhai::EmbeddedScriptSource::new(modules),
                gpui_rhai_registry::DEFAULT_THEME,
            )
            .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
            .locale_sources([("en.rhai".to_owned(), gpui_rhai_registry::EN_LOCALE.to_owned())])
            .asset_sources(gpui_rhai_registry::BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
                (
                    path.strip_suffix(".svg").unwrap_or(path).to_owned(),
                    gpui_rhai::AssetData {
                        mime_type: "image/svg+xml".to_owned(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }))
            .prepare()
            .map_err(|error| error.to_string())?;
            let (window, view) = offscreen::mount(&mut cx, prepared, 1280.0, 860.0, "probe")?;
            offscreen::settle(&mut cx, window)?;
            let mut session = Session { cx, window, view };
            let mut samples = (0..30)
                .map(|_| millis(session.measure(|_| {}).draw))
                .collect::<Vec<_>>();
            println!(
                "{path}: idle draw p50 {:.2} ms p95 {:.2} ms",
                percentile(&mut samples, 0.5),
                percentile(&mut samples, 0.95)
            );
            std::mem::forget(session);
            return Ok(());
        }
        if std::env::var_os("GALLERY_PROFILE_LOOP_NAV").is_some() {
            // Navigate back and forth, for an external sampling profiler.
            let mut session = Session::new("button")?;
            let started = Instant::now();
            let mut renders = 0;
            while started.elapsed() < Duration::from_secs(12) {
                let page = if renders % 2 == 0 { "table" } else { "button" };
                session.measure(|s| s.action("gallery.go", UiValue::String(page.into())));
                renders += 1;
            }
            println!("{renders} navigations in 12 s");
            std::mem::forget(session);
            return Ok(());
        }
        if let Ok(page) = std::env::var("GALLERY_PROFILE_LOOP") {
            // Draw one page in a loop, for an external sampling profiler.
            let mut session = Session::new(&page)?;
            let started = Instant::now();
            let mut frames = 0;
            while started.elapsed() < Duration::from_secs(12) {
                session.measure(|_| {});
                frames += 1;
            }
            println!("{frames} frames in 12 s");
            std::mem::forget(session);
            return Ok(());
        }
        let mut session = Session::new("button")?;
        let mut report = Vec::new();

        let mut stats = Stats {
            name: "idle frame".into(),
            samples: Vec::new(),
        };
        for _ in 0..iterations {
            stats.samples.push(session.measure(|_| {}));
        }
        report.push(stats);

        let pages = acceptance::page_ids();
        let mut stats = Stats {
            name: "navigate page".into(),
            samples: Vec::new(),
        };
        for index in 0..iterations {
            let page = pages[(index * 7) % pages.len()].clone();
            stats
                .samples
                .push(session.measure(|s| s.action("gallery.go", UiValue::String(page))));
        }
        report.push(stats);

        for (name, page) in [
            ("navigate to table", "table"),
            ("navigate to scene.data", "scene.data"),
        ] {
            let mut stats = Stats {
                name: name.into(),
                samples: Vec::new(),
            };
            for _ in 0..iterations {
                stats.samples.push(
                    session.measure(|s| s.action("gallery.go", UiValue::String("button".into()))),
                );
                stats.samples.pop();
                stats.samples.push(
                    session.measure(|s| s.action("gallery.go", UiValue::String(page.into()))),
                );
            }
            report.push(stats);
        }

        session.action("gallery.go", UiValue::String("button".into()));
        offscreen::settle(&mut session.cx, session.window)?;
        let mut stats = Stats {
            name: "toggle density".into(),
            samples: Vec::new(),
        };
        for _ in 0..iterations {
            stats
                .samples
                .push(session.measure(|s| s.action("gallery.toggle_density", UiValue::Null)));
        }
        report.push(stats);

        let mut stats = Stats {
            name: "toggle light/dark".into(),
            samples: Vec::new(),
        };
        for _ in 0..iterations {
            stats
                .samples
                .push(session.measure(|s| s.action("gallery.toggle_mode", UiValue::Null)));
        }
        report.push(stats);

        // Pointer hover across the sidebar rows (hover styles only).
        let mut stats = Stats {
            name: "hover sidebar".into(),
            samples: Vec::new(),
        };
        for index in 0..iterations {
            let y = 80.0 + (index % 20) as f32 * 32.0;
            stats.samples.push(session.measure(|s| {
                let window = s.window;
                s.cx.simulate_event(
                    window,
                    MouseMoveEvent {
                        position: point(px(100.0), px(y)),
                        pressed_button: None,
                        modifiers: Modifiers::default(),
                    },
                );
            }));
        }
        report.push(stats);

        let mut stats = Stats {
            name: "scroll sidebar".into(),
            samples: Vec::new(),
        };
        for index in 0..iterations {
            let delta = if index % 2 == 0 { -48.0 } else { 48.0 };
            stats.samples.push(session.measure(|s| {
                let window = s.window;
                s.cx.simulate_event(
                    window,
                    ScrollWheelEvent {
                        position: point(px(100.0), px(400.0)),
                        delta: ScrollDelta::Pixels(point(px(0.0), px(delta))),
                        ..ScrollWheelEvent::default()
                    },
                );
            }));
        }
        report.push(stats);

        session.action("gallery.go", UiValue::String("scene.operations".into()));
        offscreen::settle(&mut session.cx, session.window)?;
        let mut stats = Stats {
            name: "type in filter".into(),
            samples: Vec::new(),
        };
        // Focus the filter: Tab into main (F6), then to the field.
        offscreen::keys(&mut session.cx, session.window, "tab f6 tab")?;
        for index in 0..iterations {
            let character = if index % 2 == 0 { "e" } else { "backspace" };
            stats.samples.push(session.measure(|s| {
                let window = s.window;
                if character == "backspace" {
                    s.cx.simulate_keystrokes(window, "backspace");
                } else {
                    s.cx.simulate_input(window, character);
                }
            }));
        }
        report.push(stats);

        let mut stats = Stats {
            name: "open command palette".into(),
            samples: Vec::new(),
        };
        for _ in 0..iterations {
            stats
                .samples
                .push(session.measure(|s| s.action("gallery.palette", UiValue::Null)));
            let window = session.window;
            session.cx.simulate_keystrokes(window, "escape");
            offscreen::settle(&mut session.cx, window)?;
            session.take_rhai();
        }
        report.push(stats);

        for stats in &report {
            println!("{}", stats.line());
        }
        // Where Rhai time goes in one navigation and one keystroke.
        for (label, page) in [("navigate to table", "table")] {
            session.action("gallery.go", UiValue::String("button".into()));
            offscreen::settle(&mut session.cx, session.window)?;
            session.take_rhai();
            session.action("gallery.go", UiValue::String(page.into()));
            offscreen::settle(&mut session.cx, session.window)?;
            let view = session.view.clone();
            let snapshot = session
                .cx
                .update(|cx| view.take_performance_snapshot(cx))
                .map_err(|error| error.to_string())?;
            println!("{label}:");
            for timing in &snapshot.timings {
                println!(
                    "  {:?} {:<32} {:>7.2} ms {:>8} ops",
                    timing.operation,
                    timing.source,
                    millis(timing.duration),
                    timing.operations
                );
            }
        }
        // What the audit costs without enumerating system fonts.
        let started = Instant::now();
        let _ = session
            .cx
            .update(|cx| cx.text_system().all_font_names().len());
        println!("all_font_names: {:.1} ms", millis(started.elapsed()));
        // Idle frame cost per page, to find expensive pages.
        if std::env::var_os("GALLERY_PROFILE_PAGES").is_some() {
            let mut costs = Vec::new();
            for page in acceptance::page_ids() {
                session.action("gallery.go", UiValue::String(page.clone()));
                offscreen::settle(&mut session.cx, session.window)?;
                let mut samples = (0..5)
                    .map(|_| millis(session.measure(|_| {}).draw))
                    .collect::<Vec<_>>();
                costs.push((percentile(&mut samples, 0.5), page));
            }
            costs.sort_by(|a, b| b.0.total_cmp(&a.0));
            for (cost, page) in costs.iter().take(15) {
                println!("idle draw {cost:>7.2} ms  {page}");
            }
        }
        let view = session.view.clone();
        let retained = session
            .cx
            .update(|cx| view.take_performance_snapshot(cx))
            .map(|snapshot| snapshot.retained_nodes)
            .unwrap_or_default();
        println!("retained nodes (scene.operations): {retained}");
        std::mem::forget(session);
        Ok(())
    }
}
