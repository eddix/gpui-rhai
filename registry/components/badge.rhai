/* gpui-rhai
{
  "id": "components/badge",
  "export": "Badge",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Read-only compact status or metadata badge.
// Props: text, variant, dot, and size.
// Events/state: stateless and emits no events.
// Parts: root, dot, label.
// Example: Badge(#{ text: "Ready", variant: "success", dot: true })

define_component(#{
    metadata: #{
        id: "components/badge", "export": "Badge", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [], capabilities: #{},
    },
    schema: #{
        props: #{
            text: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            variant: #{
                schema: #{ type: "string", allowed: ["neutral", "accent", "success", "warning", "danger"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "neutral" },
            },
            dot: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            size: #{ schema: #{ type: "string", allowed: ["sm", "md"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "md" } },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{},
        parts: ["root", "dot", "label"],
    },
    render: Fn("render_Badge")
});

fn badge_color(variant) {
    if variant == "accent" { theme_color("accent") }
    else if variant == "success" { theme_color("success") }
    else if variant == "warning" { theme_color("warning") }
    else if variant == "danger" { theme_color("danger") }
    else { theme_color("text_muted") }
}

fn Badge(props) { render_component("components/badge", props) }
fn render_Badge(ctx, props) {
    let variant = if "variant" in props { props.variant } else { "neutral" };
    let size = if "size" in props { props.size } else { "md" };
    let children = [];
    if "dot" in props && props.dot {
        children.push(row([]).with_style(component_style(props, "dot", style()
            .width(px(6)).height(px(6)).radius(px(3)).background(badge_color(variant)))));
    }
    children.push(text(props.text).with_style(component_style(props, "label", style())));
    row(children).with_style(component_style(props, "root", style()
        .height(px(if size == "sm" { 18 } else { 22 }))
        .padding_x(px(if size == "sm" { 5 } else { 7 })).gap(px(5)).items_center()
        .radius(theme_radius("sm")).background(theme_color("surface_raised"))
        .border(px(1)).border_color(theme_color("border"))
        .typography(if size == "sm" { "caption" } else { "body_small" }).font_weight(700)
        .text_color(badge_color(variant))))
        .accessibility_role("status").accessibility_label(props.text)
}
