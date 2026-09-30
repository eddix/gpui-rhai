/* gpui-rhai
{
  "id": "components/drop_zone",
  "export": "DropZone",
  "version": "0.1.8",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Typed same-Host application drop target. OS/file and cross-window drops are excluded.
// Props: key, label, target_id, payload_types, operations, priority, content,
// disabled, on_drop.
// Events: drop({source_id,target_id,payload_type,payload,operation,x,y}).
// State: frame-local native target eligibility only; application ownership remains controlled.
// Parts: root, content, interaction.
// Example: DropZone(#{ key:"lane", label:"Server lane", target_id:"lane-a",
//     payload_types:["server"], operations:["move"], content:text("Lane A") })

define_component(#{
    metadata: #{ id:"components/drop_zone", "export":"DropZone", version:"0.1.8",
        runtime_api:#{min_inclusive:2,max_exclusive:3}, dependencies:[], capabilities:#{} },
    schema: #{ props: #{
        key: #{ schema:#{type:"string"}, required:true, sensitive:false },
        label: #{ schema:#{type:"string"}, required:true, sensitive:false },
        target_id: #{ schema:#{type:"string"}, required:true, sensitive:false },
        payload_types: #{ schema:#{type:"array",max_items:32,items:#{type:"string"}}, required:true, sensitive:false },
        operations: #{ schema:#{type:"array",max_items:2,items:#{type:"string",allowed:["copy","move"]}}, required:true, sensitive:false },
        priority: #{ schema:#{type:"integer"}, required:false, sensitive:false,
            "default":#{type:"integer",value:0} },
        content: #{ schema:#{type:"node"}, required:true, sensitive:false },
        disabled: #{ schema:#{type:"bool"}, required:false, sensitive:false,
            "default":#{type:"bool",value:false} },
        on_drop: #{ schema:#{type:"optional",value:#{type:"callback"}}, required:false, sensitive:false },
    }, state:#{fields:#{}}, events:#{ drop:#{ payload:#{type:"object",allow_unknown:false,fields:#{
        source_id:#{schema:#{type:"string"},required:true,sensitive:false},
        target_id:#{schema:#{type:"string"},required:true,sensitive:false},
        payload_type:#{schema:#{type:"string"},required:true,sensitive:false},
        payload:#{schema:#{type:"ui_value"},required:true,sensitive:false},
        operation:#{schema:#{type:"string",allowed:["copy","move"]},required:true,sensitive:false},
        x:#{schema:#{type:"number"},required:true,sensitive:false},
        y:#{schema:#{type:"number"},required:true,sensitive:false},
    }} } }, slots:#{}, parts:["root","content","interaction"] },
    render:Fn("render_DropZone")
});

fn DropZone(props) { render_component("components/drop_zone", props) }
fn drop_zone_commit(ctx, value) { ctx.emit("drop", value); }

fn render_DropZone(ctx, props) {
    let interaction = gpui_rhai::DropZonePrimitive(#{ key:`${props.key}-interaction`,
        target_id:props.target_id, payload_types:props.payload_types, operations:props.operations,
        priority:props.priority, disabled:props.disabled,
        on_drop:if "on_drop" in props && props.on_drop != () { Fn("drop_zone_commit") } else { () } })
        .with_key(`${props.key}-drop-zone`).with_style(ctx.component_style("interaction",
            style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))))
        .disabled(props.disabled).accessibility_role("group").accessibility_label(props.label);
    box([
        props.content.with_key(`${props.key}-content`).with_style(ctx.component_style("content",style())),
        interaction
    ]).with_key(props.key).with_style(ctx.component_style("root",style().relative()))
}
