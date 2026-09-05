/* gpui-rhai
{
  "id": "components/group_box",
  "export": "GroupBox",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Labelled desktop group surface for related controls or settings.
// Props: label, content, optional description, and disabled.
// Events/state: stateless and emits no events.
// Parts: root, label, description, content.
// Example: GroupBox(#{ label: "Sync", content: text("Settings") })

define_component(#{
    metadata: #{ id: "components/group_box", "export": "GroupBox", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ content: #{ required: true, multiple: false } },
        parts: ["root", "label", "description", "content"],
    },
    render: Fn("render_GroupBox")
});

fn GroupBox(props) { render_component("components/group_box", props) }
fn render_GroupBox(ctx, props) {
    let header = [text(props.label).with_style(component_style(props, "label", style()
        .typography("body").font_weight(700)
        .text_color(theme_color("text_primary"))))];
    if "description" in props && props.description != () {
        header.push(text(props.description).with_style(component_style(props, "description", style()
            .typography("body_small").text_color(theme_color("text_muted")))));
    }
    column([
        column(header).with_style(style().gap(px(2))),
        props.content.with_style(component_style(props, "content", style()))
    ]).with_style(component_style(props, "root", style()
        .padding(px(12)).gap(px(10)).radius(theme_radius("md"))
        .border(px(1)).border_color(theme_color("border"))
        .background(theme_color("surface"))
        .disabled(style().opacity(0.62))))
        .disabled("disabled" in props && props.disabled)
        .accessibility_role("group").accessibility_label(props.label)
}
