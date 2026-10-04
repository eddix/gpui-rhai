/* gpui-rhai
{
  "id": "patterns/list_detail",
  "export": "ListDetail",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["border", "metrics.row"],
  "environment": []
}
*/
// ListDetail shows a list and the selected item's detail, side by side or stacked.
//
// Props: label (required), list (required), detail (required), orientation
//        (horizontal|vertical), list_size.
// Events/state: none. Parts: root, list, detail.
// Example: ListDetail(#{ label: "Hosts", list: host_list, detail: host_detail })
//
// The list keeps a fixed width (or height when stacked); the detail fills. One hairline
// separates them.

define_component(#{
    metadata: #{
        id: "patterns/list_detail", "export": "ListDetail", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: ["border", "metrics.row"],
        environment: [],
    },
    schema: #{
        props: #{
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            list: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            detail: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "horizontal" } },
            list_size: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{ list: #{ required: true, multiple: false }, detail: #{ required: true, multiple: false } },
        parts: ["root", "list", "detail"],
    },
    render: Fn("render_ListDetail")
});

fn ListDetail(props) { render_component("patterns/list_detail", props) }
fn render_ListDetail(ctx, props) {
    let vertical = "orientation" in props && props.orientation == "vertical";
    let size = if "list_size" in props && props.list_size != () { props.list_size }
        else if vertical { theme_length("metrics.row") * 8 } else { px(320) };
    let list_style = style().flex_col().flex_shrink(false).min_width(px(0)).min_height(px(0))
        .border_color(theme_color("border"));
    list_style = if vertical { list_style.height(size).border_bottom(px(1)) }
        else { list_style.width(size).border_end(px(1)) };
    let panes = [
        column([props.list]).with_style(ctx.component_style("list", list_style)),
        column([props.detail]).with_style(ctx.component_style("detail", style()
            .flex_col().flex_grow().min_width(px(0)).min_height(px(0)))),
    ];
    let root = if vertical { column(panes) } else { row(panes) };
    root.with_style(ctx.component_style("root", style().flex_grow().min_width(px(0)).min_height(px(0))))
        .accessibility_role("group").accessibility_label(props.label)
}
