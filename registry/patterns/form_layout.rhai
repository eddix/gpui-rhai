/* gpui-rhai
{
  "id": "patterns/form_layout",
  "export": "FormLayout",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "metrics.control", "metrics.label_column", "space.unit", "space.related", "space.group",
    "space.section", "spacing.xxs", "border", "text_primary", "text_muted", "text.danger",
    "typography.body", "typography.caption", "typography.label"
  ],
  "environment": ["density"]
}
*/
// FormLayout aligns labels on one column, groups fields, and lines the submit row up with
// the fields.
//
// Props: key and label (required), groups (title, fields) or fields, submit, orientation
//        (horizontal|vertical), label_width.
//        A field: label, control (node), description, error, required.
// Events/state: none. Parts: root, group, group_title, field, label, control, description,
//        error, submit.
// Example: FormLayout(#{ key: "service", label: "Service", fields: [
//              #{ label: "Name", control: name_input, required: true }],
//              submit: [save_button] })
//
// Horizontal labels sit in a metrics.control high cell so they share the control's text
// line. Fields are `related`, groups `group` apart (vertical forms one step larger).

define_component(#{
    metadata: #{
        id: "patterns/form_layout", "export": "FormLayout", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "metrics.control", "metrics.label_column", "space.unit", "space.related",
            "space.group", "space.section", "spacing.xxs", "border", "text_primary",
            "text_muted", "text.danger", "typography.body", "typography.caption",
            "typography.label"
        ],
        environment: ["density"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            groups: #{ schema: #{ type: "array", max_items: 32, items: #{
                type: "object", allow_unknown: false, fields: #{
                    title: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
                    fields: #{ schema: #{ type: "array", max_items: 64, items: #{ type: "object", allow_unknown: false, fields: #{
                        label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                        control: #{ schema: #{ type: "node" }, required: true, sensitive: false },
                        description: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                            required: false, sensitive: false },
                        error: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                            required: false, sensitive: false },
                        required: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                            "default": #{ type: "bool", value: false } },
                    } } },
                        required: true, sensitive: false },
                },
            } }, required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            fields: #{ schema: #{ type: "array", max_items: 64, items: #{ type: "object", allow_unknown: false, fields: #{
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    control: #{ schema: #{ type: "node" }, required: true, sensitive: false },
                    description: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                        required: false, sensitive: false },
                    error: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                        required: false, sensitive: false },
                    required: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                } } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            submit: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "horizontal" } },
            label_width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ submit: #{ required: false, multiple: true } },
        parts: ["root", "group", "group_title", "field", "label", "control", "description", "error", "submit"],
    },
    render: Fn("render_FormLayout")
});

fn FormLayout(props) { render_component("patterns/form_layout", props) }
fn field_value(field, name) { if name in field && field[name] != () { field[name] } else { () } }

fn form_field(ctx, form_key, index, field, vertical, label_width) {
    let id = `${form_key}-field-${index}`;
    let required = "required" in field && field.required;
    let error = field_value(field, "error");
    let description = field_value(field, "description");
    let label_text = if required {
        text([span(field.label), span(" *").color(theme_color("text.danger"))])
    } else { text([span(field.label)]) };
    let label_cell = row([label_text.with_style(ctx.component_style("label", style()
            .text_color(theme_color("text_primary"))))
            .accessibility_id(`${id}-label`)])
        .with_style(style().flex_row().items_center().flex_shrink(false)
            .min_height(if vertical { px(0) } else { theme_length("metrics.control") })
            .width(if vertical { auto() } else { label_width }));
    let described_by = if error != () { `${id}-error` } else if description != () { `${id}-description` } else { "" };
    let stack = [field.control.with_style(ctx.component_style("control", style()))
        .accessibility_labelled_by(`${id}-label`)
        .accessibility_described_by(described_by)
        .accessibility_required(required)];
    if description != () {
        stack.push(text(description).with_style(ctx.component_style("description", style()
            .typography("caption").text_color(theme_color("text_muted"))))
            .accessibility_id(`${id}-description`));
    }
    if error != () {
        stack.push(text(error).with_style(ctx.component_style("error", style()
            .typography("caption").text_color(theme_color("text.danger"))))
            .accessibility_id(`${id}-error`).accessibility_role("alert"));
    }
    let control_cell = column(stack).with_style(style().flex_col().flex_grow().min_width(px(0))
        .gap(theme_spacing("xxs")));
    let pair = if vertical { column([label_cell, control_cell]) } else { row([label_cell, control_cell]) };
    pair.with_style(ctx.component_style("field", style()
        .gap(if vertical { theme_length("space.unit") } else { theme_length("space.related") })
        .items_start()))
        .accessibility_role("group").accessibility_label(field.label)
        .accessibility_invalid(error != ())
}

fn render_FormLayout(ctx, props) {
    let vertical = "orientation" in props && props.orientation == "vertical";
    let label_width = if "label_width" in props && props.label_width != () { props.label_width }
        else { theme_length("metrics.label_column") };
    let groups = if "groups" in props && props.groups.len > 0 { props.groups }
        else { [#{ fields: if "fields" in props { props.fields } else { [] } }] };
    let field_gap = if vertical { theme_length("space.group") } else { theme_length("space.related") };
    let group_gap = if vertical { theme_length("space.section") } else { theme_length("space.group") };
    let nodes = [];
    let index = 0;
    for group in groups {
        let children = [];
        if "title" in group && group.title != () {
            children.push(text(group.title.to_upper()).with_style(ctx.component_style("group_title", style()
                .typography("label").text_color(theme_color("text_muted"))))
                .accessibility_role("heading").accessibility_level(3));
        }
        for field in group.fields {
            children.push(form_field(ctx, props.key, index, field, vertical, label_width));
            index += 1;
        }
        nodes.push(column(children).with_style(ctx.component_style("group", style()
            .flex_col().gap(field_gap))));
    }
    let submit = if "submit" in props { props.submit } else { [] };
    if submit.len > 0 {
        let actions = row(submit).with_style(style().flex_row().items_center().gap(theme_length("space.related")));
        // The submit row starts on the field edge, past the label column.
        nodes.push(if vertical { actions.with_style(ctx.component_style("submit", style())) }
            else {
                row([row([]).with_style(style().width(label_width).flex_shrink(false)), actions])
                    .with_style(ctx.component_style("submit", style().gap(theme_length("space.related"))))
            });
    }
    column(nodes).with_key(props.key)
        .with_style(ctx.component_style("root", style().flex_col().gap(group_gap).typography("body")))
        .accessibility_role("group").accessibility_label(props.label)
}
