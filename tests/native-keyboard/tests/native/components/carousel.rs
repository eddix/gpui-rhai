//! Carousel: one tab stop beside the item whose keys step, arrows disabled at the ends unless
//! the items wrap, dots that select, overlay arrows that show on hover or focus, keys typed in
//! an item that stay the item's; and the slide both Carousel and Tabs take from SlidingPanel,
//! from the side the new content comes from.

use std::collections::BTreeMap;

use gpui::{Background, Modifiers, TestAppContext, VisualTestContext, point, px, rgba};
use gpui_rhai::{MotionPreference, Rgba8, ScriptViewHandle, ThemeTokenOverrides, UiValue};
use gpui_rhai_registry::AR_LOCALE;

use crate::support::{
    Mount, Step, bounds, center, click_center, last_error, node_by_test_id, painted_quads,
    settle as settle_for, snapshot, text_of,
};

const RAISED: u32 = 0x00ff_00ff;
const INK: u32 = 0xff00_ffff;
const DOT: u32 = 0x0000_ffff;

/// A view over a 320px Carousel of four text items starting at `start`; `config` adds props.
/// Each change is stored and logged; the Break button sets an index past the items.
fn carousel(start: i64, config: &str) -> String {
    format!(
        r#"import "components/carousel" as carousel;
fn state_schema(){{#{{fields:#{{
    index:#{{schema:#{{type:"integer"}},"default":#{{type:"integer",value:{start}}}}},
    log:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn changed(ctx,index){{ctx.set_state("index",index);ctx.set_state("log",`${{ctx.get_state("log")}} ${{index}}`);}}
fn pressed(ctx,payload){{ctx.set_state("log",`${{ctx.get_state("log")}} press`);}}
fn broke(ctx,payload){{ctx.set_state("index",99);}}
fn items(){{let result=[];for index in 0..4{{result.push(#{{label:`Item ${{index}}`,
    content:column([text(`Body ${{index}}`).test_id(`body-${{index}}`),
        text(`Act ${{index}}`).accessibility_role("button").accessibility_label(`Act ${{index}}`)
            .on_click(Fn("pressed"))])}});}}result}}
fn view(ctx){{column([
    carousel::Carousel(#{{key:"alerts",label:"Alerts",index:ctx.get_state("index"),items:items(),
        on_change:Fn("changed"),{config}}}).with_style(style().width(px(320))),
    text(ctx.get_state("log")).test_id("log"),
    text("Break").accessibility_role("button").accessibility_label("Break").on_click(Fn("broke")),
])}}"#
    )
}

fn mount_with(
    cx: &mut TestAppContext,
    source: String,
    motion: MotionPreference,
    rtl: bool,
) -> (VisualTestContext, ScriptViewHandle) {
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([
            ("surface_raised".to_owned(), Rgba8::from_rgba_hex(RAISED)),
            ("text_primary".to_owned(), Rgba8::from_rgba_hex(INK)),
            ("control.border".to_owned(), Rgba8::from_rgba_hex(DOT)),
        ]),
        ..Default::default()
    };
    let mut mount = Mount::components(source)
        .with(move |view| view.theme_token_overrides(overrides))
        .motion(motion)
        .id("carousel");
    if rtl {
        mount = mount.locale("ar.rhai", AR_LOCALE);
    }
    let (mut visual, view) = mount.mount(cx);
    settle(&mut visual);
    (visual, view)
}

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    mount_with(cx, source, MotionPreference::None, false)
}

fn settle(visual: &mut VisualTestContext) {
    settle_for(visual, Step::Tick, 6);
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    text_of(visual, view, "log").trim().to_owned()
}

fn has(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) -> bool {
    snapshot(visual, view)
        .find_by_role_and_name(role, name)
        .next()
        .is_some()
}

/// Painted quads filled with `color`, as (width, height).
fn fills(visual: &mut VisualTestContext, color: u32) -> Vec<(f32, f32)> {
    let fill: Background = rgba(color).into();
    painted_quads(visual)
        .into_iter()
        .filter(|quad| quad.background == fill)
        .map(|quad| (quad.bounds.width, quad.bounds.height))
        .collect()
}

