/* gpui-rhai
{
  "id": "components/radio_group",
  "export": "RadioGroup",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/radio"],
  "capabilities": {},
  "tokens": ["space.group"],
  "environment": ["size", "density"]
}
*/
// Controlled radio group with one tab stop and roving arrow-key selection.
// State: controlled by the value prop.
// Example: RadioGroup(#{ value: "one", label: "Choice", options: options, on_change: Fn("changed") })

import "components/radio" as radio;

define_component(#{
    metadata: #{
        id: "components/radio_group", "export": "RadioGroup", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/radio"], capabilities: #{},
        tokens: ["space.group"],
        environment: ["size", "density"],
    },
    schema: #{
        props: #{
            value: #{
                schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Value of the selected option; the caller stores the `change` payload and passes it back.",
            },
            label: #{
                schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the group.",
            },
            options: #{
                schema: #{
                    type: "array", max_items: 256,
                    items: #{
                        type: "object", allow_unknown: false,
                        fields: #{
                            value: #{
                                schema: #{ type: "string" }, required: true, sensitive: false,
                                doc: "Value that `change` reports when this option is chosen.",
                            },
                            label: #{
                                schema: #{ type: "string" }, required: true, sensitive: false,
                                doc: "Text beside the option's circle; also its accessible name.",
                            },
                            disabled: #{
                                schema: #{ type: "bool" }, required: false, sensitive: false,
                                "default": #{ type: "bool", value: false },
                                doc: "Blocks clicks on this option; arrow keys skip it.",
                            },
                        },
                    },
                },
                required: true, sensitive: false,
                doc: "Options in display order.",
            },
            orientation: #{
                schema: #{ type: "string", allowed: ["horizontal", "vertical"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "vertical" },
                doc: "Stacks options in a column, or lays them in a row spaced by `space.group`.",
            },
            disabled: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Disables every option and takes the group out of the tab order.",
            },
            on_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
                doc: "Called with the chosen option's value on click or arrow key; without it the group renders disabled.",
            },
        },
        state: #{ fields: #{} },
        events: #{ change: #{ payload: #{ type: "string" },
            doc: "Emitted when an option is clicked or an arrow key selects the previous or next enabled one; the payload is the new value." } },
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

fn change_action(ctx, value) {
    ctx.emit("change", value);
}

fn render_RadioGroup(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    let orientation = if "orientation" in props { props.orientation } else { "vertical" };
    let callback = if "on_change" in props { props.on_change } else { () };
    // The keyboard cursor is the selected option, or the first enabled one; only
    // its ring shows the group's focus.
    let cursor = ();
    for option in props.options {
        if cursor == () && !option_disabled(option) { cursor = option.value; }
    }
    for option in props.options {
        if option.value == props.value && !option_disabled(option) { cursor = option.value; }
    }
    let options = [];
    for option in props.options {
        options.push(
            radio::Radio(#{
                value: option.value,
                selected: option.value == props.value,
                label: option.label,
                disabled: disabled || option_disabled(option),
                focus_mark: option.value == cursor,
                on_select: if callback == () { () } else { Fn("change_action") },
            }).with_style(ctx.component_style("option", style())).tab_stop(false)
        );
    }
    let horizontal = orientation == "horizontal";
    let group = if horizontal { row(options) } else { column(options) };
    group = group
        .with_style(ctx.component_style("root", style()
            .gap(if horizontal { theme_length("space.group") } else { px(0) })
            .focus(style())))
        .accessibility_role("radiogroup")
        .accessibility_label(props.label)
        .accessibility_value(props.value);
    if !disabled && callback != () {
        let previous = next_value(props.options, props.value, -1);
        let next = next_value(props.options, props.value, 1);
        group
            .tab_stop(true)
            .on_key_value("left", Fn("change_action"), previous)
            .on_key_value("up", Fn("change_action"), previous)
            .on_key_value("right", Fn("change_action"), next)
            .on_key_value("down", Fn("change_action"), next)
    } else { group.disabled(true) }
}
