import "patterns/conflict_radar" as radar;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/conflict_radar_demo","export":"ConflictRadarDemo","version":"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/conflict_radar","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},slots:#{},parts:["root"],events:#{}},
    render:Fn("render_demo")
});
fn ConflictRadarDemo(p){render_component("gallery/conflict_radar_demo",p)}
fn state(c){
    let v=c.get_state("demo");
    if v!=(){v}else{#{run:0,revision:0,edit:0,done:false,narrow:false,deny:false,pending:(),receipts:#{},notice:""}}
}
fn session(v){"release:"+v.run.to_string()}
fn agents(v){[#{id:"atlas",name:"Atlas",model:"Release plan",status:if v.done{"done"}else{"running"}},
    #{id:"scribe",name:"Scribe",model:"Documentation",status:"done"},#{id:"nova",name:"Nova",model:"Usage",status:"done"}]}
fn files(v){[
    #{id:"guide",path:"docs/release.md",edits:[
        #{id:"atlas-head",agent:"atlas",start:18,end:24,revision:v.edit,active:!v.done,summary:"Move the setup steps above the example."},
        #{id:"scribe-head",agent:"scribe",start:20,end:28,revision:0,summary:"Keep setup next to the first worked example."},
        #{id:"atlas-tail",agent:"atlas",start:70,end:84,revision:0,summary:"Put the environment note in the checklist."},
        #{id:"nova-tail",agent:"nova",start:74,end:82,revision:0,summary:"Preserve the note in the troubleshooting section."}]},
    #{id:"spec",path:"examples/local/config.rhai",edits:[#{id:"scribe-conf",agent:"scribe",start:25,end:32,revision:0}]},
    #{id:"readme",path:"examples/local/README.md",edits:[#{id:"nova-docs",agent:"nova",start:1,end:5,revision:0}]}
]}
fn change(which,c,e){
    let v=state(c);
    switch which{
        "wide"=>{v.narrow=!v.narrow;},
        "ready"=>{v.done=true;v.revision+=1;v.pending=();v.notice="Agents finished";},
        "revise"=>{v.edit+=1;v.revision+=1;v.pending=();v.notice="First edit revised without changing its line range";},
        "deny"=>{v.deny=true;v.notice="The Host will decline the next proposal";},
        _=>{v.run+=1;v.revision=0;v.edit=0;v.done=false;v.deny=false;v.pending=();v.receipts=#{};v.notice="New session";},
    }
    c.set_state("demo",v);
}
fn propose(kind,c,e){
    let v=state(c);if v.pending!=()||e.session!=session(v)||e.revision!=v.revision{return;}
    v.pending=#{kind:kind,request:e};v.notice=if kind=="merge"{"Merge proposed"}else{"Choice proposed"};
    c.set_state("demo",v);
}
fn accept(c,payload){
    let v=state(c);let e=payload.request;
    if v.pending==()||e.session!=session(v)||e.revision!=v.revision{return;}
    if v.deny{v.deny=false;v.notice="Proposal declined by Host";}
    else if payload.kind=="merge"{v.notice="Merge accepted by Host";v.revision+=1;}
    else{v.receipts[e.overlap]=#{agent:e.agent,witness:e.witness};v.notice="Host kept "+e.agent;v.revision+=1;}
    v.pending=();c.set_state("demo",v);
}
fn control(action,label){button::Button(#{key:action,text:label,size:"xs",variant:"ghost",on_click:Fn("change").curry(action)})}
fn render_demo(c,p){
    let v=state(c);
    if v.pending!=(){timeout("receipt",360,false,Fn("accept"),v.pending);}
    let controls=row([control("wide",if v.narrow{"Wide matrix"}else{"Narrow matrix"}),control("ready","Finish agents"),
        control("revise","Revise first edit"),control("deny","Refuse next proposal"),control("reset","Reset session")])
        .with_style(style().flex_wrap().gap(theme_length("space.spacing_unit")));
    let ui=radar::ConflictRadar(#{key:"branches",session:session(v),title:"Parallel work",revision:v.revision,agents:agents(v),files:files(v),
        resolved:v.receipts,pending:v.pending!=(),on_resolve:Fn("propose").curry("resolve"),on_merge:Fn("propose").curry("merge")});
    let wrap=column([ui]).with_key("sample").with_style(style().min_width(px(0)).width(relative(1.0)).max_width(if v.narrow{px(334)}else{px(574)}));
    let notice=text(v.notice).accessibility_role("status").accessibility_label(v.notice).with_style(style().typography("caption").text_color(theme_color("text_muted")));
    column([controls,wrap,notice]).with_style(c.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2))))
}
