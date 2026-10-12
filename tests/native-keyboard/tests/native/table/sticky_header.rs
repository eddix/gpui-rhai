//! A sticky group header that the next group header pushes out slides above
//! the table body; it must not paint over or take clicks from the column
//! header row above (#113). Hit testing honors the clip, so a click in the
//! overlap must reach the column header.

use gpui::{Modifiers, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::*;

use crate::support::{Mount, Step, settle, snapshot, text_of};

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
    let (mut visual, view) = Mount::components(MAIN).id("sticky").mount(cx);
    settle(&mut visual, Step::Refresh, 1);
    (visual, view)
}

fn nodes(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> Vec<(String, String, GeometryBounds)> {
    snapshot(visual, view)
        .nodes()
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
    assert_eq!(
        text_of(&mut visual, &view, "event"),
        "sorted",
        "the click reached the column header"
    );
}
