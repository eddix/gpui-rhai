//! TabBar: document tabs that belong to the panel under them. The selected tab takes the
//! panel's surface and covers the bar's line; a press selects, the close button and a
//! middle press close, the keys move a cursor that Enter selects, a right press or
//! Shift+F10 asks for a menu, tabs reorder by drag or Alt+Arrow, the strip scrolls (also
//! with a vertical wheel) and keeps the selected tab revealed, and an optional menu lists
//! every tab.

use gpui::{
    Modifiers, MouseButton, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext,
    point, px,
};
use gpui_rhai::{GeometryBounds, MotionPreference, ScriptViewHandle};

use crate::support::{
    Mount, ReplaceOnce, Step, bounds, center, last_error, painted_quads, text_of,
};

/// A bar `width` wide over `count` tabs (t0..), the selected value in state; `extra` adds
/// TabBar props. The log line records events.
pub(super) fn script(count: usize, width: u32, extra: &str) -> String {
    format!(
        r#"import "components/tab_bar" as tab_bar;
fn state_schema(){{#{{fields:#{{
    value:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"t1"}}}},
    log:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn note(ctx,line){{ctx.set_state("log",`${{ctx.get_state("log")}} ${{line}}`);}}
fn changed(ctx,value){{ctx.set_state("value",value);note(ctx,`change:${{value}}`);}}
fn closed(ctx,value){{note(ctx,`close:${{value}}`);}}
fn asked(ctx,request){{note(ctx,`ctx:${{request.value}}:${{request.source}}`);}}
fn moved(ctx,request){{note(ctx,`move:${{request.value}}:${{request.placement}}:${{request.anchor}}`);}}
fn tabs(){{let result=[];for index in 0..{count}{{
    result.push(#{{value:`t${{index}}`,label:`Tab number ${{index}}`,closable:index<3,dirty:index==2}});}}result}}
fn view(ctx){{column([
    tab_bar::TabBar(#{{key:"files",label:"Open files",value:ctx.get_state("value"),tabs:tabs(),
        on_change:Fn("changed"),on_close:Fn("closed"),on_context_request:Fn("asked"),on_reorder:Fn("moved")
        {extra}}}).with_style(style().width(px({width}))),
    text(ctx.get_state("log")).test_id("log"),
])}}"#
    )
}

pub(super) fn mount(
    cx: &mut TestAppContext,
    source: String,
) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(source)
        .motion(MotionPreference::None)
        .id("tabs")
        .mount(cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Tick, 3);
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    text_of(visual, view, "log").trim().to_owned()
}

fn press(visual: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>, button: MouseButton) {
    visual.simulate_mouse_move(at, None, Modifiers::none());
    visual.simulate_mouse_down(at, button, Modifiers::none());
    visual.simulate_mouse_up(at, button, Modifiers::none());
    settle(visual);
}

fn focus_strip(visual: &mut VisualTestContext) {
    visual.update(|window, cx| window.focus_next(cx));
    settle(visual);
}

#[gpui::test]
fn the_selected_tab_joins_the_panel_and_covers_the_line(cx: &mut TestAppContext) {
    // A unique fill for the selected tab stands in for tabbar.active.
    let (mut visual, view) = mount(
        cx,
        script(
            3,
            600,
            ",part_styles:#{tab_selected:style().background(rgba(0x00ff00ff))}",
        ),
    );
    let selected = bounds(&mut visual, &view, "tab", "Tab number 1");
    let strip = bounds(&mut visual, &view, "tablist", "Open files");
    // Tabs fill the bar's height, down over its 1px bottom line.
    assert!(
        (selected.y + selected.height - (strip.y + strip.height)).abs() < 0.5,
        "tab {selected:?} in strip {strip:?}"
    );
    let fill: gpui::Background = gpui::rgba(0x00ff_00ff).into();
    let covers = painted_quads(&mut visual).iter().any(|quad| {
        quad.background == fill
            && (f64::from(quad.bounds.x) - selected.x).abs() < 1.5
            && (f64::from(quad.bounds.bottom()) - (strip.y + strip.height)).abs() < 0.5
    });
    assert!(
        covers,
        "the selected tab is painted down to the bar's bottom"
    );
}

