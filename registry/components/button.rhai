/* gpui-rhai
{
  "id": "components/button",
  "export": "Button",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Button presents a desktop action.
//
// Props: text (required), prefix/suffix slots, variant, size, disabled, loading, loading_text, hoverable, on_click.
// Events: click(null).
// State: stateless and controlled by caller props.
// Parts: root, label, prefix, suffix.
// Example: Button(#{ text: "Save", variant: "primary", size: "md" })

define_component(#{
    metadata: #{
        id: "components/button",
        "export": "Button",
        version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [],
        capabilities: #{},
    },
    schema: #{
        props: #{
            text: #{
                schema: #{ type: "string" },
                required: true,
                sensitive: false,
            },
            prefix: #{
                schema: #{ type: "optional", value: #{ type: "node" } },
                required: false,
                sensitive: false,
            },
            suffix: #{
                schema: #{ type: "optional", value: #{ type: "node" } },
                required: false,
                sensitive: false,
            },
            variant: #{
                schema: #{
                    type: "string",
                    allowed: [
                        "primary", "secondary", "danger", "warning",
                        "success", "ghost", "outline"
                    ]
                },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "primary" },
            },
            size: #{
                schema: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "md" },
            },
            disabled: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            loading: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            loading_text: #{
                schema: #{ type: "string" },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "" },
            },
            hoverable: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: true },
            },
            on_click: #{
                schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false,
                sensitive: false,
            },
        },
        state: #{ fields: #{} },
        events: #{ click: #{ payload: #{ type: "null" } } },
        slots: #{
            prefix: #{ required: false, multiple: false },
            suffix: #{ required: false, multiple: false },
        },
        parts: ["root", "label", "prefix", "suffix"],
    },
    render: Fn("render_Button")
});

fn button_height(size) {
    if size == "xs" { 24 }
    else if size == "sm" { 28 }
    else if size == "lg" { 36 }
    else { 32 }
}

fn button_font_size(size) { if size == "xs" { 11 } else { 12 } }

fn button_background(variant) {
    if variant == "danger" { theme_color("danger") }
    else if variant == "warning" { theme_color("warning") }
    else if variant == "success" { theme_color("success") }
    else if variant == "secondary" { theme_color("surface_raised") }
    else if variant == "ghost" || variant == "outline" { rgba(0x00000000) }
    else { theme_color("accent") }
}

fn button_border(variant) {
    if variant == "danger" { theme_color("danger") }
    else if variant == "warning" { theme_color("warning") }
    else if variant == "success" { theme_color("success") }
    else if variant == "primary" { theme_color("accent") }
    else if variant == "ghost" { rgba(0x00000000) }
    else { theme_color("border") }
}

fn button_hover(variant) {
    if variant == "primary" { theme_color("accent_hover") }
    else if variant == "secondary" || variant == "ghost" || variant == "outline" {
        theme_color("surface_hover")
    } else { button_background(variant) }
}

fn button_foreground(variant) {
    if variant == "danger" { theme_color("on_danger") }
    else if variant == "warning" { theme_color("on_warning") }
    else if variant == "success" { theme_color("on_success") }
    else if variant == "secondary" || variant == "ghost" || variant == "outline" {
        theme_color("text_primary")
    } else { theme_color("on_accent") }
}

fn Button(props) { render_component("components/button", props) }
fn render_Button(ctx, props) {
    let variant = if "variant" in props { props.variant } else { "primary" };
    let size = if "size" in props { props.size } else { "md" };
    let disabled = if "disabled" in props { props.disabled } else { false };
    let loading = if "loading" in props { props.loading } else { false };
    let loading_text = if "loading_text" in props { props.loading_text } else { "" };
    let hoverable = if "hoverable" in props { props.hoverable } else { true };
    let interactive = !disabled && !loading && "on_click" in props && props.on_click != ();
    let label = if loading && loading_text != "" {
        `${loading_text} ${props.text}`
    } else {
        props.text
    };

    let base = style()
        .flex_row()
        .items_center()
        .justify_center()
        .height(px(button_height(size)))
        .padding_x(px(10))
        .gap(px(8))
        .radius(theme_radius("md"))
        .border(px(1))
        .border_color(button_border(variant))
        .background(button_background(variant))
        .text_color(button_foreground(variant))
        .font_size(px(button_font_size(size)))
        .line_height(px(16))
        .font_weight(700)
        .focus(style().border_color(theme_color("focus_ring")))
        .active(style().opacity(0.86))
        .disabled(style()
            .background(theme_color("surface_raised"))
            .border_color(theme_color("border"))
            .text_color(theme_color("disabled"))
            .opacity(0.72));
    if hoverable && !disabled && !loading {
        base = base.hover(style().background(button_hover(variant)));
    }

    let children = [];
    if "prefix" in props && props.prefix != () {
        children.push(props.prefix.with_style(component_style(props, "prefix", style())));
    }
    children.push(text(label).with_style(component_style(props, "label",
        style().line_height(px(16)))));
    if "suffix" in props && props.suffix != () {
        children.push(props.suffix.with_style(component_style(props, "suffix", style())));
    }
    let button = row(children)
        .with_style(component_style(props, "root", base))
        .disabled(disabled || loading)
        .accessibility_role("button")
        .accessibility_label(props.text);
    if interactive {
        button.on_click(props.on_click)
    } else {
        button
    }
}
