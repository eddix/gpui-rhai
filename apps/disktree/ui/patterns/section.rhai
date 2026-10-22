/* gpui-rhai
{
  "id": "patterns/section",
  "export": "Section",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "space.related", "spacing.xxs", "metrics.control", "metrics.label_column", "text_primary", "text_muted",
    "typography.subtitle", "typography.caption", "typography.label"
  ],
  "environment": []
}
*/
// Section is a titled part of a region: a subtitle, an optional description and actions on
// the end side, then the content.
//
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
            "space.related", "spacing.xxs", "metrics.control", "metrics.label_column", "text_primary", "text_muted",
            "typography.subtitle", "typography.caption", "typography.label"
        ],
        environment: [],
    },
    schema: #{
        props: #{
            title: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Section heading, drawn in the `voice`; also the section's accessible name." },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "A muted caption under the title." },
            actions: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Controls on the end side of the header; they wrap below the heading when space runs out." },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "The section body, `related` below the header." },
            voice: #{ schema: #{ type: "string", allowed: ["title", "label"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "title" },
                doc: "`title` draws a subtitle; `label` the uppercase muted label voice, for dense side panels." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{
            actions: #{ required: false, multiple: true, doc: "Header controls on the end side." },
            content: #{ required: true, multiple: false, doc: "The section body." },
        },
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
    // The heading keeps at least a label column beside the actions; when the two do not
    // fit, the actions wrap below the heading instead of squeezing it.
    let header = row([
        column(heading).with_style(style().gap(theme_spacing("xxs")).flex_grow().min_width(px(0))
            .flex_basis(theme_length("metrics.label_column"))),
        row(actions).with_style(ctx.component_style("actions", style()
            .flex_row().items_center().gap(theme_length("space.related")))),
    ]).with_style(ctx.component_style("header", style()
        .flex_row().flex_wrap().items_center().justify_between().gap(theme_length("space.related"))
        .min_height(if actions.len > 0 { theme_length("metrics.control") } else { px(0) })));
    column([header, props.content.with_style(ctx.component_style("content", style()))])
        .with_style(ctx.component_style("root", style().flex_col().gap(theme_length("space.related"))))
        .accessibility_role("group").accessibility_label(props.title)
}
