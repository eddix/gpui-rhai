/* gpui-rhai
{
  "id": "patterns/description_list",
  "export": "DescriptionList",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "metrics.row", "metrics.label_column", "space.related", "space.unit", "space.group",
    "text_primary", "text_muted", "typography.body"
  ],
  "environment": ["density"]
}
*/
// DescriptionList shows facts as label and value pairs on one label column.
//
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
            "metrics.row", "metrics.label_column", "space.related", "space.unit", "space.group",
            "text_primary", "text_muted", "typography.body"
        ],
        environment: ["density"],
    },
    schema: #{
        props: #{
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the list." },
            items: #{ schema: #{ type: "array", max_items: 256, items: #{
                type: "object", allow_unknown: false, fields: #{
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "The term, in the muted color; also the accessible name of its pair." },
                    value: #{ schema: #{ type: "one_of", variants: [#{ type: "string" }, #{ type: "node" }] },
                        required: true, sensitive: false,
                        doc: "The fact: text in the primary color, or a node that gets the `value` part style." },
                    numeric: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Sets the value in tabular figures with the slashed zero." },
                    identifier: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Treats the value as an atom: tabular figures, slashed zero, one line truncated with an ellipsis." },
                },
            } }, required: true, sensitive: false, doc: "Term and value pairs, in display order." },
            label_width: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false,
                doc: "Width of the term column in the `columns` layout; `()` uses the `metrics.label_column` token." },
            layout: #{ schema: #{ type: "string", allowed: ["columns", "stacked"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "columns" },
                doc: "`columns` puts each term beside its value on one row; `stacked` puts the term above the value." },
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
        // Identifiers are atoms: they truncate rather than break.
        if "identifier" in item && item.identifier {
            value_style = value_style.whitespace_nowrap().text_ellipsis();
        }
        let value = if type_of(item.value) == "string" {
            text(item.value).with_style(ctx.component_style("value", value_style))
        } else { item.value.with_style(ctx.component_style("value", value_style)) };
        let pair = if stacked { column([term, value]) } else { row([term, value]) };
        // Columns keep the metrics.row rhythm; stacked pairs group by proximity instead:
        // a term sits a unit above its value, and each two-line pair is a group, so
        // pairs stand a group step apart (a related step is too close to read).
        let item_style = if stacked { style().gap(theme_length("space.unit")) }
            else { style().min_height(theme_length("metrics.row")).items_center()
                .gap(theme_length("space.related")) };
        rows.push(pair.with_style(ctx.component_style("item", item_style))
            .accessibility_role("group").accessibility_label(item.label));
    }
    let root = style().flex_col().typography("body");
    if stacked { root = root.gap(theme_length("space.group")); }
    column(rows).with_style(ctx.component_style("root", root))
        .accessibility_role("list").accessibility_label(props.label)
}
