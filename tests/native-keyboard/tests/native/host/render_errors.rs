//! A failed render keeps the last good root, and a Host can suppress the built-in error
//! banner without hiding the last error.

use crate::support::{Mount, by_role, dispatch, last_error, node_texts};
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_rhai::UiValue;

/// A view whose `BrokenPanel` throws while `broken` is set.
fn failure_view() -> Mount {
    Mount::new(
        r#"
                import "components/broken_panel" as broken_panel;
                fn state_schema() { #{ fields: #{
                    broken: #{ schema: #{ type: "bool" },
                        "default": #{ type: "bool", value: false } }
                } } }
                fn break_render(ctx, payload) { ctx.set_state("broken", true); }
                fn recover(ctx, payload) { ctx.set_state("broken", false); }
                fn view(ctx) {
                    broken_panel::BrokenPanel(#{
                        key: "panel", broken: ctx.get_state("broken"),
                        on_break: Fn("break_render"), on_recover: Fn("recover")
                    })
                }
            "#,
    )
    .module(
        "components/broken_panel",
        r#"/* gpui-rhai
{
  "id": "components/broken_panel",
  "export": "BrokenPanel",
  "version": "0.1.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {}
}
*/
                define_component(#{
                    metadata: #{ id: "components/broken_panel", "export": "BrokenPanel",
                        version: "0.1.3", runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
                        dependencies: [], capabilities: #{} },
                    schema: #{ props: #{
                        broken: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
                        on_break: #{ schema: #{ type: "callback" }, required: true, sensitive: false },
                        on_recover: #{ schema: #{ type: "callback" }, required: true, sensitive: false },
                    }, state: #{ fields: #{
                        secret: #{ schema: #{ type: "string" },
                            "default": #{ type: "string", value: "diagnostic-secret" },
                            sensitive: true }
                    } }, events: #{}, slots: #{}, parts: ["root"] },
                    render: Fn("render_BrokenPanel")
                });
                fn BrokenPanel(props) { render_component("components/broken_panel", props) }
                fn render_BrokenPanel(ctx, props) {
                    let secret = ctx.get_state("secret");
                    if props.broken { throw "dogfood render failure"; }
                    column([
                        text("last-good tree")
                            .accessibility_role("button")
                            .accessibility_label("Break render")
                            .on_click(props.on_break),
                        text("Recover")
                            .accessibility_role("button")
                            .accessibility_label("Recover")
                            .on_click(props.on_recover)
                    ]).with_style(ctx.component_style("root", style()))
                }
            "#,
    )
    .bundled_assets()
}

#[gpui::test]
fn mounted_view_exposes_failed_render_while_retaining_last_good_root(cx: &mut TestAppContext) {
    let (window, view) = failure_view()
        .ids("failure-window", "failure-view")
        .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let mut visual = VisualTestContext::from_window(*window, cx);
    assert_eq!(last_error(&mut visual, &view), None);
    let automation_error = dispatch(
        &mut visual,
        &view,
        by_role("button", "Break render"),
        "click",
    )
    .unwrap_err();
    assert!(
        automation_error
            .to_string()
            .contains("dogfood render failure")
    );
    visual.run_until_parked();

    let error = last_error(&mut visual, &view).unwrap();
    assert!(error.contains("dogfood render failure"), "{error}");
    let diagnostic = visual.update(|_, cx| view.last_diagnostic(cx).unwrap().unwrap());
    assert_eq!(
        diagnostic.component.as_deref(),
        Some("/View[failure-view]/BrokenPanel[panel]")
    );
    assert_eq!(diagnostic.key.as_deref(), Some("panel"));
    assert_eq!(diagnostic.component_state.len(), 1);
    assert_eq!(
        diagnostic.component_state[0].fields["secret"],
        UiValue::String("<redacted>".to_owned())
    );
    assert!(
        !serde_json::to_string(&diagnostic)
            .unwrap()
            .contains("diagnostic-secret")
    );
    let root = visual.update(|_, cx| view.root(cx).unwrap().unwrap());
    let mut texts = Vec::new();
    node_texts(&root, &mut texts);
    assert_eq!(texts, ["last-good tree", "Recover"]);

    let banner = visual
        .debug_bounds("gpui-rhai-error-banner:failure-view")
        .expect("default runtime error banner must remain visible");
    let start = point(banner.origin.x + px(10.0), banner.origin.y + px(10.0));
    let end = point(
        banner.origin.x + banner.size.width - px(10.0),
        banner.origin.y + banner.size.height - px(10.0),
    );
    visual.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    visual.simulate_mouse_move(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    visual.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    visual.run_until_parked();
    cx.simulate_keystrokes(*window, "cmd-c");
    cx.run_until_parked();
    let copied = cx
        .read_from_clipboard()
        .and_then(|item| item.text())
        .expect("selecting the error banner must populate the clipboard on copy");
    assert!(copied.contains("dogfood render failure"), "{copied}");

    dispatch(&mut visual, &view, by_role("button", "Recover"), "click").unwrap();
    visual.run_until_parked();
    // A later success does not hide the failure: the rolled-back event is
    // still reported until the developer dismisses it.
    let error = last_error(&mut visual, &view).unwrap();
    assert!(error.contains("dogfood render failure"), "{error}");
    let dismiss = visual
        .debug_bounds("gpui-rhai-error-banner-dismiss:failure-view")
        .expect("the banner offers a dismiss button");
    visual.simulate_click(dismiss.center(), Modifiers::default());
    visual.run_until_parked();
    assert!(last_error(&mut visual, &view).is_none());
    assert!(
        visual
            .update(|_, cx| view.last_diagnostic(cx))
            .unwrap()
            .is_none()
    );
    visual.update(|window, _| window.refresh());
    visual.run_until_parked();
    assert!(
        visual
            .debug_bounds("gpui-rhai-error-banner:failure-view")
            .is_none()
    );
}

#[gpui::test]
fn host_can_suppress_the_builtin_error_banner_without_hiding_last_error(cx: &mut TestAppContext) {
    let (window, view) = failure_view()
        .config(|config| config.show_error_banner(false))
        .ids("custom-error-window", "custom-error-view")
        .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();

    let mut visual = VisualTestContext::from_window(*window, cx);
    let automation_error = dispatch(
        &mut visual,
        &view,
        by_role("button", "Break render"),
        "click",
    )
    .unwrap_err();
    assert!(
        automation_error
            .to_string()
            .contains("dogfood render failure")
    );
    visual.run_until_parked();

    let error = last_error(&mut visual, &view).unwrap();
    assert!(error.contains("dogfood render failure"), "{error}");
    assert!(
        visual
            .debug_bounds("gpui-rhai-error-banner:custom-error-view")
            .is_none(),
        "the host-owned error surface must not compete with a built-in banner"
    );
    visual.update(|_, cx| view.clear_error(cx)).unwrap();
    assert_eq!(last_error(&mut visual, &view), None);
}
