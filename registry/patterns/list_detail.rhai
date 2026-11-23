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
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the group holding both panes." },
            list: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "The list pane; it keeps a fixed width, or a fixed height when stacked." },
            detail: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "The selected item's detail; it fills the remaining space." },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "horizontal" },
                doc: "`horizontal` puts the list beside the detail; `vertical` stacks it above." },
            list_size: #{ schema: #{ type: "optional", value: #{ type: "length" } }, required: false, sensitive: false,
                doc: "List width, or height when vertical; defaults to 320px wide or 8 `metrics.row` tall." },
        },
        state: #{ fields: #{} }, events: #{},
        slots: #{
            list: #{ required: true, multiple: false, doc: "The list pane." },
            detail: #{ required: true, multiple: false, doc: "The detail pane that fills." },
        },
        parts: ["root", "list", "detail"],
    },
    render: Fn("render_ListDetail")
});

fn ListDetail(props) { render_component("patterns/list_detail", props) }
// The catalog surfaces share their matching and focus order, without coupling
// cursor movement to the application's accepted selection.
fn catalog_opt(map,key){if key in map{map[key]}else{()}}
fn catalog_query(query){let key=query.to_lower();key.trim();key}
fn catalog_match(query,fields){
    if query==""{return true;}
    for field in fields{if field!=()&&field.to_lower().contains(query){return true;}}
    false
}
fn catalog_neighbor(order,current,delta,wrap){
    if order.len==0{return "";}
    if delta<= -256{return order[0];}
    if delta>=256{return order[order.len-1];}
    let index=order.index_of(current);
    if index<0{return if delta<0{order[order.len-1]}else{order[0]};}
    if wrap{order[(index+order.len+delta)%order.len]}
    else{order[min(max(index+delta,0),order.len-1)]}
}
fn catalog_search_icon(){
    svg("<svg viewBox='0 0 20 20' fill='none' stroke='currentColor' stroke-width='1.65' stroke-linecap='round'><circle cx='8.4' cy='8.4' r='5.1'/><path d='m12 12 4.3 4.3'/></svg>")
        .accessibility_hidden(true)
}
fn catalog_option(id,label,selected,focusable,content,paint){
    row(content).with_key(id).accessibility_role("option").accessibility_label(label)
        .accessibility_selected(selected).tab_stop(focusable).with_style(paint)
        .paint_transition(155,cubic_bezier(0.22,1.0,0.36,1.0))
}
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
