// One local event stream; no workers, provider calls, or real jobs are started.
import "patterns/agent_queue" as queue_ui;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_queue_demo","export":"AgentQueueDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/agent_queue","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentQueueDemo(props){render_component("gallery/agent_queue_demo",props)}
fn initial(generation){
    #{generation:generation,queue:"release/"+generation.to_string(),revision:0,now:1000000,paused:false,pending:(),refuse:false,notice:(),
        promotions:0,done:8,failed:2,
        workers:[#{id:"worker/1",name:"Worker 1",detail:"Text model"},#{id:"worker/2",name:"Worker 2",detail:"Local tools"}],
        rate_limits:[#{from:910000,until_ms:925000,label:"Provider cooldown"}],
        jobs:[
            #{id:"write/notes",name:"Write release notes",status:"running",kind:"llm",enqueued_ms:940000,started_ms:980000,
                estimate_ms:40000,worker:"worker/1",detail:"Summarize changes without sending a message."},
            #{id:"compare",name:"Compare snapshots",status:"queued",kind:"agent",enqueued_ms:962000,detail:"Compare the two local snapshots."},
            #{id:"refresh",name:"Refresh preview",status:"queued",kind:"tool",priority:"high",enqueued_ms:982000,detail:"Refresh only the preview."},
            #{id:"publish",name:"Publish draft",status:"failed",kind:"human",enqueued_ms:955000,started_ms:970000,ended_ms:990000,
                attempt:1,max_attempts:3,error:"The local review gate did not accept the draft."},
            #{id:"fixture",name:"Build fixture",status:"done",kind:"tool",enqueued_ms:910000,started_ms:922000,ended_ms:935000},
        ]}
}
fn state(ctx){let m=ctx.get_state("model");if m==(){initial(1)}else{m}}
fn save(ctx,m){ctx.set_state("model",m);}
fn say(ctx,m,text){m.notice=text;save(ctx,m);}
fn refuse(ctx,p){let m=state(ctx);m.refuse=true;say(ctx,m,"The next command will be refused.");}
fn reset(ctx,p){save(ctx,initial(state(ctx).generation+1));}
fn clock(ctx,p){let m=state(ctx);m.now+=15000;save(ctx,m);}
fn active_limit(m){for r in m.rate_limits{if r.from<=m.now&&m.now<r.until_ms{return true;}}false}
fn limit(ctx,p){
    let m=state(ctx);if active_limit(m){return;}
    let rs=m.rate_limits;rs.push(#{from:m.now,until_ms:m.now+30000,label:"Provider cooldown"});m.rate_limits=rs;say(ctx,m,"Sample provider throttled.");
}
fn enqueue(ctx,p){
    let m=state(ctx);let jobs=m.jobs;
    jobs.push(#{id:"queued/"+m.revision.to_string(),name:"Queued sample "+m.revision.to_string(),status:"queued",kind:"tool",enqueued_ms:m.now});
    m.jobs=jobs;m.revision+=1;m.pending=();say(ctx,m,"A new sample job is waiting.");
}
fn terminal(job,status,now){
    job.status=status;job.ended_ms=now;job.worker=();job
}
fn launch(ctx,p){
    let m=state(ctx);if m.pending!=()||m.paused||active_limit(m){return;}
    let jobs=m.jobs;let busy=#{};
    for j in jobs{if j.status=="running"&&"worker" in j&&j.worker!=(){busy[j.worker]=true;}}
    let slot="";for w in m.workers{if !(w.id in busy){slot=w.id;break;}}if slot==""{return;}
    let index=-1;
    for i in 0..jobs.len{
        if jobs[i].status=="queued"&&(index<0||queue_ui::job_order(jobs[i],jobs[index])<0){index=i;}
    }
    if index<0{return;}
    let j=jobs[index];j.status="running";j.worker=slot;j.started_ms=m.now;j.estimate_ms=30000;jobs[index]=j;m.jobs=jobs;
    m.revision+=1;say(ctx,m,"Started sample: "+j.name);
}
fn complete(ctx,p){
    let m=state(ctx);if m.pending!=()||active_limit(m){return;}
    let jobs=m.jobs;
    for i in 0..jobs.len{
        if jobs[i].status!="running"{continue;}
        jobs[i]=terminal(jobs[i],"done",m.now);m.jobs=jobs;m.done+=1;m.revision+=1;say(ctx,m,"Finished sample: "+jobs[i].name);return;
    }
}
fn submit(ctx,p){
    let m=state(ctx);if m.pending!=()||p.queue!=m.queue||p.revision!=m.revision{return;}
    let req=p;req.refuse=m.refuse;m.refuse=false;m.pending=req;say(ctx,m,"Waiting for the demo Host…");
}
fn finish(ctx,p){
    let m=state(ctx);if p!=m.pending||p.queue!=m.queue||p.revision!=m.revision{return;}
    m.pending=();
    if p.refuse{say(ctx,m,"The demo Host declined the command.");return;}
    let accepted=false;
    if p.kind=="pause"||p.kind=="resume"{m.paused=p.kind=="pause";accepted=true;}
    else if p.kind=="concurrency"&&p.workers>=1&&p.workers<=4{
        let workers=m.workers;let busy=#{};
        for j in m.jobs{if j.status=="running"&&"worker" in j&&j.worker!=(){busy[j.worker]=true;}}
        if p.workers>workers.len{
            let n=workers.len+1;workers.push(#{id:"worker/"+n.to_string(),name:"Worker "+n.to_string(),detail:"Additional sample slot"});m.workers=workers;accepted=true;
        }else if p.workers<workers.len&&!(workers[workers.len-1].id in busy){workers.pop();m.workers=workers;accepted=true;}
    }else{
        let jobs=m.jobs;
        for i in 0..jobs.len{
            let j=jobs[i];if j.id!=p.job{continue;}
            if p.kind=="promote"&&j.status=="queued"{m.promotions+=1;j.priority="high";j.priority_order= -m.promotions;accepted=true;}
            else if p.kind=="cancel"&&j.status in ["queued","running"]{j=terminal(j,"cancelled",m.now);accepted=true;}
            else if p.kind=="retry"&&j.status=="failed"&&j.attempt<j.max_attempts{
                let archived=j;archived.id=j.id+"/attempt/"+j.attempt.to_string();archived.name=j.name+" (attempt "+j.attempt.to_string()+")";
                jobs.push(archived);
                j.status="queued";j.attempt+=1;j.enqueued_ms=m.now;j.started_ms=();j.ended_ms=();j.worker=();j.error=();accepted=true;
            }
            if accepted{jobs[i]=j;m.jobs=jobs;}
            break;
        }
    }
    if accepted{m.revision+=1;say(ctx,m,"Accepted command: "+p.kind);}
    else{say(ctx,m,"The demo Host declined the command.");}
}
fn control(key,label,callback){
    button::Button(#{key:key,text:label,size:"sm",variant:"ghost",on_click:callback})
}
fn demo(ctx,props){
    let m=state(ctx);if m.pending!=(){timeout("command",430,false,Fn("finish"),m.pending);}
    column([queue_ui::AgentQueue(#{key:"queue",queue:m.queue,revision:m.revision,now_ms:m.now,name:"Release checks",jobs:m.jobs,workers:m.workers,
        rate_limits:m.rate_limits,totals:#{done:m.done,failed:m.failed},paused:m.paused,busy:m.pending!=(),notice:m.notice,
        backpressure:1,min_workers:1,max_workers:4,allow_promote:true,allow_cancel:true,allow_retry:true,on_action:Fn("submit")}),
        row([control("refuse","Decline next command",Fn("refuse")),control("clock","Advance demo clock",Fn("clock")),
            control("throttle","Limit sample provider",Fn("limit")),control("enqueue","Add sample job",Fn("enqueue")),
            control("run","Start next sample",Fn("launch")),control("finish","Finish a sample",Fn("complete")),
            control("reset","Reset queue demo",Fn("reset"))]).with_key("tools").with_style(style().gap(px(5)).min_width(px(0)).flex_wrap())
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(7))))
}
