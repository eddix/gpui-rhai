//! A tab's `head` node draws the tab's content in place of its icon, label and count, while
//! the tab keeps its name (the label), its selection, its keys and its close button (#217).

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{MotionPreference, ScriptViewHandle};

use crate::support::{Mount, Step, bounds, center, last_error, settle, snapshot, text_of};

const SOURCE: &str = r#"import "components/tabs" as tabs;
fn state_schema(){#{fields:#{
    tab:#{schema:#{type:"string"},"default":#{type:"string",value:"albums"}},
    log:#{schema:#{type:"string"},"default":#{type:"string",value:""}}}}}
fn chose(ctx,value){ctx.set_state("tab",value);ctx.set_state("log",`${ctx.get_state("log")} change:${value}`);}
fn closed(ctx,payload){ctx.set_state("log",`${ctx.get_state("log")} close:${payload.value}`);}
fn head(title,count){
    column([text(title).test_id(`head-${title}`),text(`${count} items`).test_id(`count-${title}`)])
}
fn view(ctx){ column([
    tabs::Tabs(#{key:"library",label:"Library",value:ctx.get_state("tab"),variant:"underline",
        tabs:[#{value:"albums",label:"Albums",head:head("Albums",128),closable:true},
              #{value:"artists",label:"Artists",head:head("Artists",42)},
              #{value:"songs",label:"Songs"}],
        on_change:Fn("chose"),on_close:Fn("closed")}),
    text(ctx.get_state("log")).test_id("log"),
]) }"#;

fn mount(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(SOURCE)
        .motion(MotionPreference::None)
        .id("tabs-head")
        .mount(cx);
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

#[gpui::test]
fn a_head_draws_the_tab_and_the_tab_keeps_its_behavior(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx);
    // The heads draw two lines; the tabs keep their labels as names.
    assert_eq!(text_of(&mut visual, &view, "count-Albums"), "128 items");
    let albums = bounds(&mut visual, &view, "tab", "Albums");
    let line = bounds(&mut visual, &view, "text", "128 items");
    assert!(
        line.y + line.height <= albums.y + albums.height + 0.5,
        "the underline strip grows to the head: {albums:?} {line:?}"
    );
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("tab", "Songs")
            .next()
            .is_some(),
        "a tab without a head keeps its label"
    );
    // Selection, keys and close stay the tab's.
    let artists = bounds(&mut visual, &view, "tab", "Artists");
    visual.simulate_click(center(artists), gpui::Modifiers::none());
    settle(&mut visual, Step::Tick, 3);
    visual.simulate_keystrokes("right");
    settle(&mut visual, Step::Tick, 3);
    assert_eq!(
        text_of(&mut visual, &view, "log").trim(),
        "change:artists change:songs"
    );
    let albums = bounds(&mut visual, &view, "tab", "Albums");
    visual.simulate_click(center(albums), gpui::Modifiers::none());
    settle(&mut visual, Step::Tick, 3);
    visual.simulate_keystrokes("delete");
    settle(&mut visual, Step::Tick, 3);
    assert!(
        text_of(&mut visual, &view, "log").ends_with("change:albums close:albums"),
        "{}",
        text_of(&mut visual, &view, "log")
    );
}
