/* gpui-rhai
{
  "id": "components/menu",
  "export": "Menu",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/divider", "components/kbd"],
  "capabilities": {},
  "assets": ["icons/check.svg"],
  "tokens": [
    "surface_raised", "border", "focus_ring", "surface_hover", "selection", "accent",
    "text.accent", "text_primary", "text_muted", "disabled", "metrics.row", "metrics.inset",
    "metrics.icon", "spacing.xs", "spacing.sm", "radius.lg", "typography.body",
    "typography.label"
  ],
  "environment": ["density", "corners"]
}
*/
// Controlled in-window menu with roving selection and nested submenu nodes.
// Props: key, label, parent_overlay, trigger, open, active_value, items, placement,
// initial_focus (panel|first: where focus lands on each open), callbacks.
// Items: kind item|separator|submenu|label; an item `action` dispatches that registered
// action, derives its shortcut legend and enabled state, and closes the menu.
// Events: action(string), active_change(string), open_change(bool).
// State: controlled open/active values.
// Example: Menu(#{ key: "file", label: "File menu", trigger: text("File"), open: true, active_value: "open", items: items })

import "components/divider" as divider;
import "components/kbd" as kbd;

define_component(#{
    metadata: #{
        id: "components/menu", "export": "Menu", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/divider", "components/kbd"], capabilities: #{},
        tokens: [
            "surface_raised", "border", "focus_ring", "surface_hover", "selection", "accent",
            "text.accent", "text_primary", "text_muted", "disabled", "metrics.row",
            "metrics.inset", "metrics.icon", "spacing.xs", "spacing.sm", "radius.lg",
            "typography.body", "typography.label"
        ],
        environment: ["density", "corners"], assets: ["icons/check.svg"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            parent_overlay: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            trigger: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            active_value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "bottom" } },
            initial_focus: #{ schema: #{ type: "string", allowed: ["panel", "first"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "panel" } },
            anchor: #{ schema: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                width: #{ schema: #{ type: "number", min: 0.0 }, required: false, sensitive: false,
                    "default": #{ type: "float", value: 0.0 } },
                height: #{ schema: #{ type: "number", min: 0.0 }, required: false, sensitive: false,
                    "default": #{ type: "float", value: 0.0 } },
            } } }, required: false, sensitive: false },
            activate_on_trigger: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            items: #{ schema: #{ type: "array", max_items: 512, items: #{
                type: "object", allow_unknown: false, fields: #{
                    kind: #{ schema: #{ type: "string", allowed: ["item", "separator", "submenu", "label"] }, required: true, sensitive: false },
                    value: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                    label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                    shortcut: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                    action: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                    checked: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                    submenu: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
                },
            } }, required: true, sensitive: false },
            on_action: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_active_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_context_request: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{
            action: #{ payload: #{ type: "string" } },
            active_change: #{ payload: #{ type: "string" } },
            open_change: #{ payload: #{ type: "bool" } },
            context_request: #{ payload: #{ type: "ui_value" } },
        },
        slots: #{ trigger: #{ required: true, multiple: false } },
        parts: ["root", "trigger", "content", "item", "separator", "shortcut", "submenu",
            "label", "check", "indicator_bar"],
    },
    render: Fn("render_Menu")
});

fn item_disabled(item) { "disabled" in item && item.disabled }
fn selectable(item) {
    (item.kind == "item" || item.kind == "submenu") && !item_disabled(item) && "value" in item && item.value != ()
}
fn item_action(item) { if "action" in item && item.action != () { item.action } else { () } }
// Items bound to an action dispatch it, report the activation and close the menu.
fn dispatch_item(spec, ctx, payload) {
    ctx.dispatch_action(spec.action, ());
    if spec.value != () { ctx.emit("action", spec.value); }
    ctx.emit("open_change", false);
}
fn active_index(items, value) {
    let result = 0;
    for index in 0..items.len { if selectable(items[index]) && items[index].value == value { result = index; } }
    result
}
fn next_item(items, value, step) {
    if items.len == 0 { return value; }
    let current = active_index(items, value);
    let result = value;
    for offset in 1..(items.len + 1) {
        let index = if step > 0 { (current + offset) % items.len }
            else { (current + items.len - (offset % items.len)) % items.len };
        if result == value && selectable(items[index]) { result = items[index].value; }
    }
    result
}
fn next_typeahead(items, value, key) {
    if items.len == 0 { return value; }
    let current = active_index(items, value);
    let result = value;
    for offset in 1..(items.len + 1) {
        let index = (current + offset) % items.len;
        let item = items[index];
        let label = if "label" in item && item.label != () { item.label } else { "" };
        if result == value && selectable(item) && label.to_lower().starts_with(key) {
            result = item.value;
        }
    }
    result
}

