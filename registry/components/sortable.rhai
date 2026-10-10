/* gpui-rhai
{
  "id": "components/sortable",
  "export": "Sortable",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "border", "surface", "focus_ring", "text_muted", "accent", "metrics.row", "metrics.control",
    "metrics.inset", "typography.body", "spacing.xs"
  ],
  "environment": ["density"]
}
*/
// Controlled keyed ordering with native pointer targets and keyboard moves.
// State: order is caller-owned; native state contains only one transient drag.
// Keyboard: focus a handle, then press Option/Alt with an arrow, Home or End (all three need
// Option/Alt). A horizontal row follows the reading direction, so in RTL Option/Alt+Left
// moves a row towards the end.
// Example: Sortable(#{ key:"queue", label:"Queue", items:items, on_reorder:Fn("reordered") })

define_component(#{
    metadata: #{ id:"components/sortable", "export":"Sortable", version: "0.2.0",
        runtime_api:#{min_inclusive: 3,max_exclusive: 4}, dependencies:[], capabilities:#{},
        tokens: [
            "border", "surface", "focus_ring", "text_muted", "accent", "metrics.row",
            "metrics.control", "metrics.inset", "typography.body", "spacing.xs"
        ],
        environment: ["density"] },
    schema: #{ props: #{
        key: #{ schema:#{type:"string"}, required:true, sensitive:false },
        label: #{ schema:#{type:"string"}, required:true, sensitive:false,
            doc:"Accessible name of the list." },
        items: #{ schema:#{type:"optional",value:#{type:"array",max_items:512,items:#{
            type:"object",allow_unknown:false,fields:#{
                key:#{schema:#{type:"string"},required:true,sensitive:false,
                    doc:"Identity of the row, unique in the list; `reorder` reports it."},
                label:#{schema:#{type:"string"},required:true,sensitive:false,
                    doc:"Accessible name of the row; its handle reads `Reorder <label>`."},
                content:#{schema:#{type:"node"},required:true,sensitive:false,
                    doc:"Node shown beside the drag handle."},
                disabled:#{schema:#{type:"bool"},required:false,sensitive:false,
                    "default":#{type:"bool",value:false},
                    doc:"Keeps the row from being dragged or moved by key; rows can still drop beside it."},
            }
        }}}, required:false, sensitive:false,
            doc:"Rows in their current order, owned by the caller; give exactly one of `items` and `virtual_data`." },
        virtual_data: #{ schema:#{type:"optional",value:#{type:"one_of",variants:[
            #{type:"array",max_items:10000,items:#{type:"map",values:#{type:"ui_value"}}},
            #{type:"collection"}
        ]}}, required:false, sensitive:false,
            doc:"Virtualized rows: maps with a string `key`, a `label_key` field and an optional `disabled`; each row shows its label as text and drags from anywhere on it." },
        label_key: #{schema:#{type:"string"},required:false,sensitive:false,
            "default":#{type:"string",value:"label"},
            doc:"Field of a `virtual_data` row shown as its text and used as its accessible name."},
        estimated_row_height: #{schema:#{type:"number",exclusive_min:0.0},required:false,sensitive:false,
            "default":#{type:"float",value:40.0},
            doc:"Row height the virtual list assumes before rows are measured, in logical pixels."},
        height: #{schema:#{type:"optional",value:#{type:"number",exclusive_min:0.0}},required:false,sensitive:false,
            doc:"Viewport height of the virtual list, in logical pixels; give exactly one of `height` and `fill_height`."},
        fill_height: #{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},
            doc:"Lets the virtual list fill its parent's height; give exactly one of `height` and `fill_height`."},
        direction: #{ schema:#{type:"string",allowed:["vertical","horizontal","grid"]},
            required:false,sensitive:false,"default":#{type:"string",value:"vertical"},
            doc:"Lays rows out as a column, a scrolling row or a grid of `grid_columns`; `virtual_data` needs `vertical`." },
        grid_columns: #{ schema:#{type:"integer",min:1,max:64},required:false,sensitive:false,
            "default":#{type:"integer",value:3},
            doc:"Number of columns when `direction` is `grid`." },
        gap: #{ schema:#{type:"optional",value:#{type:"length"}},required:false,sensitive:false,
            doc:"Space between rows, `spacing.xs` when unset; ignored with `virtual_data`." },
        threshold: #{ schema:#{type:"number",min:0.0,max:64.0},required:false,sensitive:false,
            "default":#{type:"float",value:4.0},
            doc:"Pointer travel in logical pixels before a press becomes a drag." },
        disabled: #{ schema:#{type:"bool"},required:false,sensitive:false,
            "default":#{type:"bool",value:false},
            doc:"Disables dragging and keyboard moves for every row." },
        on_reorder: #{ schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,
            doc:"Called with the `reorder` payload when a row is dropped or moved with Option/Alt and an arrow, Home or End." },
    }, state:#{fields:#{}}, events:#{ reorder:#{payload:#{type:"object",allow_unknown:false,fields:#{
        source_key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Key of the row that moves."},
        anchor_key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Key of the row it lands next to."},
        placement:#{schema:#{type:"string",allowed:["before","after"]},required:true,sensitive:false,
            doc:"Whether the moving row goes before or after the anchor row."},
        x:#{schema:#{type:"number"},required:true,sensitive:false,
            doc:"Pointer x at the drop in window coordinates, in logical pixels; 0 for keyboard moves."},
        y:#{schema:#{type:"number"},required:true,sensitive:false,
            doc:"Pointer y at the drop in window coordinates, in logical pixels; 0 for keyboard moves."},
    }},
        doc:"Emitted when a row is dropped or moved by key; the payload names the move, and the caller reorders its rows."} },
        slots:#{}, parts:["root","item","item_disabled","handle","handle_icon","content"] },
    render:Fn("render_Sortable")
});

