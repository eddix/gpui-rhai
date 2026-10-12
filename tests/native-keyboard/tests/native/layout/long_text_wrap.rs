//! Long Alert and InlineState text wraps in a narrow container (#123): the flex items
//! holding text may shrink below their line (`min_width(0)`), and InlineState's marker
//! rows are capped at the column width. Alert with caller `min_width(0)` part styles
//! and InlineState's description are the controls.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::{BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID};

use crate::support::{Mount, Step, settle, snapshot};

const TITLE: &str = "同步任务在上一次刷新时失败了，界面上显示的是五分钟以前缓存下来的数据，请检查网络连接以后再点一次刷新按钮重新拉取最新的数据";
const BOX_WIDTH: f64 = 300.0;

/// Layout bounds as (x, y, width, height).
type Bounds = (f64, f64, f64, f64);

fn mount(cx: &mut TestAppContext, view_body: &str) -> (VisualTestContext, ScriptViewHandle) {
    let main = format!(
        r#"
import "components/alert" as alert;
import "patterns/inline_state" as inline_state;
fn title() {{ "{TITLE}" }}
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(800)).height(px(600))) }}
"#
    );
    let (mut visual, view) = Mount::components(main)
        .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .id("wrap")
        .mount(cx);
    visual.run_until_parked();
    settle(&mut visual, Step::Refresh, 1);
    (visual, view)
}

/// Layout bounds (x, y, width, height) of the title text and of the 300px box (role group).
fn measure(cx: &mut TestAppContext, label: &str, component: &str) -> (Bounds, Bounds) {
    cx.update(gpui_rhai::install);
    let body = format!(
        r#"box([{component}]).with_style(style().width(px({BOX_WIDTH})))
            .accessibility_role("group").accessibility_label("frame")"#
    );
    let (mut visual, view) = mount(cx, &body);
    let snapshot = snapshot(&mut visual, &view);
    let bounds = |role: &str, name: &str| {
        let g = snapshot
            .find_by_role_and_name(role, name)
            .next()
            .unwrap_or_else(|| panic!("no {role} {name:?}"))
            .geometry
            .unwrap()
            .layout;
        (g.x, g.y, g.width, g.height)
    };
    let (title, frame) = (bounds("text", TITLE), bounds("group", "frame"));
    eprintln!("{label}: title (x, y, w, h) = {title:?}, box = {frame:?}");
    (title, frame)
}

fn assert_wraps_inside(label: &str, title: Bounds, frame: Bounds) {
    let (x, _, w, _) = title;
    let (fx, _, fw, _) = frame;
    assert!(
        x + w <= fx + fw + 0.5,
        "{label}: the title should wrap inside the {BOX_WIDTH}px box; it is {w}px wide and ends at {}, the box ends at {}",
        x + w,
        fx + fw
    );
}

#[gpui::test]
fn alert_title_wraps_in_a_narrow_box(cx: &mut TestAppContext) {
    let (title, frame) = measure(cx, "Alert", r#"alert::Alert(#{ title: title() })"#);
    assert_wraps_inside("Alert", title, frame);
}

#[gpui::test]
fn alert_title_wraps_with_min_width_zero_parts(cx: &mut TestAppContext) {
    let (title, frame) = measure(
        cx,
        "Alert + part_styles",
        r#"alert::Alert(#{ title: title(), part_styles: #{
            title: style().min_width(px(0)), body: style().min_width(px(0)),
            description: style().min_width(px(0)) } })"#,
    );
    assert_wraps_inside("Alert + part_styles (control)", title, frame);
}

#[gpui::test]
fn inline_state_error_title_wraps_in_a_narrow_box(cx: &mut TestAppContext) {
    let (title, frame) = measure(
        cx,
        "InlineState error",
        r#"inline_state::InlineState(#{ key: "s", state: "error", title: title() })"#,
    );
    assert_wraps_inside("InlineState error", title, frame);
}

#[gpui::test]
fn inline_state_error_description_wraps_in_a_narrow_box(cx: &mut TestAppContext) {
    // Same component, same box, same text, as the description: it is a direct child of
    // the root column, not inside a row, so it wraps.
    let (text, frame) = measure(
        cx,
        "InlineState error description",
        r#"inline_state::InlineState(#{ key: "s", state: "error", title: "Sync failed",
            description: title() })"#,
    );
    assert_wraps_inside("InlineState error description (control)", text, frame);
}

#[gpui::test]
fn inline_state_stale_and_refreshing_titles_wrap_in_a_narrow_box(cx: &mut TestAppContext) {
    // The one-line states shrink their text items too, with a description beside the title.
    for state in ["stale", "refreshing"] {
        let (title, frame) = measure(
            cx,
            state,
            &format!(
                r#"inline_state::InlineState(#{{ key: "s", state: "{state}", title: title(),
                    description: "5m" }})"#
            ),
        );
        assert_wraps_inside(state, title, frame);
    }
}
