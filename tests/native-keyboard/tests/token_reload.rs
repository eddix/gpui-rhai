//! Token and script hot reload keep the preparation contract: a candidate that
//! preparation would reject is not applied, the last-good theme keeps
//! rendering, the error is reported, and fixing the files applies them.
//!
//! Run with `cargo test --features dev-reload --test token_reload`.
#![cfg(feature = "dev-reload")]

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window, WindowHandle};
use gpui_rhai::*;

const THEME: &str = r#"fn theme(){#{family:"Reload",name:"Dark",mode:"dark",tokens:#{colors:#{accent:0x3366ffff}}}}"#;

/// A component that reads `metrics.unit` (and, when `extra` is set, also
/// `metrics.extra`) as its height.
fn component(extra: bool) -> String {
    let tokens = if extra {
        r#"["metrics.unit", "metrics.extra"]"#
    } else {
        r#"["metrics.unit"]"#
    };
    let height = if extra {
        "metrics.extra"
    } else {
        "metrics.unit"
    };
    format!(
        r#"/* gpui-rhai
{{"id":"components/probe","export":"Probe","version":"0.2.0","runtime_api":{{"min_inclusive":3,"max_exclusive":4}},"dependencies":[],"capabilities":{{}},"tokens":{tokens},"environment":[]}}
*/
define_component(#{{metadata:#{{id:"components/probe","export":"Probe",version:"0.2.0",runtime_api:#{{min_inclusive:3,max_exclusive:4}},dependencies:[],capabilities:#{{}},tokens:{tokens},environment:[]}},schema:#{{props:#{{}},state:#{{fields:#{{}}}},events:#{{}},slots:#{{}},parts:[]}},render:Fn("render_probe")}});
fn Probe(props){{render_component("components/probe",props)}}
fn render_probe(ctx,props){{box([text("Token")]).accessibility_role("group").test_id("cell").with_style(style().width(px(100)).height(theme_length("{height}")))}}"#
    )
}

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(
            self.view
                .element()
                .unwrap_or_else(|_| gpui::div().into_any_element()),
        )
    }
}

struct App {
    dir: PathBuf,
    visual: VisualTestContext,
    view: ScriptViewHandle,
}

