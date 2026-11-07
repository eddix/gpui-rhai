//! `ctx.set_local_theme` changes what a component's subtree paints (#1b): its tokens
//! resolve against the local variant, overlays opened inside it follow, and a sibling
//! keeps the view's theme.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;
use gpui_rhai_registry::DEFAULT_LIGHT_THEME;

use crate::support::{Mount, PaintedQuad, Step, by_test_id, dispatch, filled, node_by_test_id};

const DARK_SURFACE: u32 = 0x1514_12ff;
const LIGHT_SURFACE: u32 = 0xf3f1_eaff;
const LIGHT_RAISED: u32 = 0xfdfb_f4ff;

const SCRIPT: &str = r#"
import "components/popover" as popover;
define_component(#{ metadata: #{ id: "tests/panel", "export": "Panel", version: "0.1.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{} },
    schema: #{ props: #{ key: #{ schema: #{ type: "string" }, required: true, sensitive: false } },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: [] },
    render: Fn("panel") });
fn light(ctx, payload) { ctx.set_local_theme("Default", "Light"); }
fn noop(ctx, payload) {}
fn panel(ctx, props) {
    let children = [text("Light").test_id(`${props.key}-light`).on_click(Fn("light"))];
    if props.key == "a" {
        children.push(popover::Popover(#{ key: "details", label: "Details", open: true,
            trigger: text("Open"), content: text("Inside"), on_open_change: Fn("noop") }));
    }
    box(children).test_id(`${props.key}-panel`).accessibility_role("group").accessibility_label(props.key)
        .with_style(style().width(px(160)).height(px(120)).background(theme_color("surface")))
}
fn view(ctx) {
    row([render_component("tests/panel", #{ key: "a" }), render_component("tests/panel", #{ key: "b" })])
        .with_style(style().gap(px(8)))
}
"#;

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Frame, 3);
}

/// The quads filled with `color` that cover the panel `key`.
fn panel_fills(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    key: &str,
    color: u32,
) -> Vec<PaintedQuad> {
    let panel = node_by_test_id(visual, view, &format!("{key}-panel"))
        .geometry
        .unwrap()
        .visual;
    filled(visual, color)
        .into_iter()
        .filter(|quad| {
            (f64::from(quad.bounds.x) - panel.x).abs() < 0.5
                && (f64::from(quad.bounds.y) - panel.y).abs() < 0.5
                && (f64::from(quad.bounds.width) - panel.width).abs() < 0.5
        })
        .collect()
}

#[gpui::test]
fn a_local_theme_paints_its_subtree_and_its_overlays(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(SCRIPT)
        .with(|view| view.theme_sources([("light".to_owned(), DEFAULT_LIGHT_THEME.to_owned())]))
        .motion(MotionPreference::None)
        .id("local-theme")
        .mount(cx);
    settle(&mut visual);
    assert_eq!(crate::support::last_error(&mut visual, &view), None);
    assert_eq!(panel_fills(&mut visual, &view, "a", DARK_SURFACE).len(), 1);
    assert!(filled(&mut visual, LIGHT_RAISED).is_empty());

    dispatch(&mut visual, &view, by_test_id("a-light"), "click").unwrap();
    settle(&mut visual);
    assert_eq!(
        panel_fills(&mut visual, &view, "a", LIGHT_SURFACE).len(),
        1,
        "the scoped panel paints the light surface"
    );
    assert_eq!(
        panel_fills(&mut visual, &view, "b", DARK_SURFACE).len(),
        1,
        "its sibling keeps the view's theme"
    );
    assert!(
        !filled(&mut visual, LIGHT_RAISED).is_empty(),
        "the popover opened inside the panel paints the light raised surface"
    );
}
