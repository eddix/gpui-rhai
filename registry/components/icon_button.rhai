/* gpui-rhai
{
  "id": "components/icon_button",
  "export": "IconButton",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/button"],
  "capabilities": {},
  "tokens": [
    "surface_hover", "selection", "text_primary", "disabled", "focus_ring", "border",
    "metrics.control", "metrics.icon", "radius.md"
  ],
  "environment": ["size", "density", "corners"]
}
*/
// IconButton presents one icon in a square action target.
//
// State: stateless and controlled by caller props.
// Example: IconButton(#{ icon: svg(markup), label: "Close", variant: "ghost", size: "sm" })

import "components/button" as button;

define_component(#{
    metadata: #{
        id: "components/icon_button",
        "export": "IconButton",
        version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/button"],
        capabilities: #{},
        tokens: [
            "surface_hover", "selection", "text_primary", "disabled", "focus_ring", "border",
            "metrics.control", "metrics.icon", "radius.md"
        ],
        environment: ["size", "density", "corners"],
    },
    schema: #{
        props: #{
            icon: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "Icon node, sized `metrics.icon` square and drawn in the button's text color." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name; the button shows no text, so it must not be empty." },
            variant: #{
                schema: #{
                    type: "string",
                    allowed: [
                        "primary", "secondary", "danger", "warning",
                        "success", "ghost", "outline"
                    ]
                },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "ghost" },
                doc: "Fill, as on `Button`; the default `ghost` stays bare until hovered or `selected`.",
            },
            size: #{
                schema: #{
                    type: "optional",
                    value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] }
                },
                required: false, sensitive: false,
                doc: "Control size, which sets the square's side; unset, it inherits the `size` environment.",
            },
            selected: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Pressed state of a toggle; `ghost`, `outline` and `secondary` fill with `selection` and stop reacting to hover." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Blocks clicks and greys the icon; a disabled `action` disables it too." },
            loading: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Blocks clicks and shows the disabled look while work runs; it draws no spinner." },
            hoverable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true }, doc: "Changes the fill on hover; `false` keeps it still." },
            on_click: #{ schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
                doc: "Called when the button is pressed; not while disabled or loading, and not when `action` is set." },
            action: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                required: false, sensitive: false,
                doc: "Registered action dispatched on click, in place of `on_click`; it also sets the enabled state and the shortcut told to assistive tech." },
        },
        state: #{ fields: #{} },
        events: #{ click: #{ payload: #{ type: "null" },
            doc: "Emitted when the button is pressed while enabled, not loading and without an `action`." } },
        slots: #{ icon: #{ required: true, multiple: false, doc: "The icon in the square." } },
        parts: ["root", "icon"],
    },
    render: Fn("render_IconButton")
});

fn icon_button_fill(variant, selected) {
    if selected && (variant == "ghost" || variant == "outline" || variant == "secondary") {
        theme_color("selection")
    } else { button::button_fill(variant) }
}

fn IconButton(props) { render_component("components/icon_button", props) }
fn render_IconButton(ctx, props) {
    if props.label == "" { throw "IconButton label must not be empty"; }
    let variant = if "variant" in props { props.variant } else { "ghost" };
    let action = if "action" in props && props.action != () { props.action } else { () };
    let disabled = (if "disabled" in props { props.disabled } else { false })
        || (action != () && !ctx.action_enabled(action));
    let loading = if "loading" in props { props.loading } else { false };
    let selected = if "selected" in props { props.selected } else { false };
    let hoverable = if "hoverable" in props { props.hoverable } else { true };
    let has_click = "on_click" in props && props.on_click != ();
    let interactive = !disabled && !loading && (has_click || action != ());
    let foreground = button::button_foreground(variant);
    let fill = icon_button_fill(variant, selected);
    let frame = if variant == "outline" { theme_color("border") } else { fill };
    let base = style()
        .flex_row()
        .items_center()
        .justify_center()
        .width(theme_length("metrics.control"))
        .height(theme_length("metrics.control"))
        .radius(theme_radius("md"));
    base = base
        .border(px(2))
        .border_color(frame)
        .background(fill)
        .text_color(foreground)
        .focus(style().border_color(theme_color("focus_ring")))
        .active(style().opacity(0.86))
        .disabled(style()
            .background(if variant == "ghost" { rgba(0x00000000) } else { theme_color("surface_hover") })
            .text_color(theme_color("disabled")));
    if hoverable && !disabled && !loading && !selected {
        base = base.hover(style().background(button::button_hover(variant)));
    }
    let icon = props.icon.with_style(ctx.component_style("icon", style()
        .width(theme_length("metrics.icon"))
        .height(theme_length("metrics.icon"))
        .text_color(foreground)));
    let node = row([icon])
        .with_style(ctx.component_style("root", base))
        .disabled(disabled || loading)
        .accessibility_role("button")
        .accessibility_label(props.label)
        .accessibility_pressed(selected);
    if "size" in props && props.size != () {
        node = node.env(#{ size: props.size });
    }
    if action != () {
        let shortcut = ctx.action_shortcut(action);
        if shortcut != () { node = node.accessibility_key_shortcuts(shortcut.keystrokes); }
    }
    if !interactive { node }
    else if action != () { node.on_click(Fn("button::run_action").curry(action)) }
    else { node.on_click(props.on_click) }
}
