/* gpui-rhai
{
  "id": "components/icon_button",
  "export": "IconButton",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/button"],
  "capabilities": {},
  "tokens": ["border","button.selected","control.border","disabled","focus_ring","metrics.button_radius","metrics.control","metrics.icon","metrics.icon_button_h","metrics.icon_button_icon","radius.md","selection","surface_hover","text_primary"],
  "environment": ["size", "density"]
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
        version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/button"],
        capabilities: #{},
        tokens: ["border","button.selected","control.border","disabled","focus_ring","metrics.button_radius","metrics.control","metrics.icon","metrics.icon_button_h","metrics.icon_button_icon","radius.md","selection","surface_hover","text_primary"],
        environment: ["size", "density"],
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
                        "primary", "secondary", "danger", "destructive", "warning",
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
            selected: #{ schema: #{ type: "optional", value: #{ type: "bool" } }, required: false, sensitive: false,
                doc: "Omit for an ordinary action. Pass false or true for a controlled toggle and its accessible pressed state. Selected ghost, outline and secondary variants fill with selection and stop reacting to hover." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Blocks clicks and dims the complete variant to 40% opacity, as on `Button`; a disabled `action` disables it too." },
            loading: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Blocks clicks while work runs, with Button's turning spinner in place of the icon. Reduced or no motion holds it still." },
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
        parts: ["root", "icon", "spinner"],
    },
    render: Fn("render_IconButton")
});

fn IconButton(props) { render_component("components/icon_button", props) }

// Fn names belong to the module that owns the retained callback. Delegate
// through an ordinary import call to the shared Button action dispatch.
fn icon_button_action(action, ctx, payload) {
    button::run_action(action, ctx, payload);
}

fn render_IconButton(ctx, props) {
    if props.label == "" { throw "IconButton label must not be empty"; }
    let variant = if "variant" in props { props.variant } else { "ghost" };
    let action = if "action" in props && props.action != () { props.action } else { () };
    let disabled = (if "disabled" in props { props.disabled } else { false })
        || (action != () && !ctx.action_enabled(action));
    let loading = if "loading" in props { props.loading } else { false };
    let has_selection = "selected" in props && props.selected != ();
    let selected = if has_selection { props.selected } else { false };
    let hoverable = if "hoverable" in props { props.hoverable } else { true };
    let has_click = "on_click" in props && props.on_click != ();
    let interactive = !disabled && !loading && (has_click || action != ());
    let base = style().flex_row().items_center().justify_center()
        .width(theme_length("metrics.icon_button_h"))
        .height(theme_length("metrics.icon_button_h"))
        .padding(px(0)).radius(theme_length("metrics.button_radius"))
        .border(px(1)).border_color(rgba(0x00000000)).focus(style());
    let state_style = button::button_recipe(ctx, variant)
        .disabled(style().opacity(if disabled { 0.4 } else { 1.0 }));
    if selected { state_style = state_style.merge(ctx.theme_style("button.selected")); }
    if !hoverable || loading || selected { state_style = state_style.without_hover().without_active(); }
    let icon = if loading {
        button::loading_spinner(ctx)
    } else {
        props.icon.with_style(ctx.component_style("icon", style()
            .width(theme_length("metrics.icon_button_icon"))
            .height(theme_length("metrics.icon_button_icon"))
            ))
    };
    let node = row([button::icon_entry(icon, loading)])
        .with_style(ctx.component_style("root", base, state_style))
        .paint_transition(button::paint_duration(ctx), cubic_bezier(0.25,0.1,0.25,1.0))
        .disabled(disabled || loading)
        .accessibility_role("button")
        .accessibility_label(props.label);
    if has_selection { node = node.accessibility_pressed(selected); }
    if "size" in props && props.size != () {
        node = node.env(#{ size: props.size });
    }
    if action != () {
        let shortcut = ctx.action_shortcut(action);
        if shortcut != () { node = node.accessibility_key_shortcuts(shortcut.keystrokes); }
    }
    if !interactive { node }
    else if action != () { node.on_click(Fn("icon_button_action").curry(action)) }
    else { node.on_click(props.on_click) }
}
