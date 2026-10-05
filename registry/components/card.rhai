/* gpui-rhai
{
  "id": "components/card",
  "export": "Card",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_raised", "border", "text_primary", "metrics.inset", "space.group", "radius.lg",
    "typography.subtitle"
  ],
  "environment": ["density", "corners"]
}
*/
// General themed content surface with optional header and footer slots.
// Props: content, optional header/footer nodes, and elevated.
// Events/state: stateless and emits no events.
// Parts: root, header, content, footer.
// Example: Card(#{ header: text("Account"), content: text("Ada") })

define_component(#{
    metadata: #{ id: "components/card", "export": "Card", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_raised", "border", "text_primary", "metrics.inset", "space.group",
            "radius.lg", "typography.subtitle"
        ],
        environment: ["density", "corners"] },
    schema: #{
        props: #{
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            header: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            footer: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            variant: #{ schema: #{ type: "string", allowed: ["block", "outline"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "block" } },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{
            header: #{ required: false, multiple: false },
            content: #{ required: true, multiple: false },
            footer: #{ required: false, multiple: false },
        },
        parts: ["root", "header", "content", "footer"],
    },
    render: Fn("render_Card")
});

fn Card(props) { render_component("components/card", props) }
fn render_Card(ctx, props) {
    let variant = if "variant" in props { props.variant } else { "block" };
    let children = [];
    if "header" in props && props.header != () {
        children.push(props.header.with_style(ctx.component_style("header", style()
            .typography("subtitle").text_color(theme_color("text_primary")))));
    }
    children.push(props.content.with_style(ctx.component_style("content", style())));
    if "footer" in props && props.footer != () {
        children.push(props.footer.with_style(ctx.component_style("footer", style())));
    }
    // A card is a color block; the outline variant is for cards placed on a raised
    // surface. Parts are separated by space, not rules.
    let root = style().padding(theme_length("metrics.inset")).gap(theme_length("space.group"))
        .radius(theme_radius("lg"));
    root = if variant == "outline" {
        root.border(px(1)).border_color(theme_color("border"))
    } else { root.background(theme_color("surface_raised")) };
    column(children).with_style(ctx.component_style("root", root))
        .accessibility_role("group")
}