#[gpui::test]
fn the_controls_are_one_tab_stop_whose_keys_step(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, carousel(0, ""));
    assert_eq!(last_error(&mut visual, &view), None);
    // The header shows the label and the count between the arrows.
    bounds(&mut visual, &view, "text", "1 / 4");
    let controls = snapshot(&mut visual, &view)
        .find_by_role_and_name("group", "Alerts")
        .find(|node| node.value == Some(UiValue::String("Item 0, 1 / 4".to_owned())))
        .cloned();
    assert!(
        controls.is_some(),
        "the controls carry the shown item and position"
    );

    // The arrows are not tab stops: the first stop is the controls.
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    for key in ["left", "right", "right", "end", "home"] {
        visual.simulate_keystrokes(key);
        settle(&mut visual);
    }
    // Left at the first item without wrap does nothing.
    assert_eq!(log(&mut visual, &view), "1 2 3 0");
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn an_arrow_at_an_end_is_disabled_unless_the_items_wrap(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, carousel(0, ""));
    let previous = bounds(&mut visual, &view, "button", "Previous");
    click_center(&mut visual, previous);
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), "", "disabled at the first item");
    let next = bounds(&mut visual, &view, "button", "Next");
    click_center(&mut visual, next);
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), "1");

    let (mut visual, view) = mount(cx, carousel(3, "wrap:true,previous_label:\"Back\""));
    let next = bounds(&mut visual, &view, "button", "Next");
    click_center(&mut visual, next);
    settle(&mut visual);
    let back = bounds(&mut visual, &view, "button", "Back");
    click_center(&mut visual, back);
    settle(&mut visual);
    assert_eq!(
        log(&mut visual, &view),
        "0 3",
        "past the last is the first, and back"
    );
}

#[gpui::test]
fn a_dot_selects_its_item_and_the_selected_one_is_an_icon_wide(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, carousel(1, "indicator:\"dots\""));
    assert_eq!(last_error(&mut visual, &view), None);
    // The selected dot is metrics.icon (16) wide in text_primary; the others are 6px squares
    // in control.border.
    let selected = fills(&mut visual, INK)
        .into_iter()
        .filter(|(_, height)| (height - 6.0).abs() < 0.01)
        .collect::<Vec<_>>();
    assert_eq!(selected, vec![(16.0, 6.0)]);
    let others = fills(&mut visual, DOT);
    assert_eq!(others, vec![(6.0, 6.0); 3]);
    let dot = bounds(&mut visual, &view, "button", "Item 3");
    click_center(&mut visual, dot);
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), "3");
}

#[gpui::test]
fn keys_typed_in_an_item_stay_the_items(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, carousel(1, "wrap:true"));
    let act = bounds(&mut visual, &view, "button", "Act 1");
    click_center(&mut visual, act);
    settle(&mut visual);
    for key in ["right", "left", "home"] {
        visual.simulate_keystrokes(key);
        settle(&mut visual);
    }
    assert_eq!(
        log(&mut visual, &view),
        "press",
        "the item's keys never reach the controls"
    );
}

/// Moves the pointer off the carousel, to the empty part of the 1920 x 1080 window. A single
/// move right after the window activates did not register here, so there are two.
fn pointer_away(visual: &mut VisualTestContext) {
    for x in [900.0, 910.0] {
        visual.simulate_mouse_move(point(px(x), px(600.0)), None, Modifiers::none());
        settle(visual);
    }
}

