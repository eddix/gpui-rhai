//! Exercise the actual CLI application, including its fixture extension and callbacks.
use std::time::Duration;

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::{MotionPreference, ScriptViewHandle, UiValue};
use gpui_rhai_cli::acceptance::{self, AcceptanceLaunch};

use super::mount_prepared;
use crate::support::{Host, Step, bounds, click_center, last_error, nodes, settle, text_of};

fn mount(cx: &mut TestAppContext, page: &str) -> (VisualTestContext, ScriptViewHandle) {
    let prepared = acceptance::view(&AcceptanceLaunch {
        page: page.to_owned(),
        ..AcceptanceLaunch::default()
    })
    .unwrap()
    .motion_preference(MotionPreference::None)
    .prepare()
    .unwrap();
    let (window, view) = mount_prepared(
        cx,
        prepared,
        ("application", "content"),
        true,
        |host, view| Host { host, view },
    );
    let mut visual = VisualTestContext::from_window(*window, cx);
    settle(&mut visual, Step::Frame, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

fn click(visual: &mut VisualTestContext, view: &ScriptViewHandle, role: &str, name: &str) {
    let rect = bounds(visual, view, role, name);
    click_center(visual, rect);
    settle(visual, Step::Frame, 3);
    assert_eq!(last_error(visual, view), None);
}

fn field(visual: &mut VisualTestContext, view: &ScriptViewHandle, name: &str) -> UiValue {
    nodes(visual, view)
        .into_iter()
        .find(|node| node.role == "text_field" && node.name == name)
        .and_then(|node| node.value)
        .expect("native input value")
}

fn reply_source(visual: &mut VisualTestContext, view: &ScriptViewHandle) -> String {
    visual.update(|_, cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string("pending parse".to_owned()))
    });
    // Stop cancels future chunks, not the final snapshot's background parse.
    // Wait for the reader to accept Copy instead of racing its loading gate.
    for _ in 0..32 {
        settle(visual, Step::Tick, 1);
        let rect = bounds(visual, view, "document", "Reply 1");
        if rect.height <= 0.0 {
            continue;
        }
        click_center(visual, rect);
        settle(visual, Step::Frame, 1);
        visual.simulate_keystrokes("alt-cmd-c");
        if let Some(text) =
            visual.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
            && text != "pending parse"
        {
            return text;
        }
    }
    panic!("explicit Markdown source copy did not become ready");
}

#[gpui::test]
fn gallery_conversation_sends_keeps_drafting_and_stops_native_streaming(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, "scene.messages");
    click(&mut visual, &view, "text_field", "Message");
    visual.simulate_keystrokes("cmd-a backspace");
    visual.simulate_input("   ");
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::Frame, 2);
    assert!(
        !nodes(&mut visual, &view)
            .iter()
            .any(|node| node.role == "listitem")
    );

    visual.simulate_keystrokes("cmd-a");
    visual.simulate_input("Investigate the rollout");
    visual.simulate_keystrokes("enter");
    settle(&mut visual, Step::Tick, 4);
    assert_eq!(last_error(&mut visual, &view), None);
    assert!(
        nodes(&mut visual, &view)
            .iter()
            .any(|node| node.name == "Streaming")
    );
    assert_eq!(
        field(&mut visual, &view, "Message"),
        UiValue::String(String::new())
    );
    // Timer installation and the first chunk are asynchronous. Stop only
    // after a real partial revision exists, not merely when busy is visible.
    for _ in 0..12 {
        if !text_of(&mut visual, &view, "reply-progress").contains("· 0/") {
            break;
        }
        settle(&mut visual, Step::Tick, 1);
    }
    assert!(!text_of(&mut visual, &view, "reply-progress").contains("· 0/"));

    click(&mut visual, &view, "text_field", "Message");
    visual.simulate_input("Next draft · 中文");
    visual.simulate_keystrokes("enter");
    assert_eq!(
        field(&mut visual, &view, "Message"),
        UiValue::String("Next draft · 中文".to_owned())
    );
    click(&mut visual, &view, "button", "Stop");
    let partial = reply_source(&mut visual, &view);
    assert!(
        partial.starts_with("## Investigation") && !partial.contains("Stop leaves"),
        "{partial:?}"
    );
    visual.executor().advance_clock(Duration::from_secs(3));
    settle(&mut visual, Step::Frame, 5);
    assert_eq!(
        reply_source(&mut visual, &view),
        partial,
        "stopped work cannot publish a late chunk"
    );
    assert_eq!(
        field(&mut visual, &view, "Message"),
        UiValue::String("Next draft · 中文".to_owned())
    );
    assert_eq!(last_error(&mut visual, &view), None);
}

#[gpui::test]
fn gallery_retained_tabs_keep_each_native_draft_independently(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx, "retained_tabs");
    click(&mut visual, &view, "text_field", "Checkout service draft");
    visual.simulate_input("Service handoff");
    click(&mut visual, &view, "tab", "Tickets");
    assert!(
        !nodes(&mut visual, &view)
            .iter()
            .any(|node| node.name == "Checkout service draft")
    );
    click(&mut visual, &view, "text_field", "Ticket 4821 draft");
    visual.simulate_input("Ticket investigation");
    click(&mut visual, &view, "tab", "Overview");
    assert_eq!(
        field(&mut visual, &view, "Checkout service draft"),
        UiValue::String("Service handoff".to_owned())
    );
    click(&mut visual, &view, "tab", "Tickets");
    assert_eq!(
        field(&mut visual, &view, "Ticket 4821 draft"),
        UiValue::String("Ticket investigation".to_owned())
    );
    assert_eq!(last_error(&mut visual, &view), None);
}
