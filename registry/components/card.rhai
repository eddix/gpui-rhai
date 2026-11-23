/* gpui-rhai
{
  "id": "components/card",
  "export": "Card",
  "version": "0.4.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["border","card.root_interactive","card.root_outline","card.root_selected","card.surface_hover","card.surface_selected","metrics.inset","radius.lg","space.group","surface_raised","text_primary","typography.subtitle"],
  "environment": ["density"]
}
*/
// General themed content surface with optional header and footer slots.
//
// Example: Card(#{ header: text("Account"), content: text("Ada") })

define_component(#{
    metadata: #{ id: "components/card", "export": "Card", version: "0.4.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["border","card.root_interactive","card.root_outline","card.root_selected","card.surface_hover","card.surface_selected","metrics.inset","radius.lg","space.group","surface_raised","text_primary","typography.subtitle"],
        environment: ["density"] },
    schema: #{
        props: #{
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "The card body; header, content and footer sit `group` apart within `metrics.inset`." },
            header: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "A heading node above the content, in `subtitle` typography." },
            footer: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "A node below the content." },
            selected: #{ schema: #{type:"bool"},required:false,sensitive:false,
                "default":#{type:"bool",value:false},doc:"Selected visual state; selection remains controlled by the caller." },
            interactive: #{ schema: #{type:"bool"},required:false,sensitive:false,
                "default":#{type:"bool",value:false},doc:"Enables the preset's hover treatment; actions and keyboard behavior remain with the content's controls." },
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
        parts: ["root", "surface", "header", "content", "footer"],
    },
    render: Fn("render_Card")
});

fn Card(props) { render_component("components/card", props) }
fn render_Card(ctx, props) {
    let children = [];
    if "header" in props && props.header != () {
        children.push(props.header.with_key("header").with_style(ctx.component_style("header", style())));
    }
    children.push(props.content.with_key("content").with_style(ctx.component_style("content", style())));
    let selected = "selected" in props && props.selected;
    let interactive = "interactive" in props && props.interactive;
    let root_state = style();
    let surface_state = style();
    if interactive {
        root_state = ctx.theme_style("card.root_interactive");
        surface_state = style().group_hover(ctx.theme_style("card.surface_hover"));
    }
    if selected {
        root_state = root_state.merge(ctx.theme_style("card.root_selected"));
        surface_state = surface_state.merge(ctx.theme_style("card.surface_selected"));
    }
    if "variant" in props && props.variant == "outline" {
        root_state = root_state.merge(ctx.theme_style("card.root_outline"));
    }
    // Keep exactly the same node/part topology across presets. Only recipes
    // change, so a keyed editor in the content never changes its native owner.
    let surface = column(children).with_key("surface")
        .with_style(ctx.component_style("surface", style().min_width(px(0)), surface_state));
    let content = [surface];
    if "footer" in props && props.footer != () {
        content.push(props.footer.with_key("footer").with_style(ctx.component_style("footer", style())));
    }
    column(content).with_style(ctx.component_style("root", style().min_width(px(0)), root_state))
        .accessibility_role("group")
}
