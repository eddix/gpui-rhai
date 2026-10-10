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
// Labelled group of related controls or settings: a top hairline and a label-voice
// heading on the parent's content edge, with no surface of its own.
//
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
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Heading in the uppercase label voice under the top hairline; also the accessible name." },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "A muted caption under the label." },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "The grouped controls or settings, `related` below the heading." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Disables every control inside and dims the whole group." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ content: #{ required: true, multiple: false, doc: "The grouped controls or settings." } },
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
