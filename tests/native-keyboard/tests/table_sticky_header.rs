//! A sticky group header that the next group header pushes out slides above
//! the table body; it must not paint over or take clicks from the column
//! header row above (#113). Hit testing honors the clip, so a click in the
//! overlap must reach the column header.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use gpui::{
    Context, IntoElement, Modifiers, Render, TestAppContext, VisualTestContext, Window, point, px,
};
use gpui_rhai::*;
use gpui_rhai_registry::{
    BUNDLED_ASSET_SOURCES, BUNDLED_COMPONENT_SOURCES_BY_ID, DEFAULT_THEME, TOKEN_BASE_SOURCE,
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

const MAIN: &str = r#"
import "components/table" as table;
fn state_schema() { #{ fields: #{
    event: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "none" } } } } }
fn sorted(ctx, sort) { ctx.set_state("event", "sorted"); }
fn toggled(ctx, group) { ctx.set_state("event", `toggled ${group}`); }
fn view(ctx) {
    let rows = [];
    for g in ["A", "B", "C"] {
        for i in 0..8 { rows.push(#{ id: `${g}-${i}`, g: g, name: `${g} row ${i}` }); }
    }
    column([
        text(ctx.get_state("event")).test_id("event"),
        table::Table(#{ key: "t", label: "Grouped", row_key: "id", rows: rows,
            columns: [#{ key: "name", title: "Name", sortable: true, width: #{ kind: "flex", value: 1.0 } },
                #{ key: "g", title: "Group", width: #{ kind: "fixed", value: 80.0 } }],
            group_by: "g", sticky_group_headers: true, height: 200.0,
            on_sort_change: Fn("sorted"), on_group_toggle: Fn("toggled") }),
    ]).with_style(style().width(px(400)).padding(px(40)))
}
"#;

fn mount(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let mut modules = BTreeMap::new();
    for (id, source) in BUNDLED_COMPONENT_SOURCES_BY_ID {
        modules.insert(ModuleId::parse(*id).unwrap(), (*source).to_owned());
    }
    modules.insert(ModuleId::parse("main").unwrap(), MAIN.to_owned());
    let prepared = EmbeddedScriptView::new(
        ModuleId::parse("main").unwrap(),
        EmbeddedScriptSource::new(modules),
        DEFAULT_THEME,
    )
    .token_base(TOKEN_BASE_SOURCE)
    .asset_sources(BUNDLED_ASSET_SOURCES.iter().map(|(path, source)| {
        (
            path.strip_suffix(".svg").unwrap_or(path).to_owned(),
            AssetData {
                mime_type: "image/svg+xml".to_owned(),
                bytes: source.as_bytes().to_vec(),
            },
        )
    }))
    .prepare()
    .unwrap();
    let saved = Rc::new(RefCell::new(None));
    let save = saved.clone();
    let window = cx.add_window(move |window, cx| {
        let host = ScriptViewHost::new("sticky", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("sticky"), host.clone(), window, cx)
            .unwrap();
        *save.borrow_mut() = Some(view.clone());
        Host { host, view }
    });
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    let view = saved.borrow().clone().unwrap();
    (visual, view)
}

fn nodes(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> Vec<(String, String, GeometryBounds)> {
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    tree.nodes()
        .filter_map(|node| Some((node.role.clone(), node.name.clone(), node.geometry?.visual)))
        .collect()
}

#[gpui::test]
fn a_pushed_sticky_header_does_not_cover_the_column_header(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx);
    let before = nodes(&mut visual, &view);
    let column = before
        .iter()
        .find(|(role, name, _)| role == "columnheader" && name.starts_with("Name"))
        .map(|(_, _, bounds)| *bounds)
        .expect("Name column header");
    let first_group = before
        .iter()
        .find(|(role, name, _)| role == "rowheader" && name.starts_with('A'))
        .map(|(_, _, bounds)| *bounds)
        .expect("group A header");
    // Scroll until group B's header sits 10px below the body top, so the sticky
    // A header is pushed up into the column header row.
    let body_top = first_group.y;
    for _ in 0..20 {
        let b_top = nodes(&mut visual, &view)
            .iter()
            .find(|(role, name, _)| role == "rowheader" && name.starts_with('B'))
            .map_or(body_top + 1000.0, |(_, _, bounds)| bounds.y);
        let remaining = b_top - (body_top + 10.0);
        if remaining.abs() < 0.5 {
            break;
        }
        #[allow(clippy::cast_possible_truncation)]
        let step = remaining.min(first_group.height * 4.0) as f32;
        visual.simulate_event(gpui::ScrollWheelEvent {
            position: point(
                px((first_group.x + 20.0) as f32),
                px((body_top + 60.0) as f32),
            ),
            delta: gpui::ScrollDelta::Pixels(point(px(0.0), px(-step))),
            ..gpui::ScrollWheelEvent::default()
        });
        visual.run_until_parked();
        visual.update(|window, cx| window.simulate_next_frame(cx));
        visual.run_until_parked();
    }
    let sticky = nodes(&mut visual, &view)
        .into_iter()
        .find(|(role, name, _)| role == "rowheader" && name.starts_with('A'))
        .map(|(_, _, bounds)| bounds)
        .expect("sticky A header");
    let column_bottom = column.y + column.height;
    assert!(
        sticky.y < column_bottom - 5.0,
        "the setup pushes the sticky header into the column header row: {sticky:?} vs {column:?}"
    );
    // Click inside the overlap, on the Name column header.
    #[allow(clippy::cast_possible_truncation)]
    let at = point(
        px((column.x + 24.0) as f32),
        px((column_bottom - 4.0) as f32),
    );
    visual.simulate_click(at, Modifiers::default());
    visual.run_until_parked();
    let tree = visual.update(|_, cx| view.accessibility_snapshot(cx).unwrap());
    let event = tree
        .nodes()
        .find(|node| node.test_id.as_deref() == Some("event"))
        .map(|node| node.name.clone());
    assert_eq!(
        event.as_deref(),
        Some("sorted"),
        "the click reached the column header"
    );
}
