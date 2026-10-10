/* gpui-rhai
{
  "id": "components/selection_area",
  "export": "SelectionArea",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["border", "surface", "focus_ring", "accent"],
  "environment": []
}
*/
// Controlled Canvas click/range/marquee object selection.
// Example: SelectionArea(#{key:"nodes",label:"Nodes",targets:targets,selected_keys:[],content:canvas_node,on_selection_change:Fn("changed")})

define_component(#{
    metadata:#{id:"components/selection_area","export":"SelectionArea",version: "0.2.0",
        runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{},
        tokens: ["border", "surface", "focus_ring", "accent"],
        environment: []},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        label:#{schema:#{type:"string"},required:true,sensitive:false,
            doc:"Accessible name of the selection grid."},
        targets:#{schema:#{type:"array",max_items:10000,items:#{type:"map",values:#{type:"ui_value"}}},required:true,sensitive:false,
            doc:"Selectable objects, `#{key, x, y, width, height, disabled?}` in the content's logical pixels; later ones win overlaps."},
        selected_keys:#{schema:#{type:"array",max_items:10000,items:#{type:"string"}},required:true,sensitive:false,
            doc:"Keys of the selected targets; the caller stores the `selection_change` payload and passes it back."},
        active_key:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,
            doc:"Target the keyboard is on: arrows, Home and End move from it in `targets` order, Space toggles it."},
        anchor_key:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,
            doc:"Target a Shift range extends from; the caller stores it from the `selection_change` payload."},
        multiple:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:true},
            doc:"Allows several selected targets, with Shift ranges, Ctrl/Cmd toggles and additive marquees."},
        marquee:#{schema:#{type:"string",allowed:["intersect","enclose"]},required:false,sensitive:false,
            "default":#{type:"string",value:"intersect"},
            doc:"Whether a marquee drag selects targets it touches or only those wholly inside it."},
        threshold:#{schema:#{type:"number",min:0.0,max:64.0},required:false,sensitive:false,
            "default":#{type:"float",value:4.0},
            doc:"Pointer travel in logical pixels before a press becomes a marquee drag."},
        content:#{schema:#{type:"node"},required:true,sensitive:false,
            doc:"Canvas that draws the targets; target coordinates are local to it."},
        disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},
            doc:"Ignores pointer and keyboard input and removes the area from the tab order."},
        on_selection_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,
            doc:"Called with `{selected_keys, active_key, anchor_key}` when a click, marquee or key changes the selection."},
    },state:#{fields:#{}},events:#{selection_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
        selected_keys:#{schema:#{type:"array",max_items:10000,items:#{type:"string"}},required:true,sensitive:false,
            doc:"Keys of the targets to select."},
        active_key:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:false,
            doc:"Next `active_key`, or `()` for none."},
        anchor_key:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:false,
            doc:"Next `anchor_key`, or `()` for none."},
    }},
        doc:"Emitted when a click, marquee drag or key changes the selection; the payload is the next selection state."}},
        slots:#{content:#{required:true,multiple:false,doc:"The canvas the targets are drawn on."}},
        parts:["root","content","interaction"]},
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
