//! List (#119): keyed rows on the row inset with Table's selection model. A click selects
//! and opens a row, the list is one tab stop whose arrows move the selection and whose
//! Enter opens the row, disabled rows take no input and are skipped, a right press asks
//! for a context menu (Shift+F10 too, for the whole list without a selection mode), and
//! the roles follow the selection mode.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, point, px,
};
use gpui_rhai::*;

use crate::support::ReplaceOnce;

struct Host {
    host: ScriptViewHost,
    view: ScriptViewHandle,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.host.container(self.view.element().unwrap())
    }
}

fn script(mode: &str, extra: &str) -> String {
    format!(
        r#"import "components/list" as list;
fn state_schema(){{#{{fields:#{{
    selected:#{{schema:#{{type:"array",items:#{{type:"string"}}}},"default":#{{type:"array",value:[]}}}},
    log:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn selected(ctx,keys){{ctx.set_state("selected",keys);
    ctx.set_state("log",`${{ctx.get_state("log")}} sel:${{keys}}`);}}
fn opened(ctx,key){{ctx.set_state("log",`${{ctx.get_state("log")}} open:${{key}}`);}}
fn asked(ctx,request){{ctx.set_state("log",`${{ctx.get_state("log")}} ctx:${{request.key}}:${{request.source}}`);}}
fn view(ctx){{column([
    list::List(#{{key:"tickets",label:"Tickets",selection_mode:"{mode}",height:200.0,
        selected_keys:ctx.get_state("selected"),
        items:[#{{key:"a",title:"Alpha",meta:"09:41",badge:#{{text:"Run",variant:"accent"}}}},
            #{{key:"b",title:"Bravo",secondary:"T2",disabled:true}},
            #{{key:"c",title:"Charlie",secondary:"T3"}}],
        on_selection_change:Fn("selected"),on_row_click:Fn("opened"),on_context_request:Fn("asked")
        {extra}}}),
    text(ctx.get_state("log")).test_id("log"),
]).with_style(style().width(px(400)))}}"#
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
        let host = ScriptViewHost::new("list", cx).unwrap();
        let view = prepared
            .mount(ScriptViewConfig::new("list"), host.clone(), window, cx)
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
    visual.run_until_parked();
    visual.executor().advance_clock(Duration::from_millis(32));
    visual.update(|window, cx| window.simulate_next_frame(cx));
    visual.run_until_parked();
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

/// (role, name, bounds) of every row.
fn rows(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
) -> Vec<(String, String, GeometryBounds)> {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .filter(|node| node.role == "option" || node.role == "listitem")
            .map(|node| {
                (
                    node.role.clone(),
                    node.name.clone(),
                    node.geometry.unwrap().visual,
                )
            })
            .collect()
    })
}

fn press(visual: &mut VisualTestContext, bounds: GeometryBounds, button: MouseButton) {
    #[allow(clippy::cast_possible_truncation)]
    let position = point(
        px((bounds.x + bounds.width / 2.0) as f32),
        px((bounds.y + bounds.height / 2.0) as f32),
    );
    visual.simulate_mouse_move(position, None, Modifiers::none());
    visual.simulate_mouse_down(position, button, Modifiers::none());
    visual.simulate_mouse_up(position, button, Modifiers::none());
    settle(visual);
}

#[gpui::test]
fn rows_are_options_named_by_their_text_and_start_on_the_inset(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script("single", ""));
    let found = rows(&mut visual, &view);
    let names = found
        .iter()
        .map(|(role, name, _)| format!("{role}:{name}"))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "option:Alpha, 09:41",
            "option:Bravo, T2",
            "option:Charlie, T3"
        ]
    );
    let title = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("text", "Charlie")
            .next()
            .and_then(|node| node.geometry)
            .unwrap()
            .visual
    });
    // Comfortable metrics.inset.
    assert!(
        (title.x - found[2].2.x - 12.0).abs() < 0.5,
        "title at {}",
        title.x
    );
    // Without a selection mode the rows are list items.
    let (mut visual, view) = mount(cx, script("none", ""));
    let roles = rows(&mut visual, &view)
        .into_iter()
        .map(|(role, ..)| role)
        .collect::<Vec<_>>();
    assert_eq!(roles, ["listitem", "listitem", "listitem"]);
}

#[gpui::test]
fn a_click_selects_and_opens_and_a_disabled_row_does_nothing(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script("single", ""));
    let found = rows(&mut visual, &view);
    press(&mut visual, found[2].2, MouseButton::Left);
    assert_eq!(log(&mut visual, &view), r#"open:c sel:["c"]"#);
    press(&mut visual, found[1].2, MouseButton::Left);
    assert_eq!(
        log(&mut visual, &view),
        r#"open:c sel:["c"]"#,
        "disabled row"
    );
}