fn Menu(props) { render_component("components/menu", props) }
fn context_request(ctx, event) {
    if event.button != "right" { return propagate(); }
    ctx.emit("context_request", event);
    event_response().prevent_default().stop()
}
fn menu_indicator(ctx) {
    row([]).with_style(ctx.component_style("indicator_bar", style()
        .absolute().inset_start(px(0)).top(px(0)).bottom(px(0))
        .width(px(2)).background(theme_color("accent"))))
}

fn menu_item(ctx, props, item) {
    let checked = "checked" in item && item.checked;
    let action = item_action(item);
    let disabled = item_disabled(item) || (action != () && !ctx.action_enabled(action));
    let shortcut = if "shortcut" in item && item.shortcut != () { item.shortcut }
        else if action != () {
            let bound = ctx.action_shortcut(action);
            if bound == () { "" } else { bound.label }
        } else { "" };
    let label = if "label" in item && item.label != () { item.label } else { "" };
    let active = "value" in item && item.value == props.active_value;
    // List rows: metrics.row high, text at metrics.inset; the cursor is a selection
    // block plus the 2px indicator bar inside the inset.
    let item_style = style().height(theme_length("metrics.row")).padding_x(theme_length("metrics.inset"))
        .gap(theme_spacing("sm")).items_center().typography("body")
        .text_color(theme_color("text_primary"));
    item_style = if active {
        item_style.background(theme_color("selection")).hover(style().background(theme_color("selection")))
    } else { item_style.hover(style().background(theme_color("surface_hover"))) };
    item_style = item_style.disabled(style().text_color(theme_color("disabled")).background(rgba(0x00000000)));
    let children = [
        image_source(asset("app/icons/check")).with_style(ctx.component_style("check", style()
            .width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
            .text_color(theme_color("text.accent"))
            .opacity(if checked { 1.0 } else { 0.0 }))),
        text(label).with_style(style().flex_grow()),
    ];
    if shortcut != "" {
        children.push(kbd::Kbd(#{ text: shortcut, appearance: "inline" })
            .with_style(ctx.component_style("shortcut", style())));
    }
    if active && !disabled { children.push(menu_indicator(ctx)); }
    let row_node = row(children)
        .with_style(ctx.component_style("item", item_style))
        .tab_stop(false)
        .disabled(disabled)
        .accessibility_role("menuitem")
        .accessibility_label(label)
        .accessibility_checked(checked);
    if "value" in item && item.value != () { row_node = row_node.with_key(item.value); }
    if shortcut != "" { row_node = row_node.accessibility_key_shortcuts(shortcut); }
    if disabled { row_node }
    else if action != () {
        row_node.on_click(Fn("dispatch_item").curry(#{ action: action,
            value: if "value" in item { item.value } else { () } }))
    } else if "on_action" in props && props.on_action != () && "value" in item && item.value != () {
        row_node.on_click_value(props.on_action, item.value)
    } else { row_node }
}

