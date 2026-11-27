// Local execution fixture with a controlled approval boundary.
import "patterns/agent_trail" as trail;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_trail_demo","export":"AgentTrailDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_trail","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentTrailDemo(props){render_component("gallery/agent_trail_demo",props)}
fn initial(serial){
    #{serial:serial,run:"preview/trail/"+serial.to_string(),revision:0,now:29000,notice:(),pending:(),refuse:false,controlled:false,expanded:[],
        steps:[
            #{id:"gather",title:"Collect context",state:"done",elapsed_ms:6000,tool:"Workspace"},
            #{id:"build",title:"Prepare preview",state:"running",started_ms:9000,detail:"Checking output",substeps:[
                #{id:"input",title:"Read inputs"},#{id:"plan",title:"Read the plan"},#{id:"themes",title:"Resolve palettes",detail:"4 variants"},
                #{id:"types",title:"Verify types"},#{id:"layout",title:"Measure layout",detail:"3 widths"},
                #{id:"capture",title:"Capture rendered output",state:"running",detail:"2 / 4"},
                #{id:"compare",title:"Compare baselines",state:"pending"},#{id:"a11y",title:"Review accessibility",state:"pending"},
                #{id:"report",title:"Write preview summary",state:"pending"},#{id:"cleanup",title:"Clean temporary render files",state:"pending"}
            ]},
            #{id:"review/确认",title:"Draft handoff",state:"waiting",ask:"The preview is ready. Approve the local demo handoff?"},
            #{id:"handoff",title:"Record the decision",state:"pending",requires_approval:true}
        ]}
}
fn get(ctx){let m=ctx.get_state("model");if m==(){initial(1)}else{m}}
fn save(ctx,m){ctx.set_state("model",m);}
fn report(ctx,m,value){m.notice=value;save(ctx,m);}
fn reset(ctx,e){save(ctx,initial(get(ctx).serial+1));}
fn tick(ctx,e){let m=get(ctx);m.now+=7000;save(ctx,m);}
fn advance(ctx,e){
    let m=get(ctx);
    if m.pending!=()||m.steps[1].state!="running"{return;}
    let subs=m.steps[1].substeps;
    for i in 0..subs.len{
        if !("state" in subs[i])||subs[i].state!="running"{continue;}
        subs[i].state="done";m.steps[1].substeps=subs;
        if i+1<subs.len{
            m.steps[1].substeps[i+1].state="running";
            m.steps[1].detail=m.steps[1].substeps[i+1].title;
        }else{
            m.steps[1].state="done";m.steps[1].elapsed_ms=m.now-m.steps[1].started_ms;
            m.steps[1].detail="Preview saved";
        }
        m.revision+=1;report(ctx,m,"Finished "+subs[i].title);return;
    }
}
fn refuse(ctx,e){let m=get(ctx);m.refuse=true;report(ctx,m,"The next decision will be refused.");}
fn control_disclosure(ctx,e){
    let m=get(ctx);m.controlled=!m.controlled;
    report(ctx,m,if m.controlled{"The Host controls expansion."}else{"The trail owns local expansion."});
}
fn expansion(ctx,p){
    let m=get(ctx);if p.run!=m.run||p.revision!=m.revision||!m.controlled{return;}
    m.expanded=p.expanded;report(ctx,m,"Accepted disclosure for "+p.step);
}
fn begin(ctx,p){
    let m=get(ctx);
    if p.run!=m.run||p.revision!=m.revision||p.step!=m.steps[2].id||m.steps[2].state!="waiting"||m.pending!=(){return;}
    let ticket=p;ticket.refuse=m.refuse;m.pending=ticket;m.refuse=false;
    report(ctx,m,"Waiting for the demo Host…");
}
fn complete(ctx,p){
    let m=get(ctx);
    if p!=m.pending||p.run!=m.run||p.revision!=m.revision||p.step!=m.steps[2].id{return;}
    m.pending=();
    if p.refuse{report(ctx,m,"The demo Host refused the decision.");return;}
    m.revision+=1;
    if p.choice=="approve"{m.steps[2].state="done";m.steps[2].elapsed_ms=3000;m.steps[3].state="running";m.steps[3].started_ms=m.now;}
    else{m.steps[2].state="declined";}
    report(ctx,m,"Accepted "+p.choice+" for "+p.step);
}
fn control(key,label,callback,disabled){
    button::Button(#{key:key,text:label,size:"sm",variant:"ghost",on_click:callback,disabled:disabled})
}
fn demo(ctx,props){
    let m=get(ctx);if m.pending!=(){timeout("decide",400,false,Fn("complete"),m.pending);}
    column([trail::AgentTrail(#{key:"trail",run:m.run,revision:m.revision,now_ms:m.now,title:"Validate a release preview",agent:"Build assistant",
            steps:m.steps,expanded:if m.controlled{m.expanded}else{()},on_expand:Fn("expansion"),on_decision:Fn("begin"),
            notice:m.notice,busy_step:if m.pending==(){()}else{m.pending.step}}),
        row([control("tick","Advance preview clock",Fn("tick"),false),
            control("advance","Finish active substep",Fn("advance"),m.pending!=()||m.steps[1].state!="running"),
            control("refuse","Refuse next decision",Fn("refuse"),m.pending!=()),
            control("controlled","Switch disclosure owner",Fn("control_disclosure"),m.pending!=()),
            control("reset","Reset trail",Fn("reset"),false)]).with_key("tools").with_style(style().gap(px(5)).flex_wrap().min_width(px(0)))
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(4))))
}
