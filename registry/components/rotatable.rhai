/* gpui-rhai
{
  "id": "components/rotatable",
  "export": "Rotatable",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["border", "surface", "focus_ring", "metrics.control"],
  "environment": []
}
*/
// Controlled Canvas rotation around an explicit local pivot.
// State: caller owns angle.
// Example: Rotatable(#{key:"dial",label:"Rotate dial",angle:0.0,pivot:#{x:100.0,y:100.0},content:canvas_node,on_rotate:Fn("changed")})

define_component(#{
    metadata:#{id:"components/rotatable","export":"Rotatable",version: "0.2.0",
        runtime_api:#{min_inclusive: 3,max_exclusive: 4},dependencies:[],capabilities:#{},
        tokens: ["border", "surface", "focus_ring", "metrics.control"],
        environment: []},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        label:#{schema:#{type:"string"},required:true,sensitive:false,
            doc:"Accessible name of the rotation slider."},
        angle:#{schema:#{type:"number"},required:true,sensitive:false,
            doc:"Clockwise rotation in degrees; the caller stores the `rotate` payload and passes it back."},
        pivot:#{schema:#{type:"object",allow_unknown:false,fields:#{
            x:#{schema:#{type:"number",min:-1000000.0,max:1000000.0},required:true,sensitive:false,
                doc:"Pivot distance from the content's left edge, in logical pixels."},
            y:#{schema:#{type:"number",min:-1000000.0,max:1000000.0},required:true,sensitive:false,
                doc:"Pivot distance from the content's top edge, in logical pixels."},
        }},required:true,sensitive:false,
            doc:"Point the content turns around, in logical pixels from the content's top-left corner."},
        snap:#{schema:#{type:"optional",value:#{type:"number",exclusive_min:0.0,max:360.0}},required:false,sensitive:false,
            doc:"Angle step in degrees that proposed angles round to; it also replaces `keyboard_step`."},
        keyboard_step:#{schema:#{type:"number",min:0.1,max:180.0},required:false,sensitive:false,
            "default":#{type:"float",value:5.0},
            doc:"Degrees one arrow key turns, Right/Up clockwise; Shift turns four times as far and Home returns to 0."},
        threshold:#{schema:#{type:"number",min:0.0,max:64.0},required:false,sensitive:false,
            "default":#{type:"float",value:4.0},
            doc:"Pointer travel in logical pixels before a press starts rotating."},
        handle:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,
            doc:"Grip in the top-right corner that alone starts a rotation drag; without it the whole content does."},
        content:#{schema:#{type:"node"},required:true,sensitive:false,
            doc:"Node that rotates; it fills the component."},
        disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},
            doc:"Ignores pointer and keyboard input and removes the control from the tab order."},
        on_rotate:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,
            doc:"Called with the proposed angle in degrees when a rotation drag ends or a key turns the content."},
    },state:#{fields:#{}},events:#{rotate:#{payload:#{type:"number"},
            doc:"Emitted when a rotation drag ends or a key turns the content; the payload is the next angle in [0, 360)."}},
        slots:#{content:#{required:true,multiple:false,doc:"The rotated content."},
            handle:#{required:false,multiple:false,doc:"The rotation grip in the top-right corner."}},
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
            .top(px(0)).bottom(px(0)).cursor_move().border(px(2)).border_color(rgba(0x00000000))
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
                .width(theme_length("metrics.control")).height(theme_length("metrics.control"))
                .items_center().justify_center())));
    }else{children.push(interaction);}
    box(children).with_key(viewport_key).with_ref(viewport_ref)
        .with_style(ctx.component_style("root",style().relative().clip().min_width(px(0)).min_height(px(0))
            .border(px(1)).border_color(theme_color("border")).background(theme_color("surface"))))
}
