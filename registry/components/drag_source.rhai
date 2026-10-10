/* gpui-rhai
{
  "id": "components/drag_source",
  "export": "DragSource",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["focus_ring", "accent"],
  "environment": []
}
*/
// Typed in-application data drag source. It does not move the source object.
// State: Host-domain native drag session only; application data remains controlled.
// Example: DragSource(#{ key:"server", label:"Move server", source_id:"server-a",
//     payload_type:"server", payload:#{id:"a"}, content:text("Server A") })

define_component(#{
    metadata: #{ id:"components/drag_source", "export":"DragSource", version: "0.2.0",
        runtime_api:#{min_inclusive: 3,max_exclusive: 4}, dependencies:[], capabilities:#{},
        tokens: ["focus_ring", "accent"],
        environment: [] },
    schema: #{ props: #{
        key: #{ schema:#{type:"string"}, required:true, sensitive:false },
        label: #{ schema:#{type:"string"}, required:true, sensitive:false,
            doc:"Accessible name of the drag source button." },
        source_id: #{ schema:#{type:"string"}, required:true, sensitive:false,
            doc:"Identity of the dragged item, 1 to 128 characters; drop zones receive it as `source_id`." },
        payload_type: #{ schema:#{type:"string"}, required:true, sensitive:false,
            doc:"Type name, 1 to 128 characters, that drop zones match against their `payload_types`." },
        payload: #{ schema:#{type:"ui_value"}, required:true, sensitive:false,
            doc:"Data handed unchanged to the receiving zone's `drop` event." },
        operation: #{ schema:#{type:"string",allowed:["copy","move"]}, required:false,
            sensitive:false, "default":#{type:"string",value:"move"},
            doc:"What the drop means; only zones listing it in `operations` accept the drag." },
        content: #{ schema:#{type:"node"}, required:true, sensitive:false,
            doc:"Node the user presses and drags." },
        threshold: #{ schema:#{type:"number",min:0.0,max:64.0}, required:false,
            sensitive:false, "default":#{type:"float",value:4.0},
            doc:"Pointer travel in logical pixels before a press becomes a drag." },
        keyboard_target: #{ schema:#{type:"optional",value:#{type:"string"}}, required:false, sensitive:false,
            doc:"`target_id` of the zone that Enter or Space drops onto; without it the keyboard cannot drop." },
        disabled: #{ schema:#{type:"bool"}, required:false, sensitive:false,
            "default":#{type:"bool",value:false},
            doc:"Ignores pointer and keyboard input and removes the source from the tab order." },
        on_drag_end: #{ schema:#{type:"optional",value:#{type:"callback"}}, required:false, sensitive:false,
            doc:"Called with `{accepted, target_id, operation, cancelled}` when a drag or keyboard drop ends." },
    }, state:#{fields:#{}}, events:#{ drag_end:#{ payload:#{type:"object",allow_unknown:false,fields:#{
        accepted:#{schema:#{type:"bool"},required:true,sensitive:false,
            doc:"True when a drop zone received the drop; that zone's Host still decides what it does."},
        target_id:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:false,
            doc:"`target_id` of the zone that received the drop, or `()`."},
        operation:#{schema:#{type:"string",allowed:["copy","move"]},required:true,sensitive:false,
            doc:"The drag's `operation`."},
        cancelled:#{schema:#{type:"bool"},required:true,sensitive:false,
            doc:"True when the drag was cancelled, for example with Escape."},
    }},
        doc:"Emitted when a drag past the threshold ends or is cancelled, or a keyboard drop runs; the payload is the outcome." } },
        slots:#{}, parts:["root","content","interaction"] },
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
                .border(px(2)).border_color(rgba(0x00000000))
                .focus(style().border_color(theme_color("focus_ring")))))
        .disabled(props.disabled).tab_stop(!props.disabled)
        .accessibility_role("button").accessibility_label(props.label);
    box([
        props.content.with_key(`${props.key}-content`).with_style(ctx.component_style("content",style())),
        interaction
    ]).with_key(props.key).with_style(ctx.component_style("root",style().relative()))
}