fn Sortable(props) { render_component("components/sortable", props) }
fn sortable_commit(ctx, value) { ctx.emit("reorder", value); }
fn sortable_disabled(item) { "disabled" in item && item.disabled }

fn sortable_item_node(ctx,list_id,collection_id,item,previous_key,next_key,first_key,last_key,direction,threshold,all_disabled){
    let disabled=all_disabled||sortable_disabled(item);let ref_key=`${list_id}-item-${item.key}`;
    let item_ref=element_ref(ref_key);let native=#{key:`${list_id}-${item.key}-sort`,list_id:list_id,collection_id:collection_id,
        item_key:item.key,previous_key:previous_key,next_key:next_key,first_key:first_key,last_key:last_key,
        direction:direction,threshold:threshold,disabled:disabled,item_ref:item_ref,on_reorder:Fn("sortable_commit")};
    let interaction=gpui_rhai::SortableItemPrimitive(native).with_key(`${list_id}-${item.key}-interaction`)
        .with_style(style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
            .border(px(2)).border_color(rgba(0x00000000)).focus(style().border_color(theme_color("focus_ring"))))
        .disabled(disabled).tab_stop(!disabled).accessibility_role("button").accessibility_label(`Reorder ${item.label}`);
    let handle_style=style().relative().width(theme_length("metrics.control")).min_height(theme_length("metrics.row"))
        .flex_shrink(false).items_center().justify_center().cursor_move();
    let handle=box([text("≡").with_style(ctx.component_style("handle_icon",style().typography("body")
        .text_color(theme_color("text_muted")))),interaction]).with_key(`${list_id}-${item.key}-handle`)
        .with_style(ctx.component_style("handle",handle_style)).disabled(disabled);
    // Sortable items are list rows separated by hairlines.
    let item_style=style().min_width(px(0)).min_height(theme_length("metrics.row")).items_center()
        .border_bottom(px(1)).border_color(theme_color("border")).background(theme_color("surface"));let resolved=ctx.component_style("item",item_style);
    if disabled{resolved=ctx.component_style("item_disabled",resolved.opacity(0.64));}
    row([handle,item.content.with_key(`${list_id}-${item.key}-content`)
        .with_style(ctx.component_style("content",style().min_width(px(0)).flex_grow()))])
        .with_key(ref_key).with_ref(item_ref).with_style(resolved)
        .accessibility_role("listitem").accessibility_label(item.label)
}

