/* gpui-rhai
{
  "id": "components/radio",
  "export": "Radio",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled radio option. RadioGroup owns roving keyboard behavior.
// Props: value, selected, label, disabled, on_select.
// Events: select(string). Parts: root, indicator, label.
// State: controlled by the selected prop.
// Example: Radio(#{ value: "team", selected: true, label: "Team", on_select: Fn("selected") })

define_component(#{
    metadata: #{
        id: "components/radio", "export": "Radio", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{},
    },
    schema: #{
        props: #{
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            selected: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            disabled: #{
                schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            on_select: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
            },
        },
        state: #{ fields: #{} },
        events: #{ select: #{ payload: #{ type: "string" } } },
        slots: #{}, parts: ["root", "indicator", "label"],
    },
    render: Fn("render_Radio")
});

fn Radio(props) { render_component("components/radio", props) }
fn render_Radio(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    let indicator = text(if props.selected { "●" } else { "○" })
        .with_style(component_style(props, "indicator",
            style().text_color(if props.selected { theme_color("accent") } else { theme_color("text_muted") })));
    let node = row([indicator, text(props.label).with_style(component_style(props, "label", style()))])
        .with_key(props.value)
        .with_style(component_style(props, "root",
            style().gap(px(8)).items_center()
                .text_color(if disabled { theme_color("disabled") } else { theme_color("text_primary") })
        ))
        .disabled(disabled)
        .accessibility_role("radio")
        .accessibility_label(props.label)
        .accessibility_checked(props.selected)
        .accessibility_value(props.value);
    if !disabled && "on_select" in props && props.on_select != () {
        node.on_click_value(props.on_select, props.value)
    } else { node }
}
