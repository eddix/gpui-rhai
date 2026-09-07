/* gpui-rhai
{
  "id": "components/form_field",
  "export": "FormField",
  "version": "0.1.1",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": ["components/label"],
  "capabilities": {}
}
*/
// Associates label, control, description, required state, and error content.
// Props: stable id, label, control slot, description, error, required. Parts mirror these fields.
// State/events: stateless association wrapper; emits no events.
// Example: FormField(#{ id: "name", label: "Name", control: input_node, required: true })

import "components/label" as label_component;

define_component(#{
    metadata: #{
        id: "components/form_field", "export": "FormField", version: "0.1.1",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 },
        dependencies: ["components/label"], capabilities: #{},
    },
    schema: #{
        props: #{
            id: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            control: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            description: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            error: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
            required: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ control: #{ required: true, multiple: false } },
        parts: ["root", "label", "control", "description", "error"],
    },
    render: Fn("render_FormField")
});

fn FormField(props) { render_component("components/form_field", props) }
fn render_FormField(ctx, props) {
    let description = if "description" in props { props.description } else { () };
    let error = if "error" in props { props.error } else { () };
    let required = if "required" in props { props.required } else { false };
    let label_id = `${props.id}-label`;
    let description_id = `${props.id}-description`;
    let error_id = `${props.id}-error`;
    let described_by = if error != () { error_id }
        else if description != () { description_id } else { "" };
    let children = [
        label_component::Label(#{
            text: props.label,
            required: required,
        }).with_style(ctx.component_style("label", style()))
            .accessibility_id(label_id),
        props.control.with_style(ctx.component_style("control", style()))
            .accessibility_labelled_by(label_id)
            .accessibility_described_by(described_by)
            .accessibility_required(required)
    ];
    if description != () {
        children.push(text(description)
            .with_style(ctx.component_style("description",
                style().typography("body_small")
                    .text_color(theme_color("text_muted"))))
            .accessibility_id(description_id));
    }
    if error != () {
        children.push(text(error).with_style(ctx.component_style("error",
            style().typography("body_small")
                .text_color(theme_color("danger"))))
            .accessibility_id(error_id).accessibility_role("alert"));
    }
    column(children).with_key(props.id)
        .with_style(ctx.component_style("root", style().gap(px(4))))
        .accessibility_role("group")
        .accessibility_label(props.label)
        .accessibility_invalid(error != ())
}
