/* gpui-rhai
{
  "id": "components/tree",
  "export": "Tree",
  "version": "0.1.8",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/chevron_down.svg", "icons/chevron_right.svg"]
}
*/
// Controlled virtualized hierarchical outline over the shared Rust projection.
// Props: key, label, items[{key,parent,label,disabled?,loading?}], expanded,
// selected_keys, active_key, selection_mode, height/fill_height, on_*.
// Events: expanded_change(array), selection_change(array), active_change(optional string).
// Parts: root, row, row_active, row_selected, disclosure, label, loading.
// Example: Tree(#{key:"files",label:"Files",items:items,expanded:[],selected_keys:[],height:320})

define_component(#{
    metadata:#{id:"components/tree","export":"Tree",version:"0.1.8",
        runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{},
        assets:["icons/chevron_down.svg","icons/chevron_right.svg"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        label:#{schema:#{type:"string"},required:true,sensitive:false},
        items:#{schema:#{type:"array",max_items:10000,items:#{type:"object",allow_unknown:false,fields:#{
            key:#{schema:#{type:"string"},required:true,sensitive:false},
            parent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false},
            label:#{schema:#{type:"string"},required:true,sensitive:false},
            disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false}},
            loading:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false}},
        }}},required:true,sensitive:false},
        expanded:#{schema:#{type:"array",max_items:10000,items:#{type:"string"}},required:true,sensitive:false},
        selected_keys:#{schema:#{type:"array",max_items:10000,items:#{type:"string"}},required:true,sensitive:false},
        active_key:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false},
        selection_mode:#{schema:#{type:"string",allowed:["none","single","multiple"]},required:false,
            sensitive:false,"default":#{type:"string",value:"single"}},
        estimated_row_height:#{schema:#{type:"number",exclusive_min:0.0},required:false,sensitive:false,
            "default":#{type:"float",value:28.0}},
        height:#{schema:#{type:"optional",value:#{type:"number",exclusive_min:0.0}},required:false,sensitive:false},
        fill_height:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false}},
        disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false}},
        on_expanded_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false},
        on_selection_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false},
        on_active_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false},
    },state:#{fields:#{}},events:#{
        expanded_change:#{payload:#{type:"array",max_items:10000,items:#{type:"string"}}},
        selection_change:#{payload:#{type:"array",max_items:10000,items:#{type:"string"}}},
        active_change:#{payload:#{type:"optional",value:#{type:"string"}}},
    },slots:#{},parts:["root","row","row_active","row_selected","disclosure","label","loading"]},
    render:Fn("render_Tree")
});

