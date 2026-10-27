//! What any Tabs variant can carry beyond selection: a count capsule as tall as the label,
//! whose corners follow the corner style, a tone that fills it or shows a lamp, an add button
//! right after the last tab, slots on the content edge, the close successor, and the same
//! collection keys in a vertical list.

use std::collections::BTreeMap;

use gpui::{Background, MouseButton, TestAppContext, VisualTestContext, rgba};
use gpui_rhai::{MotionPreference, Rgba8, ScriptViewHandle, ThemeTokenOverrides};

use crate::support::{
    Mount, Step, bounds, center, last_error, painted_quads, settle as settle_for, text_of,
};

const COUNTER: u32 = 0x00ff_00ff;
const ACCENT: u32 = 0xff00_ffff;
const DANGER: u32 = 0x0000_ffff;

fn mount(cx: &mut TestAppContext, source: String) -> (VisualTestContext, ScriptViewHandle) {
    let overrides = ThemeTokenOverrides {
        colors: BTreeMap::from([
            ("tabs.counter".to_owned(), Rgba8::from_rgba_hex(COUNTER)),
            ("accent".to_owned(), Rgba8::from_rgba_hex(ACCENT)),
            ("danger".to_owned(), Rgba8::from_rgba_hex(DANGER)),
        ]),
        ..Default::default()
    };
    let (mut visual, view) = Mount::components(source)
        .with(move |view| view.theme_token_overrides(overrides))
        .motion(MotionPreference::None)
        .id("tabs-features")
        .mount(cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    settle_for(visual, Step::Tick, 3);
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    text_of(visual, view, "log").trim().to_owned()
}

/// Painted quads filled with `color`, as (x, y, width, height, top-left radius).
fn fills(visual: &mut VisualTestContext, color: u32) -> Vec<(f32, f32, f32, f32, f32)> {
    let fill: Background = rgba(color).into();
    painted_quads(visual)
        .into_iter()
        .filter(|quad| quad.background == fill)
        .map(|quad| {
            let r = quad.bounds;
            (r.x, r.y, r.width, r.height, quad.corner_radii[0])
        })
        .collect()
}

/// Three tabs with a count, a toned count past 99, and a lamp; `corners` sets the style.
fn counts(corners: &str) -> String {
    format!(
        r#"import "components/tabs" as tabs;
fn changed(ctx,value){{}}
fn view(ctx){{tabs::Tabs(#{{label:"Repos",value:"a",variant:"underline",panel:false,tabs:[
    #{{value:"a",label:"Alpha",count:7}},
    #{{value:"b",label:"Beta",count:1234,tone:"accent",tone_label:"new"}},
    #{{value:"c",label:"Gamma",tone:"danger",tone_label:"failed"}}
],on_change:Fn("changed")}}).env(#{{corners:"{corners}"}})}}"#
    )
}

#[gpui::test]
fn a_count_capsule_is_as_tall_as_the_label_and_joins_the_name(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, counts("square"));
    assert_eq!(last_error(&mut visual, &view), None);
    // The count and the tone join the accessible name; a lamp has only its tone label.
    let alpha = bounds(&mut visual, &view, "tab", "Alpha, 7");
    bounds(&mut visual, &view, "tab", "Beta, 1234, new");
    bounds(&mut visual, &view, "tab", "Gamma, failed");
    // Past 99 the capsule shows 99+.
    bounds(&mut visual, &view, "text", "99+");

    // A neutral capsule in tabs.counter: metrics.counter (20, the control line height) tall,
    // at least as wide, centred in the 32px tab like the label, square in the square style.
    let neutral = fills(&mut visual, COUNTER);
    assert_eq!(neutral.len(), 1, "{neutral:?}");
    let (x, y, width, height, radius) = neutral[0];
    assert!(
        (height - 20.0).abs() < 0.01 && width >= 20.0 - 0.01,
        "{neutral:?}"
    );
    #[allow(clippy::cast_possible_truncation)]
    let tab_mid = (alpha.y + alpha.height / 2.0) as f32;
    assert!(
        (y + height / 2.0 - tab_mid).abs() < 0.5,
        "{neutral:?} in {alpha:?}"
    );
    #[allow(clippy::cast_possible_truncation)]
    let tab_right = (alpha.x + alpha.width) as f32;
    assert!(
        x > alpha.x as f32 && x + width < tab_right,
        "after the label, inside the tab"
    );
    assert!(radius.abs() < 0.01, "square corners: {radius}");

    // A toned count fills the capsule with the tone; a tone alone is a 6px lamp.
    let toned = fills(&mut visual, ACCENT);
    assert!(
        toned.iter().any(|quad| (quad.3 - 20.0).abs() < 0.01),
        "{toned:?}"
    );
    let lamp = fills(&mut visual, DANGER);
    assert!(
        lamp.iter()
            .any(|quad| (quad.2 - 6.0).abs() < 0.01 && (quad.3 - 6.0).abs() < 0.01),
        "{lamp:?}"
    );
}

