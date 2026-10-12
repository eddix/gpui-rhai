//! A caller-written shortcut shows the platform legend, the same as a bound
//! action's: `cmd-p` is `⌘P` on macOS. Text that is already a legend (`⌘K`)
//! stays as written. Button draws it in the label voice that Command and Menu
//! use for their inline Kbd.

use gpui::TestAppContext;

use crate::support::{Mount, Step, settle, snapshot};

fn texts(cx: &mut TestAppContext, body: &str) -> Vec<String> {
    let (mut visual, view) = Mount::components(format!(
        r#"import "components/button" as button;
import "components/command" as command;
fn noop(ctx, payload) {{ () }}
fn view(ctx) {{ column([{body}]).with_style(style().width(px(600))) }}"#
    ))
    .id("legend")
    .mount(cx);
    settle(&mut visual, Step::Frame, 1);
    snapshot(&mut visual, &view)
        .nodes()
        .filter(|node| node.role == "text")
        .map(|node| node.name.clone())
        .collect()
}

#[gpui::test]
fn caller_written_shortcuts_show_the_platform_legend(cx: &mut TestAppContext) {
    let found = texts(
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
        assert!(found.iter().any(|text| text == "⇧⌘D"), "{found:?}");
    }
}
