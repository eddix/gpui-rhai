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
    "disabled", "metrics.control", "metrics.field_pad", "metrics.inset", "radius.md",
    "typography.control"
  ],
  "environment": ["size", "density", "corners"]
}
*/
// Input is a controlled single-line native text input.
//
// State: native keyed TextInput Entity.
//
// `appearance: "embedded"` is the search or filter line that heads a panel or a list and
// keeps focus while it is open: no frame or well of its own, the container's edges and a
// hairline under it frame it, and its text starts on metrics.inset like the rows below.
// Its caret shows focus; the hairline stays `border` (`danger` while invalid).
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
            "disabled", "metrics.control", "metrics.field_pad", "metrics.inset", "radius.md",
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
                doc: "Names the native editor, which keeps caret, selection and undo history across renders; a new key starts a fresh editor.",
            },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the field; it is not drawn, so pair the input with a `Label` or a `FormField`." },
            value: #{
                schema: #{ type: "string" },
                required: true,
                sensitive: false,
                doc: "Current text; the caller stores the `change` payload and passes it back.",
            },
            placeholder: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false,
                sensitive: false,
                doc: "Muted hint shown while `value` is empty.",
            },
            disabled: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Takes the field out of the tab order, blocks editing and submit, and dims it.",
            },
            read_only: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Keeps the text focusable, selectable and copyable but blocks edits; the text turns `text_muted`.",
            },
            autofocus: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Focuses the field once, when its editor first mounts; ignored while `disabled`.",
            },
            error: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Marks the value invalid: a `danger` frame (or hairline when embedded) and the invalid state for assistive tech.",
            },
            on_change: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
                doc: "Called with the full new text after each edit.",
            },
            on_submit: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
                doc: "Called with the current text when Enter is pressed in a field that is not `disabled`.",
            },
            on_focus: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
                doc: "Called when the field gains keyboard focus.",
            },
            on_blur: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
                doc: "Called when the field loses keyboard focus.",
            },
            size: #{
                schema: #{
                    type: "optional",
                    value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] }
                },
                required: false,
                sensitive: false,
                doc: "Control size of the field; unset, it inherits the `size` environment.",
            },
            appearance: #{
                schema: #{ type: "string", allowed: ["field", "embedded"] },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "field" },
                doc: "`field` is a framed well; `embedded` is the frameless search or filter line that heads a panel or list.",
            },
        },
        state: #{ fields: #{} },
        events: #{
            change: #{ payload: #{ type: "string" },
                doc: "Emitted after each edit; the payload is the full new text." },
            submit: #{ payload: #{ type: "string" },
                doc: "Emitted when Enter is pressed in a field that is not `disabled`; the payload is the current text." },
            focus: #{ payload: #{ type: "null" }, doc: "Emitted when the field gains keyboard focus." },
            blur: #{ payload: #{ type: "null" }, doc: "Emitted when the field loses keyboard focus." },
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
    let embedded = optional_prop(props, "appearance") == "embedded";
    let field = style()
        .min_height(theme_length("metrics.control"))
        .items_center()
        .text_color(if read_only { theme_color("text_muted") } else { theme_color("text_primary") })
        .typography("control");
    if embedded {
        // The container frames it and the hairline separates it from the rows. It holds
        // focus for as long as its panel is open, so a focus color here would never turn
        // off and would say nothing; the caret shows focus.
        field = field
            .padding_x(theme_length("metrics.inset"))
            .border_bottom(px(1))
            .border_color(if error { theme_color("danger") } else { theme_color("border") });
    } else {
        // Fields are framed wells: things you type into have a frame. The focus color
        // owns the frame while focused; an error frame returns on blur.
        field = field
            .padding_x(theme_length("metrics.field_pad"))
            .radius(theme_radius("md"))
            .border(px(2))
            .border_color(if error { theme_color("danger") } else { theme_color("border") })
            .background(theme_color("surface_raised"))
            .focus(style().border_color(theme_color("focus_ring")));
    }
    field = field.disabled(style().text_color(theme_color("disabled")).opacity(0.72));
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
