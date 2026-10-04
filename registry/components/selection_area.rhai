/* gpui-rhai
{
  "id": "components/selection_area",
  "export": "SelectionArea",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["border", "surface", "focus_ring"],
  "environment": []
}
*/
// Controlled Canvas click/range/marquee object selection.
// Props: key, label, targets[{key,x,y,width,height,disabled?}], selected_keys,
// active_key, anchor_key, multiple, marquee, threshold, content, disabled,
// on_selection_change.
// Events: selection_change({selected_keys,active_key,anchor_key}).
// Parts: root, content, interaction.
// Example: SelectionArea(#{key:"nodes",label:"Nodes",targets:targets,selected_keys:[],content:canvas_node,on_selection_change:Fn("changed")})

define_component(#{
    metadata:#{id:"components/selection_area","export":"SelectionArea",version: "0.2.0",
        runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{},
        tokens: ["border", "surface", "focus_ring"],
        environment: []},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        label:#{schema:#{type:"string"},required:true,sensitive:false},
        targets:#{schema:#{type:"array",max_items:10000,items:#{type:"map",values:#{type:"ui_value"}}},required:true,sensitive:false},
        selected_keys:#{schema:#{type:"array",max_items:10000,items:#{type:"string"}},required:true,sensitive:false},
        active_key:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false},
        anchor_key:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false},
        multiple:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:true}},
        marquee:#{schema:#{type:"string",allowed:["intersect","enclose"]},required:false,sensitive:false,
            "default":#{type:"string",value:"intersect"}},
        threshold:#{schema:#{type:"number",min:0.0,max:64.0},required:false,sensitive:false,
            "default":#{type:"float",value:4.0}},
        content:#{schema:#{type:"node"},required:true,sensitive:false},
        disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false}},
        on_selection_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false},
    },state:#{fields:#{}},events:#{selection_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
        selected_keys:#{schema:#{type:"array",max_items:10000,items:#{type:"string"}},required:true,sensitive:false},
        active_key:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:false},
        anchor_key:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:false},
    }}}},slots:#{content:#{required:true,multiple:false}},parts:["root","content","interaction"]},
    render:Fn("render_SelectionArea")
});

fn SelectionArea(props){render_component("components/selection_area",props)}
fn selection_area_commit(ctx,value){ctx.emit("selection_change",value);}
fn render_SelectionArea(ctx,props){
    let canvas_key=`${props.key}-canvas`;
    let canvas_ref=element_ref(canvas_key);
    let native=#{key:`${props.key}-selection`,targets:props.targets,selected_keys:props.selected_keys,
        active_key:if "active_key" in props{props.active_key}else{()},
        anchor_key:if "anchor_key" in props{props.anchor_key}else{()},
        multiple:props.multiple,marquee:props.marquee,threshold:props.threshold,disabled:props.disabled,
        canvas_ref:canvas_ref,on_selection_change:if "on_selection_change" in props&&props.on_selection_change!=(){
            Fn("selection_area_commit")}else{()}};
    let content=props.content.with_key(canvas_key).with_ref(canvas_ref)
        .with_style(ctx.component_style("content",style().width(relative(1.0)).height(relative(1.0))));
    let interaction=gpui_rhai::SelectionAreaPrimitive(native).with_key(`${props.key}-selection-area`)
        .with_style(ctx.component_style("interaction",style().absolute().left(px(0)).right(px(0))
            .top(px(0)).bottom(px(0)).cursor_crosshair().border(px(2)).border_color(rgba(0x00000000))
            .focus(style().border_color(theme_color("focus_ring")))))
        .disabled(props.disabled).tab_stop(!props.disabled)
        .accessibility_role("grid").accessibility_label(props.label)
        .accessibility_value(`${props.selected_keys.len} selected`);
    box([content,interaction]).with_key(props.key)
        .with_style(ctx.component_style("root",style().relative().clip().min_width(px(0)).min_height(px(0))
            .border(px(1)).border_color(theme_color("border")).background(theme_color("surface"))))
}
