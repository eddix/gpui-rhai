/* gpui-rhai
{
  "id": "components/sortable",
  "export": "Sortable",
  "version": "0.1.8",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled keyed ordering with native pointer targets and keyboard moves.
// Props: key, label, items[{key,label,content,disabled?}], direction,
// grid_columns, gap, threshold, disabled, on_reorder.
// Events: reorder({source_key,anchor_key,placement,x,y}).
// State: order is caller-owned; native state contains only one transient drag.
// Parts: root, item, item_disabled, handle, handle_icon, content.
// Keyboard: focus a handle, then Option/Alt+Arrow, Home, or End.
// Example: Sortable(#{ key:"queue", label:"Queue", items:items, on_reorder:Fn("reordered") })

define_component(#{
    metadata: #{ id:"components/sortable", "export":"Sortable", version:"0.1.8",
        runtime_api:#{min_inclusive:2,max_exclusive:3}, dependencies:[], capabilities:#{} },
    schema: #{ props: #{
        key: #{ schema:#{type:"string"}, required:true, sensitive:false },
        label: #{ schema:#{type:"string"}, required:true, sensitive:false },
        items: #{ schema:#{type:"array",max_items:512,items:#{
            type:"object",allow_unknown:false,fields:#{
                key:#{schema:#{type:"string"},required:true,sensitive:false},
                label:#{schema:#{type:"string"},required:true,sensitive:false},
                content:#{schema:#{type:"node"},required:true,sensitive:false},
                disabled:#{schema:#{type:"bool"},required:false,sensitive:false,
                    "default":#{type:"bool",value:false}},
            }
        }}, required:true, sensitive:false },
        direction: #{ schema:#{type:"string",allowed:["vertical","horizontal","grid"]},
            required:false,sensitive:false,"default":#{type:"string",value:"vertical"} },
        grid_columns: #{ schema:#{type:"integer",min:1,max:64},required:false,sensitive:false,
            "default":#{type:"integer",value:3} },
        gap: #{ schema:#{type:"optional",value:#{type:"length"}},required:false,sensitive:false },
        threshold: #{ schema:#{type:"number",min:0.0,max:64.0},required:false,sensitive:false,
            "default":#{type:"float",value:4.0} },
        disabled: #{ schema:#{type:"bool"},required:false,sensitive:false,
            "default":#{type:"bool",value:false} },
        on_reorder: #{ schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false },
    }, state:#{fields:#{}}, events:#{ reorder:#{payload:#{type:"object",allow_unknown:false,fields:#{
        source_key:#{schema:#{type:"string"},required:true,sensitive:false},
        anchor_key:#{schema:#{type:"string"},required:true,sensitive:false},
        placement:#{schema:#{type:"string",allowed:["before","after"]},required:true,sensitive:false},
        x:#{schema:#{type:"number"},required:true,sensitive:false},
        y:#{schema:#{type:"number"},required:true,sensitive:false},
    }}} }, slots:#{}, parts:["root","item","item_disabled","handle","handle_icon","content"] },
    render:Fn("render_Sortable")
});

fn Sortable(props) { render_component("components/sortable", props) }
fn sortable_commit(ctx, value) { ctx.emit("reorder", value); }
fn sortable_disabled(item) { "disabled" in item && item.disabled }

fn render_Sortable(ctx, props) {
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
            let disabled = props.disabled || sortable_disabled(item);
            let previous_key = if index > 0 { props.items[index - 1].key } else { () };
            let next_key = if index + 1 < count { props.items[index + 1].key } else { () };
            let ref_key = `${props.key}-item-${item.key}`;
            let item_ref = element_ref(ref_key);
            let native = #{ key:`${props.key}-${item.key}-sort`, list_id:props.key,
                item_key:item.key, previous_key:previous_key, next_key:next_key,
                first_key:first_key, last_key:last_key, direction:props.direction,
                threshold:props.threshold, disabled:disabled, item_ref:item_ref,
                on_reorder:if "on_reorder" in props && props.on_reorder != () {
                    Fn("sortable_commit")
                } else { () }
            };
            let interaction = gpui_rhai::SortableItemPrimitive(native)
                .with_key(`${props.key}-${item.key}-interaction`)
                .with_style(style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0)))
                .disabled(disabled).tab_stop(!disabled)
                .accessibility_role("button")
                .accessibility_label(`Reorder ${item.label}`);
            let handle_style = style().relative().width(px(28)).min_height(px(28))
                .flex_shrink(false).items_center().justify_center().cursor_move();
            handle_style = handle_style.focus(style().background(theme_color("selection")));
            let handle = box([
                text("≡").with_style(ctx.component_style("handle_icon", style()
                    .typography("body").text_color(theme_color("text_muted")))),
                interaction
            ]).with_key(`${props.key}-${item.key}-handle`)
                .with_style(ctx.component_style("handle", handle_style))
                .disabled(disabled);
            let item_style = style().min_width(px(0)).items_center()
                .border(px(1)).border_color(theme_color("border"))
                .background(theme_color("surface"));
            let resolved = ctx.component_style("item", item_style);
            if disabled { resolved = ctx.component_style("item_disabled", resolved.opacity(0.64)); }
            nodes.push(row([
                handle,
                item.content.with_key(`${props.key}-${item.key}-content`)
                    .with_style(ctx.component_style("content", style().min_width(px(0)).flex_grow()))
            ]).with_key(ref_key).with_ref(item_ref).with_style(resolved)
                .accessibility_role("listitem").accessibility_label(item.label));
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
    root.with_key(props.key).with_style(ctx.component_style("root", style()))
        .accessibility_role("list").accessibility_label(props.label)
}
