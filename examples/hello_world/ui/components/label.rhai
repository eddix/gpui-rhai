/* gpui-rhai
{
  "id": "components/label",
  "export": "Label",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "text_primary", "text_muted", "text.danger", "spacing.xxs", "typography.body",
    "typography.caption"
  ],
  "environment": []
}
*/
// Label presents a control label with optional required and description text.
//
// State: none.
// Example: Label(#{ text: "Project name", required: true })

define_component(#{
    metadata: #{
        id: "components/label",
        "export": "Label",
        version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [],
        capabilities: #{},
        tokens: [
            "text_primary", "text_muted", "text.danger", "spacing.xxs", "typography.body",
            "typography.caption"
        ],
        environment: [],
    },
    schema: #{
        props: #{
            text: #{
                schema: #{ type: "string" },
                required: true,
                sensitive: false,
                doc: "Label text in body type; also the accessible name of the label.",
            },
            required: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Appends a ` *` required mark in `text.danger` to the text.",
            },
            description: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false,
                sensitive: false,
                doc: "Secondary text in caption type and `text_muted`, under the label text.",
            },
        },
        state: #{ fields: #{} },
        events: #{},
        slots: #{},
        parts: ["root", "text", "required", "description"],
    },
    render: Fn("render_Label")
});

fn Label(props) { render_component("components/label", props) }
fn render_Label(ctx, props) {
    let required = if "required" in props { props.required } else { false };
    let description = if "description" in props { props.description } else { () };
    // Field labels are body text; weight stays 400 so labels never compete with
    // titles. The required mark is its own node in the readable danger text color,
    // so the `required` part styles or hides only the mark.
    let text_style = ctx.component_style("text", style()
        .typography("body")
        .text_color(theme_color("text_primary")));
    let title = if required {
        row([
            text(props.text).with_style(style().min_width(px(0))),
            text(" *").with_style(ctx.component_style("required", style()
                .flex_shrink(false).text_color(theme_color("text.danger")))),
        ]).with_style(text_style)
    } else { text(props.text).with_style(text_style) };
    let children = [title];
    if description != () {
        children.push(
            text(description)
                .with_style(ctx.component_style("description",
                    style().typography("caption")
                        .text_color(theme_color("text_muted"))))
        );
    }
    column(children)
        .with_style(ctx.component_style("root", style().gap(theme_spacing("xxs"))))
        .accessibility_role("label")
        .accessibility_label(props.text)
}
