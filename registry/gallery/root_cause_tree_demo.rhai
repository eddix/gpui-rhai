import "patterns/root_cause_tree" as fault;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/root_cause_tree_demo","export":"RootCauseTreeDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/root_cause_tree","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn RootCauseTreeDemo(p){render_component("gallery/root_cause_tree_demo",p)}
fn model(ctx){
    let s=ctx.get_state("sample");
    if s!=(){s}else{#{run:0,revision:0,proved:false,narrow:false,deny:false,pending:(),notice:""}}
}
fn session(s){"publish-investigation:"+s.run.to_string()}
fn causes(s){
    let proof=if s.proved{"confirmed"}else{"checking"};
    [
        #{id:"failure",title:"Publish job failed",gate:"or",detail:"Draft unchanged",verdict:"Publication stopped before making any changes."},
        #{id:"auth",parent:"failure",title:"Publisher lacks access",status:"confirmed",gate:"and",
            verdict:"Both the policy and the job identity must match to explain the failure.",files:["release/publish.rhai","config/publish.rhai"]},
        #{id:"artifact",parent:"failure",title:"Missing draft",status:"ruled-out",at_ms:3700,
            verdict:"The output artifact was present and opened normally.",evidence:#{kind:"Artifact log",content:"draft: found\nchecksum: matches"}},
        #{id:"runner",parent:"failure",title:"Runner unavailable",status:"ruled-out",at_ms:6400,
            verdict:"The runner responded before the publish step.",evidence:#{kind:"Runner log",content:"status: ready\nlatency: 28ms"}},
        #{id:"policy",parent:"auth",title:"Role not granted",status:"confirmed",at_ms:9300,
            files:["config/publish.rhai","release/roles.rhai"],verdict:"The publish role is missing from the accepted job policy.",
            evidence:#{kind:"Policy record",source:"config/publish.rhai",content:"role: reviewer\npublish: false\nrequest: publish_draft"}},
        #{id:"token",parent:"auth",title:"Wrong job identity",status:proof,at_ms:if s.proved{13200}else{()},
            files:["release/publish.rhai"],verdict:if s.proved{"The job used a review identity for the publish step."}else{()},
            evidence:#{kind:"Job log",source:"release/publish.rhai",content:"job: publish-draft\nidentity: reviewer\nrequested role: publisher"}}
    ]
}
fn operation(op,ctx,e){
    let s=model(ctx);
    switch op{
        "narrow"=>{s.narrow=!s.narrow;},
        "proof"=>{s.proved=true;s.revision+=1;s.notice="Proof accepted by Host";s.pending=();},
        "refuse"=>{s.deny=true;s.notice="The next action will be declined";},
        _=>{s.run+=1;s.revision=0;s.proved=false;s.deny=false;s.pending=();s.notice="Investigation reset";},
    }
    ctx.set_state("sample",s);
}
fn request(ctx,e){
    let s=model(ctx);
    if e.session!=session(s)||e.revision!=s.revision||s.pending!=()||!s.proved{return;}
    s.pending=e;s.notice="Action proposed: "+e.kind;ctx.set_state("sample",s);
}
fn finish(ctx,e){
    let s=model(ctx);if s.pending==()||e.session!=session(s)||e.revision!=s.revision{return;}
    s.notice=if s.deny{"Action refused by Host"}else{"Action accepted: "+e.kind};
    s.pending=();s.revision+=1;s.deny=false;ctx.set_state("sample",s);
}
fn control(op,title){button::Button(#{key:op,text:title,size:"xs",variant:"ghost",on_click:Fn("operation").curry(op)})}
fn render_demo(ctx,p){
    let s=model(ctx);
    if s.pending!=(){timeout("decide",520,false,Fn("finish"),s.pending);}
    let controls=row([control("proof","Confirm final evidence"),control("narrow",if s.narrow{"Wide fault tree"}else{"Narrow fault tree"}),
        control("refuse","Refuse next action"),control("reset","Reset investigation")]).with_style(style().flex_wrap().min_width(px(0))
            .gap(theme_length("space.spacing_unit")));
    let tree=fault::RootCauseTree(#{key:"case",session:session(s),revision:s.revision,causes:causes(s),pending:s.pending!=(),on_action:Fn("request")});
    let sized=column([tree]).with_key("sized").with_style(style().width(relative(1.0)).min_width(px(0))
        .max_width(if s.narrow{px(406)}else{px(888)}));
    let status=text(s.notice).accessibility_role("status").accessibility_label(s.notice)
        .with_style(style().typography("caption").text_color(theme_color("text_muted")));
    column([controls,sized,status]).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2.75))))
}
