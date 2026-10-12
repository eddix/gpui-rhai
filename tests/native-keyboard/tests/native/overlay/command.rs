//! Command and CommandDialog: filtering from the native input, Enter, grouped headers on the
//! first reveal, and manual scroll against a controlled active item.

use crate::support::{Mount, bounds, by_role, center, palette_texts, snapshot};
use gpui::{
    Modifiers, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext, point, px,
};
use gpui_rhai::{UiRuntimeState, UiValue};

#[gpui::test]
fn command_dialog_filters_from_native_input_and_executes_with_enter(cx: &mut TestAppContext) {
    let (window, view) = Mount::new(
        r#"
                    import "components/command_dialog" as command_dialog;
                    fn state_schema() { #{ fields: #{
                        open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: true } },
                        query: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
                        active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "open" } },
                        active_changes: #{ schema: #{ type: "integer", min: 0 }, "default": #{ type: "integer", value: 0 } },
                        last: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "none" } },
                        actions: #{ schema: #{ type: "integer", min: 0 }, "default": #{ type: "integer", value: 0 } }
                    } } }
                    fn opened(ctx, open) { ctx.set_state("open", open); }
                    fn queried(ctx, query) { ctx.set_state("query", query); }
                    fn active(ctx, value) {
                        ctx.set_state("active", value);
                        ctx.set_state("active_changes", ctx.get_state("active_changes") + 1);
                    }
                    fn action(ctx, value) {
                        ctx.set_state("last", value);
                        ctx.set_state("actions", ctx.get_state("actions") + 1);
                    }
                    fn view(ctx) {
                        column([
                            text(`command:active=${ctx.get_state("active")} changes=${ctx.get_state("active_changes")} last=${ctx.get_state("last")} actions=${ctx.get_state("actions")} open=${ctx.get_state("open")} query=${ctx.get_state("query")}`),
                            command_dialog::CommandDialog(#{ key: "palette", open: ctx.get_state("open"),
                                query: ctx.get_state("query"), active_value: ctx.get_state("active"),
                                label: "Command palette",
                                items: [
                                    #{ value: "new", label: "New file", group: "File", shortcut: "⌘N" },
                                    #{ value: "open", label: "Open file", keywords: ["load document"], group: "File", shortcut: "⌘O" }
                                ], on_open_change: Fn("opened"), on_query_change: Fn("queried"),
                                on_active_change: Fn("active"),
                                on_action: Fn("action") })
                        ])
                    }
        "#,
    )
    .registry("components/command_dialog")
    .registry("components/command")
    .registry("components/dialog")
    .registry("components/input")
    .registry("components/kbd")
    .bundled_assets()
    .ids("command-dialog-window", "command-dialog-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    visual.update(|_, cx| {
        let snapshot = view.automation_snapshot(cx).unwrap();
        assert!(snapshot.nodes.iter().any(|node| {
            node.role == "option" && node.name == "Open file" && node.bounds.is_some()
        }));
    });
    let search = bounds(&mut visual, &view, "text_field", "Command palette");
    assert!(search.width > 0.0 && search.height > 0.0);
    visual.simulate_input("file");
    visual.run_until_parked();
    let texts_after_input = palette_texts(&mut visual, &view);
    assert!(
        texts_after_input.contains(
            &"command:active=open changes=0 last=none actions=0 open=true query=file".to_owned()
        ) && texts_after_input.contains(&"New file".to_owned())
            && texts_after_input.contains(&"Open file".to_owned())
            && !texts_after_input.contains(&"No commands found".to_owned()),
        "the focused CommandDialog input must filter before submit: {texts_after_input:?}"
    );
    let initial_open_selected = snapshot(&mut visual, &view)
        .find_by_role_and_name("option", "Open file")
        .next()
        .unwrap()
        .selected;
    assert_eq!(initial_open_selected, Some(true));
    visual.simulate_keystrokes("down");
    visual.run_until_parked();
    assert!(palette_texts(&mut visual, &view).contains(
        &"command:active=new changes=1 last=none actions=0 open=true query=file".to_owned()
    ));

    let open_file = bounds(&mut visual, &view, "option", "Open file");
    visual.simulate_mouse_move(center(open_file), None, Modifiers::default());
    visual.run_until_parked();
    assert!(palette_texts(&mut visual, &view).contains(
        &"command:active=open changes=2 last=none actions=0 open=true query=file".to_owned()
    ));
    visual.simulate_mouse_move(center(open_file), None, Modifiers::default());
    visual.run_until_parked();
    assert!(palette_texts(&mut visual, &view).contains(
        &"command:active=open changes=2 last=none actions=0 open=true query=file".to_owned()
    ));

    visual.simulate_keystrokes("enter");
    visual.run_until_parked();
    let texts = palette_texts(&mut visual, &view);
    let error = visual.update(|_, cx| view.last_error(cx));
    assert!(
        texts.contains(
            &"command:active=open changes=2 last=open actions=1 open=false query=file".to_owned()
        ),
        "command dialog did not execute: search={search:?} texts={texts:?} error={error:?}"
    );
    visual.update(|window, cx| {
        let snapshot = view.automation_snapshot(cx).unwrap();
        assert!(
            !snapshot
                .nodes
                .iter()
                .any(|node| node.role == "option" && node.name == "Open file")
        );
        let query = view.automate(
            gpui_rhai::AutomationCommand::Query {
                locator: by_role("option", "Open file"),
            },
            window,
            cx,
        );
        assert!(matches!(
            query,
            Err(gpui_rhai::ScriptViewError::Automation(
                gpui_rhai::AutomationError::NoMatch(_)
            ))
        ));
    });
}

