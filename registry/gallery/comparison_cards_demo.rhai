import "patterns/project_card" as project;
import "patterns/experiment_card" as experiment;
import "components/button" as btn;

define_component(#{
    metadata:#{id:"gallery/comparison_cards_demo","export":"ComparisonCardsDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/project_card","patterns/experiment_card","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},
                    mode:#{schema:#{type:"string",allowed:["project","experiment"]},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn ComparisonCardsDemo(p){render_component("gallery/comparison_cards_demo",p)}
fn model(ctx){
    let m=ctx.get_state("model");if m!=(){return m;}
    #{revision:0,open:false,pending:(),refuse:false,narrow:false,trigger:"click",note:"",status:"progress",
      tasks:#{copy:true,qa:false,launch:false,docs:false},metric:"conversion",winner:()}
}
fn valid(p,m){p.revision==m.revision&&(get_field(p,"project","")== "release"||get_field(p,"experiment","")== "onboarding")}
fn get_field(m,k,f){if k in m&&m[k]!=(){m[k]}else{f}}
fn disclosure(ctx,p){
    let m=model(ctx);if !valid(p,m){return;}
    m.open=p.open;ctx.set_state("model",m);
}
fn begin(kind,ctx,p){
    let m=model(ctx);if !valid(p,m)||m.pending!=(){return;}
    m.pending=#{kind:kind,ticket:p,refuse:m.refuse};m.refuse=false;m.note="Waiting for the Host…";
    ctx.set_state("model",m);
}
fn settle(ctx,p){
    let m=model(ctx);if m.pending!=p||!valid(p.ticket,m){return;}
    m.pending=();let t=p.ticket;
    if p.refuse{m.note="The Host kept the current snapshot.";}
    else{
        switch p.kind{
            "task"=>{m.tasks[t.id]=t.done;m.note="Accepted task: "+t.id;},
            "status"=>{m.status=t.status;m.note="Accepted project status";},
            "metric"=>{m.metric=t.metric;m.note="Accepted metric: "+t.metric;},
            "winner"=>{m.winner=t.winner;m.note=if t.winner==(){"Candidate cleared"}else{"Nominated: "+t.winner};},
            _=>{},
        }
        m.revision+=1;
    }
    ctx.set_state("model",m);
}
fn invoke(ctx,p){
    let m=model(ctx);if !valid(p,m)||m.pending!=(){return;}
    if "experiment" in p&&(m.winner!=p.winner||m.metric!=p.metric){return;}
    m.note=if "experiment" in p{"Confirmed "+p.winner+" for "+p.metric}else{"Opened the release overview"};
    ctx.set_state("model",m);
}
fn refuse(ctx,e){let m=model(ctx);m.refuse=true;m.note="Next request will be refused.";ctx.set_state("model",m);}
fn resize(ctx,e){let m=model(ctx);m.narrow=!m.narrow;ctx.set_state("model",m);}
fn hover(ctx,e){let m=model(ctx);m.trigger=if m.trigger=="hover"{"click"}else{"hover"};ctx.set_state("model",m);}
fn control(k,label,callback){btn::Button(#{key:k,text:label,variant:"ghost",size:"sm",on_click:callback})}
fn project_demo(m){
    project::ProjectCard(#{key:"release-card",project:"release",revision:m.revision,title:"Release logistics",due:"Oct 24",
        open:m.open,trigger:m.trigger,busy:m.pending!=(),status:m.status,on_open_change:Fn("disclosure"),
        members:[#{id:"ava",name:"Ava Chen"},#{id:"lee",name:"Lee Park"},#{id:"mira",name:"Mira Silva"}],
        tasks:[#{id:"copy",label:"Prepare release notes",assignee:"ava",done:m.tasks.copy},
            #{id:"qa",label:"Run acceptance checks",assignee:"lee",done:m.tasks.qa},
            #{id:"launch",label:"Deploy the release",assignee:"mira",done:m.tasks.launch},
            #{id:"docs",label:"Archive the handbook",assignee:"ava",done:m.tasks.docs}],
        action_label:"Open overview",on_action:Fn("invoke"),on_task_change:Fn("begin").curry("task"),on_status_change:Fn("begin").curry("status")})
}
fn experiment_demo(m){
    experiment::ExperimentCard(#{key:"onboarding-card",experiment:"onboarding",revision:m.revision,title:"Onboarding split",
        open:m.open,trigger:m.trigger,busy:m.pending!=(),on_open_change:Fn("disclosure"),status:#{label:"Running",tone:"green"},note:"Day 9 of 14",
        metrics:[#{id:"conversion",label:"Conversion",decimals:1,suffix:"%"},
                 #{id:"latency",label:"Latency",decimals:0,suffix:" ms",lower_is_better:true},
                 #{id:"holdout",label:"Holdout"}],metric:m.metric,winner:m.winner,
        variants:[#{id:"a",short:"A",name:"Baseline",control:true,values:#{conversion:10.0,latency:100,holdout:0.0},visitors:821},
            #{id:"b",short:"B",name:"Guided flow",values:#{conversion:12.0,latency:120,holdout:8.0},
                confidence:#{conversion:96,latency:87},visitors:805},
            #{id:"c",short:"C",name:"Quick start",values:#{conversion:9.0,latency:75},confidence:#{latency:93},visitors:790}],
        on_metric_change:Fn("begin").curry("metric"),on_winner_change:Fn("begin").curry("winner"),on_action:Fn("invoke")})
}
fn demo(ctx,p){
    let m=model(ctx);
    if m.pending!=(){timeout("apply",300,false,Fn("settle"),m.pending);}
    let card=if p.mode=="project"{project_demo(m)}else{experiment_demo(m)};
    column([
        column([card]).with_key("width").with_style(style().width(px(if m.narrow{264}else if p.mode=="project"{320}else{340}))
            .max_width(relative(1.0)).min_width(px(0))),
        row([control("refuse","Refuse next request",Fn("refuse")),control("narrow",if m.narrow{"Wide card"}else{"Narrow card"},Fn("resize")),
             control("trigger",if m.trigger=="hover"{"Open on click"}else{"Open on hover"},Fn("hover"))])
            .with_key("controls").with_style(style().min_width(px(0)).flex_wrap()),
        text(m.note).with_key("receipt").accessibility_role("status").with_style(style().typography("caption").text_color(theme_color("text_muted"))),
    ]).with_key(p.key).with_style(ctx.component_style("root",style().gap(px(12)).min_width(px(0))))
}
