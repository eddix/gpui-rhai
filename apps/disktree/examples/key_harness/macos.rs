//! End-to-end key harness: mount the embedded skin in a real offscreen
//! window, press the navigation/control keys through the genuine keyboard
//! pipeline, and assert the run stayed error-free and the theme key did its
//! work. Mirrors gpui-rhai's own native-keyboard offscreen harness.
//!
//! macOS-only (the offscreen context uses the real macOS renderer); run as
//! an example so it owns the main thread, like gpui-rhai's own harnesses:
//! `cargo run --example key_harness`.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AnyWindowHandle, AppContext, Context, IntoElement, ParentElement, Render, VisualTestAppContext,
    Window, px, size,
};
use gpui_rhai::{PreparedScriptView, ScriptViewConfig, ScriptViewHandle, ScriptViewHost};

use disktree_rhai::capability::DisktreeExtension;

/// A shell that hosts the script view's own window container, like the app's
/// `Shell` in src/main.rs.
struct Shell {
    host: ScriptViewHost,
    view: Option<ScriptViewHandle>,
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.as_ref().map_or_else(
            || gpui::div().child("no skin mounted").into_any_element(),
            |view| {
                view.element()
                    .unwrap_or_else(|error| gpui::div().child(error.to_string()).into_any_element())
            },
        ))
    }
}

fn context() -> VisualTestAppContext {
    let mut cx = VisualTestAppContext::new(gpui_platform::current_platform(false));
    cx.update(gpui_rhai::install);
    cx
}

fn mount(
    cx: &mut VisualTestAppContext,
    prepared: PreparedScriptView,
    width: f32,
    height: f32,
    id: &str,
) -> Result<(AnyWindowHandle, ScriptViewHandle), String> {
    let bindings = prepared.key_bindings().to_vec();
    let mounted = Rc::new(RefCell::new(None));
    let capture = Rc::clone(&mounted);
    let id = id.to_owned();
    let window = cx
        .open_offscreen_window(size(px(width), px(height)), move |window, cx| {
            let host = ScriptViewHost::new(format!("{id}-host"), cx).unwrap();
            host.bind_keys(bindings, cx).unwrap();
            let view = prepared
                .mount(
                    ScriptViewConfig::new(id)
                        .paint_background(true)
                        .window_drag_areas(true),
                    host.clone(),
                    window,
                    cx,
                )
                .unwrap();
            let _ = view.focus(window, cx);
            *capture.borrow_mut() = Some(view.clone());
            cx.new(|_| Shell {
                host,
                view: Some(view),
            })
        })
        .map_err(|error| error.to_string())?;
    let view = mounted.borrow().clone().ok_or("the view did not mount")?;
    Ok((window.into(), view))
}

fn draw(cx: &mut VisualTestAppContext, window: AnyWindowHandle) -> Result<(), String> {
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        window.refresh();
        let _ = window.draw(cx);
    })
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn settle(cx: &mut VisualTestAppContext, window: AnyWindowHandle) -> Result<(), String> {
    for _ in 0..4 {
        draw(cx, window)?;
    }
    Ok(())
}

fn keys(
    cx: &mut VisualTestAppContext,
    window: AnyWindowHandle,
    keystrokes: &str,
) -> Result<(), String> {
    for keystroke in keystrokes.split_whitespace() {
        cx.simulate_keystrokes(window, keystroke);
        settle(cx, window)?;
    }
    Ok(())
}

fn assert_no_error(cx: &mut VisualTestAppContext, view: &ScriptViewHandle, stage: &str) {
    let error = cx.update(|cx| view.last_error(cx)).expect("view alive");
    assert!(
        error.is_none(),
        "script error at {stage}: {}",
        error.unwrap_or_default()
    );
}

/// The active theme variant's name, through the handle's theme snapshot.
fn variant_name(cx: &mut VisualTestAppContext, view: &ScriptViewHandle) -> String {
    cx.update(|cx| view.theme_snapshot(cx))
        .expect("theme snapshot")
        .variant
        .name
        .clone()
}

/// The side panel's separator, where the last frame put it.
fn separator_bounds(
    cx: &mut VisualTestAppContext,
    view: &ScriptViewHandle,
) -> gpui_rhai::GeometryBounds {
    cx.update(|cx| view.accessibility_snapshot(cx))
        .expect("accessibility snapshot")
        .nodes()
        .find(|node| node.role == "separator" && node.name == "Resize the side panel")
        .and_then(|node| node.geometry.as_ref())
        .map(|geometry| geometry.layout)
        .expect("the side panel separator is presented")
}

