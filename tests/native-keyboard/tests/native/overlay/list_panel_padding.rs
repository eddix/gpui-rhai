//! The panels of transient lists (Select and Combobox options, Menu, Command) pad every side
//! by what their corner needs: half of `radius.lg`, so the square row blocks clear the
//! rounded corner. The padding is the same on all four sides.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{AccessibilityNode, GeometryBounds, MotionPreference, ScriptViewHandle};
use gpui_rhai_registry::EN_LOCALE;

use crate::support::{Mount, Step, last_error, nodes, settle};

fn mount(cx: &mut TestAppContext, main: &str) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(main)
        .locale("en.rhai", EN_LOCALE)
        .motion(MotionPreference::None)
        .id("panels")
        .mount(cx);
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

fn script(body: &str) -> String {
    format!(
        r#"import "components/select" as select;
import "components/menu" as menu;
import "components/command" as command;
import "components/command_dialog" as command_dialog;
fn view(ctx) {{ column([{body}])
    .with_style(style().width(px(640)).height(px(560)).padding(px(16))) }}"#
    )
}

const SELECT: &str = r#"select::Select(#{ key: "region", label: "Region", open: true, query: "",
    value: "a", options: [#{ label: "i18n-tt", value: "a" }, #{ label: "eu-ttp", value: "b" },
    #{ label: "us-ttp", value: "c" }] })"#;

const MENU: &str = r#"menu::Menu(#{ key: "file", label: "File menu", trigger: text("File"),
    open: true, active_value: "open", items: [#{ kind: "item", label: "Open", value: "open" },
    #{ kind: "item", label: "Save", value: "save" }, #{ kind: "item", label: "Close", value: "close" }] })"#;

const COMMAND: &str = r#"command::Command(#{ key: "palette", label: "Commands", query: "",
    active_value: "deploy", items: [#{ label: "Deploy", value: "deploy" },
    #{ label: "Roll back", value: "rollback" }] })"#;

const COMMAND_DIALOG: &str = r#"command_dialog::CommandDialog(#{ key: "go", label: "Go to",
    open: true, query: "", active_value: "deploy", items: [#{ label: "Deploy", value: "deploy" },
    #{ label: "Roll back", value: "rollback" }] })"#;

/// The visual bounds of the one node with `role` and `name`.
fn bounds(nodes: &[AccessibilityNode], role: &str, name: &str) -> GeometryBounds {
    let found = nodes
        .iter()
        .filter(|node| node.role == role && node.name == name)
        .filter_map(|node| node.geometry.map(|geometry| geometry.visual))
        .collect::<Vec<_>>();
    match found.as_slice() {
        [bounds] => *bounds,
        other => panic!(
            "expected one {role} named {name}, found {other:?} among {:?}",
            nodes
                .iter()
                .map(|node| (node.role.as_str(), node.name.as_str()))
                .collect::<Vec<_>>()
        ),
    }
}

/// Distances from the panel's outer edge to its first and last rows: start, top, end, bottom.
fn insets(panel: GeometryBounds, first: GeometryBounds, last: GeometryBounds) -> [f64; 4] {
    [
        first.x - panel.x,
        first.y - panel.y,
        (panel.x + panel.width) - (first.x + first.width),
        (panel.y + panel.height) - (last.y + last.height),
    ]
}

fn assert_insets(what: &str, actual: [f64; 4], expected: f64) {
    for (side, value) in ["start", "top", "end", "bottom"].iter().zip(actual) {
        assert!(
            (value - expected).abs() < 0.01,
            "{what}: {side} inset is {value}, expected {expected} \
             (all four: {actual:?})"
        );
    }
}

/// The 1px hairline plus half of `radius.lg` (8).
const EXPECTED: f64 = 5.0;

#[gpui::test]
fn select_options_pad_every_side_by_the_corner(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(SELECT).as_str());
    let nodes = nodes(&mut visual, &view);
    let panel = bounds(&nodes, "listbox", "Region");
    let first = bounds(&nodes, "option", "i18n-tt");
    let last = bounds(&nodes, "option", "us-ttp");
    assert_insets("Select", insets(panel, first, last), EXPECTED);
}

#[gpui::test]
fn menu_items_pad_every_side_by_the_corner(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(MENU).as_str());
    let nodes = nodes(&mut visual, &view);
    let panel = bounds(&nodes, "menu", "File menu");
    let first = bounds(&nodes, "menuitem", "Open");
    let last = bounds(&nodes, "menuitem", "Close");
    assert_insets("Menu", insets(panel, first, last), EXPECTED);
}

#[gpui::test]
fn command_pads_its_search_and_rows_by_the_corner(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(COMMAND).as_str());
    let nodes = nodes(&mut visual, &view);
    let panel = bounds(&nodes, "listbox", "Commands");
    let search = bounds(&nodes, "text_field", "Commands");
    let last = bounds(&nodes, "option", "Roll back");
    // The search line heads the panel, so it carries the top inset.
    assert_insets("Command", insets(panel, search, last), EXPECTED);
}

#[gpui::test]
fn command_dialog_pads_its_panel_and_not_the_command_inside(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(COMMAND_DIALOG).as_str());
    let nodes = nodes(&mut visual, &view);
    let panel = bounds(&nodes, "dialog", "Go to");
    let search = bounds(&nodes, "text_field", "Go to");
    let last = bounds(&nodes, "option", "Roll back");
    // The title heads the dialog; the sides and the bottom carry the corner padding once,
    // on the dialog panel, not again on the Command inside it.
    let [start, _, end, bottom] = insets(panel, search, last);
    assert_insets("CommandDialog", [start, EXPECTED, end, bottom], EXPECTED);
}
