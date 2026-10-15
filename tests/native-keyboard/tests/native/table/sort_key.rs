//! Display identity remains the header/callback key while Rust compares ranks.
use crate::support::{Mount, Step, by_role, dispatch, last_error, settle, snapshot, text_of};
use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;
use std::collections::BTreeMap;

#[derive(Clone)]
struct Rows;
impl ScriptViewExtension for Rows {
    fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
        let rows = [
            ("a", "1 h", Some(3600)),
            ("b", "—", None),
            ("c", "<1 m", Some(20)),
        ]
        .into_iter()
        .map(|(id, duration, rank)| {
            let mut row = BTreeMap::from([
                ("id".into(), UiValue::String(id.into())),
                ("duration".into(), UiValue::String(duration.into())),
            ]);
            if let Some(rank) = rank {
                row.insert("seconds".into(), UiValue::Integer(rank));
            }
            row
        });
        runtime
            .native_collections
            .register("sort-rows", NativeCollection::new("id", rows).unwrap())
            .map_err(|error| error.to_string())
    }
}
fn app(native: bool) -> String {
    let rows = if native {
        "ctx.get_native_collection(\"sort-rows\")"
    } else {
        "[#{id:\"a\",duration:\"1 h\",seconds:3600},#{id:\"b\",duration:\"—\"},#{id:\"c\",duration:\"<1 m\",seconds:20}]"
    };
    format!(
        r#"
import "components/table" as table;
fn state_schema(){{#{{fields:#{{direction:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}},
    key:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn sorted(ctx,value){{ctx.set_state("direction",if value==(){{""}}else{{value.direction}});
    ctx.set_state("key",if value==(){{""}}else{{value.key}});}}
fn view(ctx){{let direction=ctx.get_state("direction");column([
    table::Table(#{{key:"jobs",label:"Jobs",row_key:"id",height:180,rows:{rows},resizable_columns:true,
        columns:[#{{key:"duration",title:"Duration",sortable:true,sort_key:"seconds",width:#{{kind:"fixed",value:180}}}}],
        sort:if direction==""{{()}}else{{#{{key:"duration",direction:direction}}}},on_sort_change:Fn("sorted")}}),
    text(`${{ctx.get_state("key")}}|${{direction}}`).test_id("sort-state"),
]).with_style(style().width(px(320)))}}
"#
    )
}
fn durations(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
    let mut rows = snapshot(visual, view)
        .nodes()
        .filter(|node| node.role == "gridcell")
        .map(|node| (node.geometry.unwrap().visual.y, node.name.clone()))
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.0.total_cmp(&right.0));
    rows.into_iter().map(|(_, name)| name).collect()
}

#[gpui::test]
fn array_and_native_headers_cycle_on_display_key_and_keep_missing_ranks_last(
    cx: &mut TestAppContext,
) {
    for native in [false, true] {
        let (mut visual, view) = Mount::components(app(native))
            .with(|view| view.extension(Rows))
            .id(if native {
                "native-sort-key"
            } else {
                "array-sort-key"
            })
            .mount(cx);
        settle(&mut visual, Step::Tick, 2);
        for (state, order) in [
            ("duration|ascending", ["<1 m", "1 h", "—"]),
            ("duration|descending", ["1 h", "<1 m", "—"]),
            ("|", ["1 h", "—", "<1 m"]),
        ] {
            dispatch(
                &mut visual,
                &view,
                by_role("columnheader", "Duration"),
                "click",
            )
            .unwrap();
            settle(&mut visual, Step::Tick, 2);
            assert_eq!(text_of(&mut visual, &view, "sort-state"), state);
            assert_eq!(durations(&mut visual, &view), order);
            assert_eq!(last_error(&mut visual, &view), None);
        }
    }
}
