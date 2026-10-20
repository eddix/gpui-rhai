//! Business tab values are not native node keys: empty is valid, duplicates are not.
use gpui::TestAppContext;
use gpui_rhai::MotionPreference;

use crate::support::{
    Mount, ReplaceOnce, Step, bounds, by_test_id, click_center, dispatch, last_error, nodes,
    settle, text_of,
};

const SWITCHER: &str = r#"
import "components/tabs" as tabs;
fn state_schema() { #{ fields: #{
    value: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "running" } },
    changes: #{ schema: #{ type: "integer" }, "default": #{ type: "integer", value: 0 } },
    duplicate: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
} } }
fn changed(ctx, value) {
    ctx.set_state("value", value);
    ctx.set_state("changes", ctx.get_state("changes") + 1);
}
fn duplicate(ctx, event) { ctx.set_state("duplicate", true); }
fn view(ctx) {
    let entries = [
        #{ value: "", label: "All" },
        #{ value: "running", label: "Running" },
        #{ value: "tab:", label: "Literal prefix" },
        #{ value: "panel:", label: "Panel prefix" },
    ];
    if ctx.get_state("duplicate") { entries.push(#{ value: "", label: "Duplicate" }); }
    column([
        tabs::Tabs(#{ key: "status", label: "Status", panel: false,
            orientation: "horizontal", value: ctx.get_state("value"), tabs: entries,
            on_change: Fn("changed") }),
        text(`<${ctx.get_state("value")}>:${ctx.get_state("changes")}`).test_id("state"),
        text("Add duplicate").test_id("duplicate").on_click(Fn("duplicate")),
    ]).with_style(style().width(px(500)).height(px(400)))
}
"#;

#[gpui::test]
fn empty_tab_business_value_mounts_as_a_panel_less_switcher(cx: &mut TestAppContext) {
    let source = SWITCHER.replace_once(
        "\"default\": #{ type: \"string\", value: \"running\" }",
        "\"default\": #{ type: \"string\", value: \"\" }",
    );
    let (mut visual, view) = Mount::components(source).mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let tabs = nodes(&mut visual, &view)
        .into_iter()
        .filter(|node| node.role == "tab")
        .collect::<Vec<_>>();
    assert_eq!(tabs.len(), 4);
    let identities = tabs
        .iter()
        .map(|node| node.id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        identities.len(),
        tabs.len(),
        "prefix-like values cannot alias"
    );
    assert_eq!(text_of(&mut visual, &view, "state"), "<>:0");
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn pointer_emits_the_empty_tab_business_value_unchanged(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(SWITCHER).mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let all = bounds(&mut visual, &view, "tab", "All");
    click_center(&mut visual, all);
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(text_of(&mut visual, &view, "state"), "<>:1");
    let literal = bounds(&mut visual, &view, "tab", "Literal prefix");
    click_center(&mut visual, literal);
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(text_of(&mut visual, &view, "state"), "<tab:>:2");
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn keyboard_emits_the_empty_tab_business_value_unchanged(cx: &mut TestAppContext) {
    for (orientation, previous, next) in
        [("horizontal", "left", "right"), ("vertical", "up", "down")]
    {
        let source = SWITCHER.replace_once(
            "orientation: \"horizontal\"",
            &format!("orientation: {orientation:?}"),
        );
        let (mut visual, view) = Mount::components(source).mount(cx);
        settle(&mut visual, Step::Frame, 3);
        // Embedded mount does not steal host focus. Start at the view root,
        // then enter its single tab stop before sending the selection key.
        visual.update(|window, cx| view.focus(window, cx).unwrap());
        let root_focus = visual.update(|window, cx| window.focused(cx)).unwrap();
        visual.simulate_keystrokes("tab");
        settle(&mut visual, Step::Frame, 3);
        let tab_focus = visual.update(|window, cx| window.focused(cx)).unwrap();
        assert_ne!(tab_focus, root_focus, "Tab must enter the tab list");
        visual.simulate_keystrokes(previous);
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(
            text_of(&mut visual, &view, "state"),
            "<>:1",
            "{orientation}"
        );
        visual.simulate_keystrokes(next);
        settle(&mut visual, Step::Frame, 3);
        assert_eq!(
            text_of(&mut visual, &view, "state"),
            "<running>:2",
            "{orientation}"
        );
        assert_eq!(last_error(&mut visual, &view), None);
    }
}

#[gpui::test]
fn duplicate_empty_and_nonempty_tab_values_are_both_rejected(cx: &mut TestAppContext) {
    for duplicate in ["", "running"] {
        let source = SWITCHER.replace_once(
            "entries.push(#{ value: \"\", label: \"Duplicate\" })",
            &format!("entries.push(#{{ value: {duplicate:?}, label: \"Duplicate\" }})"),
        );
        let (mut visual, view) = Mount::components(source).mount(cx);
        settle(&mut visual, Step::Frame, 3);
        let result = dispatch(&mut visual, &view, by_test_id("duplicate"), "click");
        settle(&mut visual, Step::Frame, 3);
        let error = result
            .err()
            .map(|error| error.to_string())
            .or_else(|| last_error(&mut visual, &view));
        assert!(
            error
                .as_deref()
                .is_some_and(|error| error.contains("Tabs values must be unique")),
            "duplicate {duplicate:?} should fail without replacing the last-good tree: {error:?}"
        );
        assert_eq!(
            nodes(&mut visual, &view)
                .iter()
                .filter(|node| node.role == "tab")
                .count(),
            4
        );
    }
}

const RETAINED_EDITOR: &str = r#"
import "components/tabs" as tabs;
import "components/textarea" as textarea;
fn state_schema() { #{ fields: #{
    value: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
    draft: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
} } }
fn changed(ctx, value) { ctx.set_state("value", value); }
fn edited(ctx, value) { ctx.set_state("draft", value); }
fn view(ctx) { column([
    tabs::Tabs(#{ key: "editor-tabs", label: "Workspace", value: ctx.get_state("value"), keep_mounted: true,
        tabs: [
            #{ value: "", label: "Draft", content: textarea::Textarea(#{ key: "draft", label: "Draft text", value: ctx.get_state("draft"), rows: 3, on_change: Fn("edited") }) },
            #{ value: "other", label: "Other", content: text("Other page") },
        ], on_change: Fn("changed") }),
    text(ctx.get_state("draft")).test_id("draft-value"),
]).with_style(style().width(px(400)).height(px(400))) }
"#;

