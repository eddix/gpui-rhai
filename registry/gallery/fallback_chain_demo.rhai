import "patterns/fallback_chain" as chain;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/fallback_chain_demo","export":"FallbackChainDemo","version":"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/fallback_chain","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn FallbackChainDemo(p){render_component("gallery/fallback_chain_demo",p)}
fn state(ctx){
    let s=ctx.get_state("demo");
    if s!=(){s}else{#{turn:0,revision:0,now:5200,state:"live",deny:false,narrow:false,notice:""}}
}
fn route(s){"request/run:"+s.turn.to_string()}
fn retry(ctx,e){
    let s=state(ctx);
    if e.run!=route(s)||e.revision!=s.revision{return;}
    if s.deny{s.deny=false;s.notice="The Host declined the retry";}
    else{s.turn+=1;s.revision=0;s.now=5200;s.state="live";s.notice="The Host started a new request";}
    ctx.set_state("demo",s);
}
fn change(op,ctx,e){
    let s=state(ctx);
    switch op{
        "tick"=>{s.now+=6000;s.revision+=1;},
        "answer"=>{s.state="done";s.revision+=1;},
        "fail"=>{s.state="failed";s.revision+=1;},
        "deny"=>{s.deny=true;s.notice="The next retry will be declined";},
        "narrow"=>{s.narrow=!s.narrow;},
        "empty"=>{s.state=if s.state=="empty"{"live"}else{"empty"};s.revision+=1;},
        _=>{s.turn+=1;s.revision=0;s.now=5200;s.state="live";s.notice="Reset the request";},
    }
    ctx.set_state("demo",s);
}
fn btn(op,title){button::Button(#{key:op,text:title,variant:"ghost",size:"xs",on_click:Fn("change").curry(op)})}
fn attempts(s){
    let done=s.state=="done";let failed=s.state=="failed";
    [
        #{id:"atlas",model:"Atlas Pro",status:"failed",provider:"Atlas Cloud",region:"us-east",reason:"Rate limited · 429",
            duration_ms:1800,timeout_ms:6000,cost_micros:1500,details:[#{label:"Code",value:"429"}]},
        #{id:"nova/备用",model:"Nova Mini",provider:"Nova Cloud",region:"eu-west",started_ms:3200,timeout_ms:4000,
            status:if done{"answered"}else if failed{"failed"}else{"trying"},duration_ms:if done||failed{5700}else{()},
            cost_micros:if done{11500}else{8500},reason:if failed{"Timed out"}else{""},note:if done{"412 tokens"}else{""},
            details:[#{label:"Tokens",value:"1,240 in · 412 out"}]},
        #{id:"sprout",model:"Sprout",provider:"Sprout Cloud",status:if done||failed{"skipped"}else{"waiting"},timeout_ms:4200}
    ]
}
fn render_demo(ctx,p){
    let s=state(ctx);
    let opts=[btn("tick","Advance clock"),btn("answer","Deliver answer"),btn("fail","Fail the chain"),btn("deny","Refuse next retry"),
        btn("narrow",if s.narrow{"Wide route"}else{"Narrow route"}),btn("empty",if s.state=="empty"{"Restore models"}else{"Empty chain"}),
        btn("reset","New request")];
    column([row(opts).with_style(style().min_width(px(0)).gap(px(6)).flex_wrap()),
        column([chain::FallbackChain(#{key:"case",run:route(s),revision:s.revision,now_ms:s.now,title:"Prepare a reply",route:"router/auto",
            attempts:if s.state=="empty"{[]}else{attempts(s)},on_retry:Fn("retry"),
            request:#{name:"Support · Ticket 4821",text:"Summarize the customer's question and propose a reply.",details:[#{label:"Origin",value:"Support desk"}]},
            response:#{name:"Support reply",text:"Your request has been reviewed; the next step is a confirmation.",details:[#{label:"Destination",value:"Case history"}]}})])
            .with_key("sized").with_style(style().width(if s.narrow{px(408)}else{relative(1.0)}).max_width(if s.narrow{relative(1.0)}else{px(748)})),
        text(s.notice).accessibility_role("status").accessibility_label(s.notice).with_style(style().typography("caption").text_color(theme_color("text_muted")))])
        .with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(14))))
}