#[gpui::test]
fn a_press_selects_and_the_close_button_and_middle_press_close(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(3, 600, ""));
    let first = bounds(&mut visual, &view, "tab", "Tab number 0");
    press(&mut visual, center(first), MouseButton::Left);
    assert_eq!(log(&mut visual, &view), "change:t0");
    let close = bounds(&mut visual, &view, "button", "Close Tab number 0");
    press(&mut visual, center(close), MouseButton::Left);
    assert_eq!(
        log(&mut visual, &view),
        "change:t0 close:t0",
        "the close button does not select"
    );
    let second = bounds(&mut visual, &view, "tab", "Tab number 1");
    press(&mut visual, center(second), MouseButton::Middle);
    assert_eq!(log(&mut visual, &view), "change:t0 close:t0 close:t1");
}

#[gpui::test]
fn the_keys_move_a_cursor_and_enter_selects_it(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(4, 600, ""));
    focus_strip(&mut visual);
    visual.simulate_keystrokes("right right");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), "", "moving does not select");
    visual.simulate_keystrokes("enter");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), "change:t3");
    visual.simulate_keystrokes("home space");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), "change:t3 change:t0");
}

#[gpui::test]
fn a_right_press_and_shift_f10_ask_for_a_menu(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(3, 600, ""));
    let first = bounds(&mut visual, &view, "tab", "Tab number 0");
    press(&mut visual, center(first), MouseButton::Right);
    assert_eq!(
        log(&mut visual, &view),
        "ctx:t0:pointer",
        "a right press does not select"
    );
    focus_strip(&mut visual);
    visual.simulate_keystrokes("shift-f10");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), "ctx:t0:pointer ctx:t1:keyboard");
}

#[gpui::test]
fn reorderable_tabs_move_by_drag_and_by_alt_arrows(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(3, 600, ",reorderable:true"));
    // A press without movement is a tap: it selects.
    let first = bounds(&mut visual, &view, "tab", "Tab number 0");
    press(
        &mut visual,
        point(px(f32_from(first.x + 20.0)), center(first).y),
        MouseButton::Left,
    );
    assert_eq!(log(&mut visual, &view), "change:t0");
    // Drag the first tab past the second.
    let second = bounds(&mut visual, &view, "tab", "Tab number 1");
    let start = point(px(f32_from(first.x + 20.0)), center(first).y);
    let end = point(
        px(f32_from(second.x + second.width * 0.6)),
        center(second).y,
    );
    visual.simulate_mouse_move(start, None, Modifiers::none());
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_move(
        point(start.x + px(12.0), start.y),
        MouseButton::Left,
        Modifiers::none(),
    );
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::none());
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), "change:t0 move:t0:after:t1");
    // Alt+Right moves the cursor tab after its neighbour.
    focus_strip(&mut visual);
    visual.simulate_keystrokes("alt-right");
    settle(&mut visual);
    assert!(
        log(&mut visual, &view).ends_with("move:t0:after:t1"),
        "{}",
        log(&mut visual, &view)
    );
}

#[allow(clippy::cast_possible_truncation)]
fn f32_from(value: f64) -> f32 {
    value as f32
}