#[gpui::test]
fn overlay_arrows_show_on_hover_or_focus_and_leave_out_an_end(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, carousel(0, "variant:\"overlay\""));
    assert_eq!(last_error(&mut visual, &view), None);
    // GPUI reports focus paths only for an active window, and a test window starts inactive.
    visual.update(|window, _| window.activate_window());
    // At the first item only the next arrow is there.
    assert!(!has(&mut visual, &view, "button", "Previous"));
    assert!(has(&mut visual, &view, "button", "Next"));
    // The arrows carry surface_raised; hidden, they paint nothing in it.
    pointer_away(&mut visual);
    assert!(
        fills(&mut visual, RAISED).is_empty(),
        "hidden until hovered"
    );

    let body = node_by_test_id(&mut visual, &view, "body-0")
        .geometry
        .unwrap()
        .visual;
    visual.simulate_mouse_move(center(body), None, Modifiers::none());
    settle(&mut visual);
    assert_eq!(
        fills(&mut visual, RAISED).len(),
        1,
        "the next arrow shows on hover"
    );

    // The item comes first in the tab order, then the controls under it.
    pointer_away(&mut visual);
    assert!(fills(&mut visual, RAISED).is_empty());
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    assert!(
        fills(&mut visual, RAISED).is_empty(),
        "focus in the item shows no arrows"
    );
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    assert_eq!(
        fills(&mut visual, RAISED).len(),
        1,
        "but focus on the controls does"
    );
    visual.simulate_keystrokes("right");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), "1");
    assert_eq!(
        fills(&mut visual, RAISED).len(),
        2,
        "both arrows past the first item"
    );
}

#[gpui::test]
fn a_carousel_rejects_an_index_past_its_items(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, carousel(0, ""));
    let broke = bounds(&mut visual, &view, "button", "Break");
    click_center(&mut visual, broke);
    settle(&mut visual);
    let error = last_error(&mut visual, &view).unwrap_or_default();
    assert!(error.contains("index 99 is past its 4 items"), "{error}");
}

/// The visual x (or y) of the node `test_id` right after `step` and once it has settled.
fn slide(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    test_id: &str,
    vertical: bool,
    step: impl FnOnce(&mut VisualTestContext),
) -> (f64, f64) {
    step(visual);
    // A frame without moving the clock: the slide is at its start.
    settle_for(visual, Step::Frame, 2);
    let at = |visual: &mut VisualTestContext| {
        let bounds = node_by_test_id(visual, view, test_id)
            .geometry
            .unwrap()
            .visual;
        if vertical { bounds.y } else { bounds.x }
    };
    let start = at(visual);
    settle(visual);
    (start, at(visual))
}

fn press(
    role: &'static str,
    name: &'static str,
    view: ScriptViewHandle,
) -> impl FnOnce(&mut VisualTestContext) {
    move |visual| {
        let target = bounds(visual, &view, role, name);
        click_center(visual, target);
    }
}

#[gpui::test]
fn a_new_item_slides_in_from_the_side_it_comes_from(cx: &mut TestAppContext) {
    let (mut visual, view) = mount_with(
        cx,
        carousel(0, "wrap:true"),
        MotionPreference::Normal,
        false,
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let (start, rest) = slide(
        &mut visual,
        &view,
        "body-1",
        false,
        press("button", "Next", view.clone()),
    );
    assert!(
        start > rest + 1.0,
        "the next item comes from the right: {start} vs {rest}"
    );
    let (start, rest) = slide(
        &mut visual,
        &view,
        "body-0",
        false,
        press("button", "Previous", view.clone()),
    );
    assert!(
        start < rest - 1.0,
        "the previous item comes from the left: {start} vs {rest}"
    );
    // In a loop the shorter way round decides: before the first is the last, from the left.
    let (start, rest) = slide(
        &mut visual,
        &view,
        "body-3",
        false,
        press("button", "Previous", view.clone()),
    );
    assert!(
        start < rest - 1.0,
        "wrapping back comes from the left: {start} vs {rest}"
    );
    assert_eq!(log(&mut visual, &view), "1 0 3");

    // Right-to-left text mirrors it.
    let (mut visual, view) = mount_with(cx, carousel(0, ""), MotionPreference::Normal, true);
    let (start, rest) = slide(
        &mut visual,
        &view,
        "body-1",
        false,
        press("button", "Next", view.clone()),
    );
    assert!(
        start < rest - 1.0,
        "in RTL the next item comes from the left: {start} vs {rest}"
    );

    // The Host's reduced motion settles it at once.
    let (mut visual, view) = mount_with(cx, carousel(0, ""), MotionPreference::Reduced, false);
    let (start, rest) = slide(
        &mut visual,
        &view,
        "body-1",
        false,
        press("button", "Next", view.clone()),
    );
    assert!(
        (start - rest).abs() < 0.01,
        "reduced motion does not slide: {start} vs {rest}"
    );
}

/// Keyed vertical Tabs over four text panels that slide; the Unkey button drops the key.
const TABS: &str = r#"import "components/tabs" as tabs;
fn state_schema(){#{fields:#{value:#{schema:#{type:"string"},"default":#{type:"string",value:"t0"}},
    keyed:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}}}}}