fn sortable_virtual_row(list_id,collection_id,label_key,direction,threshold,disabled,ctx,payload){
    let label=payload.item[label_key];
    let item_disabled=disabled||(if "disabled" in payload.item{payload.item.disabled}else{false});
    let native=#{key:`${list_id}-${payload.key}-sort`,list_id:list_id,collection_id:collection_id,
        item_key:payload.key,source_index:payload.index,source_item:payload.item,
        previous_key:payload.previous_key,next_key:payload.next_key,first_key:payload.first_key,
        last_key:payload.last_key,direction:direction,threshold:threshold,disabled:item_disabled,
        on_reorder:Fn("sortable_commit")};
    let interaction=gpui_rhai::SortableItemPrimitive(native).with_key(`${list_id}-${payload.key}-interaction`)
        .with_style(style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
            .border(px(2)).border_color(rgba(0x00000000)).focus(style().border_color(theme_color("focus_ring"))))
        .disabled(item_disabled).tab_stop(!item_disabled).accessibility_role("button")
        .accessibility_label(`Reorder ${label}`);
    let resolved=ctx.component_style("item",style().relative().height(theme_length("metrics.row")).min_width(px(0))
        .items_center().padding_end(theme_length("metrics.inset")).border_bottom(px(1)).border_color(theme_color("border"))
        .background(theme_color("surface")));
    if item_disabled{resolved=ctx.component_style("item_disabled",resolved.opacity(0.64));}
    row([text("≡").with_style(ctx.component_style("handle_icon",style().width(theme_length("metrics.control")).text_center()
        .text_color(theme_color("text_muted")))),text(label).with_style(ctx.component_style("content",style().flex_grow())),
        interaction]).with_key(`${list_id}-item-${payload.key}`).with_style(resolved)
        .accessibility_role("listitem").accessibility_label(label)
}

fn render_Sortable(ctx, props) {
    let list_ref=element_ref(`${props.key}-virtual`);
    let collection_id=list_ref.scope;
    let has_items="items" in props&&props.items!=();let has_virtual="virtual_data" in props&&props.virtual_data!=();
    if has_items==has_virtual{throw "Sortable requires exactly one of items or virtual_data";}
    if has_virtual{
        if props.direction!="vertical"{throw "virtual Sortable currently requires vertical direction";}
        if ("height" in props&&props.height!=())==props.fill_height{
            throw "virtual Sortable requires exactly one of height or fill_height";
        }
        let config=#{key:`${props.key}-virtual`,label:props.label,data:props.virtual_data,
            estimated_height:props.estimated_row_height};
        if "height" in props&&props.height!=(){config.height=props.height;}else{config.fill_height=true;}
        return virtual_collection(config,Fn("sortable_virtual_row").curry(props.key,collection_id,props.label_key,
            props.direction,props.threshold,props.disabled)).with_key(props.key).with_ref(list_ref)
            .with_style(ctx.component_style("root",style().gap(theme_spacing("xs"))))
            .accessibility_role("list").accessibility_label(props.label);
    }
    let count = props.items.len;
    let nodes = [];
    let seen = #{};
    if count > 0 {
        let first_key = props.items[0].key;
        let last_key = props.items[count - 1].key;
        for index in 0..count {
            let item = props.items[index];
            if item.key in seen { throw `Sortable item key '${item.key}' is duplicated`; }
            seen[item.key] = true;
            let previous_key = if index > 0 { props.items[index - 1].key } else { () };
            let next_key = if index + 1 < count { props.items[index + 1].key } else { () };
            nodes.push(sortable_item_node(ctx,props.key,collection_id,item,previous_key,next_key,first_key,last_key,
                props.direction,props.threshold,props.disabled));
        }
    }
    let gap = if "gap" in props && props.gap != () { props.gap } else { theme_spacing("xs") };
    let root_style = style().gap(gap).min_width(px(0));
    let root = if props.direction == "horizontal" {
        row(nodes).with_style(root_style.overflow_x_scroll())
    } else if props.direction == "grid" {
        box(nodes).with_style(root_style.grid().grid_cols(props.grid_columns))
    } else {
        column(nodes).with_style(root_style)
    };
    root.with_key(props.key).with_ref(list_ref).with_style(ctx.component_style("root", style()))
        .accessibility_role("list").accessibility_label(props.label)
}
