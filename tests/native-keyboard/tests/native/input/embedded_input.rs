//! Input `appearance: "embedded"`: the search line that heads Command, CommandDialog and a
//! searchable Combobox panel. It has no frame or well of its own, spans its container inside
//! the hairline, keeps a 1px `border` line under it while focused, and starts its text on
//! metrics.inset like the rows below.

use std::collections::BTreeMap;

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{GeometryBounds, MotionPreference, Rgba8, ScriptViewHandle, ThemeTokenOverrides};

use crate::support::{Mount, PaintedQuad, Step, painted_quads, settle, snapshot};

// Distinct border and focus colors, so the line and any focus frame are easy to find.
const BORDER: u32 = 0xff00_ffff;
const FOCUS: u32 = 0x00ff_00ff;

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(source)
        .motion(MotionPreference::None)
        .with(|view| {
            view.theme_token_overrides(ThemeTokenOverrides {
                colors: BTreeMap::from([
                    ("border".to_owned(), Rgba8::from_rgba_hex(BORDER)),
                    ("focus_ring".to_owned(), Rgba8::from_rgba_hex(FOCUS)),
                ]),
                ..Default::default()
            })
        })
        .id("embedded")
        .mount(cx);
    settle(&mut visual, Step::Tick, 3);
    (visual, view)
}

fn layout(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    snapshot(visual, view)
        .find_by_role_and_name(role, name)
        .next()
        .and_then(|node| node.geometry)
        .unwrap_or_else(|| panic!("no {role} {name}"))
        .layout
}

/// Painted quads with a visible border.
fn framed_quads(visual: &mut VisualTestContext) -> Vec<PaintedQuad> {
    painted_quads(visual)
        .into_iter()
        .filter(PaintedQuad::has_border)
        .collect()
}

/// The 1.5px caret of the focused input, as its logical x.
fn caret_x(visual: &mut VisualTestContext) -> Option<f64> {
    painted_quads(visual)
        .iter()
        .find(|quad| (quad.bounds.width - 1.5).abs() < 0.01)
        .map(|quad| f64::from(quad.bounds.x))
}

fn near(left: f64, right: f64) -> bool {
    (left - right).abs() < 0.5
}

fn command_script(density: &str) -> String {
    format!(
        r#"import "components/command" as command;
fn view(ctx) {{ column([
    command::Command(#{{ key: "palette", label: "Commands", query: "", active_value: "",
        autofocus: true, items: [#{{ value: "open", label: "Open" }}, #{{ value: "save", label: "Save" }}] }})
]).with_style(style().width(px(320)).padding(px(10))).env(#{{ density: "{density}" }}) }}"#
    )
}

#[gpui::test]
fn the_command_search_spans_the_panel_on_the_rows_inset(cx: &mut TestAppContext) {
    let border: gpui::Hsla = gpui::rgba(BORDER).into();
    let focus: gpui::Hsla = gpui::rgba(FOCUS).into();
    for (density, inset, height) in [("comfortable", 12.0, 36.0), ("compact", 8.0, 32.0)] {
        let (mut visual, view) = mount(cx, command_script(density));
        let panel = layout(&mut visual, &view, "listbox", "Commands");
        let search = layout(&mut visual, &view, "text_field", "Commands");
        // Full width inside the panel's 1px hairline, flush with its top, one size up.
        assert!(
            near(search.x, panel.x + 1.0) && near(search.width, panel.width - 2.0),
            "{density}: search {search:?} in panel {panel:?}"
        );
        assert!(
            near(search.y, panel.y + 1.0),
            "{density}: search {search:?} in panel {panel:?}"
        );
        assert!(
            near(search.height, height),
            "{density}: search height {}",
            search.height
        );
        let quads = framed_quads(&mut visual);
        // The search paints only a 1px bottom line in `border`, no frame.
        let line = quads
            .iter()
            .find(|quad| {
                near(quad.bounds.x.into(), search.x)
                    && near(quad.bounds.y.into(), search.y)
                    && near(quad.bounds.width.into(), search.width)
            })
            .unwrap_or_else(|| panic!("{density}: no line under the search in {quads:?}"));
        assert_eq!(
            line.border_widths,
            [0.0, 0.0, 1.0, 0.0],
            "{density}: search edges"
        );
        assert_eq!(line.border_color, border, "{density}: search line color");
        // It is focused, and nothing turns the focus color: the caret shows focus.
        assert!(
            quads.iter().all(|quad| quad.border_color != focus),
            "{density}: a focus color was painted: {quads:?}"
        );
        let caret = caret_x(&mut visual).expect("the autofocused search draws its caret");
        assert!(
            near(caret - search.x, inset),
            "{density}: text starts {} into the search, expected {inset}",
            caret - search.x
        );
    }
}

#[gpui::test]
fn the_dialog_panel_gives_the_command_its_whole_width(cx: &mut TestAppContext) {
    let script = r#"import "components/command_dialog" as command_dialog;
fn view(ctx) { command_dialog::CommandDialog(#{ key: "palette", open: true, query: "", active_value: "",
    label: "Commands", title_visible: false, items: [#{ value: "open", label: "Open" }] }) }"#;
    let (mut visual, view) = mount(cx, script.to_owned());
    let dialog = layout(&mut visual, &view, "dialog", "Commands");
    let search = layout(&mut visual, &view, "text_field", "Commands");
    assert!(
        near(search.x, dialog.x + 1.0)
            && near(search.y, dialog.y + 1.0)
            && near(search.width, dialog.width - 2.0),
        "search {search:?} in dialog {dialog:?}"
    );
}

#[gpui::test]
fn a_searchable_combobox_panel_starts_with_its_search(cx: &mut TestAppContext) {
    let script = r#"import "components/combobox" as combobox;
fn view(ctx) { column([combobox::Combobox(#{ key: "regions", label: "Regions",
    options: [#{ value: "us-east", label: "us-east" }, #{ value: "eu-west", label: "eu-west" }],
    selected: [], open: true, query: "", searchable: true, width: px(240) })])
    .with_style(style().padding(px(10)).width(px(400)).height(px(300))) }"#;
    let (mut visual, view) = mount(cx, script.to_owned());
    let panel = layout(&mut visual, &view, "listbox", "Regions");
    let search = layout(&mut visual, &view, "text_field", "Regions");
    assert!(
        near(search.x, panel.x + 1.0)
            && near(search.y, panel.y + 1.0)
            && near(search.width, panel.width - 2.0),
        "search {search:?} in panel {panel:?}"
    );
}
