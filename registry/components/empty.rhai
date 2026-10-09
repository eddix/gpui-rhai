/* gpui-rhai
{
  "id": "components/empty",
  "export": "Empty",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "surface_raised", "text_primary", "text_muted", "metrics.control", "metrics.row",
    "space.related", "spacing.sm", "spacing.lg", "typography.subtitle", "typography.body"
  ],
  "environment": ["density"]
}
*/
// Consistent empty-state composition with optional icon, content, and actions.
//
// Example: Empty(#{ title: "No projects", description: "Create one to begin." })

define_component(#{
    metadata: #{ id: "components/empty", "export": "Empty", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_raised", "text_primary", "text_muted", "metrics.control", "metrics.row",
            "space.related", "spacing.sm", "spacing.lg", "typography.subtitle",
            "typography.body"
        ],
        environment: ["density"] },
    schema: #{
        props: #{
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "What is empty, as a subtitle; also the accessible name." },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "Why it is empty and what to do next; centered, muted, at most 420px wide." },
            icon: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "A glyph above the title, in a raised square 1.5 times `metrics.control` on each side." },
            content: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Extra content under the description." },
            actions: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Buttons centered under the text, `related` apart." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{
            icon: #{ required: false, multiple: false, doc: "A glyph above the title." },
            content: #{ required: false, multiple: false, doc: "Extra content under the description." },
            actions: #{ required: false, multiple: true, doc: "Buttons under the text." },
        },
        parts: ["root", "icon", "title", "description", "content", "actions"],
    },
    render: Fn("render_Empty")
});

fn Empty(props) { render_component("components/empty", props) }
fn render_Empty(ctx, props) {
    let children = [];
    if "icon" in props && props.icon != () {
        children.push(props.icon.with_style(ctx.component_style("icon", style()
            .width(theme_length("metrics.control") * 1.5).height(theme_length("metrics.control") * 1.5)
            .items_center().justify_center()
            .background(theme_color("surface_raised"))
            .text_color(theme_color("text_muted")))));
    }
    children.push(text(props.title).with_style(ctx.component_style("title", style()
        .typography("subtitle").text_color(theme_color("text_primary")))));
    if "description" in props && props.description != () {
        children.push(text(props.description).with_style(ctx.component_style("description", style()
            .max_width(px(420)).typography("body").text_center()
            .text_color(theme_color("text_muted")))));
    }
    if "content" in props && props.content != () {
        children.push(props.content.with_style(ctx.component_style("content", style())));
    }
    if "actions" in props && props.actions.len > 0 {
        children.push(row(props.actions).with_style(ctx.component_style("actions", style()
            .padding_top(theme_length("space.related")).gap(theme_length("space.related"))
            .items_center().justify_center())));
    }
    // An empty state is quiet: centered type on the region's own surface, no frame.
    column(children).with_style(ctx.component_style("root", style()
        .min_height(theme_length("metrics.row") * 6).padding(theme_spacing("lg"))
        .gap(theme_spacing("sm")).items_center().justify_center()))
        .accessibility_role("status").accessibility_label(props.title)
}
