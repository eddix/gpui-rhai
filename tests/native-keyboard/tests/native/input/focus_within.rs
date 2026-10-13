//! `focus_within_change`: a node hears keyboard focus enter its subtree and leave it, whatever
//! element inside holds focus, and nothing while focus moves inside it. The buttons here are
//! plain click targets, with no ref or focus style of their own.

use gpui::{TestAppContext, VisualTestContext};

use crate::support::{Mount, Step, last_error, settle, text_of};

const SOURCE: &str = r#"
fn state_schema() { #{ fields: #{
    log: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
    plain: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } }
} } }
fn noop(ctx, payload) {}
fn valued(ctx, payload) { ctx.set_state("log", `${ctx.get_state("log")}${payload.value}=${payload.focused} `); }
fn plain(ctx, focused) { ctx.set_state("plain", `${ctx.get_state("plain")}${focused} `); }
fn button(label) {
    text(label).on_click(Fn("noop")).accessibility_role("button").accessibility_label(label)
}
fn view(ctx) {
    column([
        button("Before"),
        column([button("One"), button("Two"), text("Inside").test_id("inside")])
            .on_focus_within_value(Fn("valued"), "group")
            .on_focus_within_change(Fn("plain")),
        button("After"),
        text(ctx.get_state("log")).test_id("log"),
        text(ctx.get_state("plain")).test_id("plain"),
    ])
}
"#;

fn step(visual: &mut VisualTestContext, forward: bool) {
    visual.update(|window, cx| {
        if forward {
            window.focus_next(cx);
        } else {
            window.focus_prev(cx);
        }
    });
    // Focus events come after the frame that shows the new focus.
    settle(visual, Step::Frame, 2);
}

#[gpui::test]
fn focus_within_reports_entering_and_leaving_a_subtree(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(SOURCE).id("focus-within").mount(&mut *cx);
    // GPUI reports focus paths only for an active window; a test window starts inactive.
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 2);
    // Tab to Before, One, Two and After, then Shift+Tab back to Two.
    let mut seen = Vec::new();
    for forward in [true, true, true, true, false] {
        step(&mut visual, forward);
        seen.push(text_of(&mut visual, &view, "log"));
    }
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(
        seen,
        [
            "",
            "group=true ",
            "group=true ",
            "group=true group=false ",
            "group=true group=false group=true ",
        ]
    );
    assert_eq!(text_of(&mut visual, &view, "plain"), "true false true ");

    // The window losing focus takes it out of the subtree too.
    visual.deactivate_window();
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(
        text_of(&mut visual, &view, "plain"),
        "true false true false "
    );
}

#[gpui::test]
fn pressing_the_node_itself_focuses_it(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::new(SOURCE).id("focus-within").mount(&mut *cx);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 2);
    let inside = crate::support::node_by_test_id(&mut visual, &view, "inside")
        .geometry
        .unwrap()
        .visual;
    crate::support::click_center(&mut visual, inside);
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(text_of(&mut visual, &view, "log"), "group=true ");
}
