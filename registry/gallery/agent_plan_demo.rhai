// Local draft/execution fixture. Buttons never start tools or external jobs.
import "patterns/agent_plan" as plan;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_plan_demo","export":"AgentPlanDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_plan","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentPlanDemo(props){render_component("gallery/agent_plan_demo",props)}
fn initial(stream){
    #{id:"studio/release/"+stream.to_string(),serial:stream,revision:0,now:1000000,status:"draft",pending:(),decline:false,created:0,notice:(),
        steps:[
            #{id:"collect/evidence",title:"Collect evidence",detail:"Read the release scope and the existing checks.",tool:"read_context"},
            #{id:"preview",title:"Build preview",detail:"Assemble a preview without publishing.",tool:"prepare_preview"},
            #{id:"rollback",title:"Confirm rollback",detail:"Confirm that the local checks can be reverted.",tool:"verify_fixture"},
        ]}
}
fn model(ctx){let s=ctx.get_state("model");if s==(){initial(1)}else{s}}
fn save(ctx,m){ctx.set_state("model",m);}
fn report(ctx,m,text){m.notice=text;save(ctx,m);}
fn reset(ctx,p){save(ctx,initial(model(ctx).serial+1));}
fn decline(ctx,p){let m=model(ctx);m.decline=true;report(ctx,m,"The next request will be declined.");}
fn clock(ctx,p){let m=model(ctx);m.now+=6000;save(ctx,m);}
fn index(rows,id){for i in 0..rows.len{if rows[i].id==id{return i;}}-1}
fn order(rows){let result=[];for s in rows{result.push(s.id);}result}
fn begin(ctx,p){
    let m=model(ctx);if p.plan!=m.id||p.revision!=m.revision||m.pending!=(){return;}
    let ticket=p;ticket.refuse=m.decline;m.decline=false;m.pending=ticket;report(ctx,m,"Waiting for the demo Host…");
}
fn proposal_move(rows,p){
    let i=index(rows,p.step);let j=index(rows,p.anchor);if i<0||j<0||i==j{return ();}
    let old=rows[i];rows.remove(i);
    let anchor=index(rows,p.anchor);if anchor<0{return ();}
    rows.insert(anchor+(if p.placement=="after"{1}else{0}),old);
    rows
}
fn active(s,now){s.status="active";s.started_ms=now;s.elapsed_ms=();s.error=();s}
fn pending(s){s.status="pending";s.started_ms=();s.elapsed_ms=();s.error=();s}
fn accept(ctx,p){
    let m=model(ctx);if p!=m.pending||p.plan!=m.id||p.revision!=m.revision{return;}
    m.pending=();if p.refuse{report(ctx,m,"The demo Host declined the request.");return;}
    let rows=m.steps;let accepted=false;let message="";
    if m.status=="draft"{
        let i=index(rows,p.step);
        if p.kind=="move"{
            let moved=proposal_move(rows,p);if moved!=(){rows=moved;accepted=true;message="Accepted reorder: "+p.step;}
        }else if p.kind=="remove"&&i>=0{rows.remove(i);accepted=true;message="Removed "+p.step;}
        else if p.kind=="edit"&&i>=0&&p.title!=""{
            let s=rows[i];s.title=p.title;s.detail=p.detail;s.tool=p.tool;rows[i]=s;accepted=true;message="Saved "+p.step;
        }else if p.kind=="add"&&p.title!=""&&rows.len<48{
            m.created+=1;rows.push(#{id:"added/"+m.created.to_string(),title:p.title,detail:p.detail,tool:p.tool});
            accepted=true;message="Added a step.";
        }else if p.kind=="run"&&rows.len>0&&order(rows)==p.order{
            rows[0]=active(rows[0],m.now);m.status="running";accepted=true;message="Started "+p.order[0];
        }
    }
    if p.kind=="cancel"&&m.status in ["draft","running","failed"]{
        for i in 0..rows.len{let step=rows[i];if "status" in step&&step.status=="active"{rows[i]=pending(step);}}
        m.status="cancelled";accepted=true;message="Plan cancelled.";
    }
    if p.kind=="retry"&&m.status=="failed"{
        let i=index(rows,p.step);
        if i>=0&&rows[i].status=="failed"{
            rows[i]=active(rows[i],m.now);
            for j in i+1..rows.len{rows[j]=pending(rows[j]);}
            m.status="running";accepted=true;message="Retrying "+p.step;
        }
    }
    if accepted{m.steps=rows;m.revision+=1;report(ctx,m,message);}
    else{report(ctx,m,"The demo Host declined the request.");}
}
fn complete(ctx,p){
    let m=model(ctx);if m.pending!=()||m.status!="running"{return;}
    let rows=m.steps;
    for i in 0..rows.len{
        let s=rows[i];if !("status" in s)||s.status!="active"{continue;}
        s.status="done";s.elapsed_ms=m.now-s.started_ms;rows[i]=s;
        if i+1<rows.len{rows[i+1]=active(rows[i+1],m.now);}else{m.status="done";}
        m.steps=rows;m.revision+=1;report(ctx,m,"Finished "+s.id);return;
    }
}
fn fail(ctx,p){
    let m=model(ctx);if m.pending!=()||m.status!="running"{return;}
    let rows=m.steps;
    for i in 0..rows.len{
        let s=rows[i];if !("status" in s)||s.status!="active"{continue;}
        s.status="failed";s.elapsed_ms=m.now-s.started_ms;s.error="The local fixture rejected this preview.";rows[i]=s;
        m.steps=rows;m.status="failed";m.revision+=1;report(ctx,m,"Failed "+s.id);return;
    }
}
fn control(key,text,callback,disabled){button::Button(#{key:key,text:text,size:"sm",variant:"ghost",on_click:callback,disabled:disabled})}
fn demo(ctx,props){
    let m=model(ctx);if m.pending!=(){timeout("command",420,false,Fn("accept"),m.pending);}
    column([plan::AgentPlan(#{key:"plan",plan:m.id,revision:m.revision,now_ms:m.now,title:"Prepare a release preview",estimate:"About 2 min",
        status:m.status,steps:m.steps,notice:m.notice,on_action:Fn("begin"),busy:m.pending!=(),allow_cancel:true}),
        row([control("decline","Decline next request",Fn("decline"),m.pending!=()),control("clock","Advance demo clock",Fn("clock"),false),
            control("complete","Finish active step",Fn("complete"),m.status!="running"||m.pending!=()),
            control("fail","Fail active step",Fn("fail"),m.status!="running"||m.pending!=()),
            control("reset","Reset plan demo",Fn("reset"),false)]).with_key("tools").with_style(style().min_width(px(0)).gap(px(5)).flex_wrap())
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(4))))
}