#[gpui::test]
fn arrows_move_the_selection_past_disabled_rows_and_enter_opens(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script("single", ""));
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    visual.simulate_keystrokes("down");
    settle(&mut visual);
    assert_eq!(
        log(&mut visual, &view),
        r#"sel:["a"]"#,
        "no current row: the first"
    );
    visual.simulate_keystrokes("down");
    settle(&mut visual);
    assert_eq!(
        log(&mut visual, &view),
        r#"sel:["a"] sel:["c"]"#,
        "Bravo is disabled"
    );
    visual.simulate_keystrokes("enter");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), r#"sel:["a"] sel:["c"] open:c"#);
}

#[gpui::test]
fn a_right_press_selects_the_row_and_asks_for_a_menu(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script("single", ""));
    let found = rows(&mut visual, &view);
    press(&mut visual, found[0].2, MouseButton::Right);
    assert_eq!(log(&mut visual, &view), r#"sel:["a"] ctx:a:pointer"#);
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    visual.simulate_keystrokes("shift-f10");
    settle(&mut visual);
    assert!(
        log(&mut visual, &view).ends_with("ctx:a:keyboard"),
        "{}",
        log(&mut visual, &view)
    );
}

#[gpui::test]
fn an_empty_list_shows_its_empty_text(cx: &mut TestAppContext) {
    let source = script("single", "").replace_once(
        r#"items:[#{key:"a",title:"Alpha",meta:"09:41",badge:#{text:"Run",variant:"accent"}},
            #{key:"b",title:"Bravo",secondary:"T2",disabled:true},
            #{key:"c",title:"Charlie",secondary:"T3"}],"#,
        r#"items:[],empty_text:"No tickets","#,
    );
    let (mut visual, view) = mount(cx, source);
    let texts = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .filter(|node| node.role == "text")
            .map(|node| node.name.clone())
            .collect::<Vec<_>>()
    });
    assert!(texts.iter().any(|text| text == "No tickets"), "{texts:?}");
}

#[gpui::test]
fn a_badge_width_lines_the_titles_up(cx: &mut TestAppContext) {
    let title_x = |visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str| {
        visual.update(|_, cx| {
            view.accessibility_snapshot(cx)
                .unwrap()
                .find_by_role_and_name("text", name)
                .next()
                .and_then(|node| node.geometry)
                .unwrap()
                .visual
                .x
        })
    };
    // Alpha has a badge, Charlie none: without a slot their titles start apart.
    let (mut visual, view) = mount(cx, script("single", ""));
    let apart =
        (title_x(&mut visual, &view, "Alpha") - title_x(&mut visual, &view, "Charlie")).abs();
    assert!(apart > 10.0, "titles {apart}px apart without a slot");
    let (mut visual, view) = mount(cx, script("single", ",badge_width:80.0"));
    let alpha = title_x(&mut visual, &view, "Alpha");
    let charlie = title_x(&mut visual, &view, "Charlie");
    assert!(
        (alpha - charlie).abs() < 0.5,
        "titles at {alpha} and {charlie}"
    );
}

#[gpui::test]
fn without_a_selection_mode_shift_f10_asks_for_the_list(cx: &mut TestAppContext) {
    let source = script("none", "").replace_once(
        "ctx:${request.key}:${request.source}",
        "ctx:${request.key}:${request.source}:${request.anchor.x},${request.anchor.y},${request.anchor.width},${request.anchor.height}",
    );
    let (mut visual, view) = mount(cx, source);
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    visual.simulate_keystrokes("shift-f10");
    settle(&mut visual);
    let list = visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .find_by_role_and_name("list", "Tickets")
            .next()
            .and_then(|node| node.geometry)
            .unwrap()
            .visual
    });
    let request = format!(
        "ctx::keyboard:{:?},{:?},{:?},{:?}",
        list.x, list.y, list.width, list.height
    );
    assert_eq!(log(&mut visual, &view), request);
    visual.simulate_keystrokes("menu");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), format!("{request} {request}"));
}

