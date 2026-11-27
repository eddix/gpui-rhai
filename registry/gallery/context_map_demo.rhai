import "patterns/context_map" as map;
import "components/button" as button;
import "gallery/kit" as kit;

define_component(#{
    metadata:#{id:"gallery/context_map_demo","export":"ContextMapDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/context_map","components/button","gallery/kit"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{},timers:["finish"]},render:Fn("render_demo")
});
fn ContextMapDemo(p){render_component("gallery/context_map_demo",p)}
fn state(ctx){
    let s=ctx.get_state("demo");
    if s==(){#{revision:0,agent:"writer",side:"after",compacted:false,writing:0,working:false,
        reject:false,narrow:false,limit:120000,removed:false,note:""}}else{s}
}
fn base_blocks(s){
    if s.agent=="reviewer"{
        return [#{id:"policy",label:"Review scope",kind:"system",tokens:18000},
            #{id:"diff",label:"Patch set",kind:"file",tokens:42000,detail:"Two versions ready to compare.",items:["lib.rs","style.rs"]},
            #{id:"comments",label:"Review notes",kind:"chat",tokens:36000+s.writing,detail:"Collected from the current review."}];
    }
    let entries=[#{id:"__free",label:"System prompt",kind:"system",tokens:6200,detail:"Instructions shared by every local tool."},
        #{id:"tools",label:"Tools",kind:"tools",tokens:9600,items:["Read file","Find symbol","Compare files","Search project"]},
        #{id:"memory",label:"Memory",kind:"memory",tokens:7000,detail:"Chosen workspace conventions."},
        #{id:"files/资料",label:"Repository scan",kind:"file",tokens:18200,items:["Cargo.toml","README.md","crates/runtime.rs","registry/examples"]},
        #{id:"chat",label:"Conversation",kind:"chat",tokens:43000+s.writing,detail:"Accepted user and assistant turns."}];
    if !s.removed{entries.push(#{id:"results",label:"Tool results",kind:"result",tokens:5000});}
    entries
}
fn after_blocks(s,before){
    if !s.compacted{return before;}
    [before[0],before[1],before[2],#{id:"summary",label:"Compaction summary",kind:"summary",tokens:12500,
        detail:"Retained notes from the repository and the earlier conversation.",items:["Project structure","Important constraints"]}]
}
fn control(key,ctx,event){
    let s=state(ctx);
    switch key{
        "grow"=>{s.writing+=4500;s.revision+=1;s.note="Accepted new conversation turns";},
        "compact"=>{if s.agent!="writer"||s.working||s.compacted||s.removed{return;}s.working=true;s.note="Preparing a compaction";},
        "limit"=>{s.limit=if s.limit==120000{180000}else{120000};s.revision+=1;s.note="Accepted window resize";},
        "remove"=>{s.removed=!s.removed;s.revision+=1;s.note=if s.removed{"Tool results removed"}else{"Tool results restored"};},
        "reject"=>{s.reject=true;s.note="The next comparison request will be refused";},
        "narrow"=>{s.narrow=!s.narrow;},
        _=>{ctx.set_state("demo",());return;}
    }
    ctx.set_state("demo",s);
}
fn finish(ctx,revision){
    let s=state(ctx);
    if !s.working||s.agent!="writer"||s.revision!=revision{return;}
    s.working=false;s.compacted=true;s.revision+=1;s.side="after";s.note="Compaction accepted";
    ctx.set_state("demo",s);
}
fn change_agent(ctx,p){
    let s=state(ctx);
    if p.context!="gallery/context"||s.revision!=p.revision||s.agent==p.agent{return;}
    s.agent=p.agent;s.working=false;s.compacted=false;s.writing=0;s.removed=false;s.side="after";s.revision+=1;
    s.note=if p.agent=="writer"{"Writer context accepted"}else{"Reviewer context accepted"};
    ctx.set_state("demo",s);
}
fn compare_side(ctx,p){
    let s=state(ctx);
    if p.context!="gallery/context"||s.revision!=p.revision||!s.compacted||p.compaction!="compaction:one"{return;}
    if s.reject{s.reject=false;s.note="Comparison request refused";}
    else{s.side=p.side;s.note=if p.side=="before"{"Showing before compaction"}else{"Showing after compaction"};}
    ctx.set_state("demo",s);
}
fn selected(ctx,p){
    let s=state(ctx);
    if p.context!="gallery/context"||s.revision!=p.revision{return;}
    s.note=if p.block==""{"Inspected unused capacity"}else if p.pinned{"Pinned "+p.block}else{"Unpinned "+p.block};
    ctx.set_state("demo",s);
}
fn btn(key,label,disabled){
    button::Button(#{key:key,text:label,variant:"ghost",size:"xs",disabled:disabled,on_click:Fn("control").curry(key)})
}
fn render_demo(ctx,p){
    let s=state(ctx);
    if s.working{timeout("finish",720,false,Fn("finish"),s.revision);}
    let before=base_blocks(s);
    let compaction=if s.compacted{#{id:"compaction:one",summarized:["files/资料","chat"],dropped:["results"],summary:"summary"}}else{()};
    let card=map::ContextMap(#{key:"map",context:"gallery/context",revision:s.revision,limit:s.limit,
        agents:[#{id:"writer",name:"Writer",model:"Atlas",limit:s.limit},#{id:"reviewer",name:"Reviewer",model:"Scribe",limit:500000}],
        agent:s.agent,status:if s.working{"Compacting"}else{()},show_comparison:true,
        blocks:after_blocks(s,before),before:if s.compacted{before}else{()},compaction:compaction,
        compare:s.side,on_compare:Fn("compare_side"),on_agent_change:Fn("change_agent"),on_select:Fn("selected")});
    let stage=style().width(if s.narrow{px(280)}else{relative(1.0)}).max_width(relative(1.0));
    column([row([btn("grow","Grow context",s.working||s.compacted||s.writing>=45000),
        btn("compact","Compact context",s.working||s.compacted||s.removed||s.agent!="writer"),
        btn("limit",if s.limit==120000{"Raise limit"}else{"Restore limit"},s.working||s.agent!="writer"),
        btn("remove",if s.removed{"Restore tool results"}else{"Remove tool results"},s.working||s.compacted||s.agent!="writer"),
        btn("reject","Refuse next comparison",!s.compacted||s.reject),
        btn("narrow",if s.narrow{"Wide map"}else{"Narrow map"},false),btn("reset","Reset example",false)])
        .with_style(style().min_width(px(0)).gap(px(6)).flex_wrap()),
        column([card]).with_key("sized-map").with_style(kit::board_stage(ctx,stage)),
        text(s.note).accessibility_role("status").accessibility_label(s.note)
            .with_style(style().typography("caption").text_color(theme_color("text_muted")))])
        .with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(10))))
}
