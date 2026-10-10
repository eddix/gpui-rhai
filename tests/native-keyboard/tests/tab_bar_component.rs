//! TabBar: document tabs that belong to the panel under them. The selected tab takes the
//! panel's surface and covers the bar's line; a press selects, the close button and a
//! middle press close, the keys move a cursor that Enter selects, a right press or
//! Shift+F10 asks for a menu, tabs reorder by drag or Alt+Arrow, the strip scrolls (also
//! with a vertical wheel) and keeps the selected tab revealed, and an optional menu lists
//! every tab.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, ScrollDelta, ScrollWheelEvent,
    TestAppContext, VisualTestContext, Window, point, px,
};
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

/// A bar `width` wide over `count` tabs (t0..), the selected value in state; `extra` adds
/// TabBar props. The log line records events.
fn script(count: usize, width: u32, extra: &str) -> String {
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
        let host = ScriptViewHost::new("tabs", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("tabs"), host.clone(), window, cx)
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

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| node.test_id.as_deref() == Some("log"))
            .unwrap()
            .name
            .trim()
            .to_owned()
    })
}

fn bounds(
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
            .unwrap_or_else(|| panic!("no {role} {name:?}"))
            .visual
    })
}

fn center(bounds: GeometryBounds) -> gpui::Point<gpui::Pixels> {
    #[allow(clippy::cast_possible_truncation)]
    point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    )
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
    let fill: gpui::Hsla = gpui::rgba(0x00ff_00ff).into();
    let covers = visual.update(|window, _| {
        let scale = window.scale_factor();
        window.painted_quads().iter().any(|quad| {
            quad.background == fill.into()
                && (f64::from(quad.bounds.origin.x.0 / scale) - selected.x).abs() < 1.5
                && (f64::from((quad.bounds.origin.y + quad.bounds.size.height).0 / scale)
                    - (strip.y + strip.height))
                    .abs()
                    < 0.5
        })
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

/// The selected tab's painted corner radius and start border width, in scaled pixels. A
/// unique fill marks the tab; GPUI paints its border as separate quads over the same bounds.
fn selected_quad(visual: &mut VisualTestContext) -> (f32, f32) {
    let fill: gpui::Hsla = gpui::rgba(0x00ff_00ff).into();
    visual.update(|window, _| {
        let quads = window.painted_quads();
        let tab = quads
            .iter()
            .find(|quad| quad.background == fill.into())
            .expect("the selected tab is painted");
        let radii = tab.corner_radii;
        let start_border = quads
            .iter()
            .filter(|quad| quad.bounds == tab.bounds)
            .map(|quad| quad.border_widths.left.0)
            .fold(0.0_f32, f32::max);
        (radii.top_left.0.max(radii.top_right.0), start_border)
    })
}

#[gpui::test]
fn the_selected_tab_is_square_in_every_corner_style_and_owns_no_edge_line(cx: &mut TestAppContext) {
    // Round corners, the first tab selected on the bar's start edge.
    let source = script(
        3,
        600,
        ",part_styles:#{tab_selected:style().background(rgba(0x00ff00ff))}",
    )
    .replace(r#"value:ctx.get_state("value")"#, r#"value:"t0""#)
    .replace(
        "}).with_style(style().width(px(600))),",
        "}).env(#{corners:\"round\"}).with_style(style().width(px(600))),",
    );
    assert!(source.contains("corners"), "the round corner style is set");
    let (mut visual, _) = mount(cx, source.clone());
    let (radius, start_border) = selected_quad(&mut visual);
    assert!(
        radius.abs() < 0.01,
        "round corners leave the selected tab square: {radius}"
    );
    assert!(
        start_border.abs() < 0.01,
        "the first tab draws no start hairline: {start_border}"
    );
    // The second tab keeps its start hairline.
    let (mut visual, _) = mount(cx, source.replace(r#"value:"t0""#, r#"value:"t1""#));
    let (_, start_border) = selected_quad(&mut visual);
    assert!(
        start_border > 0.5,
        "a later tab draws its start hairline: {start_border}"
    );
}
