/* gpui-rhai
{
  "id": "components/command_dialog",
  "export": "CommandDialog",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/command", "components/dialog"],
  "capabilities": {},
  "tokens": ["metrics.inset", "metrics.command_title_inset", "radius.lg"],
  "environment": ["density"]
}
*/
// Controlled modal presentation of Command without registering a global shortcut.
//
// The command fills the dialog's content width without adding a second panel inset. The
// dialog owns its corner clearance; a visible title sits on the rows' text edge above it.
//
// State: caller owns open, query, and active_value.
// Example: CommandDialog(#{ key: "palette", open: open, query: query, active_value: active, label: "Commands", items: items, on_open_change: Fn("opened"), on_query_change: Fn("queried"), on_active_change: Fn("preview"), on_action: Fn("run") })

import "components/command" as command;
import "components/dialog" as dialog;

define_component(#{
    metadata: #{ id: "components/command_dialog", "export": "CommandDialog", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/command", "components/dialog"], capabilities: #{},
        tokens: ["metrics.inset", "metrics.command_title_inset", "radius.lg"],
        environment: ["density"] },
    schema: #{
        props: #{
            footer_visible: #{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:true},
                doc:"Shows the inner Command's keyboard-hint footer."},
            footer: #{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,
                doc:"Optional application-owned footer content for the inner Command."},
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false,
                doc: "Whether the dialog is shown; the caller stores the `open_change` payload and passes it back." },
            query: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Search text; the caller stores the `query_change` payload and passes it back." },
            active_value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Highlighted command value; the caller stores the `active_change` payload. Other values show the first enabled match." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Dialog title, also the accessible name of the dialog, the search field and the list." },
            items: #{ schema: /* @contract command-items:start */ #{"type": "one_of", "variants": [#{"type": "array", "max_items": 4096, "items": #{"type": "object", "allow_unknown": false, "fields": #{"value": #{"schema": #{"type": "string"}, "required": true, "sensitive": false, "doc": "Unique non-empty value reported by active_change and action."}, "label": #{"schema": #{"type": "string"}, "required": true, "sensitive": false, "doc": "Required searchable/accessibility text; content may replace its visual label."}, "content": #{"schema": #{"type": "optional", "value": #{"type": "node"}}, "required": false, "sensitive": false, "doc": "Rich visual label; ranking and accessible name still use label."}, "hint": #{"schema": #{"type": "optional", "value": #{"type": "string"}}, "required": false, "sensitive": false, "doc": "Inline secondary text, also searched with keyword precedence."}, "icon": #{"schema": #{"type": "optional", "value": #{"type": "one_of", "variants": [#{"type": "node"}, #{"type": "string"}]}}, "required": false, "sensitive": false, "doc": "Non-interactive glyph node or logical asset ID; the leading slot is reserved when empty."}, "logo": #{"schema": #{"type": "optional", "value": #{"type": "one_of", "variants": [#{"type": "node"}, #{"type": "string"}]}}, "required": false, "sensitive": false, "doc": "Non-interactive logo node or logical asset ID on a tile; takes precedence over icon."}, "keywords": #{"schema": #{"type": "array", "max_items": 32, "items": #{"type": "string"}}, "required": false, "sensitive": false, "default": #{"type": "array", "value": []}, "doc": "Additional search terms with keyword precedence."}, "group": #{"schema": #{"type": "optional", "value": #{"type": "string"}}, "required": false, "sensitive": false, "doc": "Heading; groups preserve first matching source order."}, "shortcut": #{"schema": #{"type": "optional", "value": #{"type": "string"}}, "required": false, "sensitive": false, "doc": "Row-end shortcut legend overriding a registered action binding."}, "action": #{"schema": #{"type": "optional", "value": #{"type": "string"}}, "required": false, "sensitive": false, "doc": "Registered action dispatched on activation; its enabled state and shortcut are inherited."}, "disabled": #{"schema": #{"type": "bool"}, "required": false, "sensitive": false, "default": #{"type": "bool", "value": false}, "doc": "Dimmed and excluded from activation and navigation."}}}}, #{"type": "collection"}]} /* @contract command-items:end */, required: true, sensitive: false,
                doc: "Exactly the Command item schema, including rich labels, hints, glyphs/logo tiles and the NativeCollection data path." },
            placeholder: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "Hint shown in the empty search field. Empty, the localized `command.placeholder` message, `Type a command` without one." },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "Message shown in place of the list when no command matches. Empty, the localized `command.empty` message, `No commands found` without one." },
            max_visible: #{ schema: #{ type: "integer", min: 1, max: 24 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 8 },
                doc: "Inner Command viewport capacity in row units; headings use natural measured height." },
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
            narrow_at: #{schema:#{type:"number",min:0.0,max:16384.0},required:false,sensitive:false,
                "default":#{type:"float",value:480.0},doc:"Forwards the inner Command's committed-card width boundary; it is independent of window viewport class."},
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
        slots: #{footer:#{required:false,multiple:false,doc:"Optional content passed to the Command footer."}}, parts: ["root", "dialog", "command"],
    },
    render: Fn("render_CommandDialog")
});

// The text prop `name`, or the localized `key` message (`fallback` without one) while it
// is empty or unset.
fn localized_prop(ctx, props, name, key, fallback) {
    let value = if name in props && props[name] != () { props[name] } else { "" };
    if value == "" { ctx.t_or(key, fallback) } else { value }
}

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
        appearance:"embedded",footer_visible:props.footer_visible,footer:prop_or(props,"footer",()),narrow_at:props.narrow_at,
        query: props.query, active_value: props.active_value, items: props.items,
        placeholder: localized_prop(ctx, props, "placeholder", "command.placeholder", "Type a command"),
        empty_text: localized_prop(ctx, props, "empty_text", "command.empty", "No commands found"),
        max_visible: prop_or(props, "max_visible", 8),
        disabled: prop_or(props, "disabled", false),
        autofocus: true,
        on_query_change: Fn("query_action"),
        on_active_change: Fn("active_action"),
        on_action: Fn("command_action").curry(prop_or(props, "close_on_action", true)),
        part_styles: prop_or(props, "command_part_styles", #{}) })
        .with_style(ctx.component_style("command", style()));
    let escape = "on_escape" in props && props.on_escape != ();
    if escape { list = list.on_key_value("escape", Fn("escape_action"), ()); }
    let dialog_parts = prop_or(props, "dialog_part_styles", #{});
    // The Dialog owns the one surrounding inset. The embedded command owns
    // search/list spacing; its title follows the same leading-content grid.
    let panel = style().padding(theme_radius("lg") * 0.5).gap(px(0));
    if "panel" in dialog_parts { panel = panel.merge(dialog_parts.panel); }
    if "width" in props && props.width != () { panel = panel.width(props.width); }
    dialog_parts.panel = panel;
    let title = style().padding_x(theme_length("metrics.command_title_inset")).padding_top(theme_length("metrics.inset"));
    dialog_parts.title = if "title" in dialog_parts { title.merge(dialog_parts.title) } else { title };
    dialog::Dialog(#{ key: props.key, open: props.open, title: props.label,
        title_visible: prop_or(props, "title_visible", true), dismiss_on_escape: !escape,
        content: list, initial_focus: "first", on_open_change: Fn("open_action"),
        part_styles: dialog_parts })
        .with_style(ctx.component_style(["root","dialog"], style()))
}