#[gpui::test]
fn a_narrow_strip_scrolls_and_keeps_the_selected_tab_revealed(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(12, 360, ""));
    let strip = bounds(&mut visual, &view, "tablist", "Open files");
    let last = bounds(&mut visual, &view, "tab", "Tab number 11");
    assert!(
        last.x > strip.x + strip.width,
        "the last tab starts past the strip: {last:?}"
    );
    // Select the last tab by keyboard: it scrolls into view.
    focus_strip(&mut visual);
    visual.simulate_keystrokes("end enter");
    settle(&mut visual);
    let last = bounds(&mut visual, &view, "tab", "Tab number 11");
    assert!(
        last.x >= strip.x - 0.5
            && last.x + last.width <= strip.x + strip.width + 0.5
            && (last.y - strip.y).abs() < 0.5,
        "the selected tab is revealed: {last:?} in {strip:?}"
    );
    // A vertical wheel scrolls the strip sideways.
    let before = bounds(&mut visual, &view, "tab", "Tab number 11").x;
    visual.simulate_event(ScrollWheelEvent {
        position: center(strip),
        delta: ScrollDelta::Pixels(point(px(0.0), px(120.0))),
        ..ScrollWheelEvent::default()
    });
    settle(&mut visual);
    let after = bounds(&mut visual, &view, "tab", "Tab number 11").x;
    assert!(
        after > before + 10.0,
        "a vertical wheel moved the strip: {before} -> {after}"
    );
}

#[gpui::test]
fn the_overflow_menu_lists_every_tab(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script(6, 360, ",overflow_menu:true"));
    let button = bounds(&mut visual, &view, "button", "All tabs");
    press(&mut visual, center(button), MouseButton::Left);
    let item = bounds(&mut visual, &view, "menuitem", "Tab number 5");
    press(&mut visual, center(item), MouseButton::Left);
    assert_eq!(log(&mut visual, &view), "change:t5");
}

/// The selected tab's painted corner radius and start border width, in logical pixels. A
/// unique fill marks the tab; GPUI paints its border as separate quads over the same bounds.
fn selected_quad(visual: &mut VisualTestContext) -> (f32, f32) {
    let fill: gpui::Background = gpui::rgba(0x00ff_00ff).into();
    let quads = painted_quads(visual);
    let tab = quads
        .iter()
        .find(|quad| quad.background == fill)
        .expect("the selected tab is painted");
    let start_border = quads
        .iter()
        .filter(|quad| quad.bounds == tab.bounds)
        .map(|quad| quad.border_widths[3])
        .fold(0.0_f32, f32::max);
    (tab.corner_radii[0].max(tab.corner_radii[1]), start_border)
}

