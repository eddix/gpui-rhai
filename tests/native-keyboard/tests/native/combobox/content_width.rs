//! A Select without a width fits its widest option (feedback #33): the trigger keeps one
//! width whatever is picked, stops at 360 logical pixels, and its list is at least as wide
//! as the trigger and wider for longer options; the width probe stays out of the
//! accessibility tree.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;

use crate::support::{Mount, by_role, last_error, snapshot};

const LONG: &str = "Claude Opus 5.5 (1M context)";

fn source(value: &str, width: &str, panel_width: &str, open: bool) -> String {
    format!(
        r#"import "components/select" as select;
fn noop(ctx, payload) {{ () }}
fn view(ctx) {{
    let options = [
        #{{ value: "one", label: "One" }},
        #{{ value: "long", label: "{LONG}" }},
    ];
    let wide = [#{{ value: "w", label: "{wide}" }}];
    row([
        select::Select(#{{ key: "model", label: "Model", options: options, value: "{value}",
            open: {open}, query: "", empty_text: "None", {width} panel_width: "{panel_width}", on_change: Fn("noop") }}),
        select::Select(#{{ key: "wide", label: "Wide", options: wide, value: "w",
            open: false, query: "", empty_text: "None", on_change: Fn("noop") }}),
    ]).with_style(style().width(px(1200)).padding(px(20)).gap(px(10)).items_start())
}}"#,
        wide = LONG.repeat(3),
    )
}

fn trigger(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) -> GeometryBounds {
    let result = visual
        .update(|window, cx| {
            view.automate(
                AutomationCommand::Query {
                    locator: by_role("combobox", name),
                },
                window,
                cx,
            )
        })
        .unwrap();
    let AutomationResult::Node { node } = result else {
        panic!("expected the {name} trigger");
    };
    let bounds = node.bounds.unwrap();
    GeometryBounds::new(bounds.x, bounds.y, bounds.width, bounds.height).unwrap()
}

fn mount(
    cx: &mut TestAppContext,
    source: String,
) -> (gpui::WindowHandle<crate::support::Host>, ScriptViewHandle) {
    Mount::components(source)
        .bundled_assets()
        .motion(MotionPreference::None)
        .ids("content-width-window", "content-width-view")
        .mount_window(cx)
}

#[gpui::test]
fn a_select_fits_its_widest_option_whatever_is_picked(cx: &mut TestAppContext) {
    let mut widths = Vec::new();
    for value in ["one", "long"] {
        let (window, view) = mount(cx, source(value, "", "content", false));
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.run_until_parked();
        assert_eq!(last_error(&mut visual, &view), None);
        let model = trigger(&mut visual, &view, "Model");
        let wide = trigger(&mut visual, &view, "Wide");
        assert!(
            model.width > 150.0 && model.width < 340.0,
            "the trigger fits the long option: {model:?}"
        );
        assert!(
            (wide.width - 360.0).abs() < 0.5,
            "a wider option stops at 360: {wide:?}"
        );
        // The probe's labels are not in the tree: one option name, on the open list only.
        let names = snapshot(&mut visual, &view)
            .nodes()
            .filter(|node| node.name == LONG)
            .count();
        assert!(names <= 1, "{names} nodes are named after the long option");
        widths.push(model.width);
    }
    assert!((widths[0] - widths[1]).abs() < 0.5, "{widths:?}");
}

#[gpui::test]
fn a_select_list_is_at_least_its_trigger_and_wider_for_longer_options(cx: &mut TestAppContext) {
    let mut panels = Vec::new();
    for panel_width in ["content", "trigger"] {
        let (window, view) = mount(cx, source("one", "width: px(120),", panel_width, true));
        let host = window.read_with(cx, |root, _| root.host.clone()).unwrap();
        let mut visual = VisualTestContext::from_window(*window, cx);
        visual.run_until_parked();
        assert_eq!(last_error(&mut visual, &view), None);
        let model = trigger(&mut visual, &view, "Model");
        assert!((model.width - 120.0).abs() < 0.5, "{model:?}");
        let panel = host
            .overlay_placement("content-width-view", "model-combobox")
            .unwrap()
            .expect("the open list has a placement")
            .bounds;
        panels.push((panel_width, model.width, panel.width));
    }
    let (_, trigger_width, content) = panels[0];
    let (_, _, matched) = panels[1];
    assert!(content > trigger_width + 100.0, "{panels:?}");
    assert!((matched - trigger_width).abs() <= 1.0, "{panels:?}");
}

/// A column that stretches its children still stretches a content-sized Select, to the cap.
#[gpui::test]
fn a_stretching_column_stretches_a_select_to_its_cap(cx: &mut TestAppContext) {
    let (window, view) = mount(
        cx,
        r#"import "components/select" as select;
fn noop(ctx, payload) { () }
fn view(ctx) {
    column([
        select::Select(#{ key: "short", label: "Short", options: [#{ value: "a", label: "One" }], value: "a",
            open: false, query: "", empty_text: "None", on_change: Fn("noop") }),
    ]).with_style(style().width(px(600)).padding(px(20)))
}"#
        .to_owned(),
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();
    assert_eq!(last_error(&mut visual, &view), None);
    let short = trigger(&mut visual, &view, "Short");
    assert!((short.width - 360.0).abs() < 0.5, "{short:?}");
}
