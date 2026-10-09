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
//
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
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "The card body; header, content and footer sit `group` apart within `metrics.inset`." },
            header: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "A heading node above the content, in `subtitle` typography." },
            footer: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "A node below the content." },
            variant: #{ schema: #{ type: "string", allowed: ["block", "outline"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "block" },
                doc: "`block` fills with `surface_raised`; `outline` draws a hairline, for cards on a raised surface." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{
            header: #{ required: false, multiple: false, doc: "The heading above the content." },
            content: #{ required: true, multiple: false, doc: "The card body." },
            footer: #{ required: false, multiple: false, doc: "The node below the content." },
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
