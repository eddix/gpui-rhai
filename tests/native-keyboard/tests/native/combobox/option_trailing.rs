//! Select and Combobox draw caller content after an option's label (feedback #4): a Node
//! factory, curried with the caller's data, is called for each realized row and beside a
//! single-mode trigger's value; a content-sized trigger measures it too.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;

use crate::support::{Mount, last_error, nodes};

fn source(trailing: bool, open: bool) -> String {
    let factory = if trailing {
        r#"option_trailing: ctx.node_factory(Fn("load").curry(loads)),"#
    } else {
        ""
    };
    format!(
        r#"import "components/select" as select;
fn noop(ctx, payload) {{ () }}
fn load(loads, ctx, option) {{
    text(`load ${{loads[option.value]}}% (${{option.place}})`)
}}
fn view(ctx) {{
    let loads = #{{ alpha: 43, beta: 7 }};
    row([
        select::Select(#{{ key: "host", label: "Host", value: "alpha", open: {open}, query: "",
            options: [#{{ value: "alpha", label: "Alpha" }}, #{{ value: "beta", label: "Beta" }}],
            empty_text: "None", {factory} on_change: Fn("noop") }}),
    ]).with_style(style().width(px(900)).padding(px(20)).items_start())
}}"#
    )
}

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    let (visual, view) = Mount::components(source)
        .bundled_assets()
        .motion(MotionPreference::None)
        .id("trailing")
        .mount(cx);
    visual.run_until_parked();
    (visual, view)
}

fn named(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    name: &str,
) -> Vec<AccessibilityNode> {
    nodes(visual, view)
        .into_iter()
        .filter(|node| node.name == name)
        .collect()
}

#[gpui::test]
fn trailing_content_follows_each_option_and_the_trigger_value(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, source(true, true));
    assert_eq!(last_error(&mut visual, &view), None);
    for (label, trailing) in [("Alpha", "load 43% (option)"), ("Beta", "load 7% (option)")] {
        let option = named(&mut visual, &view, label)
            .into_iter()
            .find(|node| node.role == "option")
            .unwrap_or_else(|| panic!("no {label} option"));
        let extra = named(&mut visual, &view, trailing);
        assert_eq!(extra.len(), 1, "{trailing}: {extra:?}");
        let (row, extra) = (
            option.geometry.unwrap().visual,
            extra[0].geometry.unwrap().visual,
        );
        assert!(
            extra.y >= row.y && extra.y + extra.height <= row.y + row.height + 0.5,
            "{trailing} sits in the {label} row: {extra:?} {row:?}"
        );
        assert!(
            extra.x > row.x + row.width / 3.0,
            "after the label: {extra:?} {row:?}"
        );
    }
    // Beside the selected value on the trigger; the width probe's copies are not in the tree.
    assert_eq!(named(&mut visual, &view, "load 43% (trigger)").len(), 1);
}

#[gpui::test]
fn a_content_sized_trigger_measures_the_trailing_content(cx: &mut TestAppContext) {
    let mut widths = Vec::new();
    for trailing in [false, true] {
        let (mut visual, view) = mount(cx, source(trailing, false));
        assert_eq!(last_error(&mut visual, &view), None);
        let trigger = nodes(&mut visual, &view)
            .into_iter()
            .find(|node| node.role == "combobox")
            .unwrap();
        widths.push(trigger.geometry.unwrap().visual.width);
    }
    assert!(widths[1] > widths[0] + 40.0, "{widths:?}");
}
