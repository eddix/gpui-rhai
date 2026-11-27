import "patterns/scheduled_runs" as week;
import "components/button" as btn;
import "gallery/kit" as kit;
define_component(#{
    metadata:#{id:"gallery/scheduled_runs_demo","export":"ScheduledRunsDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/scheduled_runs","components/button","gallery/kit"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        events:#{},slots:#{},parts:["root"],timers:["decide"]},render:Fn("demo")
});
fn ScheduledRunsDemo(p){render_component("gallery/scheduled_runs_demo",p)}
fn initial(){#{mode:"manual",revision:0,reject:false,pending:(),narrow:false,note:"",assignments:[
    #{id:"last-run",task:"history",day:"mon",period:"am",ordinal:1,units:55,origin:"booked"},
    #{id:"release-window",task:"release",day:"mon",period:"pm",ordinal:1,units:75,origin:"booked"}
]}}
fn state(ctx){
    let s=ctx.get_state("sample");if s==(){initial()}else{s}
}
fn apply(ctx,p){
    let s=state(ctx);
    if s.pending!=()||s.revision!=p.revision{return;}
    s.pending=#{proposal:p,refused:s.reject};s.reject=false;s.note="Waiting for schedule approval";
    ctx.set_state("sample",s);
}
fn decide(ctx,request){
    let s=state(ctx);let p=request.proposal;
    if s.pending!=request||s.revision!=p.revision{return;}
    if request.refused{s.note="Schedule change refused";}
    else{
        s.mode=p.mode;s.assignments=p.assignments;s.revision+=1;
        s.note="Accepted "+p.kind+" at revision "+s.revision.to_string();
    }
    s.pending=();ctx.set_state("sample",s);
}
fn action(verb,ctx,event){
    let s=state(ctx);
    switch verb{
        "refuse"=>{s.reject=!s.reject;s.note=if s.reject{"The next proposal will be refused"}else{""};},
        "narrow"=>{s.narrow=!s.narrow;},
        _=>{s=initial();}
    }
    ctx.set_state("sample",s);
}
fn tool(id,title){btn::Button(#{key:id,text:title,size:"sm",variant:"ghost",on_click:Fn("action").curry(id)})}
fn tasks(){[
    #{id:"history",title:"Reindex archive",agent:"atlas",units:55,detail:"Prior session"},
    #{id:"release",title:"Build release",agent:"atlas",units:75,detail:"Pinned by the Host"},
    #{id:"audit",title:"Audit package",agent:"orbit",units:35,detail:"Review bundle provenance",stats:[#{label:"Checks",value:"12"}]},
    #{id:"media",title:"Sync media",agent:"atlas",units:130,detail:"Cross-session job",stats:[#{label:"Files",value:"48"}]},
    #{id:"backup",title:"Archive backup",agent:"orbit",units:45,detail:"Compress the reports"}
]}
fn demo(ctx,p){
    let s=state(ctx);
    if s.pending!=(){timeout("decide",300,false,Fn("decide"),s.pending);}
    let plan=week::ScheduledRuns(#{key:"plan",board:"workspace",revision:s.revision,title:"Publishing week",note:"5-hour sessions",
       mode:s.mode,pending:s.pending!=(),minimum:15,assignments:s.assignments,
       days:[#{id:"mon",label:"Mon",date:"23"},#{id:"tue",label:"Tue",date:"24"},#{id:"wed",label:"Wed",date:"25"}],
       periods:[#{id:"am",label:"07:00",limit:100},#{id:"pm",label:"12:00",limit:100}],
       current:#{day:"mon",period:"pm"},tasks:tasks(),
       agents:[#{id:"atlas",label:"Atlas",mark:"A"},#{id:"orbit",label:"Orbit",mark:"O"}],
       on_propose:Fn("apply")});
    let stage=style().width(if s.narrow{px(480)}else{relative(1.0)}).max_width(relative(1.0));
    column([column([plan]).with_style(kit::board_stage(ctx,stage)),
        row([tool("refuse",if s.reject{"Accept next proposal"}else{"Refuse next proposal"}),tool("narrow",if s.narrow{"Wide board"}else{"Narrow board"}),
           tool("reset","Reset schedule")]).with_style(style().flex_wrap().gap(px(6))),
        text(s.note).accessibility_role("status").accessibility_label(s.note)
            .with_style(style().typography("caption").text_color(theme_color("text_muted")))])
       .with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(8))))
}