#[gpui::test]
fn a_second_click_clears_a_single_selection_and_multiple_clicks_toggle(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script("single", ""));
    let found = rows(&mut visual, &view);
    press(&mut visual, found[2].2, MouseButton::Left);
    press(&mut visual, found[2].2, MouseButton::Left);
    assert_eq!(log(&mut visual, &view), r#"open:c sel:["c"] open:c sel:[]"#);
    let (mut visual, view) = mount(cx, script("multiple", ""));
    let found = rows(&mut visual, &view);
    press(&mut visual, found[0].2, MouseButton::Left);
    press(&mut visual, found[2].2, MouseButton::Left);
    press(&mut visual, found[0].2, MouseButton::Left);
    assert_eq!(
        log(&mut visual, &view),
        r#"open:a sel:["a"] open:c sel:["a", "c"] open:a sel:["c"]"#
    );
    // A right press on an unselected row selects it alone; on a selected row it keeps the
    // selection.
    press(&mut visual, found[0].2, MouseButton::Right);
    press(&mut visual, found[0].2, MouseButton::Right);
    assert!(
        log(&mut visual, &view).ends_with(r#"sel:["a"] ctx:a:pointer ctx:a:pointer"#),
        "{}",
        log(&mut visual, &view)
    );
}

fn last_error(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Option<String> {
    visual.update(|_, cx| view.last_error(cx).unwrap())
}

/// `count` rows in `multiple` mode, every one selected when `selected` (the round-4 probe).
fn list_source(count: usize, selected: bool) -> String {
    format!(
        r#"import "components/list" as list;
fn view(ctx) {{
    let rows=[];let chosen=[];
    for i in 0..{count} {{let key=`r${{i}}`;rows.push(#{{key:key,title:key}});if {selected}{{chosen.push(key);}}}}
    list::List(#{{key:"rows",label:"Rows",items:rows,height:200.0,
        selection_mode:"multiple",selected_keys:chosen}})
        .with_style(style().width(px(360)))
}}"#
    )
}

#[gpui::test]
fn list_500_unselected_rows_control(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, list_source(500, false));
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "{error:?}");
}

#[gpui::test]
fn list_500_selected_rows_do_not_expand_into_oversized_arrays(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, list_source(500, true));
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "{error:?}");
}

#[gpui::test]
fn list_500_rows_with_20_selected_still_renders(cx: &mut TestAppContext) {
    let source = list_source(500, true).replace_once(
        "if true{chosen.push(key);}",
        "if i < 20 {chosen.push(key);}",
    );
    assert!(source.contains("i < 20"));
    let (mut visual, view) = mount(cx, source);
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "{error:?}");
}

/// 500 rows in `multiple` mode, the selection in state starting at `["r0"]`; "Many" selects
/// r0..r399. The log records each next selection's size, whether it holds r2, and its last key.
fn growing_selection_script() -> String {
    r#"import "components/list" as list;
fn state_schema(){#{fields:#{
    selected:#{schema:#{type:"array",max_items:10000,items:#{type:"string"}},"default":#{type:"array",value:[#{type:"string",value:"r0"}]}},
    log:#{schema:#{type:"string"},"default":#{type:"string",value:""}}}}}
fn note(ctx,line){ctx.set_state("log",`${ctx.get_state("log")} ${line}`);}
fn selected(ctx,keys){ctx.set_state("selected",keys);
    note(ctx,`sel:${keys.len}:${"r2" in keys}:${if keys.len==0 {"-"} else {keys[keys.len-1]}}`);}
fn many(ctx,payload){let keys=[];for i in 0..400{keys.push(`r${i}`);}
    ctx.set_state("selected",keys);note(ctx,"many");}
fn view(ctx){
    let rows=[];
    for i in 0..500{rows.push(#{key:`r${i}`,title:`Row ${i}`});}
    column([
        text("Many").accessibility_role("button").accessibility_label("Many").on_click(Fn("many"))
            .with_style(style().width(px(80)).height(px(24))),
        list::List(#{key:"rows",label:"Rows",items:rows,height:200.0,selection_mode:"multiple",
            selected_keys:ctx.get_state("selected"),on_selection_change:Fn("selected")}),
        text(ctx.get_state("log")).test_id("log"),
    ]).with_style(style().width(px(360)))
}"#
    .to_owned()
}

fn named_bounds(
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

#[gpui::test]
fn clicks_build_the_next_selection_from_few_to_many_selected_rows(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, growing_selection_script());
    let row = named_bounds(&mut visual, &view, "option", "Row 1");
    press(&mut visual, row, MouseButton::Left);
    assert_eq!(log(&mut visual, &view), "sel:2:false:r1");
    let many = named_bounds(&mut visual, &view, "button", "Many");
    press(&mut visual, many, MouseButton::Left);
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "400 of 500 rows selected: {error:?}");
    assert_eq!(log(&mut visual, &view), "sel:2:false:r1 many");
    // A click on a selected row drops it; a second click appends it again.
    let row = named_bounds(&mut visual, &view, "option", "Row 2");
    press(&mut visual, row, MouseButton::Left);
    press(&mut visual, row, MouseButton::Left);
    let error = last_error(&mut visual, &view);
    assert!(error.is_none(), "{error:?}");
    assert_eq!(
        log(&mut visual, &view),
        "sel:2:false:r1 many sel:399:false:r399 sel:400:true:r2"
    );
}

/// The bounds of the list itself (role `listbox` or `list`, named Tickets).
fn list_bounds(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> GeometryBounds {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .find(|node| (node.role == "listbox" || node.role == "list") && node.name == "Tickets")
            .and_then(|node| node.geometry)
            .expect("no list named Tickets")
            .visual
    })
}