#[gpui::test]
fn the_selected_tab_is_square_in_every_corner_style_and_owns_no_edge_line(cx: &mut TestAppContext) {
    // Round corners, the first tab selected on the bar's start edge.
    let source = script(
        3,
        600,
        ",part_styles:#{tab_selected:style().background(rgba(0x00ff00ff))}",
    )
    .replace_once(r#"value:ctx.get_state("value")"#, r#"value:"t0""#)
    .replace_once(
        "}).with_style(style().width(px(600))),",
        "}).env(#{corners:\"round\"}).with_style(style().width(px(600))),",
    );
    assert!(source.contains("corners"), "the round corner style is set");
    let (mut visual, _) = mount(cx, source.clone());
    let (radius, start_border) = selected_quad(&mut visual);
    assert!(
        radius.abs() < 0.005,
        "round corners leave the selected tab square: {radius}"
    );
    assert!(
        start_border.abs() < 0.005,
        "the first tab draws no start hairline: {start_border}"
    );
    // The second tab keeps its start hairline.
    let (mut visual, _) = mount(cx, source.replace_once(r#"value:"t0""#, r#"value:"t1""#));
    let (_, start_border) = selected_quad(&mut visual);
    assert!(
        start_border > 0.25,
        "a later tab draws its start hairline: {start_border}"
    );
}

fn tab_count(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> usize {
    crate::support::nodes(visual, view)
        .iter()
        .filter(|node| node.role == "tab")
        .count()
}

#[gpui::test]
fn empty_tab_bar_mounts_without_error(cx: &mut TestAppContext) {
    let source = script(0, 360, "").replace_once("value:\"t1\"", "value:\"\"");
    let (mut visual, view) = mount(cx, source);
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "empty TabBar must be valid: {error:?}");
    assert_eq!(tab_count(&mut visual, &view), 0);
    // Every tab disabled and none selected: there is no cursor to reveal either.
    let source = script(3, 360, "")
        .replace_once("value:\"t1\"", "value:\"\"")
        .replace_once("closable:index<3", "disabled:true,closable:index<3");
    let (mut visual, view) = mount(cx, source);
    let error = last_error(&mut visual, &view);
    assert!(
        error.is_none(),
        "all-disabled TabBar must be valid: {error:?}"
    );
    assert_eq!(tab_count(&mut visual, &view), 3);
}

/// One tab, which the caller removes in `on_close` (clearing `value`), then opens again.
#[gpui::test]
fn closing_last_tab_commits_empty_state(cx: &mut TestAppContext) {
    let source = script(1, 360, "")
        .replace_once("value:\"t1\"", "value:\"t0\"")
        .replace_once(
            "tabs:tabs()",
            "tabs:if ctx.get_state(\"value\") == \"\" { [] } else { tabs() }",
        )
        .replace_once(
            "fn closed(ctx,value){note(ctx,`close:${value}`);}",
            "fn closed(ctx,value){ctx.set_state(\"value\",\"\");note(ctx,`close:${value}`);}\n\
             fn reopen(ctx,payload){ctx.set_state(\"value\",\"t0\");note(ctx,\"open:t0\");}",
        )
        .replace_once(
            "text(ctx.get_state(\"log\")).test_id(\"log\"),",
            "text(ctx.get_state(\"log\")).test_id(\"log\"),\n    \
             text(\"Reopen\").accessibility_role(\"button\").accessibility_label(\"Reopen\")\
             .on_click(Fn(\"reopen\")).with_style(style().width(px(80)).height(px(24))),",
        );
    assert!(source.contains("ctx.set_state(\"value\",\"\")") && source.contains("Reopen"));
    let (mut visual, view) = mount(cx, source);
    let close = bounds(&mut visual, &view, "button", "Close Tab number 0");
    press(&mut visual, center(close), MouseButton::Left);
    let error = last_error(&mut visual, &view);
    assert!(
        error.is_none(),
        "closing final tab should commit: {error:?}"
    );
    assert_eq!(tab_count(&mut visual, &view), 0, "last tab removed");
    assert_eq!(log(&mut visual, &view), "close:t0");
    // 0 -> 1: the tab comes back selected, and the reveal runs again.
    let reopen = bounds(&mut visual, &view, "button", "Reopen");
    press(&mut visual, center(reopen), MouseButton::Left);
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "reopening a tab should commit: {error:?}");
    assert_eq!(log(&mut visual, &view), "close:t0 open:t0");
    let selected = crate::support::nodes(&mut visual, &view)
        .into_iter()
        .filter(|node| node.role == "tab" && node.selected == Some(true))
        .map(|node| node.name)
        .collect::<Vec<_>>();
    assert_eq!(selected, ["Tab number 0"]);
}

/// Three tabs, t1 selected; closing it drops it from `tabs`. `selection` is what the caller
/// stores as `value` afterwards (`None` leaves the closed tab's value).
fn closing_the_selected_tab(cx: &mut TestAppContext, selection: Option<&str>) {
    let store = selection.map_or(String::new(), |value| {
        format!("ctx.set_state(\"value\",\"{value}\");")
    });
    let source = script(3, 600, "")
        .replace_once(
            "log:#{schema",
            "closed:#{schema:#{type:\"array\",items:#{type:\"string\"}},\
             \"default\":#{type:\"array\",value:[]}},log:#{schema",
        )
        .replace_once(
            "fn closed(ctx,value){note(ctx,`close:${value}`);}",
            &format!(
                "fn closed(ctx,value){{let gone=ctx.get_state(\"closed\");gone.push(value);\
                 ctx.set_state(\"closed\",gone);{store}note(ctx,`close:${{value}}`);}}"
            ),
        )
        .replace_once(
            "fn view(ctx)",
            "fn open_tabs(gone){let result=[];for tab in tabs(){if !(tab.value in gone){result.push(tab);}}result}\n\
             fn view(ctx)",
        )
        .replace_once("tabs:tabs()", "tabs:open_tabs(ctx.get_state(\"closed\"))");
    assert!(source.contains("open_tabs(ctx.get_state"));
    let (mut visual, view) = mount(cx, source);
    let close = bounds(&mut visual, &view, "button", "Close Tab number 1");
    press(&mut visual, center(close), MouseButton::Left);
    let error = last_error(&mut visual, &view);
    assert!(
        error.is_none(),
        "closing the selected tab ({selection:?}) should commit: {error:?}"
    );
    assert_eq!(tab_count(&mut visual, &view), 2);
    // With no selected tab left the cursor starts on the first enabled tab, which Enter
    // selects.
    focus_strip(&mut visual);
    visual.simulate_keystrokes("enter");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), "close:t1 change:t0");
    assert!(last_error(&mut visual, &view).is_none());
}

