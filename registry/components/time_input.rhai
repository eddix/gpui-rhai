/* gpui-rhai
{"id":"components/time_input","export":"TimeInput","version":"0.2.2","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/input"],"capabilities":{},"tokens":[],"environment":["size"]}
*/
// Controlled locale-aware clock editing; partial text remains an editable draft.
// Example: TimeInput(#{key:"clock",label:"Time",value:clock,on_change:Fn("clock_changed")})
import "components/input" as input;
// Controlled native text editing: incomplete text is retained, not replaced during typing.
define_component(#{metadata:#{id:"components/time_input","export":"TimeInput",version:"0.2.2",
    runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/input"],capabilities:#{},tokens:[],environment:["size"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable native editor identity."},
        label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accessible time field name."},
        value:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Controlled localized clock text; incomplete drafts are allowed."},
        precision:#{schema:#{type:"string",allowed:["minute","second"]},required:false,sensitive:false,"default":#{type:"string",value:"minute"},doc:"Minute or second precision; Up/Down steps this unit."},
        error:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Marks the field invalid."},
        disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Blocks editing and removes the field from traversal."},
        size:#{schema:#{type:"optional",value:#{type:"string",allowed:["xs","sm","md","lg"]}},required:false,sensitive:false,doc:"Explicit control size or inherited size."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called with typed or stepped clock text."},
    },state:#{fields:#{}},events:#{change:#{payload:#{type:"string"},doc:"Emitted for every native edit or Up/Down step; parse with ctx.parse_time."}},slots:#{},parts:["root","input"]},render:Fn("render_TimeInput")});
fn TimeInput(props){render_component("components/time_input",props)}
fn changed(ctx,value){ctx.emit("change",value);}
fn step_time(value,precision,ctx,direction){
    let parsed=ctx.parse_time(value,precision);if parsed==(){return handled();}
    let ms=time_local_candidates("1970-01-01",parsed,"UTC")[0];
    let stepped=(ms+direction*if precision=="second"{1000}else{60000}+86400000)%86400000;
    ctx.emit("change",ctx.format_time(stepped,if precision=="second"{"medium"}else{"short"},"UTC"));handled()
}
fn render_TimeInput(ctx,props){
    let node=input::Input(#{key:props.key,label:props.label,value:props.value,error:props.error,
        disabled:props.disabled,on_change:Fn("changed")})
        .with_style(ctx.component_style("root",ctx.component_style("input",style())))
        .on_key_value("up",Fn("step_time").curry(props.value,props.precision),1)
        .on_key_value("down",Fn("step_time").curry(props.value,props.precision),-1);
    if "size" in props&&props.size!=(){node.env(#{size:props.size})}else{node}
}