/// A small deterministic tree the scan finishes on instantly.
fn fixture_tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for path in [
        "src/engine/mod.rs",
        "src/main.rs",
        "docs/guide.md",
        "node_modules/left-pad/index.js",
        "target/debug/some-binary",
        "README.md",
    ] {
        let file = dir.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, vec![0u8; 4096]).unwrap();
    }
    std::fs::create_dir_all(dir.path().join("empty-dir")).unwrap();
    dir
}

fn prepared(root: &std::path::Path) -> PreparedScriptView {
    let extension =
        DisktreeExtension::new().with_args(root.to_path_buf(), false, false, 3, "size", "dark");
    let mut theme_sources = vec![(
        "themes/flexoki_light.rhai".to_owned(),
        disktree_rhai::embedded::FLEXOKI_LIGHT.to_owned(),
    )];
    theme_sources.extend(
        disktree_rhai::embedded::bundled_palette_sources()
            .into_iter()
            .map(|(name, source)| (name, source.to_owned())),
    );
    gpui_rhai::EmbeddedScriptView::new(
        disktree_rhai::embedded::entry(),
        disktree_rhai::embedded::sources(),
        disktree_rhai::embedded::THEME,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .locale_sources([(
        "en".to_owned(),
        disktree_rhai::embedded::EN_LOCALE.to_owned(),
    )])
    .theme_sources(theme_sources)
    .asset_sources(disktree_rhai::embedded::assets())
    .manifest(disktree_rhai::embedded::manifest())
    .extension(extension)
    .operation_limit(10_000_000)
    .expression_depth_limits(64, 128)
    .prepare()
    .expect("the embedded skin prepares")
}

pub fn main() {
    let tree = fixture_tree();
    let mut cx = context();
    let (window, view) = mount(&mut cx, prepared(tree.path()), 1200.0, 800.0, "key-harness")
        .expect("the skin mounts");

    // Let the scan land and the first layout draw.
    std::thread::sleep(std::time::Duration::from_millis(500));
    settle(&mut cx, window).expect("settle");
    assert_no_error(&mut cx, &view, "mounted");

    // Navigation and view keys on the landed mosaic.
    // Two enters: the second level renders the crumb ▾ triggers, whose
    // popover content builds against the (possibly never-opened) menu
    // store — a real crash path found in the wild.
    keys(
        &mut cx,
        window,
        "h j k l left right up down tab enter enter u u",
    )
    .expect("navigation keys");
    assert_no_error(&mut cx, &view, "navigation");

    // Metric, depth, visibility, zoom.
    keys(&mut cx, window, "t t t [ ] - = 0 i d").expect("control keys");
    assert_no_error(&mut cx, &view, "controls");

    // The find flow: type, apply, clear.
    keys(&mut cx, window, "/ m a i n enter escape escape").expect("find flow");
    assert_no_error(&mut cx, &view, "find");

    // Help opens and closes.
    keys(&mut cx, window, "? escape").expect("help");
    assert_no_error(&mut cx, &view, "help");

    // The theme cycles to the next mounted palette.
    let before = variant_name(&mut cx, &view);
    keys(&mut cx, window, "e").expect("theme key");
    let after = variant_name(&mut cx, &view);
    assert_ne!(before, after, "the e key changed the theme");
    assert_no_error(&mut cx, &view, "theme");

    // The side panel's SplitPane: press the separator where it is drawn,
    // drag it left, release. The separator must follow and stay (the
    // release commits `panel_ratio` through on_resize).
    {
        let before = separator_bounds(&mut cx, &view);
        let press = gpui::point(
            gpui::px((before.x + before.width / 2.0) as f32),
            gpui::px((before.y + before.height / 2.0) as f32),
        );
        let wider = gpui::point(press.x - gpui::px(150.0), press.y);
        let none = gpui::Modifiers::default();
        cx.simulate_mouse_down(window, press, gpui::MouseButton::Left, none);
        cx.simulate_mouse_move(window, wider, gpui::MouseButton::Left, none);
        settle(&mut cx, window).expect("settle drag");
        cx.simulate_mouse_up(window, wider, gpui::MouseButton::Left, none);
        settle(&mut cx, window).expect("settle release");
        assert_no_error(&mut cx, &view, "panel drag");
        let after = separator_bounds(&mut cx, &view);
        let moved = before.x - after.x;
        assert!(
            (moved - 150.0).abs() <= 2.0,
            "the separator followed the drag: x {} -> {}",
            before.x,
            after.x
        );
    }

    // Rescan while mounted.
    keys(&mut cx, window, "r").expect("rescan");
    std::thread::sleep(std::time::Duration::from_millis(300));
    settle(&mut cx, window).expect("settle after rescan");
    assert_no_error(&mut cx, &view, "rescan");

    println!("key_harness: every key went through a real window, no script errors");
    // The mounted view outlives the harness on purpose; skip the leak check.
    std::mem::forget(cx);
}
