//! A Resizable grip covers its native handle's hitbox, so the handle's resize cursor does
//! not reach the pointer over it; an edge grip carries the resize cursor of its direction
//! itself, as SplitPane's grip does (#143). The Style API has no diagonal cursor, so a corner
//! grip carries none.
//!
//! cargo test --manifest-path tests/native-keyboard/Cargo.toml --test native manipulation::resizable_grip_cursor

use gpui::{TestAppContext, VisualTestContext};
use gpui_rhai::*;

use crate::support::{Mount, Step, last_error, settle};

const HANDLES: [&str; 8] = ["n", "s", "e", "w", "ne", "nw", "se", "sw"];

/// A Resizable with a grip on every handle.
fn mount(cx: &mut TestAppContext) -> (VisualTestContext, ScriptViewHandle) {
    let grips = HANDLES
        .iter()
        .map(|handle| format!("{handle}:box([]).with_style(style().width(px(12)).height(px(12)))"))
        .collect::<Vec<_>>()
        .join(",");
    let main = format!(
        r#"import "components/resizable" as resizable;
fn view(ctx) {{ column([
    resizable::Resizable(#{{ key: "card", label: "Card", content: text("Card"),
        rect: #{{ x: 100.0, y: 80.0, width: 200.0, height: 120.0 }}, grips: #{{ {grips} }} }})
        .with_style(style().width(px(500)).height(px(400)))
]) }}"#
    );
    let (mut visual, view) = Mount::components(main)
        .motion(MotionPreference::None)
        .id("resizable")
        .mount(cx);
    settle(&mut visual, Step::Frame, 1);
    assert_eq!(last_error(&mut visual, &view), None);
    (visual, view)
}

fn find_keyed<'a>(node: &'a UiNode, key: &str) -> Option<&'a UiNode> {
    if node.key().is_some_and(|node_key| node_key.as_str() == key) {
        return Some(node);
    }
    match node.kind() {
        UiNodeKind::Box { children } | UiNodeKind::Fragment { children } => {
            children.iter().find_map(|child| find_keyed(child, key))
        }
        _ => None,
    }
}

#[gpui::test]
fn each_grip_carries_the_cursor_of_its_handle(cx: &mut TestAppContext) {
    let (mut visual, view) = mount(cx);
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let cursors = HANDLES
        .iter()
        .map(|handle| {
            let grip = find_keyed(&root, &format!("card-grip-{handle}"))
                .unwrap_or_else(|| panic!("no grip for {handle}"));
            (*handle, grip.style().base.cursor)
        })
        .collect::<Vec<_>>();
    println!("{cursors:?}");
    assert_eq!(
        cursors,
        [
            ("n", Some(CursorKind::ResizeVertical)),
            ("s", Some(CursorKind::ResizeVertical)),
            ("e", Some(CursorKind::ResizeHorizontal)),
            ("w", Some(CursorKind::ResizeHorizontal)),
            ("ne", None),
            ("nw", None),
            ("se", None),
            ("sw", None),
        ]
    );
}
