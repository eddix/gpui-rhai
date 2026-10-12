//! The panels of transient lists (Select and Combobox options, Menu, Command) pad every side
//! by what their corner needs: half of `radius.lg`, so 0 in the square style, where rows
//! bleed to the hairline, and 2px or 4px in the subtle and round styles, where the square row
//! blocks clear the rounded corner. The padding is the same on all four sides.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME, EN_LOCALE,
    TOKEN_BASE_SOURCE,
};

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn mount(cx: &mut TestAppContext, main: &str) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let entry = ModuleId::parse("main").unwrap();
    let mut modules = BTreeMap::from([(entry.clone(), main.to_owned())]);
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared =
        EmbeddedScriptView::new(entry, EmbeddedScriptSource::new(modules), DEFAULT_THEME)
            .token_base(TOKEN_BASE_SOURCE)
            .locale_sources([("en.rhai".into(), EN_LOCALE.to_owned())])
            .motion_preference(MotionPreference::None)
            .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
                (
                    path.trim_end_matches(".svg").to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }))
            .prepare()
            .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("panels", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("panels"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    for _ in 0..3 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
    assert_eq!(visual.update(|_, cx| view.last_error(cx).unwrap()), None);
    (visual, view)
}

fn script(corners: &str, body: &str) -> String {
    format!(
        r#"import "components/select" as select;
import "components/menu" as menu;
import "components/command" as command;
import "components/command_dialog" as command_dialog;
fn view(ctx) {{ column([{body}]).env(#{{ corners: "{corners}" }})
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

fn nodes(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<AccessibilityNode> {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .cloned()
            .collect()
    })
}

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

fn assert_insets(what: &str, corners: &str, actual: [f64; 4], expected: f64) {
    for (side, value) in ["start", "top", "end", "bottom"].iter().zip(actual) {
        assert!(
            (value - expected).abs() < 0.01,
            "{what} in the {corners} style: {side} inset is {value}, expected {expected} \
             (all four: {actual:?})"
        );
    }
}

/// The 1px hairline plus half of `radius.lg` (square 0, subtle 4, round 8).
const CASES: [(&str, f64); 3] = [("square", 1.0), ("subtle", 3.0), ("round", 5.0)];

#[gpui::test]
fn select_options_pad_every_side_by_the_corner(cx: &mut TestAppContext) {
    for (corners, expected) in CASES {
        let (mut visual, view) = mount(cx, script(corners, SELECT).as_str());
        let nodes = nodes(&mut visual, &view);
        let panel = bounds(&nodes, "listbox", "Region");
        let first = bounds(&nodes, "option", "i18n-tt");
        let last = bounds(&nodes, "option", "us-ttp");
        assert_insets("Select", corners, insets(panel, first, last), expected);
    }
}

#[gpui::test]
fn menu_items_pad_every_side_by_the_corner(cx: &mut TestAppContext) {
    for (corners, expected) in CASES {
        let (mut visual, view) = mount(cx, script(corners, MENU).as_str());
        let nodes = nodes(&mut visual, &view);
        let panel = bounds(&nodes, "menu", "File menu");
        let first = bounds(&nodes, "menuitem", "Open");
        let last = bounds(&nodes, "menuitem", "Close");
        assert_insets("Menu", corners, insets(panel, first, last), expected);
    }
}

#[gpui::test]
fn command_pads_its_search_and_rows_by_the_corner(cx: &mut TestAppContext) {
    for (corners, expected) in CASES {
        let (mut visual, view) = mount(cx, script(corners, COMMAND).as_str());
        let nodes = nodes(&mut visual, &view);
        let panel = bounds(&nodes, "listbox", "Commands");
        let search = bounds(&nodes, "text_field", "Commands");
        let last = bounds(&nodes, "option", "Roll back");
        // The search line heads the panel, so it carries the top inset.
        assert_insets("Command", corners, insets(panel, search, last), expected);
    }
}

#[gpui::test]
fn command_dialog_pads_its_panel_and_not_the_command_inside(cx: &mut TestAppContext) {
    for (corners, expected) in CASES {
        let (mut visual, view) = mount(cx, script(corners, COMMAND_DIALOG).as_str());
        let nodes = nodes(&mut visual, &view);
        let panel = bounds(&nodes, "dialog", "Go to");
        let search = bounds(&nodes, "text_field", "Go to");
        let last = bounds(&nodes, "option", "Roll back");
        // The title heads the dialog; the sides and the bottom carry the corner padding once,
        // on the dialog panel, not again on the Command inside it.
        let [start, _, end, bottom] = insets(panel, search, last);
        assert_insets(
            "CommandDialog",
            corners,
            [start, expected, end, bottom],
            expected,
        );
    }
}
