// Time and decisions are simulated state; no tasks are started or opened externally.
import "patterns/agent_lanes" as lanes;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_lanes_demo","export":"AgentLanesDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_lanes","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentLanesDemo(props){render_component("gallery/agent_lanes_demo",props)}
fn initial(serial){
    #{serial:serial,timeline:"release/activity/"+serial.to_string(),revision:0,now:1200000,reverse:false,decline:false,pending:(),notice:(),highlight:(),
        agents:[#{id:"plan",name:"Planner",model:"Planning assistant"},#{id:"build",name:"Builder",model:"Local tools"},
            #{id:"verify",name:"Reviewer",model:"Review assistant"}],
        tasks:[
            #{id:"archived",agent:"plan",title:"Archived prep",status:"done",start_ms:520000,end_ms:600000},
            #{id:"outline/1",agent:"plan",title:"Outline rollout",status:"done",start_ms:720000,end_ms:880000,handoff_to:"build",
                summary:["Choose a narrow release cohort and record the rollback checks."]},
            #{id:"build",agent:"build",title:"Build preview",status:"running",start_ms:880000,
                summary:["Build the release preview for this Gallery fixture."],tools:[#{label:"Cargo"},#{label:"Rhai"}]},
            #{id:"overlap",agent:"build",title:"Read changelog",status:"done",start_ms:915000,end_ms:1060000,
                summary:["Compare change notes in parallel with the build."]},
            #{id:"verify",agent:"verify",title:"Confirm release copy",status:"waiting",start_ms:1050000,
                tools:[#{label:"Review"}],ask:"May the revised description go into the sample release notes?",
                summary:["The description applies only to the local example."]},
            #{id:"now-tick",agent:"verify",title:"Recorded checkpoint",status:"done",start_ms:1200000,end_ms:1200000},
            #{id:"future",agent:"plan",title:"Future work",status:"running",start_ms:1250000},
        ]}
}
fn model(ctx){let s=ctx.get_state("model");if s==(){initial(1)}else{s}}
fn save(ctx,m){ctx.set_state("model",m);}
fn notice(ctx,m,s){m.notice=s;save(ctx,m);}
fn reset(ctx,p){save(ctx,initial(model(ctx).serial+1));}
fn reorder(ctx,p){let m=model(ctx);m.reverse=!m.reverse;m.revision+=1;m.pending=();notice(ctx,m,"Lane order updated.");}
fn advance(ctx,p){let m=model(ctx);m.now+=360000;notice(ctx,m,"Advanced the Host clock.");}
fn highlight(ctx,p){let m=model(ctx);m.highlight=if m.highlight==(){"build"}else{()};save(ctx,m);}
fn decline(ctx,p){let m=model(ctx);m.decline=true;notice(ctx,m,"The next decision will be declined.");}
fn start(ctx,p){
    let m=model(ctx);if p.timeline!=m.timeline||p.revision!=m.revision||m.pending!=(){return;}
    let rows=m.tasks;
    for i in 0..rows.len{
        let t=rows[i];if t.id!=p.task{continue;}
        if p.kind=="open"{notice(ctx,m,"Host received open: "+t.title);return;}
        if t.status!="waiting"{return;}
        t.busy=true;rows[i]=t;m.tasks=rows;
        let ticket=p;ticket.refuse=m.decline;m.decline=false;m.pending=ticket;
        notice(ctx,m,"Waiting for the demo Host…");return;
    }
}
fn finish(ctx,p){
    let m=model(ctx);if p!=m.pending||p.timeline!=m.timeline||p.revision!=m.revision{return;}
    let rows=m.tasks;let message="";
    for i in 0..rows.len{
        let t=rows[i];if t.id!=p.task||t.status!="waiting"{continue;}
        t.busy=false;
        if p.refuse{message="The demo Host declined the decision.";}
        else{
            t.status=if p.kind=="approve"{"running"}else{"failed"};
            if t.status=="failed"{t.end_ms=m.now;}
            m.revision+=1;
            message=if p.kind=="approve"{"Accepted approval: "+t.title}else{"Accepted rejection: "+t.title};
        }
        rows[i]=t;break;
    }
    m.pending=();m.tasks=rows;notice(ctx,m,message);
}
fn control(key,title,callback,disabled){
    button::Button(#{key:key,text:title,size:"sm",variant:"ghost",on_click:callback,disabled:disabled})
}
fn demo(ctx,props){
    let m=model(ctx);if m.pending!=(){timeout("respond",460,false,Fn("finish"),m.pending);}
    let agents=if m.reverse{[m.agents[2],m.agents[1],m.agents[0]]}else{m.agents};
    column([lanes::AgentLanes(#{key:"timeline",timeline:m.timeline,revision:m.revision,now_ms:m.now,span_ms:600000,
        tasks:m.tasks,agents:agents,notice:m.notice,highlight:m.highlight,allow_open:true,on_action:Fn("start")}),
        row([control("decline","Decline next decision",Fn("decline"),m.pending!=()),control("reorder","Reverse lane order",Fn("reorder"),m.pending!=()),
            control("clock","Advance demo clock",Fn("advance"),false),control("focus","Highlight Builder",Fn("highlight"),false),
            control("reset","Reset timeline demo",Fn("reset"),false)]).with_key("tools").with_style(style().gap(px(5)).flex_wrap().min_width(px(0)))
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(6))))
}
