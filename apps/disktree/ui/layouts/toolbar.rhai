/* gpui-rhai
{
  "id": "layouts/toolbar",
  "export": "Toolbar",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["space.related", "space.group", "metrics.control", "metrics.label_column"],
  "environment": ["size"]
}
*/
// Toolbar arranges a region's controls by role: context and filters at the start,
// secondary actions and the one primary action at the end.
//
// Example: Toolbar(#{ label: "Hosts", filters: [filter_input], actions: [export_button],
//              primary: deploy_button })
//
// Items in a group are `related`; the two groups are at least `group` apart. The single
// `primary` slot encodes "one solid action per group". `size` reaches every control.
// `fill` is one node that takes the width the groups leave, between them: a search field,
// a query input, a path bar. It is at least `metrics.label_column` wide; the bar wraps only
// when the groups and that minimum do not fit.

define_component(#{
    metadata: #{
        id: "layouts/toolbar", "export": "Toolbar", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: ["space.related", "space.group", "metrics.control", "metrics.label_column"],
        environment: ["size"],
    },
    schema: #{
        props: #{
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the toolbar." },
            context: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "What the bar acts on (tags, a breadcrumb), first in the start group, `related` apart." },
            filters: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Controls that narrow the content, after `context` in the start group." },
            actions: #{ schema: #{ type: "array", max_items: 16, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Secondary actions in the end group, before `primary`." },
            primary: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "The one solid action of the bar, last in the end group." },
            fill: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "One field that takes the width between the groups, at least `metrics.label_column` wide." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false,
                doc: "Control size passed to every control in the bar, so they share one height." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{
            context: #{ required: false, multiple: true, doc: "What the bar acts on, at the start." },
            filters: #{ required: false, multiple: true, doc: "Controls that narrow the content." },
            actions: #{ required: false, multiple: true, doc: "Secondary actions at the end." },
            primary: #{ required: false, multiple: false, doc: "The one solid action, last." },
            fill: #{ required: false, multiple: false, doc: "The field that takes the free width." },
        },
        parts: ["root", "start", "fill", "end"],
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
    // than overlapping it. A group may shrink below its content (min_width 0), so a
    // tab list or field in it scrolls or shrinks inside the bar instead of running past it.
    let group = style().flex_row().flex_wrap().items_center().gap(theme_length("space.related"))
        .min_width(px(0));
    let bar = style().flex_row().flex_wrap().items_center().justify_between();
    bar = bar.gap(theme_length("space.group")).min_height(theme_length("metrics.control"));
    let children = [];
    if "fill" in props && props.fill != () {
        // The fill takes the free width between the groups; empty groups add no gap.
        let minimum = theme_length("metrics.label_column");
        if start.len > 0 { children.push(row(start).with_style(ctx.component_style("start", group))); }
        children.push(box([props.fill.with_style(style().width(relative(1.0)))])
            .with_style(ctx.component_style("fill", style().flex_row().items_center()
                .flex_grow().flex_basis(minimum).min_width(minimum))));
        if end.len > 0 { children.push(row(end).with_style(ctx.component_style("end", group))); }
    } else {
        // An empty start group stays so the end group keeps to the end edge; an empty end
        // group is left out, so it cannot wrap onto a line of its own.
        children.push(row(start).with_style(ctx.component_style("start", group)));
        if end.len > 0 { children.push(row(end).with_style(ctx.component_style("end", group))); }
    }
    let root = row(children).with_style(ctx.component_style("root", bar))
        .accessibility_role("toolbar")
        .accessibility_label(props.label);
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
