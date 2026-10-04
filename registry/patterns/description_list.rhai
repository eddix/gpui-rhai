/* gpui-rhai
{
  "id": "patterns/description_list",
  "export": "DescriptionList",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "metrics.row", "metrics.label_column", "space.related", "text_primary", "text_muted",
    "typography.body"
  ],
  "environment": ["density"]
}
*/
// DescriptionList shows facts as label and value pairs on one label column.
//
// Props: label (required), items (label, value as string or node, numeric, identifier),
//        label_width, layout (columns|stacked).
// Events/state: none. Parts: root, item, term, value.
// Example: DescriptionList(#{ label: "Host", items: [#{ label: "Region", value: "us-east" },
//              #{ label: "Cores", value: "32", numeric: true }] })
//
// Labels are muted data, values are primary data; numbers and identifiers use tabular
// figures and the slashed zero. Rows are metrics.row high so lists align with tables.

define_component(#{
    metadata: #{
        id: "patterns/description_list", "export": "DescriptionList", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "metrics.row", "metrics.label_column", "space.related", "text_primary", "text_muted",
            "typography.body"
        ],
        environment: ["density"],
    },
    schema: #{
        props: #{
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            items: #{ schema: #{ type: "array", max_items: 256, items: #{
                type: "object", allow_unknown: false, fields: #{
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    value: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "node" }] },
                        required: true, sensitive: false },
                    numeric: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                    identifier: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                },
            } }, required: true, sensitive: false },
            label_width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false },
            layout: #{ schema: #{ type: "string", allowed: ["columns", "stacked"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "columns" } },
        },
        state: #{ fields: #{} }, events: #{}, slots: #{},
        parts: ["root", "item", "term", "value"],
    },
    render: Fn("render_DescriptionList")
});

fn DescriptionList(props) { render_component("patterns/description_list", props) }
fn render_DescriptionList(ctx, props) {
    let stacked = "layout" in props && props.layout == "stacked";
    let label_width = if "label_width" in props && props.label_width != () { props.label_width }
        else { theme_length("metrics.label_column") };
    let rows = [];
    for item in props.items {
        let term = text(item.label).with_style(ctx.component_style("term", style()
            .text_color(theme_color("text_muted")).flex_shrink(false)
            .width(if stacked { auto() } else { label_width })));
        let value_style = style().text_color(theme_color("text_primary")).min_width(px(0));
        if ("numeric" in item && item.numeric) || ("identifier" in item && item.identifier) {
            value_style = value_style.font_feature("tnum", 1).font_feature("zero", 1);
        }
        let value = if type_of(item.value) == "string" {
            text(item.value).with_style(ctx.component_style("value", value_style))
        } else { item.value.with_style(ctx.component_style("value", value_style)) };
        let pair = if stacked { column([term, value]) } else { row([term, value]) };
        let item_style = style().min_height(theme_length("metrics.row"));
        item_style = if stacked { item_style.justify_center() }
            else { item_style.items_center().gap(theme_length("space.related")) };
        rows.push(pair.with_style(ctx.component_style("item", item_style))
            .accessibility_role("group").accessibility_label(item.label));
    }
    column(rows).with_style(ctx.component_style("root", style().flex_col().typography("body")))
        .accessibility_role("list").accessibility_label(props.label)
}
