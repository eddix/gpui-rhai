//! Formal preset ownership survives structural item identities and live themes.
use crate::support::{Mount, Step, bounds, filled, last_error, settle, wait_for_view_text};
use gpui::{ScrollDelta, ScrollEvent, TestAppContext, point, px};
use gpui_rhai::MotionPreference;

const SOURCE: &str = r#"
    import "components/table" as table;
    import "components/list" as list;
    fn state_schema(){#{fields:#{selected:#{schema:#{type:"array",items:#{type:"string"}},"default":["r0"]}}}}
    fn changed(ctx,value){ctx.set_state("selected",value);}
    fn view(ctx){let rows=[];let items=[];
        for i in 0..100{rows.push(#{id:`r${i}`,value:`Entry ${i}`});items.push(#{key:`r${i}`,title:`Entry ${i}`});}
        column([
            table::Table(#{key:"table",label:"Rows",row_key:"id",rows:rows,height:180,
                columns:[#{key:"value",title:"Value",width:#{kind:"flex",value:1}}],
                selected_keys:ctx.get_state("selected"),selection_mode:"single",on_selection_change:Fn("changed")}),
            list::List(#{key:"list",label:"Items",items:items,height:180,
                selected_keys:ctx.get_state("selected"),selection_mode:"single",on_selection_change:Fn("changed")})
        ]).with_style(style().width(px(500)).gap(px(12)))
    }
"#;

fn themed(source: &str, color: u32) -> String {
    format!(
        r#"{}
        fn theme(){{let t=original_preset();
            for id in ["components/table","components/list"] {{
                t.tokens.component_styles[id].row=style().background(rgba({color}));
                t.tokens.component_styles[id].row_selected=style().background(rgba({color}));
            }}t}}
    "#,
        source.replace("fn theme()", "fn original_preset()")
    )
}

#[gpui::test]
fn virtual_rows_keep_preset_paint_when_realized_scrolled_and_rethemed(cx: &mut TestAppContext) {
    let themes = gpui_rhai_registry::BUNDLED_DESIGN_PRESET_SOURCES
        .iter()
        .enumerate()
        .map(|(index, &(file, source))| {
            (
                file.to_owned(),
                themed(
                    source,
                    [0x18c8a6ff, 0xa618c8ff, 0xc8a618ff, 0x186ac8ff][index],
                ),
            )
        })
        .collect::<Vec<_>>();
    let (mut visual, view) = Mount::components(SOURCE)
        .theme(themes[0].1.clone())
        .motion(MotionPreference::None)
        .with(move |view| view.theme_sources(themes.into_iter().skip(1)))
        .mount(cx);
    wait_for_view_text(&mut visual, &view, "Entry 1", "initial rows presented");
    for (family, appearance, color) in [
        ("Modern", "Light", 0x18c8a6ff),
        ("Modern", "Dark", 0xa618c8ff),
        ("Clean", "Light", 0xc8a618ff),
        ("Clean", "Dark", 0x186ac8ff),
    ] {
        visual
            .update(|_, cx| view.select_theme(family, appearance, cx))
            .unwrap();
        settle(&mut visual, Step::Frame, 4);
        for (id, role, name) in [
            ("virtual-list:table-body", "table", "Rows"),
            ("virtual-list:list-rows", "listbox", "Items"),
        ] {
            let area = bounds(&mut visual, &view, role, name);
            let variant = visual.update(|_, cx| view.theme_snapshot(cx).unwrap().variant);
            let part = variant
                .tokens
                .component_styles
                .component(
                    &gpui_rhai::ModuleId::parse(if role == "table" {
                        "components/table"
                    } else {
                        "components/list"
                    })
                    .unwrap(),
                )
                .unwrap();
            assert!(
                filled(&mut visual, color)
                    .iter()
                    .any(|quad| f64::from(quad.bounds.y) >= area.y
                        && f64::from(quad.bounds.y) < area.y + area.height),
                "{family}/{appearance}/{name}: this factory must keep its preset; actual theme {}/{}; row {:?}; fills {:?}",
                variant.family,
                variant.name,
                part["row"].base.background,
                crate::support::painted_quads(&mut visual)
                    .iter()
                    .filter(|quad| f64::from(quad.bounds.y) >= area.y
                        && f64::from(quad.bounds.y) < area.y + area.height)
                    .map(|quad| quad.background)
                    .collect::<Vec<_>>()
            );
            let viewport = visual.debug_bounds(id).unwrap();
            visual.simulate_event(ScrollEvent {
                position: point(viewport.origin.x + px(40.), viewport.origin.y + px(40.)),
                delta: ScrollDelta::Pixels(point(px(0.), px(-180.))),
                ..Default::default()
            });
            settle(&mut visual, Step::Tick, 3);
            assert!(
                filled(&mut visual, color)
                    .iter()
                    .any(|quad| f64::from(quad.bounds.y) >= area.y
                        && f64::from(quad.bounds.y) < area.y + area.height),
                "{family}/{appearance}/{name}: newly realized rows keep preset paint"
            );
        }
        assert_eq!(last_error(&mut visual, &view), None);
    }
}
