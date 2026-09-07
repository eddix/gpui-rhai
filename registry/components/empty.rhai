/* gpui-rhai
{
  "id": "components/empty",
  "export": "Empty",
  "version": "0.1.1",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Consistent empty-state composition with optional icon, content, and actions.
// Props: title, optional description/icon/content, and action nodes.
// Events/state: stateless and emits no events.
// Parts: root, icon, title, description, content, actions.
// Example: Empty(#{ title: "No projects", description: "Create one to begin." })

define_component(#{
    metadata: #{ id: "components/empty", "export": "Empty", version: "0.1.1",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            icon: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            content: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            actions: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{
            icon: #{ required: false, multiple: false }, content: #{ required: false, multiple: false },
            actions: #{ required: false, multiple: true },
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
            .width(px(40)).height(px(40)).items_center().justify_center()
            .radius(px(20)).background(theme_color("surface_raised"))
            .text_color(theme_color("text_muted")))));
    }
    children.push(text(props.title).with_style(ctx.component_style("title", style()
        .typography("title").font_weight(700)
        .text_color(theme_color("text_primary")))));
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
            .padding_top(px(4)).gap(px(8)).items_center().justify_center())));
    }
    column(children).with_style(ctx.component_style("root", style()
        .min_height(px(180)).padding(px(24)).gap(px(8)).items_center().justify_center()
        .radius(theme_radius("md")).border(px(1)).border_dashed()
        .border_color(theme_color("border")).background(theme_color("surface"))))
        .accessibility_role("status").accessibility_label(props.title)
}
