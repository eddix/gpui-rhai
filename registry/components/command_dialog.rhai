/* gpui-rhai
{
  "id": "components/command_dialog",
  "export": "CommandDialog",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/command", "components/dialog"],
  "capabilities": {}
}
*/
// Controlled modal presentation of Command without registering a global shortcut.
// Props: Command props plus open, close_on_action, and on_open_change.
// Events: query_change(string), action(string), and open_change(bool).
// State: caller owns open/query; Command owns only transient roving state.
// Parts: root, dialog, command.
// Example: CommandDialog(#{ key: "palette", open: open, query: query, label: "Commands", items: items, on_open_change: Fn("opened"), on_query_change: Fn("queried"), on_action: Fn("run") })

import "components/command" as command;
import "components/dialog" as dialog;

define_component(#{
    metadata: #{ id: "components/command_dialog", "export": "CommandDialog", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/command", "components/dialog"], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            query: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            items: #{ schema: #{ type: "one_of", variants: [
                #{ type: "array", max_items: 4096, items: #{
                    type: "object", allow_unknown: false, fields: #{
                        value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                        label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                        keywords: #{ schema: #{ type: "array", max_items: 32, items: #{ type: "string" } },
                            required: false, sensitive: false, "default": #{ type: "array", value: [] } },
                        group: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                        shortcut: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                        disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                            "default": #{ type: "bool", value: false } },
                    }
                } },
                #{ type: "collection" }
            ] }, required: true, sensitive: false },
            placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Type a command" } },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "No commands found" } },
            max_visible: #{ schema: #{ type: "integer", min: 1, max: 24 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 8 } },
            row_height: #{ schema: #{ type: "integer", min: 24, max: 64 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 32 } },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            close_on_action: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            on_query_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_action: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{ query_change: #{ payload: #{ type: "string" } },
            action: #{ payload: #{ type: "string" } }, open_change: #{ payload: #{ type: "bool" } } },
        slots: #{}, parts: ["root", "dialog", "command"],
    },
    render: Fn("render_CommandDialog")
});

fn CommandDialog(props) { render_component("components/command_dialog", props) }
fn query_action(ctx, query) { ctx.emit("query_change", query); }
fn open_action(ctx, open) { ctx.emit("open_change", open); }
fn command_action(close, ctx, value) {
    ctx.emit("action", value);
    if close { ctx.emit("open_change", false); }
}
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }

fn render_CommandDialog(ctx, props) {
    let list = command::Command(#{ key: `${props.key}-command`, label: props.label,
        query: props.query, items: props.items,
        placeholder: prop_or(props, "placeholder", "Type a command"),
        empty_text: prop_or(props, "empty_text", "No commands found"),
        max_visible: prop_or(props, "max_visible", 8),
        row_height: prop_or(props, "row_height", 32),
        disabled: prop_or(props, "disabled", false),
        autofocus: true,
        on_query_change: Fn("query_action"),
        on_action: Fn("command_action").curry(prop_or(props, "close_on_action", true)) })
        .with_style(component_style(props, "command", style().border(px(0))));
    dialog::Dialog(#{ key: props.key, open: props.open, title: props.label,
        content: list, initial_focus: "first", on_open_change: Fn("open_action") })
        .with_style(component_style(props, "dialog", component_style(props, "root", style())))
}
