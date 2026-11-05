//! `debug_unreachable_controls` finds a control cut off where nothing scrolls
//! and controls laid over one another, and leaves a scrolled-away control, a
//! control wholly hidden on purpose and a fitting layout alone.

use gpui::TestAppContext;

use crate::support::{Mount, Step, settle};

fn findings(cx: &mut TestAppContext, body: &str) -> Vec<String> {
    let source = format!(
        r#"import "components/button" as button;
fn noop(ctx,payload){{()}}
fn b(key){{ button::Button(#{{ key: key, text: `Action ${{key}}`, on_click: Fn("noop") }}) }}
fn view(ctx){{ column([{body}]).with_style(style().width(px(600)).padding(px(16))) }}"#
    );
    let (mut visual, view) = Mount::components(source).mount(cx);
    settle(&mut visual, Step::Frame, 3);
    visual.update(|_, cx| view.debug_unreachable_controls(cx).unwrap())
}

#[gpui::test]
fn a_fitting_row_is_reachable(cx: &mut TestAppContext) {
    let found = findings(cx, r#"row([b("a"), b("b")]).with_style(style().gap(px(8)))"#);
    assert!(found.is_empty(), "{found:?}");
}

#[gpui::test]
fn a_control_cut_off_where_nothing_scrolls_is_found(cx: &mut TestAppContext) {
    let found = findings(
        cx,
        r#"row([b("a"), b("b").with_style(style().flex_shrink(false))])
            .with_style(style().width(px(120)).gap(px(8)).clip())"#,
    );
    assert!(
        found.iter().any(|finding| finding.contains("cut off")),
        "{found:?}"
    );
}

#[gpui::test]
fn a_control_in_a_scroll_container_is_reachable(cx: &mut TestAppContext) {
    let found = findings(
        cx,
        r#"row([b("a"), b("b"), b("c"), b("d")]).with_style(style().width(px(120)).gap(px(8)).overflow_x_scroll())"#,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[gpui::test]
fn controls_laid_over_one_another_are_found(cx: &mut TestAppContext) {
    let found = findings(
        cx,
        r#"box([b("a"), b("b").with_style(style().absolute().top(px(4)).left(px(12)))]).with_style(style().relative())"#,
    );
    assert!(
        found.iter().any(|finding| finding.contains("overlap")),
        "{found:?}"
    );
}

