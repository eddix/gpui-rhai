//! Toast `layout: "stack"` (#148): each corner piles its toasts behind the newest, expands
//! while hovered or holding keyboard focus, pauses every countdown in the corner meanwhile,
//! collapses on Escape, and dismisses the whole corner with Clear all.

use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::{
    AutomationCommand, GeometryBounds, ManualRuntimeClock, MotionPreference, ScriptViewHandle,
};

use crate::support::{Mount, Step, center, last_error, settle, snapshot, text_of};

/// Three toasts in one corner, oldest first; the host removes an item on `dismiss` and logs
/// `dismiss` and `activate` payloads.
fn source(region: &str, extra: &str) -> String {
    format!(
        r#"import "components/toast" as toast;
fn state_schema() {{ #{{ fields: #{{
    log: #{{ schema: #{{ type: "string" }}, "default": #{{ type: "string", value: "" }} }}
}} }} }}
fn dismissed(ctx, id) {{ ctx.set_state("log", `${{ctx.get_state("log")}}dismiss:${{id}} `); }}
fn activated(ctx, id) {{ ctx.set_state("log", `${{ctx.get_state("log")}}activate:${{id}} `); }}
fn noop(ctx, payload) {{}}
fn view(ctx) {{
    let log = ctx.get_state("log");
    let items = [];
    for spec in [["first", 1000], ["second", 2000], ["third", 3000]] {{
        if !log.contains(`dismiss:${{spec[0]}} `) {{
            items.push(#{{ id: spec[0], title: `Toast ${{spec[0]}}`, region: "{region}", duration_ms: spec[1] }});
        }}
    }}
    column([
        text("Main").on_click(Fn("noop")).accessibility_role("button").accessibility_label("Main"),
        text(log).test_id("log"),
        toast::Toast(#{{ key: "toasts", layout: "stack", items: items,
            on_dismiss: Fn("dismissed"){extra} }}),
    ])
}}
"#
    )
}

fn mount(cx: &mut TestAppContext, main: &str) -> (VisualTestContext, ScriptViewHandle) {
    let (mut visual, view) = Mount::components(main)
        .motion(MotionPreference::None)
        .with(|view| view.runtime_clock(ManualRuntimeClock::new(std::time::Instant::now()).clock()))
        .id("toasts")
        .mount(cx);
    visual.update(|window, _| window.activate_window());
    settle(&mut visual, Step::Frame, 3);
    (visual, view)
}

/// The toast titles that take room on screen, in order from top to bottom.
fn shown(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> Vec<String> {
    let mut shown = snapshot(visual, view)
        .nodes()
        .filter(|node| node.role == "status" && node.name.starts_with("Toast "))
        .filter_map(|node| {
            let bounds = node.geometry?.visual;
            (bounds.height > 0.0).then(|| (bounds.y, node.name.clone()))
        })
        .collect::<Vec<_>>();
    shown.sort_by(|a, b| a.0.total_cmp(&b.0));
    shown.into_iter().map(|(_, name)| name).collect()
}

fn pile(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> GeometryBounds {
    snapshot(visual, view)
        .nodes()
        .find(|node| node.role == "group" && node.name.starts_with("Notifications"))
        .and_then(|node| node.geometry)
        .expect("the pile is a named group")
        .visual
}

fn named(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) -> bool {
    snapshot(visual, view)
        .find_by_role_and_name(role, name)
        .next()
        .is_some()
}

fn hover(visual: &mut VisualTestContext, bounds: Option<GeometryBounds>) {
    let position = bounds.map_or(point(px(1.), px(1.)), center);
    visual.simulate_mouse_move(position, None::<MouseButton>, Modifiers::default());
    settle(visual, Step::Frame, 2);
}

/// Moves the view's clock, firing the countdowns that end on the way.
fn advance(visual: &mut VisualTestContext, view: &ScriptViewHandle, millis: u64) {
    visual
        .update(|window, cx| view.automate(AutomationCommand::AdvanceTime { millis }, window, cx))
        .unwrap();
    settle(visual, Step::Frame, 2);
}

#[gpui::test]
fn a_collapsed_stack_shows_the_newest_and_a_count(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &source("bottom_right", ""));
    assert_eq!(last_error(&mut visual, &view), None);
    assert_eq!(shown(&mut visual, &view), ["Toast third"]);
    assert!(named(&mut visual, &view, "group", "Notifications 3"));
    assert!(!named(&mut visual, &view, "button", "Clear all"));
}

#[gpui::test]
fn hovering_expands_the_stack_and_holds_its_countdowns(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &source("bottom_right", ""));
    let bounds = pile(&mut visual, &view);
    hover(&mut visual, Some(bounds));
    // Newest nearest the bottom corner.
    assert_eq!(
        shown(&mut visual, &view),
        ["Toast first", "Toast second", "Toast third"]
    );
    assert!(named(&mut visual, &view, "button", "Clear all"));
    advance(&mut visual, &view, 5000);
    assert_eq!(text_of(&mut visual, &view, "log"), "", "held while hovered");

    hover(&mut visual, None);
    assert_eq!(shown(&mut visual, &view), ["Toast third"]);
    advance(&mut visual, &view, 1100);
    assert_eq!(text_of(&mut visual, &view, "log"), "dismiss:first ");
}

