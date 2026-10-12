//! Collapsible's panel height follows density (#127): omitted, the panel fits its content
//! (measured after layout); a Length such as `theme_length("metrics.row") * 3` resolves in
//! the subtree's density. An explicit integer stays as given (the control).

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{GeometryBounds, MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::{BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID};

use crate::support::{Mount, Step, click_center, settle, snapshot};

const SCRIPT: &str = r#"
fn line(label) {
    text(label).with_style(style().min_height(theme_length("metrics.row")).items_center())
}
fn content() {
    column([line("Line 1"), line("Line 2")])
        .accessibility_role("group").accessibility_label("content")
}
fn state_schema() { #{ fields: #{ open: #{ schema: #{ type: "bool" },
    "default": #{ type: "bool", value: false } } } } }
fn toggle(ctx, value) { ctx.set_state("open", value); }
fn scene(density, height) {
    let part = collapsible::Collapsible(#{ key: "details", open: true,
        trigger: text("Details"), content: content(), content_height: height });
    column([part, text("after")]).env(#{ density: density })
        .with_style(style().width(px(300)))
}
"#;

fn mount(cx: &mut TestAppContext, view_body: &str) -> (VisualTestContext, ScriptViewHandle) {
    let main = format!(
        r#"
import "components/collapsible" as collapsible;
{SCRIPT}
fn view(ctx) {{ column([{view_body}]).with_style(style().width(px(400)).height(px(400))) }}
"#
    );
    let (mut visual, view) = Mount::components(main)
        .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
        .motion(MotionPreference::None)
        .id("collapsible")
        .mount(cx);
    visual.run_until_parked();
    settle(&mut visual, Step::Refresh, 1);
    (visual, view)
}

/// The layout bounds of the first node with `role` and `name`, (x, y, width, height).
fn layout_of(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> (f64, f64, f64, f64) {
    let g = snapshot(visual, view)
        .find_by_role_and_name(role, name)
        .next()
        .unwrap_or_else(|| panic!("no {role} {name:?}"))
        .geometry
        .unwrap()
        .layout;
    (g.x, g.y, g.width, g.height)
}

/// The content's and the next sibling's layout bounds, (x, y, width, height).
fn layout(cx: &mut TestAppContext, density: &str, height: &str) -> ((f64, f64, f64, f64), f64) {
    let (mut visual, view) = mount(cx, &format!(r#"scene("{density}", {height})"#));
    let content = layout_of(&mut visual, &view, "group", "content");
    let after = layout_of(&mut visual, &view, "text", "after").1;
    eprintln!("{density}, content_height {height}: content {content:?}, next sibling y {after}");
    (content, after)
}

/// The space between the content's bottom and the next sibling.
fn blank_below(cx: &mut TestAppContext, density: &str, height: &str) -> f64 {
    let (content, after) = layout(cx, density, height);
    after - (content.1 + content.3)
}

#[gpui::test]
fn an_explicit_integer_height_is_kept(cx: &mut TestAppContext) {
    // Two metrics.row lines (2 x 32) plus the content's bottom inset (12), comfortable.
    let blank = blank_below(cx, "comfortable", "76");
    assert!(blank.abs() < 0.5, "comfortable: {blank}px blank");
    // The same integer in compact keeps 76px: the caller asked for it.
    let blank = blank_below(cx, "compact", "76");
    assert!((blank - 12.0).abs() < 0.5, "compact: {blank}px blank");
}

#[gpui::test]
fn an_omitted_height_fits_the_content_in_both_densities(cx: &mut TestAppContext) {
    for density in ["comfortable", "compact"] {
        let blank = blank_below(cx, density, "()");
        assert!(blank.abs() < 0.5, "{density}: {blank}px blank");
    }
}

#[gpui::test]
fn a_length_height_follows_density(cx: &mut TestAppContext) {
    for (density, row) in [("comfortable", 32.0), ("compact", 28.0)] {
        let (content, after) = layout(cx, density, r#"theme_length("metrics.row") * 3"#);
        let panel = after - content.1;
        assert!(
            (panel - 3.0 * row).abs() < 0.5,
            "{density}: the panel is {panel}px, three rows are {}px",
            3.0 * row
        );
    }
}

#[gpui::test]
fn a_closed_panel_without_a_height_opens_to_its_content(cx: &mut TestAppContext) {
    let body = r#"column([collapsible::Collapsible(#{ key: "details", open: ctx.get_state("open"),
            trigger: text("Details"), content: content(), on_open_change: Fn("toggle") }),
        text("after")]).env(#{ density: "compact" }).with_style(style().width(px(300)))"#;
    let (mut visual, view) = mount(cx, body);
    let trigger: GeometryBounds = snapshot(&mut visual, &view)
        .nodes()
        .find(|node| node.role == "button")
        .and_then(|node| node.geometry)
        .unwrap()
        .layout;
    let closed = layout_of(&mut visual, &view, "text", "after").1;
    assert!(
        (closed - (trigger.y + trigger.height)).abs() < 0.5,
        "closed: the panel takes no height ({closed})"
    );
    click_center(&mut visual, trigger);
    settle(&mut visual, Step::Frame, 3);
    let content = layout_of(&mut visual, &view, "group", "content");
    let after = layout_of(&mut visual, &view, "text", "after").1;
    assert!(
        (after - (content.1 + content.3)).abs() < 0.5,
        "open: the panel fits the content measured while closed: content {content:?}, after {after}"
    );
}
