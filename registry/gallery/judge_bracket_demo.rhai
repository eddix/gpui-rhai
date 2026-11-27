import "patterns/judge_bracket" as bracket;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/judge_bracket_demo","export":"JudgeBracketDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/judge_bracket","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn JudgeBracketDemo(p){render_component("gallery/judge_bracket_demo",p)}
fn model(ctx){
    let s=ctx.get_state("sample");
    if s!=(){s}else{#{run:0,revision:0,step:0,narrow:false,odd:false,revised:false,deny:false,pending:(),notice:""}}
}
fn session(s){"drafts-"+s.run.to_string()}
fn entrants(odd){
    let entries=[
        #{id:"outline",name:"Concise outline",meta:"Outline · 3 sections",output:"Name the audience. Explain the change, show one example, then state what to do next."},
        #{id:"check",name:"Checklist draft",meta:"Checklist · 5 items",output:"Confirm scope, add an example, check the links, ask for a reviewer, and publish the summary."},
        #{id:"diagram",name:"Diagram draft",meta:"Diagram · 4 steps",output:"Start at the reader's goal. Show the previous workflow beside the new one. Mark where each decision happens."}
    ];
    if !odd{entries.push(#{id:"field",name:"Field notes",meta:"Notes · 2 examples",output:"Include one familiar use case and one unfamiliar one. Record the unanswered questions."});}
    entries
}
fn verdicts(s){
    let a=#{};
    if s.step>0{a["0:0"]=#{a:"outline",b:"check",winner:if s.revised{"check"}else{"outline"},scores:[8.1,7.4],reason:"The winning draft gives the reader a clear next step."};}
    if !s.odd&&s.step>1{a["0:1"]=#{a:"diagram",b:"field",winner:"diagram",scores:[9.0,7.2],reason:"The workflow makes the decision points explicit."};}
    if s.step>2||s.odd&&s.step>1{a["1:0"]=#{a:"outline",b:"diagram",winner:"diagram",scores:[8.4,9.2],reason:"The side-by-side example makes the change easiest to understand."};}
    a
}
fn operation(op,ctx,e){
    let s=model(ctx);
    switch op{
        "narrow"=>{s.narrow=!s.narrow;},
        "step"=>{if s.step<(if s.odd{2}else{3}){s.step+=1;s.revision+=1;s.pending=();s.notice="Verdict recorded by Host";}},
        "revise"=>{if s.step>0{s.revised=!s.revised;s.revision+=1;s.pending=();s.notice="First verdict replaced";}},
        "refuse"=>{s.deny=true;s.notice="The next choice will be declined";},
        "field"=>{s.odd=!s.odd;s.run+=1;s.revision=0;s.step=0;s.revised=false;s.pending=();s.notice="Field reset";},
        _=>{s.run+=1;s.revision=0;s.step=0;s.revised=false;s.deny=false;s.pending=();s.notice="Evaluation reset";},
    }
    ctx.set_state("sample",s);
}
fn request(ctx,e){
    let s=model(ctx);
    if e.session!=session(s)||e.revision!=s.revision||e.id!="diagram"||e.match_id!="1:0"||s.pending!=()||s.revised{return;}
    s.pending=e;s.notice="Choice proposed: Diagram draft";ctx.set_state("sample",s);
}
fn finish(ctx,e){
    let s=model(ctx);if s.pending==()||e.session!=session(s)||e.revision!=s.revision{return;}
    s.notice=if s.deny{"Choice declined by Host"}else{"Choice accepted by Host: Diagram draft"};
    s.pending=();s.revision+=1;s.deny=false;ctx.set_state("sample",s);
}
fn control(op,title){button::Button(#{key:op,text:title,size:"xs",variant:"ghost",on_click:Fn("operation").curry(op)})}
fn render_demo(ctx,p){
    let s=model(ctx);
    if s.pending!=(){timeout("decide",520,false,Fn("finish"),s.pending);}
    let controls=row([control("step","Record next verdict"),control("revise","Replace first verdict"),
        control("narrow",if s.narrow{"Wide bracket"}else{"Narrow bracket"}),control("field",if s.odd{"Show full field"}else{"Show odd field"}),
        control("refuse","Refuse next choice"),control("reset","Reset evaluation")]).with_style(style().flex_wrap().min_width(px(0))
            .gap(theme_length("space.spacing_unit")));
    let live=if s.step==0{"0:0"}else if !s.odd&&s.step==1{"0:1"}else if s.step<(if s.odd{2}else{3}){"1:0"}else{()};
    let view=bracket::JudgeBracket(#{key:"evaluation",session:session(s),revision:s.revision,capacity:4,contenders:entrants(s.odd),
        task:"Which draft explains the change best?",judge:"Review panel",verdicts:verdicts(s),judging:live,pending:s.pending!=(),on_pick:Fn("request")});
    let sized=column([view]).with_key("sized").with_style(style().width(relative(1.0)).min_width(px(0))
        .max_width(if s.narrow{px(356)}else{px(804)}));
    let status=text(s.notice).accessibility_role("status").accessibility_label(s.notice)
        .with_style(style().typography("caption").text_color(theme_color("text_muted")));
    column([controls,sized,status]).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2.75))))
}
