//! Full presets must drive the actual virtual rows, not just decoded Styles.

use gpui::{Modifiers, ScrollDelta, ScrollEvent, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::ThemeTokenOverrides;

use crate::support::{
    Mount, Step, bounds, center, filled, last_error, painted_quads, settle, snapshot, text_of,
    wait_for_view_text,
};

#[gpui::test]
fn preset_table_rows_and_headers_follow_design_and_reading_scale(cx: &mut TestAppContext) {
    const MAIN: &str = r#"
        import "components/table" as table;
        fn view(ctx) {
            let rows = [];
            for index in 0..1000 { rows.push(#{id:`row-${index}`, name:`Entry ${index}`}); }
            table::Table(#{key:"preset",label:"Preset rows",row_key:"id",rows:rows,height:240,
                columns:[#{key:"name",title:"Name",width:#{kind:"flex",value:1}}],
                selection_mode:"single",selected_keys:["row-0"]})
                .with_style(style().width(px(480)))
        }
    "#;
    for (name, source) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for density in ["comfortable", "compact"] {
            for scale in [100, 125, 200] {
                let factor = f64::from(scale) / 100.0;
                let clean = name.starts_with("clean");
                let reduction = if density == "compact" { 4.0 } else { 0.0 };
                let row_height = (if clean { 40.0 } else { 44.0 } - reduction) * factor;
                let header_height = (if clean { 36.0 } else { 44.0 } - reduction) * factor;
                let overrides = ThemeTokenOverrides {
                    environment_defaults: [
                        ("scale".to_owned(), scale.to_string()),
                        ("density".to_owned(), density.to_owned()),
                    ]
                    .into(),
                    ..ThemeTokenOverrides::default()
                };
                let (window, view) = super::mount_refreshed(
                    cx,
                    super::table(MAIN)
                        .theme(*source)
                        .id(&format!(
                            "preset-table-{}-{density}-{scale}",
                            name.trim_end_matches(".rhai")
                        ))
                        .with(move |view| view.theme_token_overrides(overrides)),
                );
                let mut visual = VisualTestContext::from_window(*window, cx);
                wait_for_view_text(&mut visual, &view, "Entry 0", "first row must be presented");
                settle(&mut visual, Step::Frame, 4);
                let tree = snapshot(&mut visual, &view);
                let row = tree
                    .find_by_role_and_name("row", "Preset rows row 1")
                    .next()
                    .unwrap();
                let header = tree
                    .nodes()
                    .find(|node| node.role == "row" && node.row_index == Some(1))
                    .unwrap();
                for (node, expected) in [(row, row_height), (header, header_height)] {
                    let bounds = node.geometry.as_ref().unwrap().layout;
                    assert!(
                        (bounds.height - expected).abs() < 0.5,
                        "{name} scale {scale}: {bounds:?}, expected {expected}"
                    );
                }
                assert_eq!(row.selected, Some(true));
                let viewport = visual.debug_bounds("virtual-list:preset-body").unwrap();
                #[allow(clippy::cast_possible_truncation)]
                visual.simulate_event(ScrollEvent {
                    position: point(
                        viewport.origin.x + viewport.size.width / 2.0,
                        viewport.origin.y + viewport.size.height / 2.0,
                    ),
                    delta: ScrollDelta::Pixels(point(px(0.0), px(-(row_height * 10.0) as f32))),
                    ..ScrollEvent::default()
                });
                settle(&mut visual, Step::Tick, 4);
                let performance = visual
                    .update(|_, cx| view.take_performance_snapshot(cx))
                    .unwrap();
                let collection = performance.virtual_collections.first().unwrap();
                assert_eq!(collection.item_count, 1000);
                assert!(
                    collection.visible_range.contains(&10),
                    "{name} scale {scale}: ten real row heights must reach row 10: {collection:?}"
                );
                assert!(
                    collection.realized_count < 100,
                    "not an eager 1000-row render: {collection:?}"
                );
                assert!(last_error(&mut visual, &view).is_none(), "{name}");
            }
        }
    }
}

#[gpui::test]
fn preset_header_actions_are_inset_centered_and_separate_from_resize_handles(
    cx: &mut TestAppContext,
) {
    for &(name, theme) in gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES {
        for scale in [100, 200] {
            for rtl in [false, true] {
                let factor = f64::from(scale) / 100.0;
                let clean = name.starts_with("clean");
                let main = format!(
                    r#"
                    import "components/table" as table;
                    fn state_schema(){{#{{fields:#{{count:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:0}}}}}}}}}}
                    fn init(ctx){{ctx.set_locale("{}");}}
                    fn sorted(ctx,value){{ctx.set_state("count",ctx.get_state("count")+1);}}
                    fn view(ctx){{column([
                        table::Table(#{{key:"t",label:"Inset headers",row_key:"id",height:140,
                            rows:[#{{id:"a",name:"Alpha",status:"Ready"}}],resizable_columns:true,
                            columns:[#{{key:"name",title:"Name",sortable:true,width:#{{kind:"fixed",value:200}}}},
                                #{{key:"status",title:"Status",width:#{{kind:"fixed",value:180}}}}],
                            on_sort_change:Fn("sorted")}}),text(`${{ctx.get_state("count")}}`).test_id("sort-count")
                    ]).with_style(style().width(px(440)).padding(px(20)))}}
                "#,
                    if rtl { "ar" } else { "en" }
                );
                let overrides = ThemeTokenOverrides {
                    environment_defaults: [("scale".into(), scale.to_string())].into(),
                    ..Default::default()
                };
                let (mut visual, view) = Mount::components(main)
                    .theme(theme)
                    .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
                    .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
                    .with(move |view| view.theme_token_overrides(overrides))
                    .id("table-header-action")
                    .mount(cx);
                settle(&mut visual, Step::Frame, 3);
                let header = bounds(&mut visual, &view, "columnheader", "Name");
                let action = bounds(&mut visual, &view, "button", "Sort Name");
                let cell = bounds(&mut visual, &view, "gridcell", "Ready");
                let grid: gpui::Hsla = visual.update(|_, cx| {
                    gpui::rgba(
                        view.theme_snapshot(cx)
                            .unwrap()
                            .variant
                            .tokens
                            .color("table.grid")
                            .unwrap()
                            .as_rgba_hex(),
                    )
                    .into()
                });
                let cell_quad = painted_quads(&mut visual)
                    .into_iter()
                    .find(|quad| {
                        quad.border_color == grid
                            && (f64::from(quad.bounds.x) - cell.x).abs() < 0.5
                            && (f64::from(quad.bounds.y) - cell.y).abs() < 0.5
                            && (f64::from(quad.bounds.width) - cell.width).abs() < 0.5
                    })
                    .expect("the actual cell grid must paint");
                assert!(
                    (f64::from(cell_quad.border_widths[0]) - if clean { 0.5 } else { 1.0 }).abs()
                        < 0.01,
                    "{name}/{scale}/{rtl}: reading scale must not thicken the hairline: {cell_quad:?}"
                );
                assert!(
                    (f64::from(cell_quad.border_widths[if rtl { 1 } else { 3 }])
                        - if clean { 0.0 } else { 1.0 })
                    .abs()
                        < 0.01
                );
                let height = (if clean { 28.0 } else { 32.0 }) * factor;
                assert!(
                    (action.height - height).abs() < 0.5,
                    "{name}/{scale}/{rtl}: {action:?}"
                );
                assert!(
                    (action.x - header.x - 6.0 * factor).abs() < 0.5,
                    "{name}/{scale}/{rtl}: {header:?} / {action:?}"
                );
                assert!((action.width - header.width + 12.0 * factor).abs() < 0.5);
                assert!(
                    (action.y + action.height / 2.0 - header.y - header.height / 2.0).abs() < 0.5
                );
                let handle = bounds(
                    &mut visual,
                    &view,
                    "separator",
                    if rtl {
                        "تغيير حجم العمود Name"
                    } else {
                        "Resize Name column"
                    },
                );
                assert!(
                    handle.x + handle.width <= action.x || handle.x >= action.x + action.width,
                    "{name}/{scale}/{rtl}: resize handle must not cover sort action {handle:?}/{action:?}"
                );
                let ink = visual.update(|_, cx| {
                    view.theme_snapshot(cx)
                        .unwrap()
                        .variant
                        .tokens
                        .color("table.header_hover")
                        .unwrap()
                        .as_rgba_hex()
                });
                visual.simulate_event(gpui::MouseMoveEvent {
                    position: center(action),
                    ..Default::default()
                });
                settle(&mut visual, Step::Frame, 2);
                assert!(
                    filled(&mut visual, ink)
                        .iter()
                        .any(|quad| (f64::from(quad.bounds.x) - action.x).abs() < 0.5
                            && (f64::from(quad.bounds.y) - action.y).abs() < 0.5
                            && (f64::from(quad.bounds.height) - action.height).abs() < 0.5),
                    "{name}/{scale}/{rtl}: only the inset button owns hover"
                );
                #[allow(clippy::cast_possible_truncation)]
                let inert_padding = point(
                    px((header.x + header.width / 2.0) as f32),
                    px((header.y + 1.0) as f32),
                );
                visual.simulate_click(inert_padding, Modifiers::none());
                assert_eq!(text_of(&mut visual, &view, "sort-count"), "0");
                visual.simulate_click(center(action), Modifiers::none());
                settle(&mut visual, Step::Frame, 2);
                assert_eq!(text_of(&mut visual, &view, "sort-count"), "1");
                visual.simulate_keystrokes("enter");
                settle(&mut visual, Step::Frame, 2);
                assert_eq!(
                    text_of(&mut visual, &view, "sort-count"),
                    "2",
                    "the actual sort button owns keyboard activation"
                );
                assert!(last_error(&mut visual, &view).is_none());
            }
        }
    }
}
