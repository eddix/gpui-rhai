/* gpui-rhai
{
  "id": "components/rotatable",
  "export": "Rotatable",
  "version": "0.1.8",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled Canvas rotation around an explicit local pivot.
// Props: key, label, angle, pivot{x,y}, optional snap/handle, keyboard_step,
// threshold, content, disabled, on_rotate.
// Events: rotate(number degrees in [0,360)). State: caller owns angle.
// Parts: root, content, handle, interaction.
// Example: Rotatable(#{key:"dial",label:"Rotate dial",angle:0.0,pivot:#{x:100.0,y:100.0},content:canvas_node,on_rotate:Fn("changed")})

define_component(#{
    metadata:#{id:"components/rotatable","export":"Rotatable",version:"0.1.8",
        runtime_api:#{min_inclusive:2,max_exclusive:3},dependencies:[],capabilities:#{}},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        label:#{schema:#{type:"string"},required:true,sensitive:false},
        angle:#{schema:#{type:"number"},required:true,sensitive:false},
        pivot:#{schema:#{type:"object",allow_unknown:false,fields:#{
            x:#{schema:#{type:"number"},required:true,sensitive:false},
            y:#{schema:#{type:"number"},required:true,sensitive:false},
        }},required:true,sensitive:false},
        snap:#{schema:#{type:"optional",value:#{type:"number",exclusive_min:0.0,max:360.0}},required:false,sensitive:false},
        keyboard_step:#{schema:#{type:"number",min:0.1,max:180.0},required:false,sensitive:false,
            "default":#{type:"float",value:5.0}},
        threshold:#{schema:#{type:"number",min:0.0,max:64.0},required:false,sensitive:false,
            "default":#{type:"float",value:4.0}},
        handle:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false},
        content:#{schema:#{type:"node"},required:true,sensitive:false},
        disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false}},
        on_rotate:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false},
    },state:#{fields:#{}},events:#{rotate:#{payload:#{type:"number"}}},
        slots:#{content:#{required:true,multiple:false},handle:#{required:false,multiple:false}},
        parts:["root","content","handle","interaction"]},render:Fn("render_Rotatable")
});

fn Rotatable(props){render_component("components/rotatable",props)}
fn rotatable_commit(ctx,value){ctx.emit("rotate",value);}

fn render_Rotatable(ctx,props){
    let viewport_key=`${props.key}-viewport`;
    let content_key=`${props.key}-content`;
    let viewport_ref=element_ref(viewport_key);
    let content_ref=element_ref(content_key);
    let angle_signal=signal("rotation-angle",props.angle*1.0);
    let x_signal=signal("rotation-x",0.0);
    let y_signal=signal("rotation-y",0.0);
    let token=`${props.angle}|${props.pivot.x}|${props.pivot.y}|${if "snap" in props{props.snap}else{()}}|${props.disabled}`;
    let token_signal=signal("rotation-source-token","");
    let native=#{key:`${props.key}-rotation`,source_token:token,angle:props.angle,
        pivot_x:props.pivot.x,pivot_y:props.pivot.y,
        keyboard_step:props.keyboard_step,threshold:props.threshold,disabled:props.disabled,
        content_ref:content_ref,
        angle_signal:angle_signal,x_signal:x_signal,y_signal:y_signal,
        source_token_signal:token_signal,
        on_rotate:if "on_rotate" in props&&props.on_rotate!=(){Fn("rotatable_commit")}else{()}};
    if "snap" in props&&props.snap!=(){native.snap=props.snap;}
    let interaction=gpui_rhai::RotatablePrimitive(native).with_key(`${props.key}-rotate-interaction`)
        .with_style(ctx.component_style("interaction",style().absolute().left(px(0)).right(px(0))
            .top(px(0)).bottom(px(0)).cursor_move().border(px(1)).border_color(rgba(0x00000000))
            .focus(style().border_color(theme_color("focus_ring")))))
        .disabled(props.disabled).tab_stop(!props.disabled)
        .accessibility_role("slider").accessibility_label(props.label)
        .accessibility_value(props.angle).accessibility_value_min(0.0).accessibility_value_max(360.0);
    let content_style=ctx.component_style("content",style().width(relative(1.0)).height(relative(1.0)));
    let content_node=props.content.with_key(content_key).with_ref(content_ref).with_style(content_style)
        .bind_signal("rotate",angle_signal).bind_signal("translate_x",x_signal).bind_signal("translate_y",y_signal);
    let content=box([content_node]).with_key(`${content_key}-frame`)
        .with_style(style().width(relative(1.0)).height(relative(1.0)));
    let children=[content];
    if "handle" in props&&props.handle!=(){
        children.push(box([props.handle,interaction]).with_key(`${props.key}-handle`)
            .with_style(ctx.component_style("handle",style().absolute().right(px(8)).top(px(8))
                .width(px(28)).height(px(28)).items_center().justify_center())));
    }else{children.push(interaction);}
    box(children).with_key(viewport_key).with_ref(viewport_ref)
        .with_style(ctx.component_style("root",style().relative().clip().min_width(px(0)).min_height(px(0))
            .border(px(1)).border_color(theme_color("border")).background(theme_color("surface"))))
}
