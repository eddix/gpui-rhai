/* gpui-rhai
{
  "id": "components/menu",
  "export": "Menu",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/divider"],
  "capabilities": {}
}
*/
// Controlled in-window menu with roving selection and nested submenu nodes.
// Props: key, parent_overlay, trigger, open, active_value, items, placement,
// initial_focus (panel|first: where focus lands on each open), callbacks.
// Events: action(string), active_change(string), open_change(bool).
// State: controlled open/active values.
// Example: Menu(#{ key: "file", trigger: text("File"), open: true, active_value: "open", items: items })

import "components/divider" as divider;

define_component(#{
    metadata: #{
        id: "components/menu", "export": "Menu", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/divider"], capabilities: #{},
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            parent_overlay: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            trigger: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            active_value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "bottom" } },
            initial_focus: #{ schema: #{ type: "string", allowed: ["panel", "first"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "panel" } },
            items: #{ schema: #{ type: "array", max_items: 512, items: #{
                type: "object", allow_unknown: false, fields: #{
                    kind: #{ schema: #{ type: "string", allowed: ["item", "separator", "submenu"] }, required: true, sensitive: false },
                    value: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                    label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                    shortcut: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
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
        },
        state: #{ fields: #{} },
        events: #{
            action: #{ payload: #{ type: "string" } },
            active_change: #{ payload: #{ type: "string" } },
            open_change: #{ payload: #{ type: "bool" } },
        },
        slots: #{ trigger: #{ required: true, multiple: false } },
        parts: ["root", "trigger", "content", "item", "separator", "shortcut", "submenu"],
    },
    render: Fn("render_Menu")
});

fn item_disabled(item) { "disabled" in item && item.disabled }
fn selectable(item) { item.kind != "separator" && !item_disabled(item) && "value" in item && item.value != () }
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
fn render_Menu(ctx, props) {
    let action = if "on_action" in props { props.on_action } else { () };
    let active_change = if "on_active_change" in props { props.on_active_change } else { () };
    let children = [];
    for item in props.items {
        if item.kind == "separator" {
            children.push(divider::Divider(#{}).with_style(
                component_style(props, "separator", style())
            ));
        } else if item.kind == "submenu" && "submenu" in item && item.submenu != () {
            children.push(item.submenu.with_style(component_style(props, "submenu", style())));
        } else {
            let checked = "checked" in item && item.checked;
            let shortcut = if "shortcut" in item && item.shortcut != () { item.shortcut } else { "" };
            let label = if "label" in item && item.label != () { item.label } else { "" };
            let active = item.value == props.active_value;
            let item_background = if active {
                theme_color("selection")
            } else { rgba(0x00000000) };
            let item_text = if item_disabled(item) { theme_color("disabled") }
                else { theme_color("text_primary") };
            let item_border = if active { theme_color("accent") } else { rgba(0x00000000) };
            let hover_background = if active { theme_color("selection") }
                else { theme_color("surface_hover") };
            let item_style = style().height(px(28)).padding_x(px(8)).gap(px(8))
                .items_center().background(item_background).text_color(item_text)
                .font_size(px(12)).line_height(px(16))
                .border_start(px(2)).border_color(item_border);
            item_style = item_style.hover(style().background(hover_background));
            item_style = item_style.disabled(style().opacity(0.66));
            let row_node = row([
                text(if checked { "✓" } else { "" }).with_style(style()
                    .width(px(14)).text_color(theme_color("accent"))),
                text(label).with_style(style().flex_grow()),
                text(shortcut).with_style(component_style(props, "shortcut",
                    style().font_size(px(10)).text_color(theme_color("text_muted"))))
            ])
                .with_key(item.value)
                .with_style(component_style(props, "item", item_style))
                .tab_stop(false)
                .disabled(item_disabled(item))
                .accessibility_role("menuitem")
                .accessibility_label(label)
                .accessibility_checked(checked);
            if !item_disabled(item) && action != () {
                row_node = row_node.on_click_value(action, item.value);
            }
            children.push(row_node);
        }
    }
    let content = column(children)
        .with_key(`${props.key}-content`)
        .with_style(component_style(props, "content",
            style().width(px(240)).padding(px(4)).gap(px(2)).radius(theme_radius("md"))
                .border(px(1)).border_color(theme_color("border"))
                .background(theme_color("surface"))
                .focus(style().border_color(theme_color("focus_ring")))
        ))
        .tab_stop(true)
        .accessibility_role("menu");
    if active_change != () {
        content = content
            .on_key_value("up", active_change, next_item(props.items, props.active_value, -1))
            .on_key_value("down", active_change, next_item(props.items, props.active_value, 1));
    }
    if action != () && props.active_value != "" {
        content = content.on_key_value("enter", action, props.active_value);
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
        gap: 4, modal: false, dismiss_on_escape: true, dismiss_on_outside: true,
    };
    if "parent_overlay" in props && props.parent_overlay != () { config.parent = props.parent_overlay; }
    let node = overlay(
        props.trigger.with_style(component_style(props, "trigger", style())),
        content, config
    ).with_key(props.key)
        .with_style(component_style(props, "root", style()));
    if "on_open_change" in props && props.on_open_change != () {
        node = node.on_open_change(props.on_open_change);
    }
    if props.open && active_change != () {
        node = node
            .on_key_value("up", active_change, next_item(props.items, props.active_value, -1))
            .on_key_value("down", active_change, next_item(props.items, props.active_value, 1));
    }
    if props.open && action != () && props.active_value != "" {
        node = node.on_key_value("enter", action, props.active_value);
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
