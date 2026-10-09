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
//
// The command fills the panel edge to edge: the panel has no padding, its search line spans
// the panel, and a visible title sits on the rows' inset above it.
//
// State: caller owns open, query, and active_value.
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
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false,
                doc: "Whether the dialog is shown; the caller stores the `open_change` payload and passes it back." },
            query: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Search text; the caller stores the `query_change` payload and passes it back." },
            active_value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Highlighted command value; the caller stores the `active_change` payload. Other values show the first enabled match." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Dialog title, also the accessible name of the dialog, the search field and the list." },
            items: #{ schema: #{ type: "one_of", variants: [
                #{ type: "array", max_items: 4096, items: #{
                    type: "object", allow_unknown: false, fields: #{
                        value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                            doc: "Unique, non-empty id reported by `active_change` and `action`." },
                        label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                            doc: "Non-empty text ranked against the query and shown on the row." },
                        keywords: #{ schema: #{ type: "array", max_items: 32, items: #{ type: "string" } },
                            required: false, sensitive: false, "default": #{ type: "array", value: [] },
                            doc: "Extra terms matched against the query; a keyword match ranks just below the same label match." },
                        group: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                            doc: "Heading the command is listed under; groups keep the source order of their first match." },
                        shortcut: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                            doc: "Key legend shown at the row end, written like `cmd-k`; overrides the binding of `action`." },
                        action: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                            doc: "Registered action dispatched on activation; it also supplies the shortcut legend and can disable the row." },
                        disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                            "default": #{ type: "bool", value: false },
                            doc: "Dims the command; it cannot be activated and keyboard navigation skips it." },
                    }
                } },
                #{ type: "collection" }
            ] }, required: true, sensitive: false,
                doc: "Commands to rank: item maps, or a native collection keyed by value with the same fields but no `action`." },
            placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Type a command" },
                doc: "Hint shown in the empty search field." },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "No commands found" },
                doc: "Message shown in place of the list when no command matches." },
            max_visible: #{ schema: #{ type: "integer", min: 1, max: 24 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 8 },
                doc: "Rows the list shows before it scrolls; group headings count as rows." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Disables the search field and the list's arrow, Home, End and Enter keys." },
            close_on_action: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Follows each `action` with `open_change(false)`, so activating a command closes the dialog." },
            title_visible: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Shows `label` as a title above the search; when hidden, the label still names the dialog." },
            width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false,
                doc: "Panel width in place of the dialog's 420 px; the overlay still caps it at 90% of the window." },
            command_part_styles: #{ schema: #{ type: "map", values: #{ type: "style" } },
                required: false, sensitive: false, "default": #{ type: "map", value: #{} },
                doc: "Part styles for the inner Command, such as `search`, `item` and `item_active`." },
            dialog_part_styles: #{ schema: #{ type: "map", values: #{ type: "style" } },
                required: false, sensitive: false, "default": #{ type: "map", value: #{} },
                doc: "Part styles for the inner Dialog, such as `panel` and `title`, merged over the edge-to-edge layout." },
            on_escape: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called on Escape; when set, Escape no longer closes the dialog, so the caller can close it or go back a level." },
            on_query_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the new search text on each edit; store it as `query`." },
            on_active_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the command value to highlight on Up, Down, Home, End or hover; store it as `active_value`." },
            on_action: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the command value when a row is clicked or Enter activates the highlighted command." },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `false` when Escape, an outside click or `close_on_action` closes the dialog; store it as `open`." },
        },
        state: #{ fields: #{} }, events: #{
            query_change: #{ payload: #{ type: "string" },
                doc: "Emitted on each edit of the search field; the payload is the new query." },
            active_change: #{ payload: #{ type: "string" },
                doc: "Emitted when a key or hover moves the highlight; the payload is the newly highlighted value." },
            action: #{ payload: #{ type: "string" },
                doc: "Emitted when a command is clicked or activated with Enter; the payload is its value." },
            open_change: #{ payload: #{ type: "bool" },
                doc: "Emitted when Escape, an outside click or `close_on_action` closes the dialog; the payload is `false`." },
            escape: #{ payload: #{ type: "null" },
                doc: "Emitted on Escape inside the command while `on_escape` is set; the payload is `()`." },
        },
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