#[gpui::test]
fn grouped_command_initial_reveal_keeps_its_first_header_natural(cx: &mut TestAppContext) {
    struct CommandData(gpui_rhai::NativeCollection);
    impl gpui_rhai::ScriptViewExtension for CommandData {
        fn configure_runtime(&self, runtime: &mut UiRuntimeState) -> Result<(), String> {
            runtime
                .native_collections
                .register("commands", self.0.clone())
                .map_err(|error| error.to_string())
        }
    }

    cx.update(gpui_rhai::install);
    let rows = (0..24)
        .map(|index| {
            std::collections::BTreeMap::from([
                ("id".to_owned(), UiValue::String(format!("command-{index}"))),
                (
                    "label".to_owned(),
                    UiValue::String(format!("Command {index}")),
                ),
                (
                    "group".to_owned(),
                    UiValue::String(
                        if index < 8 {
                            "Group A"
                        } else if index < 16 {
                            "Group B"
                        } else {
                            "Group C"
                        }
                        .to_owned(),
                    ),
                ),
                ("keywords".to_owned(), UiValue::Array(Vec::new())),
                ("shortcut".to_owned(), UiValue::String(String::new())),
                ("disabled".to_owned(), UiValue::Bool(false)),
            ])
        })
        .collect::<Vec<_>>();
    let commands = gpui_rhai::NativeCollection::new("id", rows).unwrap();
    let (window, view) = Mount::new(
        r#"
                    import "components/command" as command;
                    fn active(ctx, value) { () }
                    fn action(ctx, value) { () }
                    fn view(ctx) {
                        command::Command(#{ key: "grouped-commands", label: "Commands", query: "",
                            active_value: "command-0", items: ctx.get_native_collection("commands"),
                            max_visible: 12,
                            on_active_change: Fn("active"), on_action: Fn("action") })
                    }
        "#,
    )
    .registry("components/command")
    .registry("components/input")
    .registry("components/kbd")
    .bundled_assets()
    .with(|view| view.extension(CommandData(commands)))
    .ids("grouped-command-window", "grouped-command-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    let initial = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap()
        .virtual_collections
        .into_iter()
        .next()
        .unwrap();
    assert_eq!((initial.scroll_item, initial.scroll_offset), (0, 0.0));
    assert_eq!(initial.sticky_header, None);
    assert!(
        visual
            .debug_bounds("virtual-list-sticky:grouped-commands-list")
            .is_none()
    );
    assert!(
        snapshot(&mut visual, &view)
            .find_by_role_and_name("heading", "Group A")
            .next()
            .is_some(),
        "the first group header must remain in the natural row sequence"
    );

    let option = bounds(&mut visual, &view, "option", "Command 3");
    visual.simulate_event(ScrollWheelEvent {
        position: center(option),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-160.0))),
        ..ScrollWheelEvent::default()
    });
    for _ in 0..4 {
        cx.background_executor
            .advance_clock(std::time::Duration::from_millis(16));
        visual.run_until_parked();
    }
    let scrolled = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap()
        .virtual_collections
        .into_iter()
        .next()
        .unwrap();
    assert!(
        scrolled.scroll_item > 0,
        "real wheel input must still advance the grouped Command after preserving its initial origin: {scrolled:?}"
    );
}

