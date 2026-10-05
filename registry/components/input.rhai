/* gpui-rhai
{
  "id": "components/input",
  "export": "Input",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_raised", "border", "danger", "focus_ring", "text_primary", "text_muted",
    "disabled", "metrics.control", "metrics.control_pad", "radius.md", "typography.control"
  ],
  "environment": ["size", "density", "corners"]
}
*/
// Input is a controlled single-line native text input.
//
// Props: key, label, value, placeholder, size, disabled, read_only, error, autofocus, callbacks.
// Events: change(string), submit(string), focus(null), blur(null).
// State: native keyed TextInput Entity. Parts: root, input.
// Example: Input(#{ key: "name", label: "Name", value: name, on_change: Fn("name_changed") })

define_component(#{
    metadata: #{
        id: "components/input",
        "export": "Input",
        version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [],
        capabilities: #{},
        tokens: [
            "surface_raised", "border", "danger", "focus_ring", "text_primary", "text_muted",
            "disabled", "metrics.control", "metrics.control_pad", "radius.md",
            "typography.control"
        ],
        environment: ["size", "density", "corners"],
    },
    schema: #{
        props: #{
            key: #{
                schema: #{ type: "string" },
                required: true,
                sensitive: false,
            },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
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
            autofocus: #{
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
            size: #{
                schema: #{
                    type: "optional",
                    value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] }
                },
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
    render: Fn("render_Input")
});

fn optional_prop(props, name) {
    if name in props { props[name] } else { () }
}

fn Input(props) { render_component("components/input", props) }
fn render_Input(ctx, props) {
    let disabled = if "disabled" in props { props.disabled } else { false };
    let read_only = if "read_only" in props { props.read_only } else { false };
    let error = if "error" in props { props.error } else { false };
    let input = gpui_rhai::TextInputPrimitive(#{
        key: props.key,
        value: props.value,
        placeholder: optional_prop(props, "placeholder"),
        disabled: disabled,
        read_only: read_only,
        typography: "control",
        autofocus: if "autofocus" in props { props.autofocus } else { false },
        on_change: optional_prop(props, "on_change"),
        on_submit: optional_prop(props, "on_submit"),
        on_focus: optional_prop(props, "on_focus"),
        on_blur: optional_prop(props, "on_blur"),
    });
    // Fields are framed wells: things you type into have a frame. The focus color
    // owns the frame while focused; an error frame returns on blur.
    let field = style()
        .min_height(theme_length("metrics.control"))
        .padding_x(theme_length("metrics.control_pad"))
        .items_center()
        .radius(theme_radius("md"));
    field = field
        .border(px(2))
        .border_color(if error { theme_color("danger") } else { theme_color("border") })
        .background(theme_color("surface_raised"))
        .text_color(if read_only { theme_color("text_muted") } else { theme_color("text_primary") })
        .typography("control")
        .focus(style().border_color(theme_color("focus_ring")))
        .disabled(style().text_color(theme_color("disabled")).opacity(0.72));
    input = input
        .with_style(ctx.component_style("root", ctx.component_style("input", field)))
        .disabled(disabled)
        .accessibility_role("text_field")
        .accessibility_label(props.label)
        .accessibility_value(props.value)
        .accessibility_invalid(error)
        .accessibility_read_only(read_only);
    if "size" in props && props.size != () { input = input.env(#{ size: props.size }); }
    let placeholder = optional_prop(props, "placeholder");
    if placeholder != () { input.accessibility_placeholder(placeholder) } else { input }
}
