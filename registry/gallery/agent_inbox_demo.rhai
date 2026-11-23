// A deterministic Host fixture; no task runs outside this Gallery example.
import "patterns/agent_inbox" as inbox;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_inbox_demo","export":"AgentInboxDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_inbox","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentInboxDemo(props){render_component("gallery/agent_inbox_demo",props)}
fn initial(serial){
    #{id:"release/inbox/"+serial.to_string(),serial:serial,revision:0,now:2300000,view:"board",pending:(),decline:false,notice:(),only:false,added:0,tasks:[
        #{id:"gate/plan",title:"Review rollout plan",agent:"Planning assistant",status:"waiting",started_ms:2175000,sources:3,tools:2,
            description:"Check the cohort, the rollback point and the release notes before approving.",
            steps:[#{id:"opened",label:"Fetched draft",time:"4 min ago"},#{id:"cohort",label:"Compared the release cohorts",time:"3 min ago"},
                #{id:"policy",label:"Checked the policy",time:"2 min ago"},#{id:"rollback",label:"Prepared the rollback evidence",time:"1 min ago"},
                #{id:"wait",label:"Awaiting your decision",time:"now"}]},
        #{id:"recover",title:"Recover preview",agent:"Preview assistant",status:"failed",elapsed_ms:42000,tools:1,
            description:"The example builder refused an incomplete configuration.",steps:[#{id:"failed",label:"Preview configuration rejected"}]},
        #{id:"artifact",title:"Stage artifact",agent:"Build assistant",status:"running",started_ms:2277000,sources:2,tools:4,
            description:"Assemble the local bundle and inspect its contents.",
            steps:[#{id:"files",label:"Read the sample manifest"},#{id:"assemble",label:"Assembling the preview"}]},
        #{id:"archive",title:"Archive notes",agent:"Notes assistant",status:"done",elapsed_ms:71000,sources:4,
            description:"The notes were archived in the sample workspace.",steps:[#{id:"stored",label:"Stored the finished notes",time:"just now"}]},
    ]}
}
fn model(ctx){let s=ctx.get_state("model");if s==(){initial(1)}else{s}}
fn save(ctx,m){ctx.set_state("model",m);}
fn update(ctx,m,s){m.notice=s;save(ctx,m);}
fn reset(ctx,p){save(ctx,initial(model(ctx).serial+1));}
fn decline(ctx,p){let m=model(ctx);m.decline=true;update(ctx,m,"The next request will be declined.");}
fn advance(ctx,p){let m=model(ctx);m.now+=15000;save(ctx,m);}
fn filter(ctx,p){let m=model(ctx);m.only=!m.only;save(ctx,m);}
fn queue_view(ctx,p){
    let m=model(ctx);if p.inbox!=m.id||p.revision!=m.revision||m.pending!=()||p.view==m.view{return;}
    let ticket=p;ticket.kind="view";ticket.refuse=m.decline;m.decline=false;m.pending=ticket;
    update(ctx,m,"Waiting for the demo Host…");
}
fn queue_action(ctx,p){
    let m=model(ctx);if p.inbox!=m.id||p.revision!=m.revision||m.pending!=(){return;}
    let ticket=p;ticket.refuse=m.decline;m.decline=false;m.pending=ticket;update(ctx,m,"Waiting for the demo Host…");
}
fn accept(ctx,p){
    let m=model(ctx);if p!=m.pending||p.inbox!=m.id||p.revision!=m.revision{return;}
    m.pending=();
    if p.refuse{update(ctx,m,"The demo Host declined the request.");return;}
    if p.kind=="view"{
        m.view=p.view;update(ctx,m,if p.view=="list"{"Accepted list view."}else{"Accepted board view."});return;
    }
    let rows=m.tasks;
    if p.kind=="new"&&p.task==""&&rows.len<96{
        m.added+=1;rows.push(#{id:"sample/"+m.added.to_string(),title:"New sample "+m.added.to_string(),agent:"Local assistant",status:"waiting",started_ms:m.now});
        m.tasks=rows;m.revision+=1;update(ctx,m,"Created sample task.");return;
    }
    for i in 0..rows.len{
        let t=rows[i];if t.id!=p.task{continue;}
        if p.kind=="open"&&t.status in ["running","done"]{update(ctx,m,"Host received open: "+t.id);return;}
        let next="";
        if p.kind=="approve"&&t.status=="waiting"||p.kind=="retry"&&t.status=="failed"{next="running";}
        else if p.kind=="reject"&&t.status=="waiting"{next="failed";}
        if next==""{break;}
        let trail=if "steps" in t{t.steps}else{[]};
        if trail.len==24{trail.remove(0);}
        let label=if p.kind=="approve"{"The Host approved this task."}else if p.kind=="retry"{"Retry accepted."}else{"The Host rejected this task."};
        trail.push(#{id:"host/"+(m.revision+1).to_string(),label:label,time:"now"});t.steps=trail;
        t.status=next;
        if next=="running"{t.started_ms=m.now;t.elapsed_ms=();}
        else{t.elapsed_ms=if "started_ms" in t&&t.started_ms!=(){m.now-t.started_ms}else{()};}
        rows[i]=t;m.tasks=rows;m.revision+=1;
        update(ctx,m,"Accepted "+p.kind+": "+t.id);return;
    }
    update(ctx,m,"The demo Host declined the request.");
}
fn control(key,title,callback,disabled){
    button::Button(#{key:key,text:title,size:"sm",variant:"ghost",on_click:callback,disabled:disabled})
}
fn demo(ctx,props){
    let m=model(ctx);if m.pending!=(){timeout("reply",420,false,Fn("accept"),m.pending);}
    column([inbox::AgentInbox(#{key:"inbox",inbox:m.id,revision:m.revision,now_ms:m.now,view:m.view,lanes:if m.only{["needs"]}else{()},
        tasks:m.tasks,notice:m.notice,busy:m.pending!=(),allow_open:true,new_label:"New task",on_action:Fn("queue_action"),on_view_change:Fn("queue_view")}),
        row([control("decline","Decline next request",Fn("decline"),m.pending!=()),control("clock","Advance demo clock",Fn("advance"),false),
            control("lanes",if m.only{"Show all lanes"}else{"Show only Needs you"},Fn("filter"),false),control("reset","Reset inbox demo",Fn("reset"),false)])
            .with_key("tools").with_style(style().gap(px(5)).min_width(px(0)).flex_wrap())
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(4))))
}
