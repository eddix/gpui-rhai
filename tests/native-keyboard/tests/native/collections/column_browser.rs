//! One authoritative browser keyboard cursor, controlled paths, isolated statuses
//! and removal while focus is in a descendant. Loading/race policy stays in caller.
use crate::support::{
    Mount, ReplaceOnce, Step, by_role, dispatch, last_error, settle, snapshot, text_of,
};
use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;

const APP: &str = r#"
import "components/column_browser" as browser;
fn state_schema() { #{fields:#{
    path:#{schema:#{type:"array",items:#{type:"string"}},"default":#{type:"array",value:[#{type:"string",value:"a"}]}},
    log:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
    loading:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
}} }
fn chosen(ctx,event) { ctx.set_state("path",event.path); ctx.set_state("log",`${event.column}:${event.key}`); }
fn opened(ctx,event) { ctx.set_state("log",`activate:${event.column}:${event.key}`); }
fn remove_child(ctx,event) { ctx.set_state("path",[]); }
fn load_child(ctx,event) { ctx.set_state("loading",false); }
fn load_for(expected,ctx,event) { let path=ctx.get_state("path");
    if path.len>0 && path[0]==expected { ctx.set_state("loading",false); } }
fn view(ctx) {
    let path=ctx.get_state("path");
    let columns=[#{key:"root",title:"Root",items:[
        #{key:"a",title:"Alpha",has_children:true},
        #{key:"disabled",title:"Disabled",disabled:true},
        #{key:"b",title:"Bravo",has_children:true}]}];
    if path.len>0 { columns.push(#{key:`children-${path[0]}`,title:"Child",loading:ctx.get_state("loading"),items:[
        #{key:"x",title:"X ray"},#{key:"y",title:"Yankee"}]}); }
    column([
        browser::ColumnBrowser(#{key:"config",label:"Configuration",height:240,path:path,columns:columns,
            show_path:false,on_select:Fn("chosen"),on_activate:Fn("opened")}),
        text(`${path}|${ctx.get_state("log")}`).test_id("state"),
        text("Remove child").with_key("remove").on_click(Fn("remove_child")),
        text("Load child").with_key("load").on_click(Fn("load_child")),
        text("Late Alpha result").with_key("late-a").on_click(Fn("load_for").curry("a")),
        text("Current Bravo result").with_key("result-b").on_click(Fn("load_for").curry("b")),
    ]).with_style(style().width(px(400)))
}
"#;

fn press(visual: &mut VisualTestContext, key: &str) {
    visual.simulate_keystrokes(key);
    settle(visual, Step::Tick, 2);
}

#[gpui::test]
fn caller_revision_guard_discards_a_late_result_for_a_replaced_column(cx: &mut TestAppContext) {
    let source = APP.replace_once("type:\"bool\",value:false", "type:\"bool\",value:true");
    let (mut visual, view) = Mount::components(source).id("browser-race").mount(cx);
    settle(&mut visual, Step::Tick, 2);
    visual.update(|window, cx| window.focus_next(cx));
    press(&mut visual, "down");
    assert!(text_of(&mut visual, &view, "state").contains("root:b"));
    press(&mut visual, "right");
    dispatch(
        &mut visual,
        &view,
        AutomationLocator::Text {
            text: "Late Alpha result".into(),
        },
        "click",
    )
    .unwrap();
    settle(&mut visual, Step::Tick, 2);
    press(&mut visual, "down");
    assert!(text_of(&mut visual, &view, "state").contains("root:b"));
    dispatch(
        &mut visual,
        &view,
        AutomationLocator::Text {
            text: "Current Bravo result".into(),
        },
        "click",
    )
    .unwrap();
    settle(&mut visual, Step::Tick, 2);
    press(&mut visual, "down");
    assert!(text_of(&mut visual, &view, "state").contains("children-b:x"));
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn browser_arrows_select_rows_switch_columns_and_activate_only_leaves(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(APP).id("browser-keys").mount(cx);
    settle(&mut visual, Step::Tick, 2);
    assert_eq!(last_error(&mut visual, &view), None);
    visual.update(|window, cx| window.focus_next(cx));
    press(&mut visual, "down");
    assert!(text_of(&mut visual, &view, "state").contains("root:b"));
    press(&mut visual, "right");
    assert!(text_of(&mut visual, &view, "state").contains("children-b:x"));
    press(&mut visual, "end");
    assert!(text_of(&mut visual, &view, "state").contains("children-b:y"));
    press(&mut visual, "enter");
    assert!(text_of(&mut visual, &view, "state").contains("activate:children-b:y"));
    press(&mut visual, "left");
    press(&mut visual, "home");
    assert!(text_of(&mut visual, &view, "state").contains("root:a"));
    press(&mut visual, "enter");
    assert!(!text_of(&mut visual, &view, "state").contains("activate:"));
    let selected = snapshot(&mut visual, &view)
        .nodes()
        .filter(|node| node.role == "option" && node.selected == Some(true))
        .count();
    assert_eq!(selected, 1);
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn rtl_arrows_follow_logical_depth(cx: &mut TestAppContext) {
    let source = APP
        .replace_once(
            "fn view(ctx)",
            "fn init(ctx) { ctx.set_locale(\"ar\"); } fn view(ctx)",
        )
        .replace_once(
            "text(`${path}|${ctx.get_state(\"log\")}`)",
            "text(`${path}|${ctx.get_state(\"log\")}|${ctx.text_direction()}`)",
        );
    let (mut visual, view) = Mount::components(source)
        .id("browser-rtl")
        .locale("en.rhai", gpui_rhai_registry::EN_LOCALE)
        .locale("ar.rhai", gpui_rhai_registry::AR_LOCALE)
        .mount(cx);
    settle(&mut visual, Step::Tick, 2);
    visual.update(|window, cx| window.focus_next(cx));
    press(&mut visual, "left");
    let state = text_of(&mut visual, &view, "state");
    assert!(
        state.contains("children-a:x"),
        "{state}; error={:?}",
        last_error(&mut visual, &view)
    );
    press(&mut visual, "right");
    press(&mut visual, "end");
    assert!(text_of(&mut visual, &view, "state").contains("root:b"));
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn loading_and_removed_columns_do_not_keep_a_second_keyboard_cursor(cx: &mut TestAppContext) {
    let source = APP.replace_once("type:\"bool\",value:false", "type:\"bool\",value:true");
    let (mut visual, view) = Mount::components(source).id("browser-loading").mount(cx);
    settle(&mut visual, Step::Tick, 2);
    visual.update(|window, cx| window.focus_next(cx));
    press(&mut visual, "right");
    press(&mut visual, "down");
    assert!(text_of(&mut visual, &view, "state").ends_with('|'));
    dispatch(
        &mut visual,
        &view,
        AutomationLocator::Text {
            text: "Load child".into(),
        },
        "click",
    )
    .unwrap();
    settle(&mut visual, Step::Tick, 2);
    press(&mut visual, "down");
    assert!(text_of(&mut visual, &view, "state").contains("children-a:x"));
    dispatch(
        &mut visual,
        &view,
        AutomationLocator::Text {
            text: "Remove child".into(),
        },
        "click",
    )
    .unwrap();
    settle(&mut visual, Step::Tick, 2);
    press(&mut visual, "down");
    assert!(text_of(&mut visual, &view, "state").contains("root:a"));
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn path_copy_is_post_commit_and_failed_callback_preserves_the_clipboard(cx: &mut TestAppContext) {
    let source=APP.replace_once("show_path:false","show_path:true")
        .replace_once("fn view(ctx)","fn failed_copy(ctx,event) { ctx.copy_text(\"rolled back\"); throw \"copy failed\"; } fn view(ctx)")
        .replace_once("text(\"Load child\").with_key(\"load\").on_click(Fn(\"load_child\")),",
            "text(\"Load child\").with_key(\"load\").on_click(Fn(\"load_child\")), text(\"Failed copy\").with_key(\"bad-copy\").on_click(Fn(\"failed_copy\")),");
    let (mut visual, view) = Mount::components(source).id("browser-copy").mount(cx);
    settle(&mut visual, Step::Tick, 2);
    dispatch(&mut visual, &view, by_role("button", "Copy path"), "click").unwrap();
    settle(&mut visual, Step::Tick, 2);
    let copied = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(copied.as_deref(), Some("a"));
    let _ = dispatch(
        &mut visual,
        &view,
        AutomationLocator::Text {
            text: "Failed copy".into(),
        },
        "click",
    );
    settle(&mut visual, Step::Tick, 2);
    assert!(last_error(&mut visual, &view).is_some());
    let copied = visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()));
    assert_eq!(copied.as_deref(), Some("a"));
}
