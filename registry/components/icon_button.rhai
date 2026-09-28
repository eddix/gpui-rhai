/* gpui-rhai
{
  "id": "components/icon_button",
  "export": "IconButton",
  "version": "0.1.6",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": ["components/button"],
  "capabilities": {}
}
*/
// IconButton presents one icon in a square action target.
//
// Props: icon and label (required), variant, size, selected, disabled, loading, hoverable, on_click.
// Events: click(null).
// State: stateless and controlled by caller props.
// Parts: root, icon.
// Example: IconButton(#{ icon: svg(markup), label: "Close", variant: "ghost", size: "sm" })

import "components/button" as button;

define_component(#{
    metadata: #{
        id: "components/icon_button",
        "export": "IconButton",
        version: "0.1.6",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 },
        dependencies: ["components/button"],
        capabilities: #{},
    },
    schema: #{
        props: #{
            icon: #{
                schema: #{ type: "node" },
                required: true,
                sensitive: false,
            },
            label: #{
                schema: #{ type: "string" },
                required: true,
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
                "default": #{ type: "string", value: "ghost" },
            },
            size: #{
                schema: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] },
                required: false,
                sensitive: false,
                "default": #{ type: "string", value: "md" },
            },
            selected: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
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
        slots: #{ icon: #{ required: true, multiple: false } },
        parts: ["root", "icon"],
    },
    render: Fn("render_IconButton")
});

fn icon_button_icon_size(size) {
    if size == "xs" { 12 }
    else if size == "sm" { 14 }
    else if size == "lg" { 18 }
    else { 16 }
}

fn icon_button_foreground(variant, selected, unavailable) {
    if unavailable { theme_color("disabled") }
    else if selected && (variant == "ghost" || variant == "outline" || variant == "secondary") {
        theme_color("accent")
    } else { button::button_foreground(variant) }
}

fn IconButton(props) { render_component("components/icon_button", props) }
fn render_IconButton(ctx, props) {
    if props.label == "" { throw "IconButton label must not be empty"; }
    let variant = if "variant" in props { props.variant } else { "ghost" };
    let size = if "size" in props { props.size } else { "md" };
    let disabled = if "disabled" in props { props.disabled } else { false };
    let loading = if "loading" in props { props.loading } else { false };
    let selected = if "selected" in props { props.selected } else { false };
    let hoverable = if "hoverable" in props { props.hoverable } else { true };
    let interactive = !disabled && !loading && "on_click" in props && props.on_click != ();
    let side = button::button_height(size);
    let icon_side = icon_button_icon_size(size);
    let foreground = icon_button_foreground(variant, selected, disabled || loading);

    let base = style()
        .flex_row()
        .items_center()
        .justify_center()
        .width(px(side))
        .height(px(side))
        .padding(px(0))
        .radius(theme_radius("md"))
        .border(px(1))
        .border_color(button::button_border(variant))
        .background(button::button_background(variant))
        .text_color(foreground)
        .focus(style().border_color(theme_color("focus_ring")))
        .active(style().opacity(0.86))
        .disabled(style()
            .background(theme_color("surface_raised"))
            .border_color(theme_color("border"))
            .text_color(theme_color("disabled"))
            .opacity(0.72));
    if hoverable && !disabled && !loading {
        base = base.hover(style().background(button::button_hover(variant)));
    }

    let icon = props.icon.with_style(ctx.component_style("icon", style()
        .width(px(icon_side))
        .height(px(icon_side))
        .text_color(foreground)));
    let node = row([icon])
        .with_style(ctx.component_style("root", base))
        .disabled(disabled || loading)
        .accessibility_role("button")
        .accessibility_label(props.label)
        .accessibility_pressed(selected);
    if interactive { node.on_click(props.on_click) } else { node }
}
