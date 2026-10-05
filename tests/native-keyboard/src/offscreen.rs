//! Offscreen rendering helpers shared by the baseline and profiling tools.
//!
//! Windows open at (-10000, -10000) in a GPUI `VisualTestAppContext` backed by
//! the real macOS platform; frames are read back from the Metal texture, so no
//! window manager, other window or screen-recording permission is involved.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AnyWindowHandle, AppContext, Context, IntoElement, Render, VisualTestAppContext, Window, px,
    size,
};
use gpui_rhai::{PreparedScriptView, ScriptViewConfig, ScriptViewHandle, ScriptViewHost};

/// A mounted script view as a window root.
pub struct Root {
    pub host: ScriptViewHost,
    pub view: ScriptViewHandle,
}

impl Render for Root {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

/// A real-platform test context with the runtime installed.
pub fn context() -> VisualTestAppContext {
    let mut cx = VisualTestAppContext::new(gpui_platform::current_platform(false));
    cx.update(gpui_rhai::install);
    cx
}

/// Mount a prepared view in an offscreen window of `width` x `height` points.
pub fn mount(
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
                    ScriptViewConfig::new(id).paint_background(true),
                    host.clone(),
                    window,
                    cx,
                )
                .unwrap();
            let _ = view.focus(window, cx);
            *capture.borrow_mut() = Some(view.clone());
            cx.new(|_| Root { host, view })
        })
        .map_err(|error| error.to_string())?;
    let view = mounted.borrow().clone().ok_or("the view did not mount")?;
    Ok((window.into(), view))
}

/// Run pending work and draw one frame.
pub fn draw(cx: &mut VisualTestAppContext, window: AnyWindowHandle) -> Result<(), String> {
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        window.refresh();
        let _ = window.draw(cx);
    })
    .map_err(|error| error.to_string())
}

/// Draw a few frames so geometry-dependent work (virtual rows, overlays,
/// focus) settles.
pub fn settle(cx: &mut VisualTestAppContext, window: AnyWindowHandle) -> Result<(), String> {
    for _ in 0..4 {
        draw(cx, window)?;
    }
    Ok(())
}

/// Simulate keystrokes (space separated, GPUI syntax) and settle.
pub fn keys(
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

/// Read the current frame back from the GPU and close the window.
pub fn capture(
    cx: &mut VisualTestAppContext,
    window: AnyWindowHandle,
) -> Result<image::RgbaImage, String> {
    let image = cx
        .capture_screenshot(window)
        .map_err(|error| error.to_string())?;
    cx.update_window(window, |_, window, _| window.remove_window())
        .map_err(|error| error.to_string())?;
    cx.run_until_parked();
    Ok(image)
}
