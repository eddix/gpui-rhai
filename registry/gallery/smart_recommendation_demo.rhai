import "patterns/smart_recommendation" as rec;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/smart_recommendation_demo","export":"SmartRecommendationDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/smart_recommendation","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn SmartRecommendationDemo(p){render_component("gallery/smart_recommendation_demo",p)}
fn model(ctx){
    let s=ctx.get_state("model");
    if s!=(){s}else{#{run:0,revision:0,pending:(),accepted:(),narrow:false,reordered:false,deny:false,notice:""}}
}
fn session(s){"sync-strategy:"+s.run.to_string()}
fn options(s){
    let delta=#{id:"delta",label:"Incremental sync",reason:"Only 3% of the workspace has changed. Transfer the changed files and keep the existing index.",signal:3};
    let all=#{id:"full",label:"Full sync",reason:"Copy the entire workspace. This is a good alternative if you want to rebuild the existing index.",signal:2};
    let aws=#{id:"backup",label:"AWS backup",reason:"Prepare a separate backup before syncing. The current change set does not require it.",signal:1};
    let manual=#{id:"manual",label:"Manual review before moving any recently edited files",reason:"Inspect each changed file first. No signal is available for this workflow.",signal:0};
    if s.reordered{[aws,delta,all,manual]}else{[delta,all,aws,manual]}
}
fn command(op,ctx,e){
    let s=model(ctx);
    switch op{
        "narrow"=>{s.narrow=!s.narrow;},
        "reorder"=>{s.reordered=!s.reordered;s.revision+=1;s.pending=();s.notice="Options reordered";},
        "deny"=>{s.deny=true;s.notice="The next proposal will be refused";},
        _=>{s.run+=1;s.revision=0;s.accepted=();s.pending=();s.reordered=false;s.deny=false;s.notice="New recommendation";},
    }
    ctx.set_state("model",s);
}
fn request(op,ctx,e){
    let s=model(ctx);
    if e.session!=session(s)||e.revision!=s.revision||s.pending!=(){return;}
    s.pending=#{op:op,event:e};
    s.notice=if op=="undo"{"Undo requested"}else{"Acceptance requested"};
    ctx.set_state("model",s);
}
fn settle(ctx,ticket){
    let s=model(ctx);let e=ticket.event;
    if s.pending==()||e.session!=session(s)||e.revision!=s.revision{return;}
    if s.deny{s.deny=false;s.notice="Proposal refused by Host";}
    else if ticket.op=="undo"{if s.accepted==e.id{s.accepted=();}s.notice="Acceptance undone by Host";}
    else{s.accepted=e.id;s.notice="Proposal accepted by Host";}
    s.pending=();s.revision+=1;ctx.set_state("model",s);
}
fn action(k,title){button::Button(#{key:k,text:title,size:"xs",variant:"ghost",on_click:Fn("command").curry(k)})}
fn render_demo(ctx,p){
    let s=model(ctx);
    if s.pending!=(){timeout("decision",520,false,Fn("settle"),s.pending);}
    let choices=row([action("narrow",if s.narrow{"Wide recommendation"}else{"Narrow recommendation"}),
        action("reorder","Reorder options"),action("deny","Refuse next proposal"),action("reset","New recommendation")])
        .with_style(style().min_width(px(0)).flex_wrap().gap(theme_length("space.spacing_unit")));
    let widget=rec::SmartRecommendation(#{key:"case",session:session(s),revision:s.revision,options:options(s),accepted_id:s.accepted,pending:s.pending!=(),
        on_accept:Fn("request").curry("accept"),on_undo:Fn("request").curry("undo")});
    let frame=column([widget]).with_key("frame").with_style(style().min_width(px(0)).width(if s.narrow{px(300)}else{relative(1.0)}).max_width(relative(1.0)));
    let status=text(s.notice).accessibility_role("status").accessibility_label(s.notice)
        .with_style(style().typography("caption").text_color(theme_color("text_muted")));
    column([choices,frame,status]).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2.5))))
}
