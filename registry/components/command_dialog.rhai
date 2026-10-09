/* gpui-rhai
{
  "id": "components/command_dialog",
  "export": "CommandDialog",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/command", "components/dialog"],
  "capabilities": {},
  "tokens": ["metrics.inset"],
  "environment": ["density"]
}
*/
// Controlled modal presentation of Command without registering a global shortcut.
// Props: controlled Command props plus open, close_on_action, on_open_change,
//        title_visible (default true; the label always names the dialog), width,
//        on_escape, command_part_styles and dialog_part_styles.
// Events: query_change(string), active_change(string), action(string), open_change(bool),
//         and escape(null) when on_escape is set.
// State: caller owns open, query, and active_value.
// Parts: root, dialog, command. `command_part_styles` and `dialog_part_styles` reach the
// inner Command (search, item, item_active, ...) and Dialog (panel, title, ...) parts.
// With `on_escape` the dialog no longer closes on Escape: the caller sees it first and
// closes through `open`, so a palette with levels can go back one level instead.
// The command fills the panel edge to edge: the panel has no padding, its search line spans
// the panel, and a visible title sits on the rows' inset above it.
// Example: CommandDialog(#{ key: "palette", open: open, query: query, active_value: active, label: "Commands", items: items, on_open_change: Fn("opened"), on_query_change: Fn("queried"), on_active_change: Fn("preview"), on_action: Fn("run") })

import "components/command" as command;
import "components/dialog" as dialog;

define_component(#{
    metadata: #{ id: "components/command_dialog", "export": "CommandDialog", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/command", "components/dialog"], capabilities: #{},
        tokens: ["metrics.inset"],
        environment: ["density"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            query: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            active_value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
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
                        action: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
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
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            close_on_action: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            title_visible: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false },
            command_part_styles: #{ schema: #{ type: "map", values: #{ type: "style" } },
                required: false, sensitive: false, "default": #{ type: "map", value: #{} } },
            dialog_part_styles: #{ schema: #{ type: "map", values: #{ type: "style" } },
                required: false, sensitive: false, "default": #{ type: "map", value: #{} } },
            on_escape: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_query_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_active_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_action: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{ query_change: #{ payload: #{ type: "string" } },
            active_change: #{ payload: #{ type: "string" } }, action: #{ payload: #{ type: "string" } },
            open_change: #{ payload: #{ type: "bool" } }, escape: #{ payload: #{ type: "null" } } },
        slots: #{}, parts: ["root", "dialog", "command"],
    },
    render: Fn("render_CommandDialog")
});

fn CommandDialog(props) { render_component("components/command_dialog", props) }
fn query_action(ctx, query) { ctx.emit("query_change", query); }
fn active_action(ctx, value) { ctx.emit("active_change", value); }
fn open_action(ctx, open) { ctx.emit("open_change", open); }
fn escape_action(ctx, payload) { ctx.emit("escape", ()); }
fn command_action(close, ctx, value) {
    ctx.emit("action", value);
    if close { ctx.emit("open_change", false); }
}
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }

fn render_CommandDialog(ctx, props) {
    let list = command::Command(#{ key: `${props.key}-command`, label: props.label,
        query: props.query, active_value: props.active_value, items: props.items,
        placeholder: prop_or(props, "placeholder", "Type a command"),
        empty_text: prop_or(props, "empty_text", "No commands found"),
        max_visible: prop_or(props, "max_visible", 8),
        disabled: prop_or(props, "disabled", false),
        autofocus: true,
        on_query_change: Fn("query_action"),
        on_active_change: Fn("active_action"),
        on_action: Fn("command_action").curry(prop_or(props, "close_on_action", true)),
        part_styles: prop_or(props, "command_part_styles", #{}) })
        .with_style(ctx.component_style("command", style().border(px(0))));
    let escape = "on_escape" in props && props.on_escape != ();
    if escape { list = list.on_key_value("escape", Fn("escape_action"), ()); }
    let dialog_parts = prop_or(props, "dialog_part_styles", #{});
    let panel = style().padding(px(0)).gap(px(0));
    if "panel" in dialog_parts { panel = panel.merge(dialog_parts.panel); }
    if "width" in props && props.width != () { panel = panel.width(props.width); }
    dialog_parts.panel = panel;
    let title = style().padding_x(theme_length("metrics.inset")).padding_top(theme_length("metrics.inset"));
    dialog_parts.title = if "title" in dialog_parts { title.merge(dialog_parts.title) } else { title };
    dialog::Dialog(#{ key: props.key, open: props.open, title: props.label,
        title_visible: prop_or(props, "title_visible", true), dismiss_on_escape: !escape,
        content: list, initial_focus: "first", on_open_change: Fn("open_action"),
        part_styles: dialog_parts })
        .with_style(ctx.component_style("dialog", ctx.component_style("root", style())))
}
