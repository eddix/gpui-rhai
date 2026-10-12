//! Key handlers match the same way in every phase: a modifier-qualified name
//! (`key:shift+f6`) wins and a plain name (`key:f6`) still fires whatever
//! modifiers are held, for capture on an ancestor, the focused target, and
//! bubble on an ancestor alike.

use gpui::TestAppContext;

use crate::support::{Mount, text_of};

const THEME: &str = r#"fn theme(){#{family:"Keys",name:"Dark",mode:"dark",tokens:#{}}}"#;

/// Mount a parent with capture and bubble handlers around a focusable child
/// with a target handler, press `keystroke`, and return the trace.
fn trace(cx: &mut TestAppContext, parent: &str, child: &str, keystroke: &str) -> String {
    let source = format!(
        r#"fn state_schema(){{#{{fields:#{{trace:#{{schema:#{{type:"string"}},"default":#{{type:"string",value:""}}}}}}}}}}
fn mark(letter,ctx,p){{ctx.set_state("trace",ctx.get_state("trace")+letter);event_response()}}
fn stop(letter,ctx,p){{ctx.set_state("trace",ctx.get_state("trace")+letter);event_response().stop()}}
fn view(ctx){{column([
    column([text("Child").tab_stop(true){child}]){parent},
    text(ctx.get_state("trace")).test_id("trace"),
])}}"#
    );
    let (mut visual, view) = Mount::new(source)
        .theme(THEME)
        .no_token_base()
        .id("keys")
        .mount(cx);
    visual.update(|window, cx| window.focus_next(cx));
    visual.simulate_keystrokes(keystroke);
    visual.run_until_parked();
    text_of(&mut visual, &view, "trace")
}

#[gpui::test]
fn every_phase_matches_qualified_names_first_and_plain_names_as_fallback(cx: &mut TestAppContext) {
    let capture = |name: &str| format!(r#".on_capture("key:{name}", Fn("mark").curry("C"))"#);
    let bubble = |name: &str| format!(r#".on_bubble("key:{name}", Fn("mark").curry("B"))"#);
    let target = |name: &str| format!(r#".on("key:{name}", Fn("mark").curry("T"))"#);
    let cases = [
        // Qualified names in every phase.
        (
            format!("{}{}", capture("shift+f6"), bubble("shift+f6")),
            target("shift+f6"),
            "shift-f6",
            "CTB",
        ),
        // Plain names fire whatever modifiers are held, in every phase.
        (
            format!("{}{}", capture("f6"), bubble("f6")),
            target("f6"),
            "shift-f6",
            "CTB",
        ),
        // A qualified name does not fire without its modifier.
        (
            format!("{}{}", capture("shift+f6"), bubble("shift+f6")),
            target("shift+f6"),
            "f6",
            "",
        ),
        // Stop in capture keeps the event from the target and the bubble.
        (
            format!(
                r#".on_capture("key:shift+f6", Fn("stop").curry("C")){}"#,
                bubble("shift+f6")
            ),
            target("shift+f6"),
            "shift-f6",
            "C",
        ),
        // A disabled target is skipped; the ancestor still sees the key.
        (
            format!("{}{}", capture("shift+f6"), bubble("shift+f6")),
            format!("{}.disabled(true)", target("shift+f6")),
            "shift-f6",
            "CB",
        ),
    ];
    let mut report = Vec::new();
    for (parent, child, keystroke, expected) in &cases {
        let found = trace(cx, parent, child, keystroke);
        report.push(format!(
            "{keystroke} parent{parent} child{child}: {found:?} (expected {expected:?}){}",
            if found == *expected {
                ""
            } else {
                "  <- MISMATCH"
            }
        ));
    }
    let text = report.join("\n");
    println!("{text}");
    assert!(!text.contains("MISMATCH"), "{text}");
}
