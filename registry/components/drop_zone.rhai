/* gpui-rhai
{
  "id": "components/drop_zone",
  "export": "DropZone",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["accent", "danger"],
  "environment": []
}
*/
// Typed same-Host application drop target; with `os.files`, also files and folders the
// operating system drags in, when the Host accepts file drops. Cross-window drops are excluded.
// State: frame-local native target eligibility only; application ownership remains controlled.
// Example: DropZone(#{ key:"lane", label:"Server lane", target_id:"lane-a",
//     payload_types:["server"], operations:["move"], content:text("Lane A") })
// Example: DropZone(#{ key:"import", label:"Import", target_id:"import",
//     payload_types:["os.files"], operations:["copy"], content:text("Drop files"), on_drop:Fn("imported") })

define_component(#{
    metadata: #{ id:"components/drop_zone", "export":"DropZone", version: "0.4.3",
        runtime_api:#{min_inclusive: 3,max_exclusive: 4}, dependencies:[], capabilities:#{},
        tokens: ["accent", "danger"],
        environment: [] },
    schema: #{ props: #{
        key: #{ schema:#{type:"string"}, required:true, sensitive:false },
        label: #{ schema:#{type:"string"}, required:true, sensitive:false,
            doc:"Accessible name of the drop area." },
        target_id: #{ schema:#{type:"string"}, required:true, sensitive:false,
            doc:"Identity of this zone, 1 to 128 characters; `drop`, the source's `drag_end` and `keyboard_target` use it." },
        payload_types: #{ schema:#{type:"array",max_items:32,items:#{type:"string"}}, required:true, sensitive:false,
            doc:"Source `payload_type` values this zone accepts; at least one. `os.files` takes files and folders the operating system drags in, when the Host calls `ScriptView::accept_file_drops(true)`." },
        file_kinds: #{ schema:#{type:"optional",value:#{type:"array",max_items:2,items:#{type:"string",allowed:["file","dir"]}}}, required:false, sensitive:false,
            doc:"With `os.files`, the kinds a file drag may hold: `file`, `dir` or both, the default. A drag holding another kind cannot drop." },
        operations: #{ schema:#{type:"array",max_items:2,items:#{type:"string",allowed:["copy","move"]}}, required:true, sensitive:false,
            doc:"Source `operation` values this zone accepts; at least one." },
        priority: #{ schema:#{type:"integer"}, required:false, sensitive:false,
            "default":#{type:"integer",value:0},
            doc:"Picks the zone where eligible zones overlap: higher wins, then the smaller zone, then the one painted later." },
        content: #{ schema:#{type:"node"}, required:true, sensitive:false,
            doc:"Node that marks the drop area; the zone covers it." },
        disabled: #{ schema:#{type:"bool"}, required:false, sensitive:false,
            "default":#{type:"bool",value:false},
            doc:"Stops the zone from accepting drops." },
        on_drop: #{ schema:#{type:"optional",value:#{type:"callback"}}, required:false, sensitive:false,
            doc:"Called with the `drop` payload when an accepted drag is released over the zone or dropped by keyboard." },
    }, state:#{fields:#{}}, events:#{ drop:#{ payload:#{type:"object",allow_unknown:false,fields:#{
        source_id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"`source_id` of the drag source."},
        target_id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"This zone's `target_id`."},
        payload_type:#{schema:#{type:"string"},required:true,sensitive:false,
            doc:"`payload_type` of the drag source."},
        payload:#{schema:#{type:"ui_value"},required:true,sensitive:false,
            doc:"`payload` of the drag source, unchanged. For `os.files` (source `os`, operation `copy`): `#{ files: [#{ file, name, size, kind }] }`, where `file` is an opaque handle the Host turns into the path, `name` has no folders, `size` is in bytes (`()` for a folder) and `kind` is `file` or `dir`."},
        operation:#{schema:#{type:"string",allowed:["copy","move"]},required:true,sensitive:false,
            doc:"`operation` of the drag source."},
        x:#{schema:#{type:"number"},required:true,sensitive:false,
            doc:"Pointer x at the drop in window coordinates, in logical pixels; the zone's centre for keyboard drops."},
        y:#{schema:#{type:"number"},required:true,sensitive:false,
            doc:"Pointer y at the drop in window coordinates, in logical pixels; the zone's centre for keyboard drops."},
    }},
        doc:"Emitted when an accepted drag is released over the zone or a keyboard drop names it; the payload carries the drag." } },
        slots:#{}, parts:["root","content","interaction"] },
    render:Fn("render_DropZone")
});

fn DropZone(props) { render_component("components/drop_zone", props) }
fn drop_zone_commit(ctx, value) { ctx.emit("drop", value); }

fn render_DropZone(ctx, props) {
    let interaction = gpui_rhai::DropZonePrimitive(#{ key:`${props.key}-interaction`,
        target_id:props.target_id, payload_types:props.payload_types, operations:props.operations,
        file_kinds:if "file_kinds" in props { props.file_kinds } else { () },
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
