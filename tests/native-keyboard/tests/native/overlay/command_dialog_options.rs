//! CommandDialog can replace a hand-built palette (#120): the title can be hidden while the
//! label still names the dialog, `on_escape` lets the caller see Escape before the dialog
//! closes (a palette with levels goes back one level), and `width`,
//! `command_part_styles` and `dialog_part_styles` reach the inner Dialog and Command.

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{GeometryBounds, MotionPreference, ScriptViewHandle};

use crate::support::{Mount, Step, filled, snapshot};

/// A palette open from the start; `extra` adds CommandDialog props.
fn mount(cx: &mut TestAppContext, extra: &str) -> (VisualTestContext, ScriptViewHandle) {
    let main = format!(
        r#"import "components/command_dialog" as command_dialog;
fn state_schema() {{ #{{ fields: #{{
    open: #{{ schema: #{{ type: "bool" }}, "default": #{{ type: "bool", value: true }} }},
    escapes: #{{ schema: #{{ type: "integer" }}, "default": #{{ type: "integer", value: 0 }} }} }} }} }}
fn opened(ctx, open) {{ ctx.set_state("open", open); }}
fn escaped(ctx, payload) {{ ctx.set_state("escapes", ctx.get_state("escapes") + 1); }}
fn noop(ctx, payload) {{ () }}
fn view(ctx) {{ column([
    text(`${{ctx.get_state("open")}}/${{ctx.get_state("escapes")}}`).accessibility_role("status"),
    command_dialog::CommandDialog(#{{ key: "palette", open: ctx.get_state("open"), query: "",
        active_value: "", label: "Commands", on_open_change: Fn("opened"),
        on_query_change: Fn("noop"), on_active_change: Fn("noop"), on_action: Fn("noop"),
        items: [#{{ value: "theme", label: "Switch theme" }}, #{{ value: "open", label: "Open with" }}]
        {extra} }}),
]).with_style(style().width(px(900)).height(px(700))) }}"#
    );
    let (mut visual, view) = Mount::components(main)
        .motion(MotionPreference::None)
        .id("palette")
        .mount(cx);
    settle(&mut visual);
    (visual, view)
}

fn settle(visual: &mut VisualTestContext) {
    crate::support::settle(visual, Step::Frame, 3);
}

fn status(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    crate::support::nodes(visual, view)
        .into_iter()
        .find(|node| node.role == "status")
        .unwrap()
        .name
}

fn nodes(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<(String, String)> {
    crate::support::nodes(visual, view)
        .into_iter()
        .map(|node| (node.role, node.name))
        .collect()
}

fn press_escape(visual: &mut VisualTestContext) {
    visual.simulate_keystrokes("escape");
    settle(visual);
}

#[gpui::test]
fn a_hidden_title_still_names_the_dialog(cx: &mut TestAppContext) {
    for (visible, extra) in [(true, ""), (false, ", title_visible: false")] {
        let (mut visual, view) = mount(cx, extra);
        let found = nodes(&mut visual, &view);
        assert!(
            found
                .iter()
                .any(|(role, name)| role == "dialog" && name == "Commands"),
            "title_visible {visible}: {found:?}"
        );
        let title_text = found
            .iter()
            .any(|(role, name)| role == "text" && name == "Commands");
        assert_eq!(title_text, visible, "title_visible {visible}: {found:?}");
    }
}

#[gpui::test]
fn on_escape_sees_escape_before_the_dialog_closes(cx: &mut TestAppContext) {
    // The control: without on_escape, Escape closes the dialog.
    let (mut visual, view) = mount(cx, "");
    press_escape(&mut visual);
    assert_eq!(status(&mut visual, &view), "false/0");
    // With on_escape the caller gets the key and the dialog stays open.
    let (mut visual, view) = mount(cx, r#", on_escape: Fn("escaped")"#);
    press_escape(&mut visual);
    assert_eq!(status(&mut visual, &view), "true/1");
    press_escape(&mut visual);
    assert_eq!(status(&mut visual, &view), "true/2");
}

/// The panel's layout bounds and the green `search` part's left edge inside it.
fn search_inset(cx: &mut TestAppContext, extra: &str) -> (GeometryBounds, f64) {
    let (mut visual, view) = mount(
        cx,
        &format!(
            r#", command_part_styles: #{{ search: style().background(rgba(0x00ff00ff)) }} {extra}"#
        ),
    );
    let dialog = snapshot(&mut visual, &view)
        .find_by_role_and_name("dialog", "Commands")
        .next()
        .and_then(|node| node.geometry)
        .unwrap()
        .layout;
    let search_x = filled(&mut visual, 0x00ff_00ff)
        .first()
        .map(|quad| f64::from(quad.bounds.x))
        .expect("the search part takes the caller's background");
    (dialog, search_x - dialog.x)
}

#[gpui::test]
fn width_and_part_styles_reach_the_inner_parts(cx: &mut TestAppContext) {
    let (default, flush) = search_inset(cx, "");
    // The documented 420px, no longer 90% of itself.
    assert!(
        (default.width - 420.0).abs() < 0.5,
        "default width {}",
        default.width
    );
    // The command fills the panel: its search starts inside the 1px hairline.
    assert!((flush - 1.0).abs() < 0.5, "search inset {flush}");
    let (wide, padded) = search_inset(
        cx,
        r#", width: px(560), dialog_part_styles: #{ panel: style().padding(px(16)) }"#,
    );
    assert!(
        (wide.width - 560.0).abs() < 0.5,
        "panel width {}",
        wide.width
    );
    // A caller's panel padding still wins over the flush default.
    assert!(
        (padded - flush - 16.0).abs() < 0.5,
        "search inset {padded} with a 16px panel padding, {flush} without"
    );
}
