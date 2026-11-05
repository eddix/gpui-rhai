//! A Node factory renders with the receiving component's `ctx`, but the
//! callbacks in what it returns belong to the scope that bound it: a handler
//! on a node it builds, and the callback props of a component it renders,
//! update the application's state. A `virtual_collection` item renderer may
//! carry a bound factory, so rows the collection realizes later, after a
//! scroll, call back into the application too.

use gpui::{ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::ScriptViewHandle;

use crate::support::{Mount, Step, by_role, by_test_id, dispatch, last_error, settle, text_of};

/// A component that renders an application row factory once per item, in a
/// virtual collection or in a plain column.
const ROWS: &str = r#"/* gpui-rhai
{"id":"components/rows","export":"Rows","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"tokens":[],"environment":[]}
*/
define_component(#{metadata:#{id:"components/rows","export":"Rows",version:"0.1.0",
    runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{},tokens:[],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity."},
        items:#{schema:#{type:"array",items:#{type:"ui_value"}},required:true,sensitive:false,doc:"Rows."},
        row:#{schema:#{type:"callback"},required:true,sensitive:false,doc:"Row factory."},
        virtual:#{schema:#{type:"bool"},required:true,sensitive:false,doc:"Virtualized."},
    },state:#{fields:#{}},events:#{},slots:#{},parts:["root"]},render:Fn("render_Rows")});
fn Rows(props){render_component("components/rows",props)}
fn item_row(row, ctx, payload){
    row([ctx.render_node(row, #{ key: payload.key, item: payload.item })]).with_key(payload.key)
}
fn render_Rows(ctx, props){
    if props.virtual {
        return virtual_collection(#{key:`${props.key}-rows`, label:"Rows", data: props.items,
            estimated_height: 32.0, height: 200.0}, Fn("item_row").curry(props.row)).with_key(props.key);
    }
    let rows = [];
    for item in props.items {
        rows.push(row([ctx.render_node(props.row, #{ key: item.key, item: item })]).with_key(item.key));
    }
    column(rows).with_key(props.key)
}
"#;

fn main(virtual_rows: bool) -> String {
    format!(
        r#"import "components/rows" as rows;
import "components/button" as button;
fn state_schema(){{#{{fields:#{{log:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn play(ctx, key){{ ctx.set_state("log", `${{ctx.get_state("log")}} play:${{key}}`); }}
fn more(key, ctx, payload){{ ctx.set_state("log", `${{ctx.get_state("log")}} more:${{key}}`); }}
fn tracks(){{ let items = []; for i in 0..40 {{ items.push(#{{key:`k${{i}}`,title:`Track ${{i}}`}}); }} items }}
fn track_row(ctx, row){{
    row([
        text(row.item.title),
        text("Play").test_id(`play-${{row.key}}`).on_click_value(Fn("play"), row.key),
        button::Button(#{{ key: `more-${{row.key}}`, text: `More ${{row.key}}`, on_click: Fn("more").curry(row.key) }}),
    ])
}}
fn view(ctx){{ column([
    rows::Rows(#{{ key: "tracks", virtual: {virtual_rows}, row: ctx.node_factory(Fn("track_row")), items: tracks() }}),
    text(ctx.get_state("log")).test_id("log"),
]).with_style(style().width(px(400)))}}"#
    )
}

fn mount(cx: &mut TestAppContext, virtual_rows: bool) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(main(virtual_rows))
        .module("components/rows", ROWS)
        .id("factories")
        .mount(cx);
    settle(&mut visual, Step::Tick, 2);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

/// Clicks the row's own handler and its Button, and returns the log.
fn play_and_more(visual: &mut VisualTestContext, view: &ScriptViewHandle, key: &str) -> String {
    dispatch(visual, view, by_test_id(&format!("play-{key}")), "click").unwrap();
    settle(visual, Step::Tick, 2);
    dispatch(
        visual,
        view,
        by_role("button", &format!("More {key}")),
        "click",
    )
    .unwrap();
    settle(visual, Step::Tick, 2);
    assert_eq!(last_error(visual, view), None);
    text_of(visual, view, "log").trim().to_owned()
}

#[gpui::test]
fn callbacks_in_factory_nodes_update_the_scope_that_bound_the_factory(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, false);
    assert_eq!(play_and_more(&mut visual, &view, "k1"), "play:k1 more:k1");
}

#[gpui::test]
fn a_virtual_collection_renders_a_factory_in_rows_it_realizes_later(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, true);
    assert_eq!(play_and_more(&mut visual, &view, "k1"), "play:k1 more:k1");
    for _ in 0..6 {
        let viewport = visual
            .debug_bounds("virtual-list:tracks-rows")
            .expect("mounted rows");
        visual.simulate_event(ScrollWheelEvent {
            position: point(
                viewport.origin.x + viewport.size.width / 2.0,
                viewport.origin.y + viewport.size.height / 2.0,
            ),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-400.0))),
            ..Default::default()
        });
        settle(&mut visual, Step::Tick, 10);
    }
    assert_eq!(
        play_and_more(&mut visual, &view, "k35"),
        "play:k1 more:k1 play:k35 more:k35"
    );
}
