//! Drill down through the real CLI Gallery shell, not an isolated demo mount.

use gpui::{FocusHandle, TestAppContext, VisualTestContext, px, size};
use gpui_rhai::{AccessibilityNode, MotionPreference, ScriptViewHandle, UiValue};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::mount_prepared;
use crate::support::{Host, Step, bounds, click_center, last_error, nodes, settle};

fn mount(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
    cx.update(gpui_rhai::install);
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: "navigation_stack".to_owned(),
        ..AcceptanceLaunch::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let (window, view) = mount_prepared(
        cx,
        prepared,
        ("navigation-workflow", "gallery"),
        true,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.simulate_resize(size(px(1_100.0), px(920.0)));
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    let shell = nodes(&mut visual, &view);
    assert!(
        shell
            .iter()
            .any(|node| { node.role == "toolbar" && node.name == "gpui-rhai Gallery title bar" })
    );
    assert!(
        shell
            .iter()
            .any(|node| node.role == "tree" && node.name == "Gallery pages")
    );
    (visual, view)
}

fn node(
    visual: &mut VisualTestContext,
    view: &ScriptViewHandle,
    role: &str,
    name: &str,
) -> AccessibilityNode {
    nodes(visual, view)
        .into_iter()
        .find(|node| node.role == role && node.name == name)
        .unwrap_or_else(|| panic!("Gallery has no {role} named {name:?}"))
}

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let rect = bounds(visual, view, role, name);
    click_center(visual, rect);
    settle(visual, Step::Frame, 3);
    assert_eq!(last_error(visual, view), None);
}

fn write(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str, value: &str) {
    click(visual, view, "text_field", name);
    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input(value);
    settle(visual, Step::Frame, 3);
    assert_eq!(
        node(visual, view, "text_field", name).value,
        Some(UiValue::String(value.to_owned()))
    );
    assert_eq!(last_error(visual, view), None);
}

fn assert_service_values(visual: &mut VisualTestContext, view: &ScriptViewHandle) {
    assert_eq!(
        node(visual, view, "text_field", "checkout-api filter").value,
        Some(UiValue::String("edge-7".to_owned()))
    );
    assert_eq!(
        node(visual, view, "text_field", "checkout-api draft").value,
        Some(UiValue::String("Service handoff · 保留草稿".to_owned()))
    );
}

#[gpui::test]
fn cli_gallery_navigation_keeps_opener_state_and_resets_popped_visits(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx);
    let service_editor = node(&mut visual, &view, "text_field", "checkout-api draft").id;
    write(&mut visual, &view, "checkout-api filter", "edge-7");
    write(
        &mut visual,
        &view,
        "checkout-api draft",
        "Service handoff · 保留草稿",
    );

    // PagePanel places the caller-owned Open action immediately after its
    // native draft. Tab/Enter exercises the real shell's keyboard route and
    // records the actual opener before the controlled push hides its page.
    let opener_node = node(&mut visual, &view, "button", "Open Ticket 4821").id;
    let draft_focus = visual.update(|window, cx| window.focused(cx));
    visual.simulate_keystrokes("tab");
    settle(&mut visual, Step::Frame, 2);
    let opener: FocusHandle = visual
        .update(|window, cx| window.focused(cx))
        .expect("the Open Ticket 4821 action is focusable");
    assert_ne!(Some(opener.clone()), draft_focus);
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    let first_ticket_editor = node(&mut visual, &view, "text_field", "Ticket 4821 draft").id;
    assert!(
        !nodes(&mut visual, &view)
            .iter()
            .any(|node| node.name == "checkout-api draft"),
        "covered service content must leave presented accessibility"
    );
    write(
        &mut visual,
        &view,
        "Ticket 4821 draft",
        "First ticket visit · 不应复活",
    );

    // GPUI chord syntax uses '-', while the Gallery's Rhai back_key uses '+'.
    // Dispatch the real Command+[ chord, not NavigateBack or an automation pop.
    visual.simulate_keystrokes("cmd-[");
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(
        visual.update(|window, cx| window.focused(cx)),
        Some(opener.clone())
    );
    assert_service_values(&mut visual, &view);
    assert_eq!(
        node(&mut visual, &view, "button", "Open Ticket 4821").id,
        opener_node
    );
    assert_eq!(
        node(&mut visual, &view, "text_field", "checkout-api draft").id,
        service_editor
    );

    // Enter now acts on the restored opener. Visiting the same record after
    // a committed pop is a new mount, not resurrection of the discarded draft.
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::Frame, 4);
    let fresh_ticket = node(&mut visual, &view, "text_field", "Ticket 4821 draft");
    assert_ne!(fresh_ticket.id, first_ticket_editor);
    assert_eq!(fresh_ticket.value, Some(UiValue::String(String::new())));

    click(&mut visual, &view, "tab", "None");
    assert_eq!(node(&mut visual, &view, "tab", "None").selected, Some(true));
    assert_eq!(
        node(&mut visual, &view, "text_field", "Ticket 4821 draft").id,
        fresh_ticket.id,
        "changing header presentation must not replace the current visit"
    );
    assert!(!nodes(&mut visual, &view).iter().any(|node| {
        node.role == "button"
            && matches!(node.name.as_str(), "checkout-api" | "Back to checkout-api")
    }));
    // The mode tabs are outside the stack's focus scope: enter the current
    // draft again before asking the headerless stack to handle keyboard Back.
    write(&mut visual, &view, "Ticket 4821 draft", "Fresh visit draft");
    visual.simulate_keystrokes("cmd-[");
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(visual.update(|window, cx| window.focused(cx)), Some(opener));
    assert_service_values(&mut visual, &view);
    assert_eq!(last_error(&mut visual, &view), None);
}