fn changed(ctx,value){ctx.set_state("value",value);}
fn unkey(ctx,payload){ctx.set_state("keyed",false);}
fn items(){let result=[];for index in 0..4{result.push(#{value:`t${index}`,label:`Box ${index}`,
    content:text(`Panel ${index}`).test_id(`panel-${index}`)});}result}
fn view(ctx){
    let config=#{label:"Boxes",value:ctx.get_state("value"),orientation:"vertical",
        panel_motion:"slide",tabs:items(),on_change:Fn("changed")};
    if ctx.get_state("keyed"){config.key="boxes";}
    column([tabs::Tabs(config).with_style(style().width(px(420)).height(px(220))),
        text("Unkey").accessibility_role("button").accessibility_label("Unkey").on_click(Fn("unkey"))])
}"#;

#[gpui::test]
fn sliding_tabs_need_a_key_and_follow_the_tab_order(cx: &mut TestAppContext) {
    let (mut visual, view) = mount_with(cx, TABS.to_owned(), MotionPreference::Normal, false);
    assert_eq!(last_error(&mut visual, &view), None);
    // A vertical list slides the panel along y: a later tab's panel comes from below.
    let (start, rest) = slide(
        &mut visual,
        &view,
        "panel-2",
        true,
        press("tab", "Box 2", view.clone()),
    );
    assert!(start > rest + 1.0, "from below: {start} vs {rest}");
    let (start, rest) = slide(
        &mut visual,
        &view,
        "panel-1",
        true,
        press("tab", "Box 1", view.clone()),
    );
    assert!(start < rest - 1.0, "from above: {start} vs {rest}");

    let unkey = bounds(&mut visual, &view, "button", "Unkey");
    click_center(&mut visual, unkey);
    settle(&mut visual);
    let error = last_error(&mut visual, &view).unwrap_or_default();
    assert!(error.contains("needs a key"), "{error}");
}

#[gpui::test]
fn the_count_keeps_one_width_so_the_arrows_stay_put(cx: &mut TestAppContext) {
    // Twelve items from the ninth: the count goes from one digit to two, in the narrowest
    // control (compact xs), where two controls' width would not hold it.
    let source = r#"import "components/carousel" as carousel;
fn state_schema(){#{fields:#{index:#{schema:#{type:"integer"},"default":#{type:"integer",value:8}}}}}
fn changed(ctx,index){ctx.set_state("index",index);}
fn items(){let result=[];for index in 0..12{result.push(#{label:`Item ${index}`,content:text(`Body ${index}`)});}result}
fn view(ctx){carousel::Carousel(#{key:"pages",label:"Pages",index:ctx.get_state("index"),items:items(),
    size:"xs",on_change:Fn("changed")}).with_style(style().width(px(320))).env(#{density:"compact"})}"#;
    let (mut visual, view) = mount(cx, source.to_owned());
    bounds(&mut visual, &view, "text", "9 / 12");
    let before = bounds(&mut visual, &view, "button", "Previous");
    let next = bounds(&mut visual, &view, "button", "Next");
    click_center(&mut visual, next);
    settle(&mut visual);
    bounds(&mut visual, &view, "text", "10 / 12");
    let after = bounds(&mut visual, &view, "button", "Previous");
    assert!((before.x - after.x).abs() < 0.01, "{before:?} -> {after:?}");
}
