/* gpui-rhai
{
  "id": "components/label",
  "export": "Label",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Label presents a control label with optional required and description text.
//
// Props: text (required), required, description.
// Events: none. State: none. Parts: root, text, required, description.
// Example: Label(#{ text: "Project name", required: true })

define_component(#{
    metadata: #{
        id: "components/label",
        "export": "Label",
        version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: [],
        capabilities: #{},
    },
    schema: #{
        props: #{
            text: #{
                schema: #{ type: "string" },
                required: true,
                sensitive: false,
            },
            required: #{
                schema: #{ type: "bool" },
                required: false,
                sensitive: false,
                "default": #{ type: "bool", value: false },
            },
            description: #{
                schema: #{ type: "optional", value: #{ type: "string" } },
                required: false,
                sensitive: false,
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
    let title = if required { `${props.text} *` } else { props.text };
    let children = [
        text(title).with_style(component_style(props, "required", component_style(props, "text",
            style().typography("body").font_weight(700)
                .text_color(theme_color("text_primary")))))
    ];
    if description != () {
        children.push(
            text(description)
                .with_style(component_style(props, "description",
                    style().typography("body_small").text_color(theme_color("text_muted"))))
        );
    }
    column(children)
        .with_style(component_style(props, "root", style().gap(px(4))))
        .accessibility_role("label")
        .accessibility_label(props.text)
}
