/* gpui-rhai
{
  "id": "components/input",
  "export": "Input",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Input is a controlled single-line native text input.
//
// Props: key, value, placeholder, disabled, read_only, error, callbacks.
// Events: change(string), submit(string), focus(null), blur(null).
// State: native keyed TextInput Entity. Parts: root, input.
// Example: Input(#{ key: "name", value: name, on_change: Fn("name_changed") })

export_component(#{
    metadata: #{
        id: "components/input",
        "export": "Input",
        version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [],
        capabilities: #{},
    },
    schema: #{
        props: #{
            key: #{
                schema: #{ type: "string" },
                required: true,
                sensitive: false,
            },
            value: #{
                schema: #{ type: "string" },
                required: true,
                sensitive: false,
            },
            placeholder: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false,
                sensitive: false,
            },
            disabled: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            read_only: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            error: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            on_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
            },
            on_submit: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
            },
            on_focus: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
            },
            on_blur: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
            },
        },
        state: #{ fields: #{} },
        events: #{
            change: #{ payload: #{ type: "string" } },
            submit: #{ payload: #{ type: "string" } },
            focus: #{ payload: #{ type: "null" } },
            blur: #{ payload: #{ type: "null" } },
        },
        slots: #{},
        parts: ["root", "input"],
    },
});

fn optional_prop(props, name) {
    if name in props { props[name] } else { () }
}

fn Input(props) {
    component_render("components/input", props, Fn("render_Input"))
}
fn render_Input(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    let read_only = if "read_only" in props { props.read_only } else { false };
    let error = if "error" in props { props.error } else { false };
    let border = if error { theme_color("danger") } else { theme_color("border") };
    gpui_rhai::TextInputPrimitive(#{
        key: props.key,
        value: props.value,
        placeholder: optional_prop(props, "placeholder"),
        disabled: disabled,
        read_only: read_only,
        on_change: optional_prop(props, "on_change"),
        on_submit: optional_prop(props, "on_submit"),
        on_focus: optional_prop(props, "on_focus"),
        on_blur: optional_prop(props, "on_blur"),
    })
        .with_style(component_style(props, "root", component_style(props, "input",
            style()
                .height(px(32))
                .padding_x(px(8))
                .radius(px(6))
                .border(px(1))
                .border_color(border)
                .background(theme_color("surface_raised"))
        )))
        .accessibility_role("text_field")
        .accessibility_label(
            if "placeholder" in props { props.placeholder } else { "" }
        )
}
