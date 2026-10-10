//! Input `appearance: "embedded"`: the search line that heads Command, CommandDialog and a
//! searchable Combobox panel. It has no frame or well of its own, spans its container inside
//! the hairline, keeps a 1px `border` line under it while focused, and starts its text on
//! metrics.inset like the rows below.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{Context, IntoElement, Render, TestAppContext, VisualTestContext, Window};
use gpui_rhai::*;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

// Distinct border and focus colors, so the line and any focus frame are easy to find.
const BORDER: u32 = 0xff00_ffff;
const FOCUS: u32 = 0x00ff_00ff;

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::from([(ModuleId::parse("main").unwrap(), source)]);
    for (id, source) in gpui_rhai_registry::BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    let prepared = EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(modules),
        gpui_rhai_registry::DEFAULT_THEME,
    )
    .token_base(gpui_rhai_registry::TOKEN_BASE_SOURCE)
    .theme_token_overrides(ThemeTokenOverrides {
        colors: BTreeMap::from([
            ("border".to_owned(), Rgba8::from_rgba_hex(BORDER)),
            ("focus_ring".to_owned(), Rgba8::from_rgba_hex(FOCUS)),
        ]),
        ..Default::default()
    })
    .motion_preference(MotionPreference::None)
    .asset_sources(
        gpui_rhai_registry::BUNDLED_ASSET_SOURCES
            .iter()
            .map(|(path, source)| {
                (
                    path.trim_end_matches(".svg").to_owned(),
                    AssetData {
                        mime_type: "image/svg+xml".into(),
                        bytes: source.as_bytes().to_vec(),
                    },
                )
            }),
    )
    .prepare()
    .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("embedded", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("embedded"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    for _ in 0..3 {
        visual.run_until_parked();
        visual.executor().advance_clock(Duration::from_millis(32));
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
}

fn layout(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> GeometryBounds {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name(role, name)
            .next()
            .and_then(|node| node.geometry)
            .unwrap_or_else(|| panic!("no {role} {name}"))
            .layout
    })
}

/// Painted quads with a visible border, as logical (x, y, width, height, [top, right,
/// bottom, left]) and the border color.
type Framed = ((f64, f64, f64, f64), [f64; 4], gpui::Hsla);

fn framed_quads(visual: &mut VisualTestContext) -> Vec<Framed> {
    visual.update(|window, _| {
        let scale = f64::from(window.scale_factor());
        let unscale = |value: f32| f64::from(value) / scale;
        window
            .painted_quads()
            .iter()
            .filter(|quad| {
                let edges = quad.border_widths;
                edges.top.0 + edges.right.0 + edges.bottom.0 + edges.left.0 > 0.0
            })
            .map(|quad| {
                let bounds = quad.bounds;
                let edges = quad.border_widths;
                (
                    (
                        unscale(bounds.origin.x.0),
                        unscale(bounds.origin.y.0),
                        unscale(bounds.size.width.0),
                        unscale(bounds.size.height.0),
                    ),
                    [
                        unscale(edges.top.0),
                        unscale(edges.right.0),
                        unscale(edges.bottom.0),
                        unscale(edges.left.0),
                    ],
                    quad.border_color,
                )
            })
            .collect()
    })
}

/// The 1.5px caret of the focused input, as its logical x.
fn caret_x(visual: &mut VisualTestContext) -> Option<f64> {
    visual.update(|window, _| {
        let scale = window.scale_factor();
        window
            .painted_quads()
            .iter()
            .find(|quad| (quad.bounds.size.width.0 / scale - 1.5).abs() < 0.01)
            .map(|quad| f64::from(quad.bounds.origin.x.0 / scale))
    })
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
            .find(|(bounds, _, _)| {
                near(bounds.0, search.x) && near(bounds.1, search.y) && near(bounds.2, search.width)
            })
            .unwrap_or_else(|| panic!("{density}: no line under the search in {quads:?}"));
        assert_eq!(line.1, [0.0, 0.0, 1.0, 0.0], "{density}: search edges");
        assert_eq!(line.2, border, "{density}: search line color");
        // It is focused, and nothing turns the focus color: the caret shows focus.
        assert!(
            quads.iter().all(|(_, _, color)| *color != focus),
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