impl App {
    fn new(cx: &mut TestAppContext, name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("gpui-rhai-reload-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("components")).unwrap();
        std::fs::write(dir.join("app.toml"), "entry = \"main\"\nruntime_api = 3\n").unwrap();
        std::fs::write(
            dir.join("main.rhai"),
            "import \"components/probe\" as p; fn view(ctx){p::Probe(#{})}",
        )
        .unwrap();
        std::fs::write(dir.join("components/probe.rhai"), component(false)).unwrap();
        std::fs::write(dir.join("theme.rhai"), THEME).unwrap();
        std::fs::write(
            dir.join("tokens.rhai"),
            "fn tokens(){#{metrics:#{unit:px(40)}}}",
        )
        .unwrap();
        let prepared = FileScriptView::new(dir.join("main.rhai"))
            .development(true)
            .prepare()
            .unwrap();
        cx.update(gpui_rhai::install);
        let saved = Rc::new(RefCell::new(None));
        let save = saved.clone();
        let window: WindowHandle<Host> = cx.add_window(move |window, cx| {
            let host = ScriptViewHost::new("reload", cx).unwrap();
            let view = prepared
                .mount(ScriptViewConfig::new("reload"), host.clone(), window, cx)
                .unwrap();
            *save.borrow_mut() = Some(view.clone());
            Host { host, view }
        });
        cx.run_until_parked();
        let view = saved.borrow().clone().unwrap();
        let visual = VisualTestContext::from_window(*window, cx);
        let mut app = Self { dir, visual, view };
        app.settle();
        app
    }

    fn write(&self, file: &str, source: &str) {
        std::fs::write(self.dir.join(file), source).unwrap();
    }

    /// Let the file watcher see the edit and the view apply it.
    fn settle(&mut self) {
        for _ in 0..15 {
            std::thread::sleep(Duration::from_millis(100));
            self.visual
                .executor()
                .advance_clock(Duration::from_millis(120));
            self.visual.run_until_parked();
        }
    }

    fn height(&mut self) -> f64 {
        self.visual
            .update(|window, cx| window.simulate_next_frame(cx));
        self.visual.run_until_parked();
        let view = self.view.clone();
        self.visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .nodes()
                .find(|node| node.test_id.as_deref() == Some("cell"))
                .and_then(|node| node.geometry)
                .unwrap()
                .visual
                .height
        })
    }

    fn error(&mut self) -> Option<String> {
        let view = self.view.clone();
        self.visual.update(|_, cx| view.last_error(cx).unwrap())
    }

    fn clear_error(&mut self) {
        let view = self.view.clone();
        self.visual.update(|_, cx| view.clear_error(cx)).unwrap();
    }

    fn dispose(mut self) {
        let view = self.view.clone();
        self.visual.update(|_, cx| view.dispose(cx)).unwrap();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn prepare_error(dir: &Path) -> Option<String> {
    FileScriptView::new(dir.join("main.rhai"))
        .prepare()
        .err()
        .map(|error| error.to_string())
}

#[gpui::test]
fn a_token_edit_preparation_would_reject_keeps_the_last_good_theme(cx: &mut TestAppContext) {
    let mut app = App::new(cx, "required");
    assert_eq!(app.height(), 40.0);
    app.write("tokens.rhai", "fn tokens(){#{metrics:#{unit:px(50)}}}");
    app.settle();
    assert_eq!(app.height(), 50.0, "a valid edit applies");
    assert_eq!(app.error(), None);

    // Removing a token a component declares: rejected, as by preparation.
    app.write("tokens.rhai", "fn tokens(){#{}}");
    app.settle();
    let error = app.error().expect("the rejected edit is reported");
    assert!(error.contains("metrics.unit"), "{error}");
    assert_eq!(app.height(), 50.0, "the last-good theme keeps rendering");
    let cold = prepare_error(&app.dir).expect("preparation rejects the same files");
    assert!(cold.contains("metrics.unit"), "{cold}");

    // A broken expression: rejected the same way.
    app.write("tokens.rhai", "fn tokens(){#{metrics:#{unit:px(}}");
    app.settle();
    assert!(app.error().is_some());
    assert_eq!(app.height(), 50.0);

    // Fixing the file applies it and clears the error.
    app.write("tokens.rhai", "fn tokens(){#{metrics:#{unit:px(60)}}}");
    app.settle();
    assert_eq!(app.error(), None);
    assert_eq!(app.height(), 60.0);
    app.dispose();
}

#[gpui::test]
fn a_new_component_requirement_and_its_token_apply_in_one_batch(cx: &mut TestAppContext) {
    let mut app = App::new(cx, "batch");
    assert_eq!(app.height(), 40.0);
    // The component starts reading `metrics.extra` while the token appears:
    // checked against the new theme, so the batch applies.
    app.write(
        "tokens.rhai",
        "fn tokens(){#{metrics:#{unit:px(40), extra:px(70)}}}",
    );
    app.write("components/probe.rhai", &component(true));
    app.settle();
    assert_eq!(app.error(), None);
    assert_eq!(app.height(), 70.0);
    app.dispose();
}

#[gpui::test]
fn a_component_that_needs_a_missing_token_is_not_reloaded(cx: &mut TestAppContext) {
    let mut app = App::new(cx, "script");
    assert_eq!(app.height(), 40.0);
    app.write("components/probe.rhai", &component(true));
    app.settle();
    let error = app.error().expect("the script edit is rejected");
    assert!(error.contains("metrics.extra"), "{error}");
    assert_eq!(app.height(), 40.0, "the last-good program keeps rendering");
    // Adding the token retries the rejected script with it.
    app.clear_error();
    app.write(
        "tokens.rhai",
        "fn tokens(){#{metrics:#{unit:px(40), extra:px(70)}}}",
    );
    app.settle();
    assert_eq!(app.error(), None);
    assert_eq!(app.height(), 70.0);
    app.dispose();
}