fn texts(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
    visual.update(|_, cx| {
        view.accessibility_snapshot(cx)
            .unwrap()
            .nodes()
            .filter(|node| node.role == "text")
            .map(|node| node.name.clone())
            .collect()
    })
}

/// `badge_width` takes a Length (#139b) as well as pixels; both line the titles up alike.
#[gpui::test]
fn a_badge_width_length_lines_the_titles_up_like_pixels(cx: &mut TestAppContext) {
    let title_x = |visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str| {
        named_bounds(visual, view, "text", name).x
    };
    let (mut visual, view) = mount(cx, script("single", ",badge_width:64.0"));
    assert_eq!(last_error(&mut visual, &view), None);
    let pixels = title_x(&mut visual, &view, "Alpha");
    let (mut visual, view) = mount(
        cx,
        script("single", r#",badge_width:theme_length("metrics.row")*2"#),
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let alpha = title_x(&mut visual, &view, "Alpha");
    let charlie = title_x(&mut visual, &view, "Charlie");
    assert!(
        (alpha - charlie).abs() < 0.5,
        "titles at {alpha} and {charlie}"
    );
    assert!(
        (alpha - pixels).abs() < 0.5,
        "metrics.row * 2 is 64px in comfortable density: {alpha} vs {pixels}"
    );
}

/// `height` takes a Length (#139b), resolved in the list's density, and the list keeps its
/// keyboard model.
#[gpui::test]
fn a_height_length_sizes_the_list_in_its_density(cx: &mut TestAppContext) {
    let length = script("single", "")
        .replace_once("height:200.0", r#"height:theme_length("metrics.row")*2"#);
    assert!(length.contains("metrics.row"));
    let (mut visual, view) = mount(cx, length.clone());
    assert_eq!(last_error(&mut visual, &view), None);
    let list = list_bounds(&mut visual, &view);
    assert!(
        (list.height - 64.0).abs() < 0.5,
        "two comfortable rows: {list:?}"
    );
    visual.update(|window, cx| window.focus_next(cx));
    settle(&mut visual);
    visual.simulate_keystrokes("down");
    settle(&mut visual);
    assert_eq!(log(&mut visual, &view), r#"sel:["a"]"#);
    let compact = length.replace_once(
        "]).with_style(style().width(px(400)))",
        r#"]).with_style(style().width(px(400))).env(#{density:"compact"})"#,
    );
    assert!(compact.contains("compact"));
    let (mut visual, view) = mount(cx, compact);
    assert_eq!(last_error(&mut visual, &view), None);
    let list = list_bounds(&mut visual, &view);
    assert!(
        (list.height - 56.0).abs() < 0.5,
        "two compact rows: {list:?}"
    );
    // Without a selection mode a context request keeps its framed wrapper.
    let (mut visual, view) = mount(
        cx,
        length.replace_once(r#"selection_mode:"single""#, r#"selection_mode:"none""#),
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let list = list_bounds(&mut visual, &view);
    assert!((list.height - 64.0).abs() < 0.5, "{list:?}");
}

/// `loading` (#139c) shows `loading_content`, or a muted line, in the list's place and size,
/// as Table does.
#[gpui::test]
fn a_loading_list_shows_its_loading_content_in_place_of_the_rows(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, script("single", ",loading:true"));
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(rows(&mut visual, &view).is_empty(), "no rows while loading");
    let shown = texts(&mut visual, &view);
    assert!(shown.iter().any(|text| text == "Loading…"), "{shown:?}");
    let list = list_bounds(&mut visual, &view);
    assert!(
        (list.height - 200.0).abs() < 0.5,
        "the list's height: {list:?}"
    );
    let (mut visual, view) = mount(
        cx,
        script(
            "single",
            r#",loading:true,loading_content:text("Fetching tickets")"#,
        ),
    );
    assert_eq!(last_error(&mut visual, &view), None);
    let shown = texts(&mut visual, &view);
    assert!(
        shown.iter().any(|text| text == "Fetching tickets")
            && !shown.iter().any(|text| text == "Loading…"),
        "{shown:?}"
    );
    // Loading wins over an empty list.
    let source = script("single", ",loading:true").replace_once(
        r#"items:[#{key:"a",title:"Alpha",meta:"09:41",badge:#{text:"Run",variant:"accent"}},
            #{key:"b",title:"Bravo",secondary:"T2",disabled:true},
            #{key:"c",title:"Charlie",secondary:"T3"}],"#,
        r#"items:[],empty_text:"No tickets","#,
    );
    let (mut visual, view) = mount(cx, source);
    let shown = texts(&mut visual, &view);
    assert!(
        shown.iter().any(|text| text == "Loading…")
            && !shown.iter().any(|text| text == "No tickets"),
        "{shown:?}"
    );
}
