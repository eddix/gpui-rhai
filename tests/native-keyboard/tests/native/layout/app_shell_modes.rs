//! AppShell's side regions take the place the application chooses (#218): a
//! docked inspector beside the main area, a hidden one out of layout and out of
//! F6, and a drawer over a scrim that Escape or a press outside asks to close,
//! after which focus returns where it was. The inspector's content keeps its
//! state through every change.

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::*;
use gpui_rhai_registry::{BUNDLED_LAYOUT_SOURCES_BY_ID, BUNDLED_PATTERN_SOURCES_BY_ID};

use crate::support::{Mount, Step, bounds, center, last_error, settle, snapshot, text_of};

/// A component with its own count, to see the inspector's content keep its state.
const COUNTER: &str = r#"/* gpui-rhai
{"id":"components/counter","export":"Counter","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button"],"capabilities":{},"tokens":[],"environment":[]}
*/
import "components/button" as button;
define_component(#{metadata:#{id:"components/counter","export":"Counter",version:"0.1.0",
    runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/button"],capabilities:#{},
    tokens:[],environment:[]},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity."}},
        state:#{fields:#{count:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("render_Counter")});
fn Counter(props){render_component("components/counter",props)}
fn bump(ctx,payload){ctx.set_state("count",ctx.get_state("count")+1);}
fn render_Counter(ctx,props){
    button::Button(#{key:`${props.key}-button`,text:`Count ${ctx.get_state("count")}`,on_click:Fn("bump")})
}
"#;

const MAIN: &str = r#"import "patterns/app_shell" as app_shell;
import "components/button" as button;
import "components/counter" as counter;
fn state_schema(){#{fields:#{
    mode:#{schema:#{type:"string"},"default":#{type:"string",value:"dock"}},
    log:#{schema:#{type:"string"},"default":#{type:"string",value:""}}}}}
fn note(ctx,line){ctx.set_state("log",`${ctx.get_state("log")} ${line}`);}
fn set_mode(mode,ctx,payload){ctx.set_state("mode",mode);note(ctx,mode);}
fn dismissed(ctx,payload){ctx.set_state("mode","hidden");note(ctx,"dismiss");}
fn nav(ctx,payload){note(ctx,"nav");}
fn view(ctx){
    app_shell::AppShell(#{key:"shell",label:"Tool",
        sidebar:column([button::Button(#{key:"nav",text:"Nav",on_click:Fn("nav")})]),
        main:column([
            button::Button(#{key:"dock",text:"Dock",on_click:Fn("set_mode").curry("dock")}),
            button::Button(#{key:"hide",text:"Hide",on_click:Fn("set_mode").curry("hidden")}),
            button::Button(#{key:"open",text:"Open",on_click:Fn("set_mode").curry("overlay")}),
            text(ctx.get_state("log")).test_id("log"),
        ]),
        inspector:column([counter::Counter(#{key:"count"})]),
        sidebar_width:px(160),inspector_width:px(240),
        inspector_mode:ctx.get_state("mode"),on_inspector_dismiss:Fn("dismissed")})
}"#;

fn mount(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(MAIN)
        .modules(BUNDLED_LAYOUT_SOURCES_BY_ID)
        .modules(BUNDLED_PATTERN_SOURCES_BY_ID)
        .module("components/counter", COUNTER)
        .motion(MotionPreference::None)
        .id("shell")
        .mount(cx);
    visual.simulate_resize(size(px(900.0), px(500.0)));
    settle(&mut visual, Step::Refresh, 3);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

fn press(visual: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>) {
    visual.simulate_mouse_move(at, None, Modifiers::none());
    visual.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    visual.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    settle(visual, Step::Refresh, 3);
}

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) {
    let target = bounds(visual, view, "button", name);
    press(visual, center(target));
}

fn keys(visual: &mut VisualTestContext, keystrokes: &str) {
    visual.simulate_keystrokes(keystrokes);
    settle(visual, Step::Refresh, 3);
}

fn shows(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) -> bool {
    snapshot(visual, view)
        .find_by_role_and_name(role, name)
        .next()
        .is_some()
}

fn log(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    text_of(visual, view, "log").trim().to_owned()
}

#[gpui::test]
fn the_inspector_keeps_its_state_docked_hidden_and_in_a_drawer(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx);
    click(&mut visual, &view, "Count 0");
    click(&mut visual, &view, "Count 1");
    assert!(shows(&mut visual, &view, "button", "Count 2"));

    click(&mut visual, &view, "Hide");
    assert!(
        !shows(&mut visual, &view, "button", "Count 2"),
        "a hidden inspector is out of view"
    );
    assert!(
        !shows(&mut visual, &view, "dialog", "Tool inspector"),
        "and no drawer"
    );

    click(&mut visual, &view, "Open");
    let drawer = bounds(&mut visual, &view, "dialog", "Tool inspector");
    let viewport = visual.update(|window, _| f64::from(window.viewport_size().width));
    assert!((drawer.width - 240.0).abs() < 1.0, "{drawer:?}");
    assert!(
        (drawer.x + drawer.width - viewport).abs() < 1.0,
        "at the end edge: {drawer:?}"
    );
    assert!(shows(&mut visual, &view, "button", "Count 2"));

    click(&mut visual, &view, "Dock");
    assert_eq!(
        last_error(&mut visual, &view),
        None,
        "the drawer is modal: Dock is behind its scrim"
    );
    // The press on the scrim asked to close the drawer; Dock was not pressed.
    assert_eq!(log(&mut visual, &view), "hidden overlay dismiss");
    click(&mut visual, &view, "Dock");
    let inspector = bounds(&mut visual, &view, "button", "Count 2");
    assert!(
        inspector.x > 600.0,
        "docked at the end again: {inspector:?}"
    );
}

#[gpui::test]
fn escape_closes_the_drawer_and_focus_returns_to_the_opener(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx);
    click(&mut visual, &view, "Hide");
    // Focus the Open button from the keyboard, then open the drawer with Enter.
    let open = bounds(&mut visual, &view, "button", "Open");
    press(&mut visual, center(open));
    keys(&mut visual, "escape");
    assert_eq!(log(&mut visual, &view), "hidden overlay dismiss");
    // Focus is back on Open: Enter opens the drawer again.
    keys(&mut visual, "enter");
    assert_eq!(log(&mut visual, &view), "hidden overlay dismiss overlay");
    assert!(shows(&mut visual, &view, "dialog", "Tool inspector"));
}

#[gpui::test]
fn f6_skips_a_hidden_inspector(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx);
    click(&mut visual, &view, "Hide");
    // From the main area F6 goes to the sidebar, and on to the main area again: Tab
    // then enters the region and Enter presses its first control.
    for step in ["f6", "tab", "enter", "f6", "tab", "enter"] {
        keys(&mut visual, step);
    }
    assert_eq!(log(&mut visual, &view), "hidden nav dock");
}
