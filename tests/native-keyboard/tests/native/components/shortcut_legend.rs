//! A caller-written shortcut shows the platform legend, the same as a bound
//! action's: `cmd-p` is `⌘P` on macOS. Text that is already a legend (`⌘K`)
//! stays as written. Button keeps its inline legend; Command paints individual
//! decorative caps and exposes the complete legend on its semantic option.

use gpui::TestAppContext;

use crate::support::{Mount, Step, palette_texts, settle, snapshot};

fn legends(cx: &mut TestAppContext, body: &str) -> (Vec<String>, Vec<String>, String) {
    let (mut visual, view) = Mount::components(format!(
        r#"import "components/button" as button;
import "components/command" as command;
fn noop(ctx, payload) {{ () }}
fn view(ctx) {{ column([{body}]).with_style(style().width(px(600))) }}"#
    ))
    .id("legend")
    .mount(cx);
    settle(&mut visual, Step::Frame, 1);
    let tree = snapshot(&mut visual, &view);
    let texts = tree
        .nodes()
        .filter(|node| node.role == "text")
        .map(|node| node.name.clone())
        .collect();
    let shortcut = tree
        .find_by_role_and_name("option", "Deploy")
        .next()
        .unwrap()
        .key_shortcuts
        .clone()
        .unwrap();
    (texts, palette_texts(&mut visual, &view), shortcut)
}

#[gpui::test]
fn caller_written_shortcuts_show_the_platform_legend(cx: &mut TestAppContext) {
    let (found, painted, command) = legends(
        cx,
        r#"button::Button(#{ text: "Menu", shortcut: "cmd-p", on_click: Fn("noop") }),
        button::Button(#{ text: "Find", shortcut: "⌘F", on_click: Fn("noop") }),
        command::Command(#{ key: "palette", label: "Commands", query: "", active_value: "",
            items: [#{ value: "deploy", label: "Deploy", shortcut: "cmd-shift-d" }] })"#,
    );
    println!("{found:?}");
    for raw in ["cmd-p", "cmd-shift-d"] {
        assert!(
            !found.iter().any(|text| text == raw),
            "raw `{raw}` shown: {found:?}"
        );
    }
    assert!(
        found.iter().any(|text| text == "⌘F"),
        "a legend stays: {found:?}"
    );
    if cfg!(target_os = "macos") {
        assert!(found.iter().any(|text| text == "⌘P"), "{found:?}");
        assert_eq!(command, "⇧⌘D");
        for cap in ["⇧", "⌘", "D"] {
            assert!(
                painted.iter().any(|text| text == cap),
                "individual keycap {cap:?} must paint: {painted:?}"
            );
        }
    }
}
