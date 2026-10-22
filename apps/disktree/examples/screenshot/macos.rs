//! Mount the embedded skin in a real offscreen window and save a PNG of the
//! frame — the visual companion to key_harness. usage:
//! `cargo run --example screenshot [PATH] [--keys "t t e"] [out.png]`

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AnyWindowHandle, AppContext, Context, IntoElement, ParentElement, Render, VisualTestAppContext,
    Window, px, size,
};
use gpui_rhai::{PreparedScriptView, ScriptViewConfig, ScriptViewHandle, ScriptViewHost};

use disktree_rhai::capability::DisktreeExtension;

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

fn draw(cx: &mut VisualTestAppContext, window: AnyWindowHandle) -> Result<(), String> {
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        window.refresh();
        let _ = window.draw(cx);
    })
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn main() {
    // Headless helper: SHOT_STITCH="a.png,b.png,out.png" composes two
    // equal-height images side by side (the side-by-side asset).
    if let Ok(spec) = std::env::var("SHOT_STITCH") {
        let parts: Vec<&str> = spec.split(',').collect();
        assert_eq!(parts.len(), 3, "usage: SHOT_STITCH=left,right,out");
        let left = image::open(parts[0]).expect("left png").to_rgba8();
        let right = image::open(parts[1]).expect("right png").to_rgba8();
        let height = left.height().max(right.height());
        let mut canvas = image::RgbaImage::new(left.width() + right.width(), height);
        image::imageops::overlay(&mut canvas, &left, 0, 0);
        image::imageops::overlay(&mut canvas, &right, i64::from(left.width()), 0);
        canvas.save(&parts[2]).expect("save stitch");
        println!("stitched {}", parts[2]);
        return;
    }

    let mut args = std::env::args().skip(1);
    let scan_path = args
        .next()
        .unwrap_or_else(|| env!("CARGO_MANIFEST_DIR").to_owned());
    let keys = std::env::var("SHOT_KEYS").unwrap_or_default();
    // dark, light or system, as --theme takes it.
    let theme_mode = std::env::var("SHOT_THEME").unwrap_or_else(|_| "dark".to_owned());
    let out = std::env::var("SHOT_OUT").unwrap_or_else(|_| "/tmp/disktree-shot.png".to_owned());

    let win_w: f32 = std::env::var("SHOT_W")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1200.0);
    let mut cx = VisualTestAppContext::new(gpui_platform::current_platform(false));
    cx.update(gpui_rhai::install);

    let extension = DisktreeExtension::new().with_args(
        std::path::PathBuf::from(&scan_path),
        false,
        false,
        3,
        "size",
        &theme_mode,
    );
    let mut theme_sources = vec![(
        "themes/flexoki_light.rhai".to_owned(),
        disktree_rhai::embedded::FLEXOKI_LIGHT.to_owned(),
    )];
    theme_sources.extend(
        disktree_rhai::embedded::bundled_palette_sources()
            .into_iter()
            .map(|(name, source)| (name, source.to_owned())),
    );
    let skin = std::env::var("SHOT_SKIN").unwrap_or_else(|_| "ui/main.rhai".to_owned());
    let builder: Box<dyn FnOnce() -> Result<PreparedScriptView, gpui_rhai::ScriptViewError>> =
        if std::env::var_os("SHOT_FILE").is_some() || std::env::var_os("SHOT_SKIN").is_some() {
            Box::new(move || {
                gpui_rhai::FileScriptView::new(std::path::PathBuf::from(skin))
                    .extension(extension)
                    .operation_limit(10_000_000)
                    .expression_depth_limits(64, 128)
                    .prepare()
            })
        } else {
            Box::new(move || {
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
            })
        };
    let prepared = builder().expect("the skin prepares");

    let bindings = prepared.key_bindings().to_vec();
    let mounted = Rc::new(RefCell::new(None));
    let capture = Rc::clone(&mounted);
    let window = cx
        .open_offscreen_window(size(px(win_w), px(800.0)), move |window, cx| {
            let host = ScriptViewHost::new("shot-host".to_owned(), cx).unwrap();
            host.bind_keys(bindings, cx).unwrap();
            let view = prepared
                .mount(
                    ScriptViewConfig::new("shot")
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
        .expect("the window opens");
    let window: AnyWindowHandle = window.into();
    let view = mounted.borrow().clone().expect("the view mounts");

    // Let the scan land and the layout draw.
    let wait_ms: u64 = std::env::var("SHOT_WAIT_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(700);
    std::thread::sleep(std::time::Duration::from_millis(wait_ms));
    for _ in 0..6 {
        draw(&mut cx, window).expect("draw");
    }
    if !keys.is_empty() {
        for keystroke in keys.split_whitespace() {
            cx.simulate_keystrokes(window, keystroke);
            for _ in 0..4 {
                draw(&mut cx, window).expect("draw");
            }
        }
    }
    // Optional: drag the side panel wider before capturing.
    if std::env::var_os("SHOT_DRAG").is_some() {
        let handle = gpui::point(gpui::px(1200.0 - 368.0 - 4.0), gpui::px(400.0));
        let wider = gpui::point(gpui::px(1200.0 - 520.0), gpui::px(400.0));
        let none = gpui::Modifiers::default();
        cx.simulate_mouse_down(window, handle, gpui::MouseButton::Left, none);
        cx.simulate_mouse_move(window, wider, gpui::MouseButton::Left, none);
        for _ in 0..3 {
            draw(&mut cx, window).expect("draw");
        }
        cx.simulate_mouse_up(window, wider, gpui::MouseButton::Left, none);
        for _ in 0..3 {
            draw(&mut cx, window).expect("draw");
        }
    }
    let error = cx.update(|cx| view.last_error(cx)).expect("view alive");

    // Optional: probe every expected palette through the handle.
    if std::env::var_os("SHOT_PROBE").is_some() {
        let pairs = [
            ("Disktree", "Tokyo Night"),
            ("Disktree", "Flexoki Light"),
            ("Default", "Dark"),
            ("Default", "Light"),
            ("Nord", "Dark"),
            ("Gruvbox", "Dark"),
            ("Catppuccin", "Latte"),
            ("Catppuccin", "Mocha"),
            ("Everforest", "Dark"),
            ("TokyoNight", "Night"),
            ("TokyoNight", "Storm"),
            ("Aetheria", "Dark"),
            ("Ethereal", "Dark"),
            ("Futurism", "Dark"),
            ("Hackerman", "Dark"),
            ("Hermarchy", "Dark"),
            ("Retro82", "Dark"),
        ];
        for (family, name) in pairs {
            let result = cx.update(|cx| view.select_theme(family, name, cx));
            match result {
                Ok(_) => println!("theme ok: {family}/{name}"),
                Err(error) => println!("theme MISSING: {family}/{name}: {error}"),
            }
        }
        std::mem::forget(cx);
        return;
    }

    // Motion/animation may still be settling: draw a few more frames.
    std::thread::sleep(std::time::Duration::from_millis(300));
    for _ in 0..4 {
        draw(&mut cx, window).expect("draw");
    }
    let image = cx.capture_screenshot(window).expect("capture settled");
    image.save(&out).expect("save png");
    println!(
        "saved {out} ({}x{}){}",
        image.width(),
        image.height(),
        error.map_or_else(String::new, |e| format!(" — SCRIPT ERROR: {e}"))
    );
    std::mem::forget(cx);
}
