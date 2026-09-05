/* gpui-rhai
{
  "id": "components/toggle_group",
  "export": "ToggleGroup",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/button_group", "components/toggle"],
  "capabilities": {}
}
*/
// Controlled single/multiple pressed-button group with roving arrow navigation.
// Props: key, label, items, values, mode, orientation, allow_empty, disabled, and on_change.
// Events: change(array<string>).
// State: caller owns values; component owns only the roving active item.
// Parts: root, item.
// Example: ToggleGroup(#{ key: "align", label: "Alignment", values: ["left"], items: items, on_change: Fn("changed") })

import "components/button_group" as button_group;
import "components/toggle" as toggle;

define_component(#{
    metadata: #{ id: "components/toggle_group", "export": "ToggleGroup", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/button_group", "components/toggle"], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            items: #{ schema: #{ type: "array", max_items: 64, items: #{
                type: "object", allow_unknown: false, fields: #{
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                }
            } }, required: true, sensitive: false },
            values: #{ schema: #{ type: "array", max_items: 64, items: #{ type: "string" } },
                required: true, sensitive: false },
            mode: #{ schema: #{ type: "string", allowed: ["single", "multiple"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "single" } },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "horizontal" } },
            allow_empty: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{ active: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "" } } } },
        events: #{ change: #{ payload: #{ type: "array", max_items: 64, items: #{ type: "string" } } } },
        slots: #{}, parts: ["root", "item"],
    },
    render: Fn("render_ToggleGroup")
});

fn ToggleGroup(props) { render_component("components/toggle_group", props) }
fn item_disabled(item) { "disabled" in item && item.disabled }
fn includes(values, value) {
    let found = false;
    for current in values { if current == value { found = true; } }
    found
}
fn enabled_values(items) {
    let values = [];
    for item in items { if !item_disabled(item) { values.push(item.value); } }
    values
}
fn selected_next(values, value, mode, allow_empty) {
    if mode == "single" {
        if includes(values, value) && allow_empty { [] } else { [value] }
    } else {
        let next = [];
        for current in values { if current != value { next.push(current); } }
        if next.len == values.len { next.push(value); }
        next
    }
}
fn adjacent(values, active, step) {
    if values.len == 0 { return ""; }
    let index = 0;
    for i in 0..values.len { if values[i] == active { index = i; } }
    if step > 0 { values[(index + 1) % values.len] }
    else { values[(index + values.len - 1) % values.len] }
}
fn active_action(ctx, value) { ctx.set_state("active", value); }
fn change_action(values, ctx, payload) { ctx.emit("change", values); }

fn render_ToggleGroup(ctx, props) {
    let mode = if "mode" in props { props.mode } else { "single" };
    if mode == "single" && props.values.len > 1 { throw "single toggle group accepts at most one value"; }
    let enabled = enabled_values(props.items);
    let stored_active = ctx.get_state("active");
    let active = if includes(enabled, stored_active) { stored_active }
        else if props.values.len > 0 && includes(enabled, props.values[0]) { props.values[0] }
        else if enabled.len > 0 { enabled[0] } else { "" };
    let disabled = "disabled" in props && props.disabled;
    let controls = [];
    for item in props.items {
        let pressed = includes(props.values, item.value);
        let next = selected_next(props.values, item.value, mode,
            if "allow_empty" in props { props.allow_empty } else { true });
        controls.push(toggle::Toggle(#{ text: item.label, pressed: pressed,
            disabled: disabled || item_disabled(item),
            on_pressed_change: if "on_change" in props && props.on_change != () {
                Fn("change_action").curry(next)
            } else { () }
        }).with_style(component_style(props, "item", style())).tab_stop(false));
    }
    let orientation = if "orientation" in props { props.orientation } else { "horizontal" };
    let active_next = if active == "" { props.values }
        else { selected_next(props.values, active, mode,
            if "allow_empty" in props { props.allow_empty } else { true }) };
    let root = button_group::ButtonGroup(#{ label: props.label, buttons: controls, orientation: orientation })
        .with_key(props.key).with_style(component_style(props, "root", style()))
        .tab_stop(!disabled && enabled.len > 0)
        .accessibility_role("group").accessibility_label(props.label)
        .accessibility_orientation(orientation);
    if !disabled && enabled.len > 0 {
        let previous = adjacent(enabled, active, -1);
        let next = adjacent(enabled, active, 1);
        root = root.on_key_value("enter", Fn("change_action").curry(active_next), ())
            .on_key_value("space", Fn("change_action").curry(active_next), ());
        if orientation == "horizontal" {
            root.on_key_value("left", Fn("active_action"), previous)
                .on_key_value("right", Fn("active_action"), next)
        } else {
            root.on_key_value("up", Fn("active_action"), previous)
                .on_key_value("down", Fn("active_action"), next)
        }
    } else { root }
}
