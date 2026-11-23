// Host-owned timelines and state transitions for a local preview demonstration.
import "patterns/agent_monitor" as monitor;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_monitor_demo","export":"AgentMonitorDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_monitor","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentMonitorDemo(props){render_component("gallery/agent_monitor_demo",props)}
fn initial(serial){
    #{serial:serial,run:"preview/monitor/"+serial.to_string(),revision:0,now:50000,started:30000,paused:0,halt:(),ended:(),
        phase:"running",cost:142000,notice:(),pending:(),refuse:false,ask:(),outcome:(),error:(),steps:[
        #{id:"read/材料",label:"Read preview inputs",kind:"read",state:"done",elapsed_ms:2000,tool:"Workspace",
            detail:"The fixture is ready for the preview.",output:"Fixture inputs: 4 palettes"},
        #{id:"search",label:"Find reference baselines",kind:"search",state:"done",elapsed_ms:3200,tool:"Local search"},
        #{id:"measure",label:"Measure the controls",kind:"run",state:"done",elapsed_ms:3700,tool:"Preview"},
        #{id:"colors",label:"Read palette tokens",kind:"read",state:"done",elapsed_ms:2400,tool:"Workspace"},
        #{id:"build",label:"Build the preview",kind:"run",state:"running",started_ms:44000,tool:"Preview"},
        #{id:"compare",label:"Compare final images",kind:"edit",state:"queued"},
    ]}
}
fn model(ctx){let m=ctx.get_state("sample");if m==(){initial(1)}else{m}}
fn report(ctx,m,label){m.notice=label;ctx.set_state("sample",m);}
fn reset(ctx,e){ctx.set_state("sample",initial(model(ctx).serial+1));}
fn tick(ctx,e){let m=model(ctx);m.now+=12000;report(ctx,m,"Advanced the Host clock.");}
fn refuse(ctx,e){let m=model(ctx);m.refuse=true;report(ctx,m,"The next transition will be refused.");}
fn active(m){for i in 0..m.steps.len{if m.steps[i].state=="running"{return i;}}-1}
fn step_id(m){let i=active(m);if i<0{""}else{m.steps[i].id}}
fn elapsed(m,s){
    let at=if m.halt==(){m.now}else{m.halt};
    let pauses=if "paused_ms" in s{s.paused_ms}else{0};
    max(0,at-s.started_ms-pauses)
}
fn next_step(ctx,e){
    let m=model(ctx);if m.phase!="running"||m.pending!=(){return;}
    let i=active(m);if i<0{return;}
    m.steps[i].elapsed_ms=elapsed(m,m.steps[i]);m.steps[i].state="done";m.revision+=1;
    if i+1<m.steps.len{m.steps[i+1].state="running";m.steps[i+1].started_ms=m.now;}
    report(ctx,m,"Published the next step.");
}
fn wait_for_review(ctx,e){
    let m=model(ctx);if m.phase!="running"||m.pending!=(){return;}
    m.revision+=1;m.phase="waiting";
    m.ask=#{id:"request/配色/"+m.revision.to_string(),title:"Review the preview before handoff",
        command:"preview --fixture local"};
    report(ctx,m,"The Host is waiting for a decision.");
}
fn terminal(ctx,phase,value){
    let m=model(ctx);if !(m.phase in ["running","paused","waiting"])||m.pending!=(){return;}
    report(ctx,stopped(m,phase),value);
}
fn stopped(m,phase){
    let i=active(m);
    if i>=0{
        m.steps[i].elapsed_ms=elapsed(m,m.steps[i]);m.steps[i].state=if phase=="failed"{"failed"}else{"done"};
    }
    m.ended=if m.halt==(){m.now}else{m.halt};m.halt=();m.ask=();m.phase=phase;m.revision+=1;
    if phase=="finished"{m.outcome=#{files:3,added:18,removed:4,note:"Preview saved locally."};}
    if phase=="failed"{m.error="The fixture is missing an output image.";}
    m
}
fn fail(ctx,e){terminal(ctx,"failed","The Host published a failure.");}
fn finish(ctx,e){terminal(ctx,"finished","The Host completed the run.");}
fn matches(m,p){p.run==m.run&&p.revision==m.revision&&p.step==step_id(m)}
fn action(ctx,p){
    let m=model(ctx);if !matches(m,p){return;}
    if p.action=="open_log"{report(ctx,m,"The Host opened the run log.");return;}
    if m.pending!=(){return;}
    let expected=if p.action in ["allow","decline"]&&m.ask!=(){m.ask.id}else{""};
    if p.request!=expected{return;}
    if (p.action=="pause"&&m.phase=="running")||(p.action=="resume"&&m.phase=="paused")
        ||(p.action=="cancel"&&m.phase in ["running","paused"])||(p.action in ["allow","decline"]&&m.phase=="waiting"&&m.ask!=())
        ||(p.action=="retry"&&m.phase=="failed"){
        let ticket=p;ticket.refuse=m.refuse;m.refuse=false;m.pending=ticket;report(ctx,m,"Waiting for the demo Host…");
    }
}
fn complete(ctx,p){
    let m=model(ctx);if p!=m.pending||!matches(m,p){return;}
    m.pending=();
    if p.refuse{report(ctx,m,"The Host refused the transition.");return;}
    if p.action=="retry"{report(ctx,initial(m.serial+1),"Started a new run.");return;}
    if p.action=="cancel"||p.action=="decline"{
        report(ctx,stopped(m,"cancelled"),if p.action=="decline"{"Declined "+p.request}else{"Cancelled the run."});return;
    }
    m.revision+=1;
    if p.action=="pause"{m.halt=m.now;m.phase="paused";report(ctx,m,"Paused the run.");return;}
    if p.action=="resume"{
        let pause=m.now-m.halt;m.paused+=pause;m.halt=();m.phase="running";
        let i=active(m);if i>=0{m.steps[i].paused_ms=pause+(if "paused_ms" in m.steps[i]{m.steps[i].paused_ms}else{0});}
        report(ctx,m,"Resumed the run.");return;
    }
    m.phase="running";m.ask=();report(ctx,m,"Allowed "+p.request);
}
fn control(key,text,f,d){button::Button(#{key:key,text:text,size:"sm",variant:"ghost",on_click:f,disabled:d})}
fn demo(ctx,props){
    let m=model(ctx);let busy=m.pending!=();
    if busy{timeout("receipt",420,false,Fn("complete"),m.pending);}
    column([monitor::AgentMonitor(#{key:"run",run:m.run,revision:m.revision,now_ms:m.now,started_ms:m.started,paused_ms:m.paused,paused_at_ms:m.halt,
            ended_ms:m.ended,state:m.phase,name:"Preview assistant",role:"Local release fixture",cost_micros:m.cost,steps:m.steps,
            ask:m.ask,outcome:m.outcome,error:m.error,busy:busy,notice:m.notice,on_action:Fn("action")}),
        row([control("clock","Advance monitor clock",Fn("tick"),false),control("next","Finish monitor step",Fn("next_step"),busy||m.phase!="running"),
            control("wait","Request monitor approval",Fn("wait_for_review"),busy||m.phase!="running"),
            control("refuse","Refuse next transition",Fn("refuse"),busy),
            control("fail","Fail monitor run",Fn("fail"),busy||!(m.phase in ["running","paused","waiting"])),
            control("complete","Finish monitor run",Fn("finish"),busy||!(m.phase in ["running","paused","waiting"])),
            control("reset","Reset monitor",Fn("reset"),false)]).with_key("controls").with_style(style().min_width(px(0)).gap(px(5)).flex_wrap())
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(4))))
}