#[gpui::test]
fn command_preserves_manual_scroll_and_reveals_controlled_active_item(cx: &mut TestAppContext) {
    let (window, view) = Mount::new(
        r#"
                    import "components/command" as command;
                    fn state_schema() { #{ fields: #{
                        active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "command-0" } }
                    } } }
                    fn active(ctx, value) { ctx.set_state("active", value); }
                    fn action(ctx, value) { () }
                    fn commands() {
                        let items = [];
                        for index in 0..16 {
                            items.push(#{ value: `command-${index}`, label: `Command ${index}` });
                        }
                        items
                    }
                    fn view(ctx) {
                        column([
                            text(`active:${ctx.get_state("active")}`),
                            command::Command(#{ key: "commands", label: "Commands", query: "",
                                active_value: ctx.get_state("active"), items: commands(),
                                max_visible: 12, autofocus: true,
                                on_active_change: Fn("active"), on_action: Fn("action") })
                        ]).with_style(style().width(relative(1)).height(relative(1)).padding(px(16)))
                    }
        "#,
    )
    .registry("components/command")
    .registry("components/input")
    .registry("components/kbd")
    .bundled_assets()
    .ids("command-scroll-window", "command-scroll-view")
    .mount_window(cx);
    cx.refresh().unwrap();
    cx.run_until_parked();
    let mut visual = VisualTestContext::from_window(*window, cx);
    let row = bounds(&mut visual, &view, "option", "Command 6");
    visual.simulate_event(ScrollWheelEvent {
        position: center(row),
        delta: ScrollDelta::Pixels(point(px(0.0), px(-160.0))),
        ..ScrollWheelEvent::default()
    });
    visual.run_until_parked();
    let manually_scrolled = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap()
        .virtual_collections
        .into_iter()
        .next()
        .unwrap();
    assert!(
        manually_scrolled.scroll_item > 0,
        "manual wheel scroll must survive realization rerenders: {manually_scrolled:?}"
    );

    let hovered = bounds(&mut visual, &view, "option", "Command 8");
    visual.simulate_mouse_move(center(hovered), None, Modifiers::default());
    visual.run_until_parked();
    let after_hover = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap()
        .virtual_collections
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(
        (after_hover.scroll_item, after_hover.scroll_offset),
        (
            manually_scrolled.scroll_item,
            manually_scrolled.scroll_offset
        ),
        "activating an already-visible hovered row must not snap the list: before={manually_scrolled:?} after={after_hover:?}"
    );

    for _ in 0..8 {
        visual.simulate_keystrokes("up");
        visual.run_until_parked();
    }
    assert!(palette_texts(&mut visual, &view).contains(&"active:command-0".to_owned()));

    for _ in 0..13 {
        visual.simulate_keystrokes("down");
        visual.run_until_parked();
    }
    assert!(palette_texts(&mut visual, &view).contains(&"active:command-13".to_owned()));
    let followed = visual
        .update(|_, cx| view.take_performance_snapshot(cx))
        .unwrap()
        .virtual_collections
        .into_iter()
        .next()
        .unwrap();
    assert!(
        followed.scroll_item > 0 && followed.visible_range.contains(&13),
        "controlled active item must be revealed: {followed:?}"
    );
    let active = snapshot(&mut visual, &view)
        .find_by_role_and_name("option", "Command 13")
        .next()
        .and_then(|node| node.geometry);
    assert!(
        active.is_some(),
        "active option must participate in the frame"
    );
}
