/* gpui-rhai
{
  "id": "components/drag_source",
  "export": "DragSource",
  "version": "0.1.8",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Typed in-application data drag source. It does not move the source object.
// Props: key, label, source_id, payload_type, payload, operation, content,
// threshold, keyboard_target, disabled, on_drag_end.
// Events: drag_end({accepted,target_id,operation,cancelled}).
// State: Host-domain native drag session only; application data remains controlled.
// Parts: root, content, interaction.
// Example: DragSource(#{ key:"server", label:"Move server", source_id:"server-a",
//     payload_type:"server", payload:#{id:"a"}, content:text("Server A") })

define_component(#{
    metadata: #{ id:"components/drag_source", "export":"DragSource", version:"0.1.8",
        runtime_api:#{min_inclusive: 3,max_exclusive: 4}, dependencies:[], capabilities:#{} },
    schema: #{ props: #{
        key: #{ schema:#{type:"string"}, required:true, sensitive:false },
        label: #{ schema:#{type:"string"}, required:true, sensitive:false },
        source_id: #{ schema:#{type:"string"}, required:true, sensitive:false },
        payload_type: #{ schema:#{type:"string"}, required:true, sensitive:false },
        payload: #{ schema:#{type:"ui_value"}, required:true, sensitive:false },
        operation: #{ schema:#{type:"string",allowed:["copy","move"]}, required:false,
            sensitive:false, "default":#{type:"string",value:"move"} },
        content: #{ schema:#{type:"node"}, required:true, sensitive:false },
        threshold: #{ schema:#{type:"number",min:0.0,max:64.0}, required:false,
            sensitive:false, "default":#{type:"float",value:4.0} },
        keyboard_target: #{ schema:#{type:"optional",value:#{type:"string"}}, required:false, sensitive:false },
        disabled: #{ schema:#{type:"bool"}, required:false, sensitive:false,
            "default":#{type:"bool",value:false} },
        on_drag_end: #{ schema:#{type:"optional",value:#{type:"callback"}}, required:false, sensitive:false },
    }, state:#{fields:#{}}, events:#{ drag_end:#{ payload:#{type:"object",allow_unknown:false,fields:#{
        accepted:#{schema:#{type:"bool"},required:true,sensitive:false},
        target_id:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:false},
        operation:#{schema:#{type:"string",allowed:["copy","move"]},required:true,sensitive:false},
        cancelled:#{schema:#{type:"bool"},required:true,sensitive:false},
    }} } }, slots:#{}, parts:["root","content","interaction"] },
    render:Fn("render_DragSource")
});

fn DragSource(props) { render_component("components/drag_source", props) }
fn drag_source_end(ctx, value) { ctx.emit("drag_end", value); }

fn render_DragSource(ctx, props) {
    let native = #{ key:`${props.key}-interaction`, source_id:props.source_id,
        payload_type:props.payload_type, payload:props.payload, operation:props.operation,
        threshold:props.threshold, disabled:props.disabled,
        on_drag_end:if "on_drag_end" in props && props.on_drag_end != () { Fn("drag_source_end") } else { () } };
    if "keyboard_target" in props && props.keyboard_target != () {
        native.keyboard_target = props.keyboard_target;
    }
    let interaction = gpui_rhai::DragSourcePrimitive(native)
        .with_key(`${props.key}-drag-source`).with_style(ctx.component_style("interaction",
            style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
                .border(px(1)).border_color(theme_color("border"))
                .focus(style().border_color(theme_color("focus_ring")))))
        .disabled(props.disabled).tab_stop(!props.disabled)
        .accessibility_role("button").accessibility_label(props.label);
    box([
        props.content.with_key(`${props.key}-content`).with_style(ctx.component_style("content",style())),
        interaction
    ]).with_key(props.key).with_style(ctx.component_style("root",style().relative()))
}
