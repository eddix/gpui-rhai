//! A native editor runs its ancestors' script capture handlers before its own
//! key bindings (feedback #27): a completion list above a Textarea or an Input
//! can take Up, Down and Enter with `prevent_default()` or `stop()`, and the
//! editor then neither moves its caret, inserts a line nor submits. When no
//! handler takes the key the editor acts as before, and a key the editor does
//! not bind (Tab) still reaches the handler once.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;

use crate::support::{Mount, by_test_id, dispatch, last_error, nodes, text_of};

fn script(editor: &str) -> String {
    format!(
        r#"import "components/textarea" as textarea;
import "components/input" as input;
fn state_schema(){{#{{fields:#{{
    open:#{{schema:#{{type:"bool"}},"default":#{{type:"bool",value:true}}}},
    text:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:"one\ntwo"}}}},
    trace:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn note(ctx,letter){{ctx.set_state("trace",ctx.get_state("trace")+letter);}}
fn pick(letter,ctx,payload){{
    note(ctx,letter);
    if ctx.get_state("open") {{ event_response().prevent_default().stop() }} else {{ event_response() }}
}}
fn seen(letter,ctx,payload){{ note(ctx,letter); event_response() }}
fn set_text(ctx,value){{ctx.set_state("text",value);}}
fn submitted(ctx,value){{note(ctx,"S");}}
fn close(ctx,payload){{ctx.set_state("open",false);}}
fn view(ctx){{column([
    column([{editor}])
        .on_capture("key:up",Fn("pick").curry("U"))
        .on_capture("key:down",Fn("pick").curry("D"))
        .on_capture("key:enter",Fn("pick").curry("E"))
        .on_capture("key:tab",Fn("seen").curry("T")),
    text(ctx.get_state("trace")).test_id("trace"),
    text("Close").test_id("close").on_click(Fn("close")),
])}}"#
    )
}

fn mount(cx: &mut TestAppContext, editor: &str) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(script(editor))
        .motion(MotionPreference::None)
        .id("capture")
        .mount(cx);
    visual.run_until_parked();
    assert_eq!(last_error(&mut visual, &view), None);
    visual.update(|window, cx| window.focus_next(cx));
    visual.run_until_parked();
    (visual, view)
}

fn keys(visual: &mut VisualTestContext, keystrokes: &str) {
    visual.simulate_keystrokes(keystrokes);
    visual.run_until_parked();
}

fn field(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) -> UiValue {
    nodes(visual, view)
        .into_iter()
        .find(|node| node.name == name && node.value.is_some())
        .and_then(|node| node.value)
        .expect("native editor value")
}

fn trace(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    text_of(visual, view, "trace")
}

const TEXTAREA: &str = r#"textarea::Textarea(#{key:"draft",label:"Draft",value:ctx.get_state("text"),
    rows:3,on_change:Fn("set_text"),on_submit:Fn("submitted")})"#;

#[gpui::test]
fn a_completion_list_takes_the_arrows_and_enter_from_a_textarea(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, TEXTAREA);
    // The caret ends the text. Taken: Up keeps it on the last line, Enter adds no line.
    keys(&mut visual, "up enter down");
    visual.simulate_input("!");
    visual.run_until_parked();
    assert_eq!(trace(&mut visual, &view), "UED");
    assert_eq!(field(&mut visual, &view, "Draft"), UiValue::String("one\ntwo!".into()));

    // With the list closed the handlers still see the keys, and the editor acts on them.
    // An automation click leaves the focus in the editor.
    dispatch(&mut visual, &view, by_test_id("close"), "click").unwrap();
    visual.run_until_parked();
    keys(&mut visual, "up");
    visual.simulate_input("^");
    visual.run_until_parked();
    assert_eq!(trace(&mut visual, &view), "UEDU");
    let value = field(&mut visual, &view, "Draft");
    assert_ne!(value, UiValue::String("one\ntwo!^".into()), "Up moved the caret: {value:?}");
}

#[gpui::test]
fn a_key_the_editor_does_not_bind_reaches_the_handler_once(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, TEXTAREA);
    keys(&mut visual, "tab");
    assert_eq!(trace(&mut visual, &view), "T");
}

#[gpui::test]
fn enter_taken_from_an_input_does_not_submit(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        r#"input::Input(#{key:"query",label:"Query",value:"rollout",
            on_change:Fn("set_text"),on_submit:Fn("submitted")})"#,
    );
    keys(&mut visual, "enter");
    assert_eq!(trace(&mut visual, &view), "E", "taken, so not submitted");
}
