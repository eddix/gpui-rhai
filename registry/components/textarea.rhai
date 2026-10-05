/* gpui-rhai
{
  "id": "components/textarea",
  "export": "Textarea",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "assets": [],
  "tokens": [
    "surface_raised", "border", "danger", "focus_ring", "text_primary", "text_muted",
    "text.danger", "disabled", "selection", "accent", "metrics.control_pad", "spacing.xxs", "spacing.xs",
    "radius.md", "typography.body", "typography.body_small", "typography.caption"
  ],
  "environment": ["size", "density"]
}
*/
// Textarea is a controlled multiline field with native IME, selection, wrapping, and auto-grow.
//
// Props: key, label, value, placeholder, disabled/read_only/error, size, rows or min/max rows,
//        max_length, show_count, autofocus, and change/focus/blur callbacks.
// Events: change(string), focus(null), blur(null). State: keyed native transient editor state.
// Example: Textarea(#{ key: "notes", label: "Notes", value: notes, max_length: 500, show_count: true, on_change: Fn("notes_changed") })

define_component(#{
    metadata: #{
        id: "components/textarea", "export": "Textarea", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "surface_raised", "border", "danger", "focus_ring", "text_primary", "text_muted",
            "text.danger", "disabled", "selection", "accent", "metrics.control_pad",
            "spacing.xxs", "spacing.xs", "radius.md", "typography.body", "typography.body_small",
            "typography.caption"
        ],
        environment: ["size", "density"], assets: [],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            placeholder: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            read_only: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            error: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false },
            min_rows: #{ schema: #{ type: "integer", min: 1, max: 1000 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 3 } },
            max_rows: #{ schema: #{ type: "integer", min: 1, max: 1000 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 8 } },
            rows: #{ schema: #{ type: "optional", value: #{ type: "integer", min: 1, max: 1000 } }, required: false, sensitive: false },
            max_length: #{ schema: #{ type: "optional", value: #{ type: "integer", min: 0, max: 1000000 } }, required: false, sensitive: false },
            show_count: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            autofocus: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_focus: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_blur: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{
            change: #{ payload: #{ type: "string" } },
            focus: #{ payload: #{ type: "null" } },
            blur: #{ payload: #{ type: "null" } },
        },
        slots: #{},
        parts: ["root", "editor", "placeholder", "selection", "caret", "scroll", "counter", "limit"],
    },
    render: Fn("render_Textarea")
});

fn optional_prop(props, name) {
    if name in props { props[name] } else { () }
}

// Multi-line text keeps the reading line box of body text.
fn textarea_typography(size) {
    if size == "xs" { "body_small" } else { "body" }
}

fn Textarea(props) { render_component("components/textarea", props) }
fn render_Textarea(ctx, props) {
    let size = if "size" in props && props.size != () { props.size } else { "md" };
    let error = if "error" in props { props.error } else { false };
    let disabled = if "disabled" in props { props.disabled } else { false };
    let read_only = if "read_only" in props { props.read_only } else { false };
    let config = #{
        key: props.key,
        value: props.value,
        placeholder: optional_prop(props, "placeholder"),
        disabled: disabled,
        read_only: read_only,
        min_rows: if "min_rows" in props { props.min_rows } else { 3 },
        max_rows: if "max_rows" in props { props.max_rows } else { 8 },
        typography: textarea_typography(size),
        autofocus: if "autofocus" in props { props.autofocus } else { false },
        on_change: optional_prop(props, "on_change"),
        on_focus: optional_prop(props, "on_focus"),
        on_blur: optional_prop(props, "on_blur"),
        placeholder_style: ctx.component_style("placeholder",
            style().text_color(theme_color("text_muted"))),
        selection_style: ctx.component_style("selection",
            style().background(theme_color("selection"))),
        caret_style: ctx.component_style("caret",
            style().background(theme_color("accent"))),
        scroll_style: ctx.component_style("scroll", style()),
    };
    if "rows" in props && props.rows != () { config.rows = props.rows; }
    if "max_length" in props && props.max_length != () { config.max_length = props.max_length; }

    let border = if error { theme_color("danger") } else { theme_color("border") };
    let editor = gpui_rhai::TextareaPrimitive(config)
        .with_style(ctx.component_style("editor",
            // The text start matches Input: 2px frame plus the control padding.
            style()
                .padding_x(theme_length("metrics.control_pad"))
                .padding_y(theme_spacing("xs"))
                .radius(theme_radius("md"))
                .border(px(2))
                .border_color(border)
                .background(theme_color("surface_raised"))
                .text_color(if read_only { theme_color("text_muted") } else { theme_color("text_primary") })
                .focus(style().border_color(theme_color("focus_ring")))
                .disabled(style().text_color(theme_color("disabled")).opacity(0.72))
        ))
        .disabled(disabled)
        .accessibility_role("text_field")
        .accessibility_label(props.label)
        .accessibility_value(props.value)
        .accessibility_invalid(error)
        .accessibility_read_only(read_only);
    let editor = if config.placeholder != () {
        editor.accessibility_placeholder(config.placeholder)
    } else { editor };

    let children = [editor];
    if "show_count" in props && props.show_count {
        let count = grapheme_count(props.value);
        let max = optional_prop(props, "max_length");
        let label = if max == () {
            ctx.format_number(count)
        } else {
            `${ctx.format_number(count)} / ${ctx.format_number(max)}`
        };
        let reached = max != () && count >= max;
        let counter_style = ctx.component_style("counter",
            style().typography("caption")
                .text_color(theme_color("text_muted")));
        let counter_style = if reached {
            ctx.component_style("limit", counter_style.text_color(theme_color("text.danger")))
        } else { counter_style };
        children.push(row([text(label)]).with_style(counter_style.justify_end()));
    }
    let root = column(children)
        .with_key(props.key)
        // The counter belongs to the field: it sits closer than any helper text
        // a form places under the field.
        .with_style(ctx.component_style("root", style().gap(theme_spacing("xxs"))));
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
