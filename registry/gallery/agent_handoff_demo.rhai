// Local sample project; the Host here changes only fixture state.
import "patterns/agent_handoff" as handoff;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_handoff_demo","export":"AgentHandoffDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/agent_handoff","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentHandoffDemo(props){render_component("gallery/agent_handoff_demo",props)}
fn initial(serial){
    #{serial:serial,relay:"release/relay/"+serial.to_string(),revision:0,decline:false,pending:(),notice:(),tick:0,
        agents:[
            #{id:"plan",name:"Planner",model:"Planning assistant",cost_cents:24,description:"Define the release checks."},
            #{id:"build",name:"Builder",model:"Local tools",cost_cents:58,description:"Assemble a preview."},
            #{id:"review",name:"Reviewer",model:"Review assistant",accepting:false,description:"Verify the delivered context."},
        ],
        tasks:[
            #{id:"draft/old",title:"Release draft",status:"done",agent:"plan",description:"Archived release draft."},
            #{id:"draft/current",title:"Release draft",status:"todo",description:"The current copy, not the archived draft."},
            #{id:"preview",title:"Build preview",status:"working",agent:"build",note:"Prepare checks without publishing.",
                tools:[#{id:"compose",name:"Compose fixture",detail:"2 calls"},#{id:"verify",name:"Render preview",detail:"1 call"}]},
        ],
        handoffs:[
            #{id:"pass/old",from:"plan",to:"build",task:"draft/old",items:[
                #{id:"context",label:"Archived checklist",kind:"file",lost:true,why:"The older packet left it out."},
                #{id:"notes",label:"Baseline description",kind:"note"},
            ]},
            #{id:"pass/current",from:"plan",to:"build",task:"draft/current",items:[
                #{id:"context",label:"Rollback checklist",kind:"file",lost:true,why:"The recipient did not get the rollback evidence."},
                #{id:"notes",label:"Release scope",kind:"decision"},
                #{id:"tickets",label:"Preview issue",kind:"source"},
            ]},
            #{id:"pass/verify",from:"build",to:"review",task:"preview",items:[
                #{id:"evidence",label:"Preview rendered",kind:"fact"},
            ]},
        ]}
}
fn model(ctx){let s=ctx.get_state("model");if s==(){initial(1)}else{s}}
fn save(ctx,m){ctx.set_state("model",m);}
fn notify(ctx,m,s){m.notice=s;save(ctx,m);}
fn reset(ctx,p){save(ctx,initial(model(ctx).serial+1));}
fn decline(ctx,p){let m=model(ctx);m.decline=true;notify(ctx,m,"The next proposal will be declined.");}
fn refresh(ctx,p){
    let m=model(ctx);if m.pending!=(){return;}
    m.tick+=1;let incoming=m.handoffs;incoming.push(#{id:"pass/next/"+m.tick.to_string(),from:"plan",to:"build",task:"draft/current",
        items:[#{id:"notes",label:"New release scope",kind:"decision"},#{id:"context",label:"Rollback checklist",kind:"file"}]});
    m.handoffs=incoming;m.revision+=1;notify(ctx,m,"A newer transfer arrived.");
}
fn submit(kind,ctx,p){
    let m=model(ctx);if m.pending!=()||p.relay!=m.relay||p.revision!=m.revision{return;}
    let ticket=p;ticket.kind=kind;ticket.refuse=m.decline;m.pending=ticket;m.decline=false;
    notify(ctx,m,"Waiting for the demo Host…");
}
fn restore(m,p){
    let records=m.handoffs;
    for i in 0..records.len{
        let h=records[i];if h.id!=p.handoff||h.task!=p.task{continue;}
        let materials=h.items;
        for j in 0..materials.len{
            let item=materials[j];if item.id!=p.item||!("lost" in item)||!item.lost{continue;}
            item.lost=false;materials[j]=item;h.items=materials;records[i]=h;return records;
        }
    }
    ()
}
fn assign(m,p){
    let eligible=false;
    for a in m.agents{if a.id==p.agent{eligible=!("accepting" in a)||a.accepting;}}
    if !eligible||p.from==p.agent{return ();}
    let records=m.tasks;
    for i in 0..records.len{
        let t=records[i];if t.id!=p.task||!(t.status in ["todo","working"]){continue;}
        let owner=if "agent" in t&&t.agent!=(){t.agent}else{""};if owner!=p.from{return ();}
        t.agent=p.agent;t.status="working";records[i]=t;return records;
    }
    ()
}
fn finish(ctx,p){
    let m=model(ctx);if m.pending!=p||p.relay!=m.relay||p.revision!=m.revision{return;}
    m.pending=();if p.refuse{notify(ctx,m,"The demo Host declined the proposal.");return;}
    if p.kind=="restore"{
        let records=restore(m,p);if records!=(){m.handoffs=records;m.revision+=1;notify(ctx,m,"Restored "+p.item+" in "+p.handoff);return;}
    }else{
        let records=assign(m,p);if records!=(){m.tasks=records;m.revision+=1;notify(ctx,m,"Assigned "+p.task+" to "+p.agent);return;}
    }
    notify(ctx,m,"The demo Host declined the proposal.");
}
fn control(key,title,callback,disabled){
    button::Button(#{key:key,text:title,size:"sm",variant:"ghost",on_click:callback,disabled:disabled})
}
fn demo(ctx,props){
    let m=model(ctx);if m.pending!=(){timeout("command",420,false,Fn("finish"),m.pending);}
    column([handoff::AgentHandoff(#{key:"relay",relay:m.relay,revision:m.revision,agents:m.agents,tasks:m.tasks,handoffs:m.handoffs,busy:m.pending!=(),
        notice:m.notice,on_assign:Fn("submit").curry("assign"),on_restore:Fn("submit").curry("restore")}),
        row([control("decline","Decline next proposal",Fn("decline"),m.pending!=()),control("deliver","Deliver new transfer",Fn("refresh"),m.pending!=()),
            control("reset","Reset relay demo",Fn("reset"),false)]).with_key("tools").with_style(style().gap(px(5)).min_width(px(0)).flex_wrap())
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(6))))
}
