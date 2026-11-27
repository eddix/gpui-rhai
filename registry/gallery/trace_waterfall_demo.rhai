import "patterns/trace_waterfall" as trace;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/trace_waterfall_demo","export":"TraceWaterfallDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/trace_waterfall","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn TraceWaterfallDemo(p){render_component("gallery/trace_waterfall_demo",p)}
fn state(ctx){
    let s=ctx.get_state("sample");
    if s!=(){s}else{#{run:0,revision:0,now:8000,finished:false,end:(),narrow:false,deny:false,folded:[],detail:(),notice:""}}
}
fn id(s){"draft-review:"+s.run.to_string()}
fn spans(s){
    let status=if s.finished{"done"}else{"running"};
    [
        #{id:"root",name:"Prepare onboarding draft",kind:"agent",status:status,start_ms:0,end_ms:s.end,
            model:"Atlas 2.5",input:"Prepare an onboarding draft from the available project notes.",output:if s.finished{"Draft ready for review."}else{()}},
        #{id:"outline",parent:"root",name:"Plan the guide",kind:"llm",status:"done",start_ms:150,end_ms:1800,
            detail:"Atlas",model:"Atlas 2.5",tokens_in:2700,tokens_out:413,cost_micros:14000,input:"Plan a three-part guide.",output:"Setup, first run, and troubleshooting."},
        #{id:"research",parent:"root",name:"Collect references",kind:"subagent",status:"done",start_ms:1900,end_ms:5950,
            detail:"Researcher",input:"Find the setup reference and old diagrams."},
        #{id:"search",parent:"research",name:"Search library",kind:"tool",status:"done",start_ms:2350,end_ms:3100,
            detail:"setup",input:"query: setup",output:"Two matching guides."},
        #{id:"reconcile",parent:"research",name:"Reconcile notes",kind:"llm",status:"done",start_ms:1950,end_ms:5600,
            detail:"Nova",model:"Nova",cost_micros:31000,tokens_in:8044,tokens_out:710},
        #{id:"archive",parent:"research",name:"Open archive",kind:"tool",status:"failed",start_ms:4900,end_ms:5850,
            detail:"diagrams.md",error:"The archive is unavailable.",input:"diagrams.md"},
        #{id:"draft",parent:"root",name:"Write draft",kind:"llm",status:status,start_ms:6100,end_ms:s.end,
            detail:"Atlas",model:"Atlas 2.5",tokens_in:3822,tokens_out:1140,cost_micros:24300}
    ]
}
fn change(which,ctx,e){
    let s=state(ctx);
    switch which{
        "advance"=>{s.now+=6600;s.revision+=1;s.notice="Trace clock advanced";},
        "finish"=>{if !s.finished{s.end=s.now;s.finished=true;}s.revision+=1;s.notice="Trace finished";},
        "narrow"=>{s.narrow=!s.narrow;},
        "deny"=>{s.deny=true;s.notice="The next proposal will be declined";},
        _=>{s.run+=1;s.revision=0;s.now=8000;s.finished=false;s.end=();s.folded=[];s.detail=();s.deny=false;s.notice="New trace";},
    }
    ctx.set_state("sample",s);
}
fn accept(which,ctx,e){
    let s=state(ctx);if e.session!=id(s)||e.revision!=s.revision{return;}
    if s.deny{s.deny=false;s.notice="Trace proposal declined";}
    else if which=="fold"{
        s.folded=e.collapsed;
        if s.detail!=()&&e.id=="research"&&"research" in s.folded&&s.detail in ["search","reconcile","archive"]{s.detail=();}
        s.notice="Trace folding accepted";
    }else{s.detail=e.id;s.notice=if e.id==(){"Details closed"}else{"Details opened"};}
    s.revision+=1;ctx.set_state("sample",s);
}
fn tool(which,title){button::Button(#{key:which,text:title,variant:"ghost",size:"xs",on_click:Fn("change").curry(which)})}
fn render_demo(ctx,p){
    let s=state(ctx);
    let buttons=row([tool("advance","Advance trace clock"),tool("finish","Finish trace"),
        tool("narrow",if s.narrow{"Wide trace"}else{"Narrow trace"}),tool("deny","Refuse next trace proposal"),tool("reset","New trace")])
        .with_style(style().min_width(px(0)).flex_wrap().gap(theme_length("space.spacing_unit")));
    let board=trace::TraceWaterfall(#{key:"run",session:id(s),revision:s.revision,now_ms:s.now,spans:spans(s),
        collapsed:s.folded,open_id:s.detail,on_collapsed_change:Fn("accept").curry("fold"),on_open_change:Fn("accept").curry("detail")});
    let frame=column([board]).with_key("sized").with_style(style().min_width(px(0)).width(if s.narrow{px(436)}else{relative(1.0)})
        .max_width(if s.narrow{relative(1.0)}else{px(920)}));
    let status=text(s.notice).accessibility_role("status").accessibility_label(s.notice)
        .with_style(style().typography("caption").text_color(theme_color("text_muted")));
    column([buttons,frame,status]).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2.5))))
}