#[gpui::test]
fn a_top_corner_lists_the_newest_first(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &source("top_left", ""));
    assert_eq!(shown(&mut visual, &view), ["Toast third"]);
    let bounds = pile(&mut visual, &view);
    hover(&mut visual, Some(bounds));
    assert_eq!(
        shown(&mut visual, &view),
        ["Toast third", "Toast second", "Toast first"]
    );
}

#[gpui::test]
fn keyboard_focus_expands_the_stack_and_escape_collapses_it(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &source("bottom_right", ""));
    // Main, then the newest toast's close button, in the corner's layer.
    for _ in 0..2 {
        visual.update(|window, cx| window.focus_next(cx));
        settle(&mut visual, Step::Frame, 2);
    }
    assert_eq!(shown(&mut visual, &view).len(), 3, "focus inside expands");
    advance(&mut visual, &view, 5000);
    assert_eq!(text_of(&mut visual, &view, "log"), "", "held while focused");

    visual.simulate_keystrokes("escape");
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(shown(&mut visual, &view), ["Toast third"]);

    // Focus leaving lets the countdowns run again.
    visual.update(|window, cx| window.focus_prev(cx));
    settle(&mut visual, Step::Frame, 2);
    advance(&mut visual, &view, 1100);
    assert_eq!(text_of(&mut visual, &view, "log"), "dismiss:first ");
}

#[gpui::test]
fn clear_all_dismisses_every_toast_in_the_corner(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, &source("bottom_right", ""));
    let bounds = pile(&mut visual, &view);
    hover(&mut visual, Some(bounds));
    let clear = crate::support::bounds(&mut visual, &view, "button", "Clear all");
    visual.simulate_click(center(clear), Modifiers::none());
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(
        text_of(&mut visual, &view, "log"),
        "dismiss:first dismiss:second dismiss:third "
    );
    assert!(shown(&mut visual, &view).is_empty());
}

#[gpui::test]
fn on_activate_makes_a_toast_a_button(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(
        cx,
        &source("bottom_right", r#", on_activate: Fn("activated")"#),
    );
    let bounds = pile(&mut visual, &view);
    hover(&mut visual, Some(bounds));
    let newest = crate::support::bounds(&mut visual, &view, "status", "Toast third");
    // Pointed at first, as a real pointer is: the hover expands the pile. Then the title
    // row's start, away from the close button.
    let position = point(
        px(newest.x as f32 + 24.),
        px((newest.y + newest.height / 2.) as f32),
    );
    visual.simulate_click(position, Modifiers::none());
    settle(&mut visual, Step::Frame, 2);
    assert_eq!(text_of(&mut visual, &view, "log"), "activate:third ");
}
