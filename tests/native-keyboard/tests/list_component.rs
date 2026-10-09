//! List (#119): keyed rows on the row inset with Table's selection model. A click selects
//! and opens a row, the list is one tab stop whose arrows move the selection and whose
//! Enter opens the row, disabled rows take no input and are skipped, a right press asks
//! for a context menu, and the roles follow the selection mode.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, VisualTestContext,
    Window, point, px,
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
    let source = script("single", "").replace(
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
