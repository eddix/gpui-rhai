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
//
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
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable identity of this instance; also the overlay id a submenu names in `parent_overlay`." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the menu panel." },
            parent_overlay: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "`key` of the open menu this one is a submenu of; closing that menu closes this one too." },
            trigger: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "Node that anchors the menu; clicking it, or Enter and Space on it, toggles the menu." },
            open: #{ schema: #{ type: "bool" }, required: true, sensitive: false,
                doc: "Whether the menu is shown; the caller stores the `open_change` payload and passes it back." },
            active_value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "`value` of the highlighted item, which Enter activates; the caller stores the `active_change` payload." },
            placement: #{ schema: #{ type: "string", allowed: ["top", "bottom", "left", "right"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "bottom" },
                doc: "Preferred side of the trigger, or of `anchor`, that the menu opens on." },
            initial_focus: #{ schema: #{ type: "string", allowed: ["panel", "first"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "panel" },
                doc: "Where focus lands on each open: `panel` for arrow-key menus, `first` for the first focusable element inside." },
            anchor: #{ schema: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                x: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Left edge of the anchor in window coordinates, in logical pixels." },
                y: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Top edge of the anchor in window coordinates, in logical pixels." },
                width: #{ schema: #{ type: "number", min: 0.0 }, required: false, sensitive: false,
                    "default": #{ type: "float", value: 0.0 },
                    doc: "Anchor width in logical pixels; 0 anchors the menu to a point." },
                height: #{ schema: #{ type: "number", min: 0.0 }, required: false, sensitive: false,
                    "default": #{ type: "float", value: 0.0 },
                    doc: "Anchor height in logical pixels; 0 anchors the menu to a point." },
            } } }, required: false, sensitive: false,
                doc: "Window rectangle the menu is placed against instead of the trigger, such as the pointer position." },
            activate_on_trigger: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Lets a click, Enter or Space on the trigger toggle the menu; when off the menu opens only through `open`." },
            items: #{ schema: #{ type: "array", max_items: 512, items: #{
                type: "object", allow_unknown: false, fields: #{
                    kind: #{ schema: #{ type: "string", allowed: ["item", "separator", "submenu", "label"] }, required: true, sensitive: false,
                        doc: "`item` is an activatable row, `separator` a divider, `label` a heading, `submenu` a nested menu row." },
                    value: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Identifier passed to `on_action` and used as `active_value`; rows without one are skipped by the keys." },
                    label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Row text, also matched by letter-key type-ahead; on a `label` row, the heading text." },
                    shortcut: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Key legend shown at the row end, written like `cmd-o`; overrides the binding of `action`." },
                    action: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Registered action the row dispatches before closing the menu; it also supplies the legend and enabled state." },
                    checked: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Shows the check mark at the row start and reports the row as checked." },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Dims the row; it cannot be clicked and keyboard navigation skips it." },
                    submenu: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Nested Menu node drawn for a `submenu` row; set its `parent_overlay` to this menu's `key`." },
                },
            } }, required: true, sensitive: false,
                doc: "Rows of the menu, in order." },
            on_action: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the item `value` when a row is clicked or activated with Enter; rows without an `action` leave the menu open." },
            on_active_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the item `value` to highlight on Up, Down or a letter key; store it as `active_value`." },
            on_open_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `true` when the trigger opens the menu and `false` on dismissal or after an `action` row; store it as `open`." },
            on_context_request: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the pointer event when the trigger is pressed with the right button." },
        },
        state: #{ fields: #{} },
        events: #{
            action: #{ payload: #{ type: "string" },
                doc: "Emitted when a row is clicked or activated with Enter; the payload is the item `value`." },
            active_change: #{ payload: #{ type: "string" },
                doc: "Emitted when Up, Down or a letter key moves the highlight; the payload is the new item `value`." },
            open_change: #{ payload: #{ type: "bool" },
                doc: "Emitted when the menu asks to open or close; the payload is the requested `open`." },
            context_request: #{ payload: #{ type: "ui_value" },
                doc: "Emitted on a right-button press on the trigger; the payload is the pointer event, with its `window` position." },
        },
        slots: #{ trigger: #{ required: true, multiple: false, doc: "Node that anchors and toggles the menu." } },
        parts: ["root", "trigger", "content", "item", "separator", "shortcut", "submenu",
            "label", "check", "indicator_bar", "focus_frame"],
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

// The 2px focus frame, drawn over the panel's edge while the panel itself holds focus: the
// panel's hairline (painted over its children) turns the focus color and this frame adds
// the inner pixel. A control inside shows its own frame instead.
fn panel_focus_frame(ctx) {
    row([]).with_style(ctx.component_style("focus_frame", style().absolute()
        .left(offset_px(-1)).right(offset_px(-1)).top(offset_px(-1)).bottom(offset_px(-1))
        .radius(theme_radius("lg")).border(px(2)).border_color(rgba(0x00000000))
        .group_focus(style().border_color(theme_color("focus_ring")))))
}
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
    let shortcut = if "shortcut" in item && item.shortcut != () { key_shortcut(item.shortcut).label }
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

// The keys treat a row whose action is disabled as disabled, as the row itself draws it.
fn key_items(ctx, items) {
    let result = [];
    for item in items {
        let action = item_action(item);
        if action != () && !ctx.action_enabled(action) { item.disabled = true; }
        result.push(item);
    }
    result
}

fn menu_activation(props, items, value) {
    for item in items {
        if "value" in item && item.value == value {
            if item_disabled(item) { return (); }
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
    let items = key_items(ctx, props.items);
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
    let activation = if props.active_value != "" { menu_activation(props, items, props.active_value) } else { () };
    children.push(panel_focus_frame(ctx));
    let content = column(children)
        .with_key(`${props.key}-content`)
        .with_style(ctx.component_style("content",
        // Overlays are raised blocks with a hairline; rows carry their own inset.
        style().relative().min_width(px(240)).padding_y(theme_spacing("xs")).radius(theme_radius("lg"))
                .border(px(1)).border_color(theme_color("border"))
                .background(theme_color("surface_raised"))
                .focus(style().border_color(theme_color("focus_ring")))
        ))
        .tab_stop(true)
        .accessibility_role("menu")
        .accessibility_label(props.label);
    if active_change != () {
        content = content
            .on_key_value("up", active_change, next_item(items, props.active_value, -1))
            .on_key_value("down", active_change, next_item(items, props.active_value, 1));
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
                key, active_change, next_typeahead(items, props.active_value, key)
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
            .on_key_value("up", active_change, next_item(items, props.active_value, -1))
            .on_key_value("down", active_change, next_item(items, props.active_value, 1));
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
                key, active_change, next_typeahead(items, props.active_value, key)
            );
        }
    }
    node
}