#[gpui::test]
fn closing_the_selected_tab_and_clearing_the_value_commits(cx: &mut TestAppContext) {
    closing_the_selected_tab(cx, Some(""));
}

#[gpui::test]
fn closing_the_selected_tab_and_keeping_its_value_commits(cx: &mut TestAppContext) {
    closing_the_selected_tab(cx, None);
}

/// Twelve tabs in a 360px strip, t1 selected, in the order the caller keeps in state:
/// `on_reorder` writes the moved order back, Reverse reverses it, and Note only adds to the
/// log, a render that changes neither the order nor the selection.
fn controlled_order_script() -> String {
    let source = script(12, 360, ",reorderable:true")
        .replace_once(
            "log:#{schema",
            "order:#{schema:#{type:\"array\",items:#{type:\"string\"}},\
             \"default\":#{type:\"array\",value:[]}},log:#{schema",
        )
        .replace_once(
            "fn moved(ctx,request){",
            "fn moved(ctx,request){let order=current_order(ctx);order.remove(order.index_of(request.value));\
             let at=order.index_of(request.anchor);if request.placement==\"after\"{at+=1;}\
             order.insert(at,request.value);ctx.set_state(\"order\",order);",
        )
        .replace_once(
            "fn view(ctx)",
            "fn current_order(ctx){let order=ctx.get_state(\"order\");\
             if order.len==0{for tab in tabs(){order.push(tab.value);}}order}\n\
             fn ordered_tabs(ctx){let all=tabs();let result=[];\
             for value in current_order(ctx){for tab in all{if tab.value==value{result.push(tab);}}}result}\n\
             fn reverse(ctx,payload){let order=current_order(ctx);order.reverse();ctx.set_state(\"order\",order);}\n\
             fn ping(ctx,payload){note(ctx,\"ping\");}\n\
             fn view(ctx)",
        )
        .replace_once("tabs:tabs()", "tabs:ordered_tabs(ctx)")
        .replace_once(
            "text(ctx.get_state(\"log\")).test_id(\"log\"),",
            "text(ctx.get_state(\"log\")).test_id(\"log\"),\n    \
             row([text(\"Reverse\").accessibility_role(\"button\").accessibility_label(\"Reverse\")\
             .on_click(Fn(\"reverse\")).with_style(style().width(px(80)).height(px(24))),\
             text(\"Note\").accessibility_role(\"button\").accessibility_label(\"Note\")\
             .on_click(Fn(\"ping\")).with_style(style().width(px(80)).height(px(24)))]),",
        );
    assert!(source.contains("ordered_tabs(ctx)") && source.contains("Reverse"));
    source
}

fn assert_in_strip(tab: GeometryBounds, strip: GeometryBounds, when: &str) {
    assert!(
        tab.x >= strip.x - 0.5 && tab.x + tab.width <= strip.x + strip.width + 0.5,
        "{when}: the selected tab must stay revealed: {tab:?} in {strip:?}"
    );
}