#[gpui::test]
fn retained_empty_valued_panel_preserves_native_editor_selection_and_undo(cx: &mut TestAppContext) {
    let (mut visual, view) = Mount::components(RETAINED_EDITOR)
        .motion(MotionPreference::None)
        .mount(cx);
    settle(&mut visual, Step::Frame, 3);
    let editor = bounds(&mut visual, &view, "text_field", "Draft text");
    click_center(&mut visual, editor);
    visual.simulate_input("abcd");
    visual.simulate_keystrokes("shift-left shift-left");
    let focus = visual.update(|window, cx| window.focused(cx)).unwrap();
    let other = bounds(&mut visual, &view, "tab", "Other");
    click_center(&mut visual, other);
    settle(&mut visual, Step::Frame, 3);
    assert!(
        !nodes(&mut visual, &view)
            .iter()
            .any(|node| node.role == "text_field")
    );
    let draft = bounds(&mut visual, &view, "tab", "Draft");
    click_center(&mut visual, draft);
    settle(&mut visual, Step::Frame, 3);
    visual.update(|window, cx| focus.focus(window, cx));
    visual.simulate_input("Q");
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(text_of(&mut visual, &view, "draft-value"), "abQ");
    visual.simulate_keystrokes("cmd-z");
    settle(&mut visual, Step::Frame, 3);
    assert_eq!(text_of(&mut visual, &view, "draft-value"), "abcd");
    assert_eq!(last_error(&mut visual, &view), None);
}
