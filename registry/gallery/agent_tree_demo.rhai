// Interactive sample dispatch run; all decisions stay in the demo Host.
import "patterns/agent_tree" as tree;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_tree_demo","export":"AgentTreeDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_tree","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentTreeDemo(props){render_component("gallery/agent_tree_demo",props)}
fn initial(serial){
    #{serial:serial,run:"sample/team/"+serial.to_string(),revision:0,now:55000,selected:(),collapsed:[],pending:(),later_fold:false,
        notice:(),refuse:false,deny_fold:false,
        agents:[
            #{id:"coord",name:"Coordinator",task:"Prepare the release preview",state:"running",model:"Planner",started_ms:11000,
                cost_micros:202000,tools:[#{label:"Notes"},#{label:"Tasks"}]},
            #{id:"research",parent:"coord",name:"Research",task:"Check the source material",state:"done",model:"Reader",
                started_ms:15000,ended_ms:35000,cost_micros:100000},
            // Its parent is deliberately later in the array.
            #{id:"visual",parent:"build",name:"Visual checks",task:"Compare the four palettes",state:"failed",model:"Reviewer",
                started_ms:21000,ended_ms:48000,cost_micros:75000,error:"The light palette snapshot has changed."},
            #{id:"build",parent:"coord",name:"Build",task:"Assemble a local preview",state:"waiting",model:"Builder",started_ms:18000,
                cost_micros:50000,ask:"The preview is ready. Continue to the local handoff?"},
            #{id:"watch/监督",name:"Supervisor",task:"Observe the run",state:"running",model:"Monitor",started_ms:13000,cost_micros:25000}
        ]}
}
fn model(ctx){let m=ctx.get_state("sample");if m==(){initial(1)}else{m}}
fn save(ctx,m){ctx.set_state("sample",m);}
fn message(ctx,m,label){m.notice=label;save(ctx,m);}
fn index(agents,id){for i in 0..agents.len{if agents[i].id==id{return i;}}-1}
fn current(m,p){p.run==m.run&&p.revision==m.revision;}
fn tick(ctx,e){let m=model(ctx);m.now+=7000;save(ctx,m);}
fn reset(ctx,e){save(ctx,initial(model(ctx).serial+1));}
fn refuse(ctx,e){let m=model(ctx);m.refuse=true;message(ctx,m,"The next resolution will be refused.");}
fn deny_fold(ctx,e){let m=model(ctx);m.deny_fold=true;message(ctx,m,"The next fold will be refused.");}
fn selected(ctx,p){
    let m=model(ctx);if !current(m,p)||index(m.agents,p.agent)<0{return;}
    m.selected=p.agent;message(ctx,m,"Selected "+p.agent);
}
fn collapsed(ctx,p){
    let m=model(ctx);if !current(m,p){return;}
    if m.deny_fold{m.deny_fold=false;message(ctx,m,"The Host kept the old fold state.");return;}
    m.collapsed=p.collapsed;message(ctx,m,"Updated fold for "+p.agent);
}
fn external_fold(ctx,e){
    let m=model(ctx);m.collapsed=if "build" in m.collapsed{[]}else{["build"]};message(ctx,m,"The Host changed the Build fold.");
}
fn schedule_fold(ctx,e){let m=model(ctx);m.later_fold=true;save(ctx,m);}
fn fold_when_ready(ctx,run){
    let m=model(ctx);if m.run!=run||!m.later_fold{return;}
    m.later_fold=false;m.collapsed=["build"];message(ctx,m,"The Host folded Build asynchronously.");
}
fn begin(ctx,p){
    let m=model(ctx);if !current(m,p)||m.pending!=(){return;}
    let i=index(m.agents,p.agent);if i<0{return;}
    let phase=m.agents[i].state;
    if (phase=="waiting"&&p.action in ["approve","reject"])||(phase=="failed"&&p.action in ["retry","dismiss"]){
        let ticket=p;ticket.refuse=m.refuse;m.refuse=false;m.pending=ticket;message(ctx,m,"Waiting for the demo Host…");
    }
}
fn finish(ctx,p){
    let m=model(ctx);if p!=m.pending||!current(m,p){return;}
    m.pending=();if p.refuse{message(ctx,m,"The demo Host refused the resolution.");return;}
    let i=index(m.agents,p.agent);if i<0{return;}
    let row=m.agents[i];
    if p.action in ["approve","retry"]{row.state="running";row.started_ms=m.now;row.ended_ms=();}
    else{row.state="stopped";row.ended_ms=m.now;}
    m.agents[i]=row;m.revision+=1;message(ctx,m,"Accepted "+p.action+" for "+p.agent);
}
fn control(key,label,f,disabled){
    button::Button(#{key:key,text:label,size:"sm",variant:"ghost",on_click:f,disabled:disabled})
}
fn demo(ctx,props){
    let m=model(ctx);if m.pending!=(){timeout("review",430,false,Fn("finish"),m.pending);}
    if m.later_fold{timeout("fold",510,false,Fn("fold_when_ready"),m.run);}
    column([
        tree::AgentTree(#{key:"tree",title:"Preview dispatch",run:m.run,revision:m.revision,now_ms:m.now,agents:m.agents,
            root:"coord",supervisor:"watch/监督",selected:m.selected,collapsed:m.collapsed,on_select:Fn("selected"),on_collapse:Fn("collapsed"),
            on_resolve:Fn("begin"),notice:m.notice,busy_agent:if m.pending==(){()}else{m.pending.agent}}),
        row([control("clock","Advance run clock",Fn("tick"),false),control("refuse","Refuse next resolution",Fn("refuse"),m.pending!=()),
            control("no-fold","Refuse next fold",Fn("deny_fold"),false),control("host-fold","Switch Build fold externally",Fn("external_fold"),false),
            control("schedule-fold","Schedule Build fold",Fn("schedule_fold"),m.later_fold),
            control("reset","Reset dispatch",Fn("reset"),false)]).with_key("tools").with_style(style().gap(px(5)).min_width(px(0)).flex_wrap()),
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(4))))
}
