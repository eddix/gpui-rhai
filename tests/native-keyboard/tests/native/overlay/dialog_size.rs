//! A Dialog's size (feedback #7): `width` sets the panel's width, and a panel taller than
//! 90% of the window stops there, its body scrolling under a title and above actions that
//! stay in view.

use gpui::{ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{GeometryBounds, MotionPreference, ScriptViewHandle};

use crate::support::{Mount, Step, last_error, nodes};

fn mount(cx: &mut TestAppContext, dialog: &str) -> (VisualTestContext, ScriptViewHandle) {
    let main = format!(
        r#"import "components/button" as button;
import "components/dialog" as dialog;
fn noop(ctx, payload) {{ () }}
fn lines(count) {{
    let rows = [];
    for index in 1..=count {{ rows.push(text(`Line ${{index}}`)); }}
    column(rows)
}}
fn view(ctx) {{ column([{dialog}]).with_style(style().width(px(800)).height(px(600))) }}"#
    );
    let (mut visual, view) = Mount::components(main)
        .motion(MotionPreference::None)
        .id("dialog-size")
        .mount(cx);
    crate::support::settle(&mut visual, Step::Frame, 2);
    (visual, view)
}

fn geometry(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    nodes(visual, view)
        .into_iter()
        .find(|node| node.role == role && node.name == name)
        .and_then(|node| node.geometry)
        .unwrap_or_else(|| panic!("no {role} {name}"))
        .visual
}

#[gpui::test]
fn a_dialog_takes_its_width(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"dialog::Dialog(#{ key: "wide", open: true, title: "Wide", width: px(600),
            content: text("Continue?") })"#,
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let panel = geometry(&mut visual, &view, "dialog", "Wide");
    assert!((panel.width - 600.0).abs() < 0.5, "{panel:?}");
}

#[gpui::test]
fn a_tall_dialog_stops_at_ninety_percent_and_scrolls_its_body(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"dialog::Dialog(#{ key: "tall", open: true, title: "Tall", content: lines(80),
            actions: [button::Button(#{ key: "ok", text: "OK", on_click: Fn("noop") })] })"#,
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let viewport = visual.update(|window, _| f64::from(f32::from(window.viewport_size().height)));
    let panel = geometry(&mut visual, &view, "dialog", "Tall");
    assert!(
        panel.height <= viewport * 0.9 + 1.0 && panel.height > viewport * 0.5,
        "{panel:?} in a {viewport} window"
    );
    // The title and the action stay inside the panel; the body's last line does not.
    let title = geometry(&mut visual, &view, "text", "Tall");
    let ok = geometry(&mut visual, &view, "button", "OK");
    let bottom = panel.y + panel.height;
    assert!(
        title.y >= panel.y && ok.y + ok.height <= bottom + 0.5,
        "{title:?} {ok:?} {panel:?}"
    );
    let last = geometry(&mut visual, &view, "text", "Line 80");
    assert!(
        last.y > bottom,
        "the body overflows into its scroll: {last:?} {panel:?}"
    );

    // A wheel over the body scrolls it.
    let first = geometry(&mut visual, &view, "text", "Line 1");
    #[allow(clippy::cast_possible_truncation)]
    let position = point(px((first.x + 10.0) as f32), px((first.y + 40.0) as f32));
    visual.simulate_event(ScrollEvent {
        position,
        delta: ScrollDelta::Pixels(point(px(0.), px(-200.))),
        ..Default::default()
    });
    crate::support::settle(&mut visual, Step::Frame, 2);
    let moved = geometry(&mut visual, &view, "text", "Line 80");
    assert!(moved.y < last.y - 100.0, "{last:?} -> {moved:?}");
    let ok_after = geometry(&mut visual, &view, "button", "OK");
    assert!(
        (ok_after.y - ok.y).abs() < 0.5,
        "the actions stay: {ok:?} -> {ok_after:?}"
    );
}
