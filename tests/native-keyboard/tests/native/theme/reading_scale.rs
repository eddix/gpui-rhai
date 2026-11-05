//! The reading scale (`scale` environment): a subtree at 150% sets its type
//! and token geometry half again as large, a sibling at the default keeps its
//! own, and literal lengths stay as written.

use gpui::TestAppContext;

use crate::support::{Mount, Step, bounds, settle};

const SOURCE: &str = r#"import "components/button" as button;
fn noop(ctx,payload){()}
fn view(ctx){ column([
    button::Button(#{ key: "plain", text: "Plain", on_click: Fn("noop") }),
    column([
        button::Button(#{ key: "large", text: "Large", on_click: Fn("noop") }),
        row([]).with_key("literal").accessibility_role("group").accessibility_label("Literal")
            .with_style(style().width(px(120)).height(px(10))),
    ]).env(#{ scale: "150" }),
]).with_style(style().width(px(600)).items_start()) }"#;

#[gpui::test]
fn a_subtree_reads_at_its_own_scale(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(SOURCE).mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let plain = bounds(&mut visual, &view, "button", "Plain");
    let large = bounds(&mut visual, &view, "button", "Large");
    // metrics.control: 32px at md and comfortable.
    assert!((plain.height - 32.0).abs() < 0.5, "{plain:?}");
    assert!((large.height - 48.0).abs() < 0.5, "{large:?}");
    assert!(large.width > plain.width * 1.3, "{plain:?} {large:?}");
    let literal = bounds(&mut visual, &view, "group", "Literal");
    assert!((literal.width - 120.0).abs() < 0.5, "{literal:?}");
}