fn menu_activation(props, value) {
    for item in props.items {
        if "value" in item && item.value == value && !item_disabled(item) {
            let action = item_action(item);
            if action != () {
                return #{ handler: Fn("dispatch_item").curry(#{ action: action, value: value }), payload: () };
            }
        }
    }
    if "on_action" in props && props.on_action != () { #{ handler: props.on_action, payload: value } }
    else { () }
}

fn render_Menu(ctx, props) {
    let active_change = if "on_active_change" in props { props.on_active_change } else { () };
    let children = [];
    for item in props.items {
        if item.kind == "separator" {
            children.push(divider::Divider(#{}).with_style(
                ctx.component_style("separator", style().margin_y(theme_spacing("xs")))
            ));
        } else if item.kind == "label" {
            let label = if "label" in item && item.label != () { item.label } else { "" };
            let column_spacer = row([]).with_style(style().width(theme_length("metrics.icon")));
            children.push(row([column_spacer, text(label.to_upper())]).with_style(
                ctx.component_style("label", style().height(theme_length("metrics.row"))
                    .padding_x(theme_length("metrics.inset")).gap(theme_spacing("sm")).items_center()
                    .typography("label").text_color(theme_color("text_muted"))))
                .accessibility_role("presentation").accessibility_label(label));
        } else if item.kind == "submenu" && "submenu" in item && item.submenu != () {
            children.push(item.submenu.with_style(ctx.component_style("submenu", style())));
        } else {
            children.push(menu_item(ctx, props, item));
        }
    }
    let activation = if props.active_value != "" { menu_activation(props, props.active_value) } else { () };
    let content = column(children)
        .with_key(`${props.key}-content`)
        .with_style(ctx.component_style("content",
        // Overlays are raised blocks with a hairline; rows carry their own inset.
        style().min_width(px(240)).padding_y(theme_spacing("xs")).radius(theme_radius("lg"))
                .border(px(1)).border_color(theme_color("border"))
                .background(theme_color("surface_raised"))
                .focus(style().border_color(theme_color("focus_ring")))
        ))
        .tab_stop(true)
        .accessibility_role("menu")
        .accessibility_label(props.label);
    if active_change != () {
        content = content
            .on_key_value("up", active_change, next_item(props.items, props.active_value, -1))
            .on_key_value("down", active_change, next_item(props.items, props.active_value, 1));
    }
    if activation != () {
        content = content.on_key_value("enter", activation.handler, activation.payload);
    }
    if active_change != () {
        for key in [
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m",
            "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z"
        ] {
            content = content.on_key_value(
                key, active_change, next_typeahead(props.items, props.active_value, key)
            );
        }
    }
    let config = #{
        id: props.key, kind: "menu", open: props.open,
        placement: if "placement" in props { props.placement } else { "bottom" },
        // initial_focus "first" lands focus on the first focusable content
        // element on every open (a filter input, say); "panel" keeps the
        // historical panel-focus behavior for arrow-key menus.
        initial_focus: if "initial_focus" in props { props.initial_focus } else { "panel" },
        align: "start", gap: 4, modal: false, dismiss_on_escape: true, dismiss_on_outside: true,
        activate_on_trigger: if "activate_on_trigger" in props { props.activate_on_trigger } else { true },
    };
    if "parent_overlay" in props && props.parent_overlay != () { config.parent = props.parent_overlay; }
    if "anchor" in props && props.anchor != () { config.anchor = props.anchor; }
    let trigger = props.trigger.with_style(ctx.component_style("trigger", style()));
    if "on_context_request" in props && props.on_context_request != () {
        trigger = trigger.on("pointer_down", Fn("context_request"));
    }
    let node = overlay(
        trigger,
        content, config
    ).with_key(props.key)
        .with_style(ctx.component_style("root", style()));
    if "on_open_change" in props && props.on_open_change != () {
        node = node.on_open_change(props.on_open_change);
    }
    if props.open && active_change != () {
        node = node
            .on_key_value("up", active_change, next_item(props.items, props.active_value, -1))
            .on_key_value("down", active_change, next_item(props.items, props.active_value, 1));
    }
    if props.open && activation != () {
        node = node.on_key_value("enter", activation.handler, activation.payload);
    }
    if props.open && active_change != () {
        for key in [
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m",
            "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z"
        ] {
            node = node.on_key_value(
                key, active_change, next_typeahead(props.items, props.active_value, key)
            );
        }
    }
    node
}
