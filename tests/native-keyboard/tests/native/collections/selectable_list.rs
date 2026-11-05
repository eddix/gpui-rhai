//! SelectableList (#217): rows the application draws with List's behavior. A click selects
//! and opens a row, the list is one tab stop whose arrows move the selection and skip
//! disabled rows, Enter opens the current row, a right press asks for a context menu, the
//! rows are options named by their label field, a control in a row calls back into the
//! application and keeps its own tab stop, and rows realized after a scroll behave the same.

use gpui::{
    Modifiers, MouseButton, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext,
    point, px,
};
use gpui_rhai::*;

use crate::support::{
    Mount, Step, bounds, by_role, center, dispatch, last_error, settle, snapshot, text_of,
};

fn script(extra: &str) -> String {
    format!(
        r#"import "components/selectable_list" as selectable_list;
import "components/button" as button;
fn state_schema(){{#{{fields:#{{
    selected:#{{schema:#{{type:"array",items:#{{type:"string"}}}},"default":#{{type:"array",value:[]}}}},
    log:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn note(ctx,line){{ctx.set_state("log",`${{ctx.get_state("log")}} ${{line}}`);}}
fn selected(ctx,keys){{ctx.set_state("selected",keys);note(ctx,`sel:${{keys}}`);}}
fn opened(ctx,key){{note(ctx,`open:${{key}}`);}}
fn asked(ctx,request){{note(ctx,`ctx:${{request.key}}:${{request.source}}`);}}
fn play(key,ctx,payload){{note(ctx,`play:${{key}}`);}}
fn tracks(){{
    let items=[];
    for i in 0..60 {{
        items.push(#{{key:`t${{i}}`,label:`Track ${{i}}`,title:`Track ${{i}}`,artist:`Artist ${{i % 7}}`,disabled:i==2}});
    }}
    items
}}
fn track_row(ctx,row){{
    row([
        box([]).with_style(style().width(px(40)).height(px(40)).background(theme_color("surface_hover"))),
        column([text(row.item.title),text(row.item.artist)]).with_style(style().flex_grow().min_width(px(0))),
        button::Button(#{{key:`play-${{row.key}}`,text:`Play ${{row.key}}`,size:"sm",on_click:Fn("play").curry(row.key)}}),
    ]).with_style(style().gap(px(8)).items_center())
}}
fn view(ctx){{column([
    selectable_list::SelectableList(#{{key:"tracks",label:"Tracks",items:tracks(),selection_mode:"single",
        selected_keys:ctx.get_state("selected"),row:ctx.node_factory(Fn("track_row")),height:320.0,
        on_selection_change:Fn("selected"),on_row_click:Fn("opened"),on_context_request:Fn("asked")
        {extra}}}),
    text(ctx.get_state("log")).test_id("log"),
]).with_style(style().width(px(480)))}}"#
    )
}

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(source)
        .motion(MotionPreference::None)
        .id("selectable")
        .mount(cx);
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    text_of(visual, view, "log").trim().to_owned()
}

fn keys(visual: &mut VisualTestContext, keystrokes: &str) {
    visual.simulate_keystrokes(keystrokes);
    settle(visual, Step::Tick, 3);
}

#[gpui::test]
fn rows_select_open_and_move_with_the_keys(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(""));
    let row = bounds(&mut visual, &view, "option", "Track 1");
    assert!(
        row.height >= 40.0,
        "a row takes its content's height: {row:?}"
    );
    visual.simulate_click(center(row), Modifiers::none());
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(log(&mut visual, &view), "open:t1 sel:[\"t1\"]");
    // Down skips the disabled third row; Enter opens the current one.
    keys(&mut visual, "down");
    keys(&mut visual, "enter");
    assert_eq!(
        log(&mut visual, &view),
        "open:t1 sel:[\"t1\"] sel:[\"t3\"] open:t3"
    );
    let selected: Vec<_> = snapshot(&mut visual, &view)
        .nodes()
        .filter(|node| node.role == "option" && node.selected == Some(true))
        .map(|node| node.name.clone())
        .collect();
    assert_eq!(selected, vec!["Track 3"]);
    keys(&mut visual, "home");
    assert!(log(&mut visual, &view).ends_with("sel:[\"t0\"]"));
}

#[gpui::test]
fn a_control_in_a_row_calls_back_into_the_application(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(""));
    dispatch(&mut visual, &view, by_role("button", "Play t4"), "click").unwrap();
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(log(&mut visual, &view), "play:t4");
    // Rows realized after a scroll are the same.
    let viewport = visual
        .debug_bounds("virtual-list:tracks-rows")
        .expect("mounted rows");
    for _ in 0..8 {
        visual.simulate_event(ScrollWheelEvent {
            position: point(
                viewport.origin.x + viewport.size.width / 2.0,
                viewport.origin.y + viewport.size.height / 2.0,
            ),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-400.0))),
            ..Default::default()
        });
        settle(&mut visual, Step::Tick, 5);
    }
    dispatch(&mut visual, &view, by_role("button", "Play t55"), "click").unwrap();
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(log(&mut visual, &view), "play:t4 play:t55");
    let late = bounds(&mut visual, &view, "option", "Track 55");
    visual.simulate_click(center(late), Modifiers::none());
    settle(&mut visual, Step::Tick, 3);
    assert!(log(&mut visual, &view).ends_with("open:t55 sel:[\"t55\"]"));
}

#[gpui::test]
fn a_right_press_asks_for_a_context_menu_after_selecting_the_row(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(""));
    let row = center(bounds(&mut visual, &view, "option", "Track 3"));
    visual.simulate_mouse_move(row, None, Modifiers::none());
    visual.simulate_mouse_down(row, MouseButton::Right, Modifiers::none());
    visual.simulate_mouse_up(row, MouseButton::Right, Modifiers::none());
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(log(&mut visual, &view), "sel:[\"t3\"] ctx:t3:pointer");
}

#[gpui::test]
fn bare_rows_and_the_empty_state(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(",row_chrome:false"));
    // Without chrome the row is as wide as the list: no inset from its sides.
    let list = bounds(&mut visual, &view, "listbox", "Tracks");
    let row = bounds(&mut visual, &view, "option", "Track 1");
    assert!((row.x - list.x).abs() < 0.5, "{list:?} {row:?}");
    let empty = script("").replace("items:tracks()", "items:[]");
    let (mut visual, view) = mount(cx, empty);
    bounds(&mut visual, &view, "listbox", "Tracks");
    assert!(
        snapshot(&mut visual, &view)
            .nodes()
            .any(|node| node.name == "No items")
    );
}

#[gpui::test]
fn a_control_in_a_row_keeps_its_own_tab_stop(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(""));
    let row = bounds(&mut visual, &view, "option", "Track 1");
    visual.simulate_click(center(row), Modifiers::none());
    settle(&mut visual, Step::Tick, 3);
    // Tab leaves the list for the rows' controls in row order; Shift+Tab comes back to
    // the list, whose Enter still opens the selected row.
    for step in ["tab", "enter", "tab", "enter", "shift-tab", "shift-tab", "enter"] {
        keys(&mut visual, step);
    }
    assert_eq!(
        log(&mut visual, &view),
        "open:t1 sel:[\"t1\"] play:t0 play:t1 open:t1"
    );
}