/// The round-4 review probe (#142): reversing the tabs keeps t1 selected and revealed.
#[gpui::test]
fn reorder_keeps_selected_tab_in_view(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, controlled_order_script());
    let strip = bounds(&mut visual, &view, "tablist", "Open files");
    let before = bounds(&mut visual, &view, "tab", "Tab number 1");
    assert_in_strip(before, strip, "at first");
    let reverse = bounds(&mut visual, &view, "button", "Reverse");
    press(&mut visual, center(reverse), MouseButton::Left);
    assert!(last_error(&mut visual, &view).is_none());
    let first = bounds(&mut visual, &view, "tab", "Tab number 11");
    let selected = bounds(&mut visual, &view, "tab", "Tab number 1");
    println!("after reversing: t11 {first:?}, t1 {selected:?}, strip {strip:?}");
    assert!(first.x < selected.x, "the order is reversed");
    assert_in_strip(selected, strip, "after reversing");
}

/// Alt+Right moves the selected tab one place at a time, and the caller writes each new order
/// back: the tab stays in view all the way to the end of the strip.
#[gpui::test]
fn a_controlled_reorder_keeps_the_moved_tab_in_view(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, controlled_order_script());
    let strip = bounds(&mut visual, &view, "tablist", "Open files");
    focus_strip(&mut visual);
    for step in 1..=10 {
        visual.simulate_keystrokes("alt-right");
        settle(&mut visual);
        let selected = bounds(&mut visual, &view, "tab", "Tab number 1");
        assert_in_strip(selected, strip, &format!("after move {step}"));
    }
    assert!(last_error(&mut visual, &view).is_none());
    assert!(
        log(&mut visual, &view).ends_with("move:t1:after:t11"),
        "{}",
        log(&mut visual, &view)
    );
}

/// After the reveal, the strip scrolled by hand away from the selected tab stays where it was:
/// through renders that change neither the order nor the selection, and through a reorder,
/// which keeps the selected tab in view only when it was in view.
#[gpui::test]
fn scrolling_by_hand_after_a_reorder_is_not_pulled_back(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, controlled_order_script());
    let strip = bounds(&mut visual, &view, "tablist", "Open files");
    let reverse = bounds(&mut visual, &view, "button", "Reverse");
    press(&mut visual, center(reverse), MouseButton::Left);
    assert_in_strip(
        bounds(&mut visual, &view, "tab", "Tab number 1"),
        strip,
        "after reversing",
    );
    visual.simulate_event(ScrollWheelEvent {
        position: center(strip),
        delta: ScrollDelta::Pixels(point(px(0.0), px(800.0))),
        ..ScrollWheelEvent::default()
    });
    settle(&mut visual);
    let scrolled = bounds(&mut visual, &view, "tab", "Tab number 1");
    assert!(
        scrolled.x > strip.x + strip.width,
        "the wheel scrolled the selected tab out of view: {scrolled:?} in {strip:?}"
    );
    let note = bounds(&mut visual, &view, "button", "Note");
    press(&mut visual, center(note), MouseButton::Left);
    settle(&mut visual);
    assert!(log(&mut visual, &view).ends_with("ping"));
    let after = bounds(&mut visual, &view, "tab", "Tab number 1");
    assert!(
        (after.x - scrolled.x).abs() < 0.5,
        "a render without a reorder leaves the strip where it was scrolled: {} -> {}",
        scrolled.x,
        after.x
    );
    // Reverse again, back to t0..t11: the strip keeps its scroll offset, measured from the
    // first tab, though the selected tab is still out of view.
    let offset = strip.x - bounds(&mut visual, &view, "tab", "Tab number 11").x;
    press(&mut visual, center(reverse), MouseButton::Left);
    settle(&mut visual);
    let first = bounds(&mut visual, &view, "tab", "Tab number 0");
    let selected = bounds(&mut visual, &view, "tab", "Tab number 1");
    assert!(first.x < selected.x, "the order is back to t0..t11");
    assert!(
        (strip.x - first.x - offset).abs() < 0.5,
        "a reorder while the selected tab is scrolled out of view keeps the scroll offset: \
         {offset} -> {} (t1 {selected:?})",
        strip.x - first.x
    );
}
