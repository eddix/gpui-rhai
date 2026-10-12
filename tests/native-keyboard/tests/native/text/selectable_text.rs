//! Selectable text uses native selection and copy semantics.

use crate::support::{Mount, node_texts};
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};

#[gpui::test]
fn selectable_text_uses_native_selection_and_copy_semantics(cx: &mut TestAppContext) {
    let (window, view) = Mount::new(
        r#"
                fn state_schema() { #{ fields: #{
                    clicks: #{ schema: #{ type: "integer" },
                        "default": #{ type: "integer", value: 0 } }
                } } }
                fn parent_clicked(ctx, payload) {
                    ctx.set_state("clicks", ctx.get_state("clicks") + 1);
                }
                fn view(ctx) {
                    column([
                        text("Copy this 错误\nsecond line").selectable(true)
                            .with_key("err")
                            .test_id("err")
                            .accessibility_role("document")
                            .accessibility_label("error text"),
                        text(`clicks: ${ctx.get_state("clicks")}`)
                    ]).on_click(Fn("parent_clicked"))
                }
            "#,
    )
    .bundled_assets()
    .ids("selectable-window", "selectable-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.run_until_parked();

    let result = visual
        .update(|window, cx| {
            view.automate(
                gpui_rhai::AutomationCommand::Query {
                    locator: gpui_rhai::AutomationLocator::TestId {
                        id: "err".to_owned(),
                    },
                },
                window,
                cx,
            )
        })
        .unwrap();
    let gpui_rhai::AutomationResult::Node { node } = result else {
        panic!("expected node result");
    };
    let bounds = node.bounds.expect("selectable text has committed bounds");

    cx.write_to_clipboard(gpui::ClipboardItem::new_string("sentinel".to_owned()));
    // Selection itself must not mutate the clipboard or trigger the clickable
    // ancestor. Copy remains a separate, focus-routed platform action.
    let start = point(px((bounds.x + 2.0) as f32), px((bounds.y + 2.0) as f32));
    let end = point(
        px((bounds.x + bounds.width - 1.0) as f32),
        px((bounds.y + bounds.height - 1.0) as f32),
    );
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();

    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()),
        Some("sentinel".to_owned())
    );
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut texts = Vec::new();
    node_texts(&root, &mut texts);
    assert!(texts.contains(&"clicks: 0".to_owned()), "{texts:?}");

    cx.simulate_keystrokes(*window, "cmd-c");
    cx.run_until_parked();
    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()),
        Some("Copy this 错误\nsecond line".to_owned())
    );
}
