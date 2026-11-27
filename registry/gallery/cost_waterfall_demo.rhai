import "patterns/cost_waterfall" as cost;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/cost_waterfall_demo","export":"CostWaterfallDemo","version":"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/cost_waterfall","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn CostWaterfallDemo(p){render_component("gallery/cost_waterfall_demo",p)}
fn state(ctx){
    let old=ctx.get_state("demo");
    if old==(){#{run:0,mode:"cost",folded:["scout"],open:"",narrow:false,empty:false,refuse:false,extra:0,notice:""}}else{old}
}
fn id(s){"cost/demo:"+s.run.to_string()}
fn mode(ctx,p){
    let s=state(ctx);
    if p.run!=id(s){return;}
    if s.refuse{s.refuse=false;s.notice="The Host kept the previous measure";}
    else{s.mode=p.mode;s.notice="Accepted measure: "+p.mode;}
    ctx.set_state("demo",s);
}
fn fold(ctx,p){
    let s=state(ctx);if p.run!=id(s){return;}
    s.folded=p.collapsed;ctx.set_state("demo",s);
}
fn open(ctx,p){
    let s=state(ctx);if p.run!=id(s){return;}
    s.open=p.id;ctx.set_state("demo",s);
}
fn control(key,ctx,event){
    let s=state(ctx);
    switch key{
        "refuse"=>{s.refuse=true;s.notice="The next measure will be refused";},
        "grow"=>{s.extra+=20000;s.notice="Accepted another response chunk";},
        "narrow"=>{s.narrow=!s.narrow;},
        "empty"=>{s.empty=!s.empty;s.run+=1;s.open="";},
        _=>{s.run+=1;s.mode="cost";s.folded=["scout"];s.open="";s.extra=0;s.notice="New ledger";}
    }
    ctx.set_state("demo",s);
}
fn btn(key,t){button::Button(#{key:key,text:t,variant:"ghost",size:"xs",on_click:Fn("control").curry(key)})}
fn sample(extra){
    [#{id:"plan",name:"Draft a plan",kind:"llm",status:"done",model:"Atlas Mini",cost_micros:12000,tokens_in:760,tokens_out:120,duration_ms:160,
        input:"Find the strongest evidence.",output:"Split the review into two parts."},
     #{id:"scout",name:"Metadata scout",kind:"agent",status:"done",cost_micros:6000,tokens_in:400,tokens_out:100,duration_ms:2340},
     #{id:"summary/资料",parent:"scout",name:"Summarize evidence",kind:"llm",status:"done",model:"Vista 4",cost_micros:40000,tokens_in:5100,tokens_out:730,duration_ms:890,
        input:"Review the relevant metadata and compare it with the manifest.",output:"Three evidence groups, one shared release condition."},
     #{id:"read",parent:"scout",name:"Read manifest",kind:"tool",status:"failed",detail:"fixture/manifest.toml",duration_ms:67,
        input:"fixture/manifest.toml",error:"The fixture was not available."},
     #{id:"refund",name:"Usage credit",kind:"tool",status:"done",cost_micros:-75000,duration_ms:42},
     #{id:"final",name:"Write the answer",kind:"llm",status:"running",model:"Atlas",cost_micros:8000+extra,tokens_in:700,
        tokens_out:400+extra/20,duration_ms:1790+extra/50, input:"Explain the accepted ledger."}]
}
fn render_demo(ctx,p){
    let s=state(ctx);
    let view=cost::CostWaterfall(#{key:"cost",run:id(s),task:"Evidence review",steps:if s.empty{[]}else{sample(s.extra)},
        mode:s.mode,collapsed:s.folded,open_id:s.open,max_height:308,on_mode_change:Fn("mode"),on_fold_change:Fn("fold"),on_open_change:Fn("open")});
    column([row([btn("refuse","Refuse next measure"),btn("grow","Grow live cost"),btn("narrow",if s.narrow{"Wide ledger"}else{"Narrow ledger"}),
        btn("empty",if s.empty{"Restore steps"}else{"Empty ledger"}),btn("reset","New ledger")])
        .with_style(style().min_width(px(0)).gap(px(6)).flex_wrap()),
        column([view]).with_key("sized").with_style(style().width(if s.narrow{px(350)}else{relative(1.0)}).max_width(if s.narrow{relative(1.0)}else{px(750)})),
        text(s.notice).accessibility_role("status").accessibility_label(s.notice).with_style(style().typography("caption").text_color(theme_color("text_muted")))])
        .with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(10))))
}
