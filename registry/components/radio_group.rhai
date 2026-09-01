/* gpui-rhai
{
  "id": "components/radio_group",
  "export": "RadioGroup",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/radio"],
  "capabilities": {}
}
*/
// Controlled radio group with one tab stop and roving arrow-key selection.
// Props: value, options, orientation, disabled, label, on_change.
// Events: change(string). Parts: root, option.
// State: controlled by the value prop.
// Example: RadioGroup(#{ value: "one", label: "Choice", options: options, on_change: Fn("changed") })

import "components/radio" as radio;

define_component(#{
    metadata: #{
        id: "components/radio_group", "export": "RadioGroup", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/radio"], capabilities: #{},
    },
    schema: #{
        props: #{
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            options: #{
                schema: #{
                    type: "array", max_items: 256,
                    items: #{
                        type: "object", allow_unknown: false,
                        fields: #{
                            value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                            disabled: #{
                                schema: #{ type: "bool" }, required: false, sensitive: false,
                                "default": #{ type: "bool", value: false },
                            },
                        },
                    },
                },
                required: true, sensitive: false,
            },
            orientation: #{
                schema: #{ type: "string", allowed: ["horizontal", "vertical"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "vertical" },
            },
            disabled: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            on_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
            },
        },
        state: #{ fields: #{} },
        events: #{ change: #{ payload: #{ type: "string" } } },
        slots: #{}, parts: ["root", "option"],
    },
    render: Fn("render_RadioGroup")
});

fn option_disabled(option) {
    if "disabled" in option { option.disabled } else { false }
}

fn selected_index(options, value) {
    for index in 0..options.len {
        if options[index].value == value { return index; }
    }
    0
}

fn next_value(options, value, step) {
    if options.len == 0 { return value; }
    let current = selected_index(options, value);
    for offset in 1..(options.len + 1) {
        let index = if step > 0 {
            (current + offset) % options.len
        } else {
            (current + options.len - (offset % options.len)) % options.len
        };
        if !option_disabled(options[index]) { return options[index].value; }
    }
    value
}

fn RadioGroup(props) { render_component("components/radio_group", props) }
fn render_RadioGroup(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    let orientation = if "orientation" in props { props.orientation } else { "vertical" };
    let callback = if "on_change" in props { props.on_change } else { () };
    let options = [];
    for option in props.options {
        options.push(
            radio::Radio(#{
                value: option.value,
                selected: option.value == props.value,
                label: option.label,
                disabled: disabled || option_disabled(option),
                on_select: callback,
            }).with_style(component_style(props, "option", style())).tab_stop(false)
        );
    }
    let group = if orientation == "horizontal" { row(options) } else { column(options) };
    group = group
        .with_style(component_style(props, "root", style().gap(px(4))))
        .accessibility_role("radiogroup")
        .accessibility_label(props.label)
        .accessibility_value(props.value);
    if !disabled && callback != () {
        let previous = next_value(props.options, props.value, -1);
        let next = next_value(props.options, props.value, 1);
        group
            .tab_stop(true)
            .on_key_value("left", callback, previous)
            .on_key_value("up", callback, previous)
            .on_key_value("right", callback, next)
            .on_key_value("down", callback, next)
    } else { group.disabled(true) }
}