#[gpui::test]
fn the_capsule_corners_follow_the_corner_style(cx: &mut TestAppContext) {
    // Round markers are capsules: radius.sm clamps to half the 20px height.
    let (mut visual, _) = mount(cx, counts("round"));
    let neutral = fills(&mut visual, COUNTER);
    assert!((neutral[0].4 - 10.0).abs() < 0.01, "round: {neutral:?}");
    let (mut visual, _) = mount(cx, counts("subtle"));
    let neutral = fills(&mut visual, COUNTER);
    assert!((neutral[0].4 - 2.0).abs() < 0.01, "subtle: {neutral:?}");
}

/// `count` tabs in a strip `width` wide with an add button; adding logs.
fn adding(count: usize, width: u32) -> String {
    format!(
        r#"import "components/tabs" as tabs;
fn state_schema(){{#{{fields:#{{log:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn changed(ctx,value){{}}
fn added(ctx,payload){{ctx.set_state("log",`${{ctx.get_state("log")}} add`);}}
fn items(){{let result=[];for index in 0..{count}{{result.push(#{{value:`t${{index}}`,label:`Tab ${{index}}`}});}}result}}
fn view(ctx){{column([
    tabs::Tabs(#{{label:"Files",value:"t0",variant:"underline",panel:false,tabs:items(),
        on_change:Fn("changed"),on_add:Fn("added"),add_label:"New tab"}}).with_style(style().width(px({width}))),
    text(ctx.get_state("log")).test_id("log"),
])}}"#
    )
}

#[gpui::test]
fn the_add_button_follows_the_last_tab_and_stays_in_view(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, adding(3, 600));
    let list = bounds(&mut visual, &view, "tablist", "Files");
    let add = bounds(&mut visual, &view, "button", "New tab");
    assert!(
        (add.x - (list.x + list.width)).abs() < 0.5,
        "right after the last tab: {add:?} after {list:?}"
    );
    let press = center(add);
    visual.simulate_mouse_move(press, None, gpui::Modifiers::none());
    visual.simulate_mouse_down(press, MouseButton::Left, gpui::Modifiers::none());
    visual.simulate_mouse_up(press, MouseButton::Left, gpui::Modifiers::none());
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), "add");

    // Too many tabs: the list scrolls and the button stays at the strip's end.
    let (mut visual, view) = mount(cx, adding(12, 300));
    let list = bounds(&mut visual, &view, "tablist", "Files");
    let add = bounds(&mut visual, &view, "button", "New tab");
    let last = bounds(&mut visual, &view, "tab", "Tab 11");
    assert!(last.x > list.x + list.width, "the list overflows: {last:?}");
    assert!(
        add.x + add.width <= 300.5 && (add.x - (list.x + list.width)).abs() < 0.5,
        "pinned at the end: {add:?}, list {list:?}"
    );
}

#[gpui::test]
fn slots_sit_on_the_content_edge_of_an_unfilled_strip(cx: &mut TestAppContext) {
    let source = r#"import "components/tabs" as tabs;
fn changed(ctx,value){}
fn view(ctx){tabs::Tabs(#{label:"Repos",value:"a",variant:"underline",
    tabs:[#{value:"a",label:"Alpha",content:text("Alpha panel")},#{value:"b",label:"Beta",content:text("Beta panel")}],
    start:[text("Prev").accessibility_role("button").accessibility_label("Prev")],
    end:[text("Remove").accessibility_role("button").accessibility_label("Remove")],
    on_change:Fn("changed")}).with_style(style().width(px(480)))}"#
        .to_owned();
    let (mut visual, view) = mount(cx, source);
    let panel = bounds(&mut visual, &view, "text", "Alpha panel");
    let start = bounds(&mut visual, &view, "button", "Prev");
    let end = bounds(&mut visual, &view, "button", "Remove");
    let root = bounds(&mut visual, &view, "group", "Repos");
    assert!(
        (start.x - panel.x).abs() < 0.5,
        "the first control starts on the panel's edge: {start:?} over {panel:?}"
    );
    assert!(
        (end.x + end.width - (root.x + root.width)).abs() < 0.5,
        "the last control ends on the far edge: {end:?} in {root:?}"
    );
}

#[gpui::test]
fn close_successor_prefers_the_next_enabled_tab(cx: &mut TestAppContext) {
    let source = r#"import "components/tabs" as tabs;
fn view(ctx){
    let items=[#{value:"a",label:"A"},#{value:"b",label:"B",disabled:true},#{value:"c",label:"C"}];
    text(`${tabs::close_successor(items,"a")}|${tabs::close_successor(items,"c")}|${tabs::close_successor([#{value:"a",label:"A"}],"a")}|${tabs::close_successor(items,"z")}`)
        .test_id("log")
}"#
    .to_owned();
    let (mut visual, view) = mount(cx, source);
    assert_eq!(log(&mut visual, &view), "c|a||");
}

/// A vertical track of four closable, reorderable tabs with an add button.
fn vertical() -> String {
    r#"import "components/tabs" as tabs;
fn state_schema(){#{fields:#{
    value:#{schema:#{type:"string"},"default":#{type:"string",value:"t0"}},
    log:#{schema:#{type:"string"},"default":#{type:"string",value:""}}}}}
fn note(ctx,line){ctx.set_state("log",`${ctx.get_state("log")} ${line}`);}
fn changed(ctx,value){ctx.set_state("value",value);note(ctx,`change:${value}`);}
fn closed(ctx,request){note(ctx,`close:${request.value}>${request.next}`);}
fn moved(ctx,request){note(ctx,`move:${request.value}:${request.placement}:${request.anchor}`);}
fn added(ctx,payload){note(ctx,"add");}
fn items(){let result=[];for index in 0..4{result.push(#{value:`t${index}`,label:`Box ${index}`,closable:true,
    count:index,content:text(`Panel ${index}`)});}result}
fn view(ctx){column([
    tabs::Tabs(#{key:"boxes",label:"Boxes",value:ctx.get_state("value"),tabs:items(),orientation:"vertical",
        reorderable:true,on_change:Fn("changed"),on_close:Fn("closed"),on_reorder:Fn("moved"),on_add:Fn("added"),
        add_label:"New box"}).with_style(style().width(px(420)).height(px(240))),
    text(ctx.get_state("log")).test_id("log"),
])}"#
        .to_owned()
}

#[gpui::test]
fn a_vertical_list_selects_moves_closes_and_adds(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, vertical());
    assert_eq!(last_error(&mut visual, &view), None);
    let list = bounds(&mut visual, &view, "tablist", "Boxes");
    let add = bounds(&mut visual, &view, "button", "New box");
    assert!(
        (add.y - (list.y + list.height)).abs() < 0.5,
        "the add button follows the last tab: {add:?} under {list:?}"
    );
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    visual.simulate_keystrokes("down");
    settle(&mut visual);
    visual.simulate_keystrokes("alt-down");
    settle(&mut visual);
    visual.simulate_keystrokes("delete");
    settle(&mut visual);
    assert_eq!(
        log(&mut visual, &view),
        "change:t1 move:t1:after:t2 close:t1>t2"
    );
}
