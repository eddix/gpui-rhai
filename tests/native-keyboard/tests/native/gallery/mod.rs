//! The Gallery application: every story, acceptance gates, the title bar and the catalog story.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Render, TestAppContext, WindowHandle};
use gpui_rhai::{PreparedScriptView, ScriptViewConfig, ScriptViewHandle, ScriptViewHost};

mod acceptance;
mod catalog_story;
mod inputs_and_choices;
mod navigation_workflow;
mod overlay_triggers;
mod stories;
mod title_bar;

/// Mounts a Gallery view the CLI prepared as the root of a new window and runs until parked.
/// `root` builds the window root from the host and the view; `bind_keys` installs the view's
/// key bindings on the host first.
fn mount_prepared<R: Render>(
    cx: &mut TestAppContext,
    prepared: PreparedScriptView,
    (host_id, view_id): (&str, &str),
    bind_keys: bool,
    root: fn(ScriptViewHost, ScriptViewHandle) -> R,
) -> (WindowHandle<R>, ScriptViewHandle) {
    let bindings = prepared.key_bindings().to_vec();
    let (host_id, view_id) = (host_id.to_owned(), view_id.to_owned());
    let mounted = Rc::new(RefCell::new(None));
    let capture = Rc::clone(&mounted);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(host_id, cx).unwrap();
        if bind_keys {
            host.bind_keys(bindings, cx).unwrap();
        }
        let view = prepared
            .mount(ScriptViewConfig::new(view_id), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        root(host, view)
    });
    cx.run_until_parked();
    let view = mounted.borrow().clone().unwrap();
    (window, view)
}