fn Tree(props){render_component("components/tree",props)}
fn tree_active(ctx,key){ctx.emit("active_change",key);handled()}
fn tree_toggle(expanded,ctx,key){
    let next=[];for value in expanded{if value!=key{next.push(value);}}
    if !(key in expanded){next.push(key);}ctx.emit("expanded_change",next);handled()
}
fn tree_select(mode,selected,ctx,row){
    ctx.emit("active_change",row.key);
    if mode=="none"{return handled();}
    if mode=="single"{ctx.emit("selection_change",[row.key]);return handled();}
    let next=[];for key in selected{if key!=row.key{next.push(key);}}
    if !(row.key in selected){next.push(row.key);}ctx.emit("selection_change",next);handled()
}
fn tree_keyboard(expanded,mode,selected,ctx,action){
    if action.kind=="active"{ctx.emit("active_change",action.key);}
    else if action.kind=="toggle"{tree_toggle(expanded,ctx,action.key);}
    else if action.kind=="select"{tree_select(mode,selected,ctx,action);}
    handled()
}
fn tree_row(expanded,selected,active,mode,disabled,ctx,payload){
    let item=payload.item;let is_active=active!=()&&active==item.key;let is_selected=item.key in selected;
    let children=[];
    if item.has_children{
        let disclosure=image_source(asset(if item.expanded{"app/icons/chevron_down"}else{"app/icons/chevron_right"}))
            .with_style(ctx.component_style("disclosure",style().width(px(14)).height(px(14))
                .text_color(theme_color("text_muted"))));
        let trigger=box([disclosure]).with_key(`${item.key}-disclosure`).with_style(style().width(px(20)).height(px(24))
            .items_center().justify_center()).disabled(disabled||item.disabled).accessibility_role("button")
            .accessibility_label(if item.expanded{`Collapse ${item.label}`}else{`Expand ${item.label}`});
        if !disabled&&!item.disabled{trigger=trigger.on_click_value(Fn("tree_toggle").curry(expanded),item.key);}
        children.push(trigger);
    }else{children.push(row([]).with_style(style().width(px(20))));}
    children.push(text(item.label).with_style(ctx.component_style("label",style().flex_grow().min_width(px(0))
        .whitespace_nowrap().text_ellipsis())));
    if item.loading{children.push(text("…").with_style(ctx.component_style("loading",style().text_color(theme_color("text_muted")))));}
    let row_style=style().height(px(28)).padding_end(theme_spacing("sm")).items_center()
        .padding_start(px(4.0+item.depth*16.0)).typography("body_small")
        .background(if is_selected{theme_color("selection")}else{rgba(0x00000000)})
        .border(px(1)).border_color(if is_active{theme_color("focus_ring")}else{rgba(0x00000000)});
    let resolved=ctx.component_style("row",row_style);
    if is_active{resolved=ctx.component_style("row_active",resolved);}
    if is_selected{resolved=ctx.component_style("row_selected",resolved);}
    let node=row(children).with_key(payload.key).with_style(resolved).disabled(disabled||item.disabled)
        .accessibility_role("treeitem").accessibility_label(item.label)
        .accessibility_selected(is_selected);
    if item.has_children{node=node.accessibility_expanded(item.expanded);}
    if !disabled&&!item.disabled{node=node.on_click_value(Fn("tree_select").curry(mode,selected),item);}
    node
}
fn eligible_rows(rows){let result=[];for row in rows{if !row.disabled{result.push(row);}}result}
fn row_index(rows,key){for index in 0..rows.len{if rows[index].key==key{return index;}}()}
fn render_Tree(ctx,props){
    if ("height" in props&&props.height!=())==props.fill_height{throw "Tree requires exactly one of height or fill_height";}
    let rows=outline_projection(props.items,props.expanded);let eligible=eligible_rows(rows);
    let source_active=if "active_key" in props{props.active_key}else{()};
    let active_index=if source_active==(){()}else{row_index(eligible,source_active)};
    let active=if active_index==(){()}else{source_active};
    let current=if active_index==(){0}else{active_index};
    let previous=if eligible.len==0{()}else{eligible[if current>0{current-1}else{0}].key};
    let next=if eligible.len==0{()}else{eligible[if current+1<eligible.len{current+1}else{eligible.len-1}].key};
    let first=if eligible.len==0{()}else{eligible[0].key};let last=if eligible.len==0{()}else{eligible[eligible.len-1].key};
    let active_row=if eligible.len==0{()}else{eligible[current]};
    let parent=if active_row==(){()}else{active_row.eligible_parent};
    let left=if active_row==(){#{kind:"none"}}else if active_row.has_children&&active_row.expanded{
        #{kind:"toggle",key:active_row.key}}else if parent!=(){#{kind:"active",key:parent}}else{#{kind:"none"}};
    let right=if active_row==(){#{kind:"none"}}else if active_row.has_children&&!active_row.expanded{
        #{kind:"toggle",key:active_row.key}}else if current+1<eligible.len&&eligible[current+1].depth>active_row.depth{
        #{kind:"active",key:eligible[current+1].key}}else{#{kind:"none"}};
    let renderer=Fn("tree_row").curry(props.expanded,props.selected_keys,active,props.selection_mode,props.disabled);
    let config=#{key:`${props.key}-rows`,label:props.label,data:rows,estimated_height:props.estimated_row_height};
    if active!=(){config.reveal_key=active;}
    if "height" in props&&props.height!=(){config.height=props.height;}else{config.fill_height=true;}
    let root=virtual_collection(config,renderer).with_key(props.key)
        .with_style(ctx.component_style("root",style().border(px(1)).border_color(theme_color("border"))))
        .tab_stop(!props.disabled).disabled(props.disabled).accessibility_role("tree").accessibility_label(props.label);
    if eligible.len>0{
        root=root.on_key_value("up",Fn("tree_active"),previous).on_key_value("down",Fn("tree_active"),next)
            .on_key_value("home",Fn("tree_active"),first).on_key_value("end",Fn("tree_active"),last)
            .on_key_value("left",Fn("tree_keyboard").curry(props.expanded,props.selection_mode,props.selected_keys),left)
            .on_key_value("right",Fn("tree_keyboard").curry(props.expanded,props.selection_mode,props.selected_keys),right)
            .on_key_value("enter",Fn("tree_keyboard").curry(props.expanded,props.selection_mode,props.selected_keys),
                #{kind:"select",key:active_row.key,label:active_row.label});
    }
    root
}
