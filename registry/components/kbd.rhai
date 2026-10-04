/* gpui-rhai
{
  "id": "components/kbd",
  "export": "Kbd",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "border", "text_muted", "metrics.marker", "spacing.xxs", "spacing.xs", "radius.sm",
    "typography.label"
  ],
  "environment": []
}
*/
// Kbd presents a key legend: a framed keycap, or an inline legend inside menus and buttons.
//
// Props: text or keys (one is required), label, appearance.
// Events/state: stateless and emits no events.
// Parts: root, key.
// Example: Kbd(#{ keys: ["⌘", "K"], label: "Command K" })
//
// `keys` renders one keycap per key. Markers keep their size in both densities.

define_component(#{
    metadata: #{ id: "components/kbd", "export": "Kbd", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "border", "text_muted", "metrics.marker", "spacing.xxs", "spacing.xs", "radius.sm",
            "typography.label"
        ],
        environment: [] },
    schema: #{
        props: #{
            text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" } },
            keys: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "string" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            appearance: #{ schema: #{ type: "string", allowed: ["keycap", "inline"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "keycap" } },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{}, parts: ["root", "key"],
    },
    render: Fn("render_Kbd")
});

fn kbd_key(ctx, legend, inline) {
    let key = style()
        .flex_row().items_center().justify_center()
        .min_height(theme_length("metrics.marker"));
    if !inline {
        // Keycaps are at least square; the 1px frame is a structural hairline.
        key = key
            .min_width(theme_length("metrics.marker"))
            .padding_x(theme_spacing("xs"))
            .radius(theme_radius("sm"))
            .border(px(1)).border_color(theme_color("border"));
    }
    row([text(legend)]).with_style(ctx.component_style("key", key))
}

fn Kbd(props) { render_component("components/kbd", props) }
fn render_Kbd(ctx, props) {
    let keys = if "keys" in props { props.keys } else { [] };
    let legend = if "text" in props { props.text } else { "" };
    if keys.is_empty() && legend == "" { throw "Kbd needs text or keys"; }
    let inline = "appearance" in props && props.appearance == "inline";
    let caps = [];
    if keys.is_empty() { caps.push(kbd_key(ctx, legend, inline)); }
    for key in keys { caps.push(kbd_key(ctx, key, inline)); }
    let name = if "label" in props && props.label != () { props.label }
        else if keys.is_empty() { legend }
        else { keys.reduce(|sum, key| if sum == () { key } else { `${sum} ${key}` }) };
    row(caps)
        .with_style(ctx.component_style("root", style()
            .flex_row().items_center()
            .gap(if inline { px(0) } else { theme_spacing("xxs") })
            .text_color(theme_color("text_muted"))
            .typography("label")))
        .accessibility_role("text")
        .accessibility_label(name)
}
