//! A shared-layout group belongs to the component instance that named it
//! (#109): two instances of one component with an animated Tabs keep separate
//! identities, while one instance naming the same group and id twice is still
//! a duplicate.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{MotionPreference, ScriptViewConfig, ScriptViewHost};

use crate::support::{Host, Mount, Step, last_error, settle};

const PANE: &str = r#"
import "components/tabs" as tabs;
define_component(#{
    metadata: #{ id: "test/pane", "export": "Pane", version: "0.0.1",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/tabs"], capabilities: #{}, tokens: [], environment: [] },
    schema: #{
        props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root"],
    },
    render: Fn("render_Pane")
});
fn Pane(props) { render_component("test/pane", props) }
fn changed(ctx, value) { () }
fn views() {
    tabs::Tabs(#{ value: "a", label: "Views", panel: false, motion_key: "views",
        tabs: [#{ value: "a", label: "A" }, #{ value: "b", label: "B" }],
        on_change: Fn("changed") })
}
fn render_Pane(ctx, props) { column([views()]) }
"#;

/// Mount a view; the error the mount or its first frames report, if any.
fn mount_error(cx: &mut TestAppContext, view_body: &str) -> Option<String> {
    cx.update(gpui_rhai::install);
    let prepared = Mount::components(format!(
        r#"import "test/pane" as pane;
import "components/tabs" as tabs;
fn changed(ctx, value) {{ () }}
fn view(ctx) {{ column([{view_body}]) }}"#
    ))
    .module("test/pane", PANE)
    .motion(MotionPreference::Normal)
    .prepare()
    .unwrap();
    // Mounted in place of the view when it fails to mount: no token base, no assets.
    let fallback = Mount::new("fn view(ctx) { column([]) }")
        .no_token_base()
        .prepare()
        .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let failure = Rc::new(RefCell::new(None));
    let fail = failure.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("panes", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("panes"), host.clone(), window, cx)
            .unwrap_or_else(|error| {
                *fail.borrow_mut() = Some(error.to_string());
                fallback
                    .mount(ScriptViewConfig::new("empty"), host.clone(), window, cx)
                    .unwrap()
            });
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    if let Some(error) = failure.borrow().clone() {
        return Some(error);
    }
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 1);
    last_error(&mut visual, &view)
}

#[gpui::test]
fn two_instances_keep_their_own_shared_layout_group(cx: &mut TestAppContext) {
    let error = mount_error(
        cx,
        r#"pane::Pane(#{ key: "left" }), pane::Pane(#{ key: "right" })"#,
    );
    assert_eq!(error, None);
}

#[gpui::test]
fn one_instance_with_the_same_group_twice_is_still_a_duplicate(cx: &mut TestAppContext) {
    let node = r#"row([text("x")]).shared_layout("cards", "first")"#;
    let error = mount_error(cx, &format!("{node}, {node}"));
    println!("{error:?}");
    assert!(
        error
            .as_deref()
            .is_some_and(|error| error.contains("cards") && error.contains("first")),
        "{error:?}"
    );
}
