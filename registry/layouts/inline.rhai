/* gpui-rhai
{
  "id": "layouts/inline",
  "export": "Inline",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "space.unit", "space.related", "space.group", "space.section", "spacing.xxs", "spacing.xs",
    "spacing.sm", "spacing.md", "spacing.lg", "spacing.xl"
  ],
  "environment": ["size"]
}
*/
// Inline places children in a row with one spacing relationship and one control size.
//
// Props: children, gap (default related), align (start|center|end, default center),
//        justify (start|between|end), wrap, size, label.
// Events/state: none. Parts: root.
// Example: Inline(#{ size: "sm", children: [button_a, button_b] })
//
// `size` sets the environment for every control inside, so a row shares one height.

define_component(#{
    metadata: #{
        id: "layouts/inline", "export": "Inline", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "space.unit", "space.related", "space.group", "space.section", "spacing.xxs",
            "spacing.xs", "spacing.sm", "spacing.md", "spacing.lg", "spacing.xl"
        ],
        environment: ["size"],
    },
    schema: #{
        props: #{
            children: #{ schema: #{ type: "array", max_items: 256, items: #{ type: "node" } },
                required: true, sensitive: false },
            gap: #{ schema: #{ type: "string", allowed: ["none", "unit", "related", "group", "section", "xxs", "xs", "sm", "md", "lg", "xl"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "related" } },
            align: #{ schema: #{ type: "string", allowed: ["start", "center", "end"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "center" } },
            justify: #{ schema: #{ type: "string", allowed: ["start", "between", "end"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "start" } },
            wrap: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false },
            label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ children: #{ required: true, multiple: true } },
        parts: ["root"],
    },
    render: Fn("render_Inline")
});

// Relationship names map to the space.* aliases, scale names to spacing.*.
fn gap_length(name) {
    if name == "none" { px(0) }
    else if name == "unit" || name == "related" || name == "group" || name == "section" {
        theme_length(`space.${name}`)
    } else { theme_spacing(name) }
}

fn Inline(props) { render_component("layouts/inline", props) }
fn render_Inline(ctx, props) {
    let align = if "align" in props { props.align } else { "center" };
    let justify = if "justify" in props { props.justify } else { "start" };
    let root = style().flex_row().gap(gap_length(if "gap" in props { props.gap } else { "related" }));
    root = if align == "start" { root.items_start() } else if align == "end" { root.items_end() }
        else { root.items_center() };
    root = if justify == "between" { root.justify_between() } else if justify == "end" { root.justify_end() }
        else { root };
    if "wrap" in props && props.wrap { root = root.flex_wrap(); }
    let node = row(props.children).with_style(ctx.component_style("root", root));
    if "size" in props && props.size != () { node = node.env(#{ size: props.size }); }
    if "label" in props && props.label != () {
        node.accessibility_role("group").accessibility_label(props.label)
    } else { node }
}
