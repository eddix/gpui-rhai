//! InlineState draws `detail` (the raw error, selectable) in the error, stale and
//! refreshing states (#122): stale is where a refresh failed while the data stays on
//! screen. The error state is the control.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::{BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID};

use crate::support::{Mount, Step, nodes, settle};

fn mount(cx: &mut TestAppContext, view_body: &str) -> (VisualTestContext, ScriptViewHandle) {
    let main = format!(
        r#"
import "patterns/inline_state" as inline_state;
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(400)).height(px(400))) }}
"#
    );
    let (mut visual, view) = Mount::components(main)
        .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .id("state")
        .mount(cx);
    visual.run_until_parked();
    settle(&mut visual, Step::Refresh, 1);
    (visual, view)
}

/// Every text node in the accessibility tree, in tree order.
fn texts(cx: &mut TestAppContext, state: &str) -> Vec<String> {
    let body = format!(
        r#"inline_state::InlineState(#{{ key: "s", state: "{state}", title: "刷新失败", detail: "原文" }})"#
    );
    let (mut visual, view) = mount(cx, &body);
    let found = nodes(&mut visual, &view)
        .into_iter()
        .filter(|node| node.role == "text")
        .map(|node| node.name)
        .collect::<Vec<_>>();
    eprintln!("{state}: text nodes {found:?}");
    found
}

#[gpui::test]
fn error_state_draws_detail(cx: &mut TestAppContext) {
    let found = texts(cx, "error");
    assert!(
        found.iter().any(|text| text == "刷新失败"),
        "error: the title is drawn (precondition): {found:?}"
    );
    assert!(
        found.iter().any(|text| text == "原文"),
        "error: detail should be drawn (control): {found:?}"
    );
}

#[gpui::test]
fn stale_state_draws_detail(cx: &mut TestAppContext) {
    let found = texts(cx, "stale");
    assert!(
        found.iter().any(|text| text == "刷新失败"),
        "stale: the title is drawn (precondition): {found:?}"
    );
    assert!(
        found.iter().any(|text| text == "原文"),
        "stale: the schema accepts `detail` but it is not drawn: {found:?}"
    );
}

#[gpui::test]
fn refreshing_state_draws_detail(cx: &mut TestAppContext) {
    let found = texts(cx, "refreshing");
    assert!(
        found.iter().any(|text| text == "刷新失败"),
        "refreshing: the title is drawn (precondition): {found:?}"
    );
    assert!(
        found.iter().any(|text| text == "原文"),
        "refreshing: the schema accepts `detail` but it is not drawn: {found:?}"
    );
}
