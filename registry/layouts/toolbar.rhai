/* gpui-rhai
{
  "id": "layouts/toolbar",
  "export": "Toolbar",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["space.related", "space.group", "metrics.control"],
  "environment": ["size"]
}
*/
// Toolbar arranges a region's controls by role: context and filters at the start,
// secondary actions and the one primary action at the end.
//
// Props: label (required), context, filters, actions, primary, size.
// Events/state: none. Parts: root, start, end.
// Example: Toolbar(#{ label: "Hosts", filters: [filter_input], actions: [export_button],
//              primary: deploy_button })
//
// Items in a group are `related`; the two groups are at least `group` apart. The single
// `primary` slot encodes "one solid action per group". `size` reaches every control.

define_component(#{
    metadata: #{
        id: "layouts/toolbar", "export": "Toolbar", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: ["space.related", "space.group", "metrics.control"],
        environment: ["size"],
    },
    schema: #{
        props: #{
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            context: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            filters: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            actions: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            primary: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{
            context: #{ required: false, multiple: true }, filters: #{ required: false, multiple: true },
            actions: #{ required: false, multiple: true }, primary: #{ required: false, multiple: false },
        },
        parts: ["root", "start", "end"],
    },
    render: Fn("render_Toolbar")
});

fn Toolbar(props) { render_component("layouts/toolbar", props) }
fn render_Toolbar(ctx, props) {
    let start = [];
    for node in if "context" in props { props.context } else { [] } { start.push(node); }
    for node in if "filters" in props { props.filters } else { [] } { start.push(node); }
    let end = [];
    for node in if "actions" in props { props.actions } else { [] } { end.push(node); }
    if "primary" in props && props.primary != () { end.push(props.primary); }
    // When the region is too narrow the end group wraps below the start group rather
    // than overlapping it.
    let group = style().flex_row().flex_wrap().items_center().gap(theme_length("space.related"));
    let bar = style().flex_row().flex_wrap().items_center().justify_between();
    bar = bar.gap(theme_length("space.group")).min_height(theme_length("metrics.control"));
    let root = row([
        row(start).with_style(ctx.component_style("start", group)),
        row(end).with_style(ctx.component_style("end", group)),
    ]).with_style(ctx.component_style("root", bar))
        .accessibility_role("toolbar")
        .accessibility_label(props.label);
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
