/* gpui-rhai
{
  "id": "components/card",
  "export": "Card",
  "version": "0.1.1",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// General themed content surface with optional header and footer slots.
// Props: content, optional header/footer nodes, and elevated.
// Events/state: stateless and emits no events.
// Parts: root, header, content, footer.
// Example: Card(#{ header: text("Account"), content: text("Ada") })

define_component(#{
    metadata: #{ id: "components/card", "export": "Card", version: "0.1.1",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            header: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            footer: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            elevated: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
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
    let children = [];
    if "header" in props && props.header != () {
        children.push(props.header.with_style(ctx.component_style("header", style()
            .padding_bottom(px(10)).border_bottom(px(1)).border_color(theme_color("border")))));
    }
    children.push(props.content.with_style(ctx.component_style("content", style())));
    if "footer" in props && props.footer != () {
        children.push(props.footer.with_style(ctx.component_style("footer", style()
            .padding_top(px(10)).border_top(px(1)).border_color(theme_color("border")))));
    }
    let elevated = "elevated" in props && props.elevated;
    column(children).with_style(ctx.component_style("root", style()
        .padding(px(14)).gap(px(12)).radius(theme_radius("md"))
        .border(px(1)).border_color(theme_color("border"))
        .background(if elevated { theme_color("surface_raised") } else { theme_color("surface") })))
        .accessibility_role("group")
}
