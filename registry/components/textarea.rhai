/* gpui-rhai
{
  "id": "components/textarea",
  "export": "Textarea",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {},
  "assets": []
}
*/
// Textarea is a controlled multiline field with native IME, selection, wrapping, and auto-grow.
//
// Props: key, value, placeholder, disabled/read_only/error, size, rows or min/max rows,
//        max_length, show_count, autofocus, and change/focus/blur callbacks.
// Events: change(string), focus(null), blur(null). State: keyed native transient editor state.
// Example: Textarea(#{ key: "notes", value: notes, max_length: 500, show_count: true, on_change: Fn("notes_changed") })

define_component(#{
    metadata: #{
        id: "components/textarea", "export": "Textarea", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{}, assets: [],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            placeholder: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            read_only: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            error: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            size: #{ schema: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "md" } },
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

fn textarea_line_height(size) {
    if size == "xs" { 14 }
    else if size == "lg" { 18 }
    else { 16 }
}

fn textarea_font_size(size) {
    if size == "xs" { 11 }
    else if size == "lg" { 13 }
    else { 12 }
}

fn Textarea(props) { render_component("components/textarea", props) }
fn render_Textarea(ctx, props) {
    let size = if "size" in props { props.size } else { "md" };
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
        line_height: textarea_line_height(size) * 1.0,
        font_size: textarea_font_size(size) * 1.0,
        autofocus: if "autofocus" in props { props.autofocus } else { false },
        on_change: optional_prop(props, "on_change"),
        on_focus: optional_prop(props, "on_focus"),
        on_blur: optional_prop(props, "on_blur"),
        placeholder_style: component_style(props, "placeholder",
            style().text_color(theme_color("text_muted"))),
        selection_style: component_style(props, "selection",
            style().background(theme_color("selection"))),
        caret_style: component_style(props, "caret",
            style().background(theme_color("accent"))),
        scroll_style: component_style(props, "scroll", style()),
    };
    if "rows" in props && props.rows != () { config.rows = props.rows; }
    if "max_length" in props && props.max_length != () { config.max_length = props.max_length; }

    let border = if error { theme_color("danger") } else { theme_color("border") };
    let editor = gpui_rhai::TextareaPrimitive(config)
        .with_style(component_style(props, "editor",
            style()
                .padding(theme_spacing("sm"))
                .radius(theme_radius("md"))
                .border(px(1))
                .border_color(border)
                .background(theme_color("surface"))
                .text_color(theme_color("text_primary"))
                .focus(style().border_color(theme_color("focus_ring")))
                .disabled(style()
                    .background(theme_color("surface_raised"))
                    .text_color(theme_color("disabled"))
                    .opacity(0.72))
        ))
        .accessibility_role("text_field")
        .accessibility_label(if "placeholder" in props { props.placeholder } else { "" })
        .accessibility_value(props.value)
        .accessibility_invalid(error);

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
        let counter_style = component_style(props, "counter",
            style().font_size(px(11)).line_height(px(14))
                .text_color(theme_color("text_muted")));
        let counter_style = if reached {
            component_style(props, "limit", counter_style.text_color(theme_color("danger")))
        } else { counter_style };
        children.push(row([text(label)]).with_style(counter_style.justify_end()));
    }
    column(children)
        .with_key(props.key)
        .with_style(component_style(props, "root", style().gap(theme_spacing("xs"))))
}
