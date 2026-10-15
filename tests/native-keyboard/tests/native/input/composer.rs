//! Submit keys run through the real focused native editor, not script key capture.
use gpui::TestAppContext;
use gpui_rhai::UiValue;
use gpui_rhai_registry::{DEFAULT_LIGHT_THEME, DEFAULT_THEME};

use crate::support::{Mount, ReplaceOnce, bounds, click_center, last_error, nodes, text_of};

const SOURCE: &str = r#"
import "patterns/composer" as composer;
import "components/textarea" as textarea;
import "components/input" as input;
fn state_schema() { #{ fields: #{
    draft: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
    sent: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "none" } },
    busy: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
    stops: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
    sends: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
} } }
fn changed(ctx, value) { ctx.set_state("draft", value); }
fn sent(ctx, value) { ctx.set_state("sends", ctx.get_state("sends") + 1); ctx.set_state("sent", value); ctx.set_state("busy", true); }
fn stop(ctx, payload) { ctx.set_state("busy", false); ctx.set_state("stops", ctx.get_state("stops") + 1); }
fn view(ctx) { column([
    FIELD,
    text(ctx.get_state("draft")).test_id("draft"),
    text(ctx.get_state("sent")).test_id("sent"),
    text(`${ctx.get_state("sends")}`).test_id("sends"),
    text(`${ctx.get_state("busy")}:${ctx.get_state("stops")}`).test_id("state"),
]).with_style(style().width(px(400))) }
"#;
const COMPOSER: &str = r#"composer::Composer(#{ key: "prompt", label: "Prompt", value: ctx.get_state("draft"),
    busy: ctx.get_state("busy"), on_change: Fn("changed"), on_submit: Fn("sent"), on_stop: Fn("stop"), autofocus: true })"#;
const TEXTAREA: &str = r#"textarea::Textarea(#{ key: "prompt", label: "Prompt", value: ctx.get_state("draft"),
    on_change: Fn("changed"), on_submit: Fn("sent"), submit_key: "mod_enter", autofocus: true })"#;

