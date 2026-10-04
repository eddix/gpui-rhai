/* gpui-rhai
{
  "id": "components/group_box",
  "export": "GroupBox",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "border", "text_muted", "space.related", "spacing.xxs", "typography.label",
    "typography.caption"
  ],
  "environment": ["density"]
}
*/
// Labelled desktop group surface for related controls or settings.
// Props: label, content, optional description, and disabled.
// Events/state: stateless and emits no events.
// Parts: root, label, description, content.
// Example: GroupBox(#{ label: "Sync", content: text("Settings") })

define_component(#{
    metadata: #{ id: "components/group_box", "export": "GroupBox", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "border", "text_muted", "space.related", "spacing.xxs", "typography.label",
            "typography.caption"
        ],
        environment: ["density"] },
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
    // A labelled group: a hairline rule, a label-voice heading and the content, all on
    // the parent's content edge.
    let header = [text(props.label.to_upper()).with_style(ctx.component_style("label", style()
        .typography("label").text_color(theme_color("text_muted"))))];
    if "description" in props && props.description != () {
        header.push(text(props.description).with_style(ctx.component_style("description", style()
            .typography("caption").text_color(theme_color("text_muted")))));
    }
    column([
        column(header).with_style(style().gap(theme_spacing("xxs"))),
        props.content.with_style(ctx.component_style("content", style()))
    ]).with_style(ctx.component_style("root", style()
        .padding_top(theme_length("space.related")).gap(theme_length("space.related"))
        .border_top(px(1)).border_color(theme_color("border"))
        .disabled(style().opacity(0.62))))
        .disabled("disabled" in props && props.disabled)
        .accessibility_role("group").accessibility_label(props.label)
}
