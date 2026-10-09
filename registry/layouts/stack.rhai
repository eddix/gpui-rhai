/* gpui-rhai
{
  "id": "layouts/stack",
  "export": "Stack",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "space.unit", "space.related", "space.group", "space.section", "spacing.xxs", "spacing.xs",
    "spacing.sm", "spacing.md", "spacing.lg", "spacing.xl"
  ],
  "environment": []
}
*/
// Stack places children in a column with one spacing relationship between them.
//
// Example: Stack(#{ gap: "group", children: [section_a, section_b] })
//
// Layouts are visually neutral: no padding, color or frame. Prefer relationship names so
// the intent stays readable and the nesting audit can explain itself.

define_component(#{
    metadata: #{
        id: "layouts/stack", "export": "Stack", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "space.unit", "space.related", "space.group", "space.section", "spacing.xxs",
            "spacing.xs", "spacing.sm", "spacing.md", "spacing.lg", "spacing.xl"
        ],
        environment: [],
    },
    schema: #{
        props: #{
            children: #{ schema: #{ type: "array", max_items: 512, items: #{ type: "node" } },
                required: true, sensitive: false, doc: "Nodes placed in one column, in order." },
            gap: #{ schema: #{ type: "string", allowed: ["none", "unit", "related", "group", "section", "xxs", "xs", "sm", "md", "lg", "xl"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "related" },
                doc: "Space between children: a relationship (`space.*`), a scale step (`spacing.*`) or `none`." },
            align: #{ schema: #{ type: "string", allowed: ["stretch", "start", "center", "end"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "stretch" },
                doc: "Horizontal alignment of the children; `stretch` makes each as wide as the stack." },
            fill: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Grows the stack to the remaining height of its parent." },
            label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                doc: "When set, exposes the column as an accessible group with this name." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ children: #{ required: true, multiple: true, doc: "The nodes in the column." } },
        parts: ["root"],
    },
    render: Fn("render_Stack")
});

// Relationship names map to the space.* aliases, scale names to spacing.*.
fn gap_length(name) {
    if name == "none" { px(0) }
    else if name == "unit" || name == "related" || name == "group" || name == "section" {
        theme_length(`space.${name}`)
    } else { theme_spacing(name) }
}

fn Stack(props) { render_component("layouts/stack", props) }
fn render_Stack(ctx, props) {
    let align = if "align" in props { props.align } else { "stretch" };
    let root = style().flex_col().gap(gap_length(if "gap" in props { props.gap } else { "related" }));
    root = if align == "start" { root.items_start() }
        else if align == "center" { root.items_center() }
        else if align == "end" { root.items_end() }
        else { root };
    if "fill" in props && props.fill { root = root.flex_grow().min_height(px(0)); }
    let node = column(props.children).with_style(ctx.component_style("root", root));
    if "label" in props && props.label != () {
        node.accessibility_role("group").accessibility_label(props.label)
    } else { node }
}
