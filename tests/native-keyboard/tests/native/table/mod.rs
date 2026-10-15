//! Table: columns, keyboard, context requests, sticky and group headers, extent, focus and virtual
//! rows.

// The 1000-row performance example, shared by the column and virtual-row tests.
#[allow(dead_code)]
#[path = "../../../../../crates/gpui-rhai/examples/internal/performance/table_1000.rs"]
mod table_1000_example;

mod boundaries;
mod click_focus;
mod columns;
mod context_requests;
mod extent;
mod frame_demand;
mod group_value_case;
mod header_on_raised_surface;
mod keyboard;
mod sort_key;
mod sticky_header;
mod virtual_rows;

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{TestAppContext, WindowHandle};
use gpui_rhai::{EmbeddedScriptView, ScriptViewConfig, ScriptViewHandle, ScriptViewHost};

use crate::support::{Host, Mount};

/// `main` over Table and Badge (the modules a Table needs) with the bundled assets.
fn table(main: impl Into<String>) -> Mount {
    Mount::new(main)
        .registry("components/table")
        .registry("components/badge")
        .bundled_assets()
}

/// Mounts, then refreshes every window and runs until parked once more.
fn mount_refreshed(
    cx: &mut TestAppContext,
    mount: Mount,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    let mounted = mount.mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();
    mounted
}

/// Mounts a view built elsewhere (the 1000-row example) as `host_id`/`view_id`, the way
/// [`mount_refreshed`] does.
fn mount_example(
    cx: &mut TestAppContext,
    view: EmbeddedScriptView,
    host_id: &'static str,
    view_id: &'static str,
) -> (WindowHandle<Host>, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let prepared = view.prepare().unwrap();
    let mounted = Rc::new(RefCell::new(None));
    let capture = Rc::clone(&mounted);
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new(host_id, cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new(view_id), host.clone(), window, cx)
            .unwrap();
        *capture.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    cx.refresh().unwrap();
    cx.run_until_parked();
    let view = mounted.borrow().clone().unwrap();
    (window, view)
}
