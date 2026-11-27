// Demo Host for the installed AgentFleet; all displayed execution is accepted data.
import "patterns/agent_fleet" as fleet;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_fleet_demo","export":"AgentFleetDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_fleet","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentFleetDemo(props){render_component("gallery/agent_fleet_demo",props)}
fn initial(generation){
    #{serial:generation,run:"preview/fleet/"+generation.to_string(),revision:0,now:53000,open:"delivery",controlled:true,
        highlight:(),notice:(),pending:(),refuse:false,refuse_open:false,agents:[
        #{id:"delivery",name:"Delivery",task:"Prepare a local preview",state:"running",model:"Coordinator",
            cost_micros:120000,started_ms:11000,tools:[#{label:"Preview"},#{label:"Artifacts"}]},
        #{id:"research",name:"Research",task:"Review the release notes",state:"done",model:"Reader",
            cost_micros:79000,started_ms:10000,ended_ms:35000},
        #{id:"visual/配色",parent:"runner",name:"Palette review",task:"Compare the four palettes",state:"waiting",
            cost_micros:42000,started_ms:29000,ask:"The light palette has been checked. Keep the new preview?",
            steps:[#{id:"capture",title:"Capture light palette",tool:"Snapshot",elapsed_ms:2200},
                #{id:"decision",title:"Confirm palette",state:"waiting"}]},
        #{id:"runner",parent:"delivery",name:"Build runner",task:"Build the preview assets",state:"running",
            cost_micros:83000,started_ms:21000,steps:[#{id:"resolve",title:"Resolve preview inputs",elapsed_ms:1800}]},
        #{id:"artifact",parent:"delivery",name:"Artifact scan",task:"Verify the output archive",state:"failed",
            cost_micros:31000,started_ms:22000,ended_ms:47000,error:"The archive is missing its manifest.",
            steps:[#{id:"unpack",title:"Read the archive",tool:"Artifacts",elapsed_ms:820},
                #{id:"manifest",title:"Find the manifest",state:"failed",elapsed_ms:1500}]},
        #{id:"copy",parent:"research",name:"Copy review",task:"Check the published examples",state:"done",
            cost_micros:-3000,started_ms:20000,ended_ms:30000,steps:[#{id:"check",title:"Review example copy",elapsed_ms:1700}]},
        #{id:"maintenance",name:"Maintenance",task:"Clean staging artifacts",state:"queued"},
    ]}
}
fn model(ctx){let m=ctx.get_state("demo");if m==(){initial(1)}else{m}}
fn save(ctx,m,label){m.notice=label;ctx.set_state("demo",m);}
fn reset(ctx,e){ctx.set_state("demo",initial(model(ctx).serial+1));}
fn tick(ctx,e){let m=model(ctx);m.now+=9000;save(ctx,m,"Advanced the run clock.");}
fn refuse(ctx,e){let m=model(ctx);m.refuse=true;save(ctx,m,"The next resolution will be refused.");}
fn refuse_open(ctx,e){let m=model(ctx);m.refuse_open=true;save(ctx,m,"The next disclosure will be refused.");}
fn highlight(ctx,e){
    let m=model(ctx);m.controlled=false;m.highlight=if m.highlight==(){"copy"}else{()};save(ctx,m,"Changed the external highlight.");
}
fn valid(m,p){p.run==m.run&&p.revision==m.revision}
fn change_open(ctx,p){
    let m=model(ctx);if !valid(m,p){return;}
    if m.controlled&&m.refuse_open{m.refuse_open=false;save(ctx,m,"The Host kept the old branch.");return;}
    m.open=p.lead;save(ctx,m,if p.lead==""{"Closed the lead branch."}else{"Opened "+p.lead});
}
fn selected(ctx,p){
    let m=model(ctx);if valid(m,p){save(ctx,m,"Inspecting "+p.agent);}
}
fn index(m,id){for i in 0..m.agents.len{if m.agents[i].id==id{return i;}}-1}
fn begin(ctx,p){
    let m=model(ctx);if !valid(m,p)||m.pending!=(){return;}
    let i=index(m,p.agent);if i<0{return;}
    let state=m.agents[i].state;
    if (state=="waiting"&&p.action in ["approve","reject"])||(state=="failed"&&p.action in ["retry","dismiss"]){
        let ticket=p;ticket.refuse=m.refuse;m.refuse=false;m.pending=ticket;save(ctx,m,"Waiting for the demo Host…");
    }
}
fn resolved(ctx,p){
    let m=model(ctx);if p!=m.pending||!valid(m,p){return;}
    m.pending=();
    if p.refuse{save(ctx,m,"The demo Host refused the resolution.");return;}
    let i=index(m,p.agent);if i<0{return;}
    let a=m.agents[i];
    if p.action in ["approve","retry"]{a.state="running";a.started_ms=m.now;a.ended_ms=();}
    else{a.state="stopped";a.ended_ms=m.now;}
    m.agents[i]=a;m.revision+=1;save(ctx,m,"Accepted "+p.action+" for "+p.agent);
}
fn action(key,text,callback){button::Button(#{key:key,text:text,size:"sm",variant:"ghost",on_click:callback})}
fn demo(ctx,props){
    let m=model(ctx);
    if m.pending!=(){timeout("accept",430,false,Fn("resolved"),m.pending);}
    column([fleet::AgentFleet(#{key:"fleet",title:"Release preview fleet",run:m.run,revision:m.revision,now_ms:m.now,agents:m.agents,
            controlled_open:m.controlled,open:if m.open==""{()}else{m.open},default_open:"delivery",highlight:m.highlight,notice:m.notice,
            busy_agent:if m.pending==(){()}else{m.pending.agent},on_open_change:Fn("change_open"),on_select:Fn("selected"),on_resolve:Fn("begin")}),
        row([action("clock","Advance fleet clock",Fn("tick")),action("refuse-open","Refuse next disclosure",Fn("refuse_open")),
            action("refuse","Refuse next resolution",Fn("refuse")),action("highlight","Toggle copy highlight",Fn("highlight")),
            action("reset","Reset fleet",Fn("reset"))]).with_key("controls").with_style(style().min_width(px(0)).gap(px(5)).flex_wrap())
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(4))))
}
