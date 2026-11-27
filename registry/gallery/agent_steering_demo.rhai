// Local, Host-owned queue and receipts for the interactive Gallery.
import "patterns/agent_steering" as steering;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_steering_demo","export":"AgentSteeringDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_steering","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentSteeringDemo(props){render_component("gallery/agent_steering_demo",props)}
fn initial(serial){
    #{serial:serial,run:"preview/steer/"+serial.to_string(),revision:0,now:60000,started:44000,paused:0,halt:(),ended:(),
        status:"running",number:0,refuse:false,pending:(),receipt:(),notice:(),controlled:false,field:"",
        steps:[
            #{id:"read",title:"Read the preview inputs",status:"done",elapsed_ms:4100,notes:[
                #{id:"accepted",text:"Preserve the original color names.",picked:"next"}]},
            #{id:"build",title:"Assemble the preview",status:"active",started_ms:49000},
            #{id:"review",title:"Review the preview",status:"pending"},
        ],queue:[#{id:"note/保留",text:"Keep the dark palette while measuring."}]}
}
fn model(ctx){let m=ctx.get_state("sample");if m==(){initial(1)}else{m}}
fn save(ctx,m,text){m.notice=text;ctx.set_state("sample",m);}
fn reset(ctx,e){ctx.set_state("sample",initial(model(ctx).serial+1));}
fn tick(ctx,e){let m=model(ctx);m.now+=9000;save(ctx,m,"Advanced the Host clock.");}
fn refuse(ctx,e){let m=model(ctx);m.refuse=true;save(ctx,m,"The next proposal will be refused.");}
fn switch_owner(ctx,e){
    let m=model(ctx);if m.pending!=(){return;}m.controlled=!m.controlled;
    save(ctx,m,if m.controlled{"The Host is editing the draft."}else{"The editor owns its draft."});
}
fn draft_change(ctx,p){
    let m=model(ctx);if m.run==p.run&&m.controlled{m.field=p.text;ctx.set_state("sample",m);}
}
fn active(m){for i in 0..m.steps.len{if m.steps[i].status=="active"{return i;}}-1}
fn active_id(m){let i=active(m);if i<0{""}else{m.steps[i].id}}
fn index(m,id){for i in 0..m.queue.len{if m.queue[i].id==id{return i;}}-1}
fn eligible(m,p){
    if p.run!=m.run||p.revision!=m.revision||p.active!=active_id(m){return false;}
    if p.action=="pause"{return m.status=="running";}
    if p.action=="resume"{return m.status=="paused";}
    if m.status=="done"{return false;}
    let i=if p.note!=""{index(m,p.note)}else{-1};
    if p.action=="remove"{return i>=0;}
    if p.text==""{return false;}
    if p.action=="queue"{return p.note==""&&m.queue.len<24;}
    p.action=="interrupt"&&m.status=="running"&&p.active!=""&&(p.note==""||(i>=0&&m.queue[i].text==p.text))
}
fn begin(ctx,p){
    let m=model(ctx);if m.pending!=()||!eligible(m,p){return;}
    let ticket=p;ticket.refuse=m.refuse;m.refuse=false;m.pending=ticket;
    save(ctx,m,"Waiting for the demo Host…");
}
fn add_note(m,i,n,kind){
    let step=m.steps[i];let notes=if "notes" in step{step.notes}else{[]};
    n.picked=kind;notes.push(n);step.notes=notes;m.steps[i]=step;m
}
fn new_note(m,p){#{id:"draft/"+m.number.to_string(),text:p.text}}
fn resolve(ctx,p){
    let m=model(ctx);if p!=m.pending||!eligible(m,p){return;}
    m.pending=();m.receipt=#{proposal:p.proposal,accepted:!p.refuse};
    if p.refuse{save(ctx,m,"The Host refused the proposal.");return;}
    m.revision+=1;let i=active(m);let message="";
    if p.action=="queue"{
        m.number+=1;m.queue.push(new_note(m,p));message="Queued "+p.text;
    }else if p.action=="interrupt"{
        let n=();
        if p.note==""{m.number+=1;n=new_note(m,p);}else{n=m.queue.remove(index(m,p.note));}
        m=add_note(m,i,n,"now");message="Applied "+n.id+" to "+p.active;
    }else if p.action=="remove"{
        m.queue.remove(index(m,p.note));message="Removed "+p.note;
    }else if p.action=="pause"{
        m.status="paused";m.halt=m.now;message="Paused steering.";
    }else{
        let delta=m.now-m.halt;m.paused+=delta;m.halt=();m.status="running";message="Resumed steering.";
        if i>=0{m.steps[i].paused_ms=delta+(if "paused_ms" in m.steps[i]{m.steps[i].paused_ms}else{0});}
    }
    // In controlled mode this fixture intentionally keeps its submitted text.
    // A Host may choose to clear it, but the steering surface cannot do so.
    save(ctx,m,message);
}
fn elapsed(m,s,at){
    max(0,at-s.started_ms-(if "paused_ms" in s{s.paused_ms}else{0}))
}
fn next_step(ctx,e){
    let m=model(ctx);let i=active(m);if m.status!="running"||m.pending!=()||i<0||i+1>=m.steps.len{return;}
    m.steps[i].elapsed_ms=elapsed(m,m.steps[i],m.now);m.steps[i].status="done";
    m.steps[i+1].status="active";m.steps[i+1].started_ms=m.now;
    if m.queue.len>0{let n=m.queue.shift();m=add_note(m,i+1,n,"next");}
    m.revision+=1;save(ctx,m,"The Host moved to the next step.");
}
fn finish_run(ctx,e){
    let m=model(ctx);if m.status=="done"||m.pending!=(){return;}
    let at=if m.halt==(){m.now}else{m.halt};let i=active(m);
    if i>=0{m.steps[i].elapsed_ms=elapsed(m,m.steps[i],at);m.steps[i].status="done";}
    m.status="done";m.halt=();m.ended=at;m.revision+=1;save(ctx,m,"The Host finished the run.");
}
fn control(key,text,f,d){button::Button(#{key:key,text:text,size:"sm",variant:"ghost",on_click:f,disabled:d})}
fn demo(ctx,props){
    let m=model(ctx);let busy=m.pending!=();
    if busy{timeout("receipt",460,false,Fn("resolve"),m.pending);}
    column([steering::AgentSteering(#{key:"surface",run:m.run,revision:m.revision,now_ms:m.now,started_ms:m.started,paused_ms:m.paused,
            paused_at_ms:m.halt,ended_ms:m.ended,status:m.status,title:"Prepare a local preview",agent:"Preview assistant",
            steps:m.steps,queue:m.queue,draft:if m.controlled{m.field}else{()},busy:busy,receipt:m.receipt,
            notice:m.notice,on_draft_change:Fn("draft_change"),on_action:Fn("begin")}),
        row([control("clock","Advance steering clock",Fn("tick"),false),control("next","Begin next steering step",Fn("next_step"),busy||m.status!="running"),
            control("refuse","Refuse next proposal",Fn("refuse"),busy),control("owner","Switch draft owner",Fn("switch_owner"),busy),
            control("finish","Finish steering run",Fn("finish_run"),busy||m.status=="done"),
            control("reset","Reset steering",Fn("reset"),false)]).with_key("controls").with_style(style().min_width(px(0)).gap(px(5)).flex_wrap())
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(4))))
}