#[gpui::test]
fn composer_enter_shift_enter_busy_stop_and_native_undo(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(SOURCE.replace_once("FIELD", COMPOSER))
        .registry("patterns/composer")
        .mount(cx);
    visual.simulate_input("one");
    visual.simulate_keystrokes("shift-enter");
    visual.simulate_input("two");
    assert_eq!(text_of(&mut visual, &view, "draft"), "one\ntwo");
    visual.simulate_keystrokes("enter");
    visual.run_until_parked();
    assert_eq!(text_of(&mut visual, &view, "sent"), "one\ntwo");
    assert_eq!(text_of(&mut visual, &view, "state"), "true:0");
    visual.simulate_keystrokes("enter");
    visual.simulate_input("!");
    assert_eq!(
        text_of(&mut visual, &view, "sends"),
        "1",
        "busy cannot resubmit"
    );
    assert_eq!(text_of(&mut visual, &view, "draft"), "one\ntwo!");
    visual.simulate_keystrokes("cmd-z");
    // Submission does not insert an edit-history boundary: ordinary adjacent
    // typing remains coalesced exactly as in the native Textarea.
    assert_eq!(text_of(&mut visual, &view, "draft"), "one\n");
    visual.simulate_keystrokes("shift-cmd-z");
    assert_eq!(text_of(&mut visual, &view, "draft"), "one\ntwo!");
    let stop_bounds = bounds(&mut visual, &view, "button", "Stop");
    click_center(&mut visual, stop_bounds);
    assert_eq!(text_of(&mut visual, &view, "state"), "false:1");
    let send_bounds = bounds(&mut visual, &view, "button", "Send");
    click_center(&mut visual, send_bounds);
    visual.simulate_input("x");
    assert_eq!(
        text_of(&mut visual, &view, "draft"),
        "one\ntwo!x",
        "Send restores editor focus"
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn textarea_modifier_submit_and_absent_configuration_preserve_newlines(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(SOURCE.replace_once("FIELD", TEXTAREA))
        .registry("patterns/composer")
        .mount(cx);
    visual.simulate_input("a");
    visual.simulate_keystrokes("enter shift-enter");
    visual.simulate_input("b");
    assert_eq!(text_of(&mut visual, &view, "draft"), "a\n\nb");
    visual.simulate_keystrokes(if cfg!(target_os = "macos") {
        "cmd-enter"
    } else {
        "ctrl-enter"
    });
    assert_eq!(text_of(&mut visual, &view, "sent"), "a\n\nb");
    assert_eq!(text_of(&mut visual, &view, "draft"), "a\n\nb");
    assert_eq!(last_error(&mut visual, &view), None);

    let field = TEXTAREA.replace_once(", submit_key: \"mod_enter\"", "");
    let (mut visual, view) = Mount::components(SOURCE.replace_once("FIELD", &field))
        .registry("patterns/composer")
        .mount(cx);
    visual.simulate_input("a");
    visual.simulate_keystrokes("enter");
    assert_eq!(text_of(&mut visual, &view, "draft"), "a\n");
    assert_eq!(text_of(&mut visual, &view, "sent"), "none");
}

#[gpui::test]
fn readonly_textarea_and_input_never_submit(cx: &mut TestAppContext) {
    for field in [
        TEXTAREA.replace_once("autofocus: true", "read_only: true, autofocus: true"),
        r#"input::Input(#{ key: "reference", label: "Reference", value: "copy me",
            read_only: true, autofocus: true, on_change: Fn("changed"), on_submit: Fn("sent") })"#
            .to_owned(),
    ] {
        let (mut visual, view) = Mount::components(SOURCE.replace_once("FIELD", &field))
            .registry("patterns/composer")
            .mount(cx);
        visual.simulate_keystrokes("enter cmd-enter ctrl-enter");
        visual.simulate_input("blocked");
        assert_eq!(text_of(&mut visual, &view, "sent"), "none");
        assert_eq!(text_of(&mut visual, &view, "draft"), "");
        let fields = nodes(&mut visual, &view);
        assert!(
            fields
                .iter()
                .any(|node| node.role == "text_field" && node.read_only)
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn composer_controlled_clear_keeps_focus_and_caret_editing_native(cx: &mut TestAppContext) {
    let source = SOURCE.replace_once("FIELD", COMPOSER).replace_once(
        "ctx.set_state(\"busy\", true);",
        "ctx.set_state(\"draft\", \"\");",
    );
    let (mut visual, view) = Mount::components(source)
        .registry("patterns/composer")
        .mount(cx);
    visual.simulate_input("abc");
    visual.simulate_keystrokes("left shift-left shift-enter");
    assert_eq!(text_of(&mut visual, &view, "draft"), "a\nc");
    visual.simulate_keystrokes("enter");
    visual.simulate_input("next");
    assert_eq!(text_of(&mut visual, &view, "sent"), "a\nc");
    assert_eq!(text_of(&mut visual, &view, "draft"), "next");
    assert!(
        nodes(&mut visual, &view)
            .iter()
            .any(|node| node.value == Some(UiValue::String("next".into())))
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn composer_validation_blocks_send_without_blocking_drafting(cx: &mut TestAppContext) {
    let field = COMPOSER.replace_once(
        "busy: ctx.get_state(\"busy\")",
        "can_submit: false, busy: ctx.get_state(\"busy\")",
    );
    let (mut visual, view) = Mount::components(SOURCE.replace_once("FIELD", &field))
        .registry("patterns/composer")
        .mount(cx);
    visual.simulate_input("draft");
    visual.simulate_keystrokes("enter");
    assert_eq!(text_of(&mut visual, &view, "draft"), "draft");
    assert_eq!(text_of(&mut visual, &view, "sends"), "0");
    assert!(
        nodes(&mut visual, &view)
            .iter()
            .any(|node| node.role == "button" && node.name == "Send" && node.disabled)
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn composer_native_rows_and_counter_in_both_themes_and_densities(cx: &mut TestAppContext) {
    for theme in [DEFAULT_THEME, DEFAULT_LIGHT_THEME] {
        for density in ["comfortable", "compact"] {
            let field = COMPOSER.replace_once(
                "autofocus: true",
                "autofocus: true, min_rows: 2, max_rows: 3, max_length: 40",
            );
            let source = SOURCE.replace_once("FIELD", &field).replace_once(
                "style().width(px(400)))",
                &format!("style().width(px(400))).env(#{{ density: \"{density}\" }})"),
            );
            let (mut visual, view) = Mount::components(source)
                .registry("patterns/composer")
                .theme(theme)
                .mount(cx);
            let initial = bounds(&mut visual, &view, "text_field", "Prompt");
            visual.simulate_keystrokes(
                "shift-enter shift-enter shift-enter shift-enter shift-enter shift-enter",
            );
            visual.run_until_parked();
            let grown = bounds(&mut visual, &view, "text_field", "Prompt");
            assert!(
                grown.height > initial.height && grown.height < initial.height * 2.0,
                "native field grows from two to three rows then scrolls: {initial:?} -> {grown:?}"
            );
            assert!(
                nodes(&mut visual, &view)
                    .iter()
                    .any(|node| node.name == "6 / 40")
            );
            assert_eq!(text_of(&mut visual, &view, "sends"), "0");
            assert_eq!(last_error(&mut visual, &view), None);
        }
    }
}

#[gpui::test]
fn textarea_becoming_disabled_or_readonly_blocks_further_native_submit(cx: &mut TestAppContext) {
    for gate in ["disabled", "read_only"] {
        let field = TEXTAREA.replace_once(
            "submit_key: \"mod_enter\"",
            &format!("submit_key: \"enter\", {gate}: ctx.get_state(\"busy\")"),
        );
        let (mut visual, view) = Mount::components(SOURCE.replace_once("FIELD", &field))
            .registry("patterns/composer")
            .mount(cx);
        visual.simulate_input("accepted");
        visual.simulate_keystrokes("enter");
        assert_eq!(text_of(&mut visual, &view, "sends"), "1");
        visual.simulate_input("blocked");
        visual.simulate_keystrokes("enter shift-enter");
        assert_eq!(text_of(&mut visual, &view, "sends"), "1");
        assert_eq!(text_of(&mut visual, &view, "draft"), "accepted");
        assert_eq!(last_error(&mut visual, &view), None);
    }
}
