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
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Keys the root and prefixes the accessibility ids of the fields, such as `<key>-field-0-label`." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the form; it is not drawn." },
            groups: #{ schema: #{ type: "array", max_items: 32, items: #{
                type: "object", allow_unknown: false, fields: #{
                    title: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Heading above the group, drawn upper-case in `label` type and `text_muted`." },
                    fields: #{ schema: #{ type: "array", max_items: 64, items: #{ type: "object", allow_unknown: false, fields: #{
                        label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                            doc: "Label text, in the label column or above the control in a vertical form." },
                        control: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                            doc: "The control; it keeps its own width, so a control that should be wide sets one." },
                        description: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                            required: false, sensitive: false,
                            doc: "Helper text in caption type under the control; it describes the control while there is no `error`." },
                        error: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                            required: false, sensitive: false,
                            doc: "Message in `text.danger` under the control, announced as an alert; also set the control's own `error`." },
                        required: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                            "default": #{ type: "bool", value: false },
                            doc: "Adds a `text.danger` asterisk to the label and marks the control required." },
                    } } },
                        required: true, sensitive: false, doc: "Fields of the group, in order." },
                },
            } }, required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Field groups, each under an optional title; when not empty, `fields` is ignored." },
            fields: #{ schema: #{ type: "array", max_items: 64, items: #{ type: "object", allow_unknown: false, fields: #{
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Label text, in the label column or above the control in a vertical form." },
                    control: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                        doc: "The control; it keeps its own width, so a control that should be wide sets one." },
                    description: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                        required: false, sensitive: false,
                        doc: "Helper text in caption type under the control; it describes the control while there is no `error`." },
                    error: #{ schema: #{ type: "optional", value: #{ type: "string" } },
                        required: false, sensitive: false,
                        doc: "Message in `text.danger` under the control, announced as an alert; also set the control's own `error`." },
                    required: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Adds a `text.danger` asterisk to the label and marks the control required." },
                } } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Fields of a form without groups, in order; used only while `groups` is empty." },
            submit: #{ schema: #{ type: "array", max_items: 4, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Action buttons in a row under the fields; in a horizontal form the row starts past the label column." },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "horizontal" },
                doc: "`horizontal` puts the labels in a column beside the controls; `vertical` stacks each label above its control." },
            label_width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false,
                doc: "Width of the label column of a horizontal form; unset, it is `metrics.label_column`. Ignored when vertical." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ submit: #{ required: false, multiple: true, doc: "The action buttons under the fields." } },
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
    // Controls keep their own width (contract C4): a field that should be wide says so.
    // A vertical field is one column (label, control, description, error a unit apart), so
    // it has no nested gap equal to its own.
    let pair = if vertical {
        let items = [label_cell];
        for node in stack { items.push(node); }
        column(items)
    } else {
        row([label_cell, column(stack).with_style(style().flex_col().flex_grow().min_width(px(0))
            .items_start().gap(theme_length("space.unit")))])
    };
    // A horizontal field row is a grid: the label column is a fixed track, so its gap
    // is not a proximity relationship for the control's own layout.
    let row_node = pair.with_style(ctx.component_style("field", style()
        .gap(if vertical { theme_length("space.unit") } else { theme_length("space.related") })
        .items_start()));
    if !vertical { row_node = row_node.audit_allow(["spacing-not-nested"]); }
    row_node
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
        // Horizontally the spacer is the label track, as in the field rows: its gap is
        // grid spacing, not a proximity relationship with the actions' own gap.
        nodes.push(if vertical { actions.with_style(ctx.component_style("submit", style())) }
            else {
                row([row([]).with_style(style().width(label_width).flex_shrink(false)), actions])
                    .with_style(ctx.component_style("submit", style().gap(theme_length("space.related"))))
                    .audit_allow(["spacing-not-nested"])
            });
    }
    column(nodes).with_key(props.key)
        .with_style(ctx.component_style("root", style().flex_col().gap(group_gap).typography("body")))
        .accessibility_role("group").accessibility_label(props.label)
}
