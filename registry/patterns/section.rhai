/* gpui-rhai
{
  "id": "patterns/section",
  "export": "Section",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "space.related", "spacing.xxs", "metrics.control", "text_primary", "text_muted",
    "typography.subtitle", "typography.caption", "typography.label"
  ],
  "environment": []
}
*/
// Section is a titled part of a region: a subtitle, an optional description and actions on
// the end side, then the content.
//
// Props: title (required), content (required), description, actions, voice (title|label).
// Events/state: none. Parts: root, header, title, description, actions, content.
// Example: Section(#{ title: "Replicas", actions: [scale_button], content: table_node })
//
// `voice: "label"` uses the label voice instead of a subtitle, for dense side panels. A
// region shows one title; sections inside it never repeat the title role.

define_component(#{
    metadata: #{
        id: "patterns/section", "export": "Section", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "space.related", "spacing.xxs", "metrics.control", "text_primary", "text_muted",
            "typography.subtitle", "typography.caption", "typography.label"
        ],
        environment: [],
    },
    schema: #{
        props: #{
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            actions: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            voice: #{ schema: #{ type: "string", allowed: ["title", "label"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "title" } },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ actions: #{ required: false, multiple: true }, content: #{ required: true, multiple: false } },
        parts: ["root", "header", "title", "description", "actions", "content"],
    },
    render: Fn("render_Section")
});

fn Section(props) { render_component("patterns/section", props) }
fn render_Section(ctx, props) {
    let label_voice = "voice" in props && props.voice == "label";
    let title = if label_voice {
        text(props.title.to_upper()).with_style(ctx.component_style("title", style()
            .typography("label").text_color(theme_color("text_muted"))))
    } else {
        text(props.title).with_style(ctx.component_style("title", style()
            .typography("subtitle").text_color(theme_color("text_primary"))))
    };
    let heading = [title.accessibility_role("heading").accessibility_level(2)];
    if "description" in props && props.description != () {
        heading.push(text(props.description).with_style(ctx.component_style("description", style()
            .typography("caption").text_color(theme_color("text_muted")))));
    }
    let actions = if "actions" in props { props.actions } else { [] };
    let header = row([
        column(heading).with_style(style().gap(theme_spacing("xxs")).flex_grow().min_width(px(0))),
        row(actions).with_style(ctx.component_style("actions", style()
            .flex_row().items_center().gap(theme_length("space.related")))),
    ]).with_style(ctx.component_style("header", style()
        .flex_row().items_center().justify_between().gap(theme_length("space.related"))
        .min_height(if actions.len > 0 { theme_length("metrics.control") } else { px(0) })));
    column([header, props.content.with_style(ctx.component_style("content", style()))])
        .with_style(ctx.component_style("root", style().flex_col().gap(theme_length("space.related"))))
        .accessibility_role("group").accessibility_label(props.title)
}
