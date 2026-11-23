// The demo Host publishes accepted calls and replies; the graph only displays them.
import "patterns/agent_graph" as graph;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_graph_demo","export":"AgentGraphDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_graph","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentGraphDemo(props){render_component("gallery/agent_graph_demo",props)}
fn initial(serial){
    #{serial:serial,run:"sample/calls/"+serial.to_string(),revision:0,ticket:0,now:55000,refuse:false,notice:(),selected:(),nodes:[
        #{id:"lead",name:"Coordinator",kind:"agent",lead:true,status:"working",detail:"Planner",doing:"Compare the draft changes",cost_micros:210000},
        #{id:"review",name:"Review assistant",short:"Review",kind:"agent",status:"waiting",detail:"Reviewer",cost_micros:90000},
        #{id:"preview",name:"Preview assistant",short:"Preview",kind:"agent",status:"idle",detail:"Builder",cost_micros:50000},
        #{id:"search/资料",name:"Workspace search",short:"Search",kind:"tool",detail:"Local search"},
        #{id:"files",name:"Run artifacts",short:"Artifacts",kind:"tool",detail:"Preview files",status:"done"},
        #{id:"owner",name:"You",kind:"user",detail:"Approves the preview"}
    ],calls:[
        #{id:"initial",from:"preview",to:"files",label:"Read the metadata",at_ms:50000,status:"ok",elapsed_ms:290},
        #{id:"review",from:"lead",to:"review",label:"Review the draft",at_ms:52000,status:"pending"}
    ],edges:[#{from:"lead",to:"review"},#{from:"preview",to:"files"},#{from:"lead",to:"preview"},#{from:"review",to:"owner"},#{from:"lead",to:"search/资料"}]}
}
fn model(ctx){let m=ctx.get_state("sample");if m==(){initial(1)}else{m}}
fn put(ctx,m,notice){m.notice=notice;ctx.set_state("sample",m);}
fn reset(ctx,e){ctx.set_state("sample",initial(model(ctx).serial+1));}
fn refuse(ctx,e){let m=model(ctx);m.refuse=true;put(ctx,m,"The next selection will be refused.");}
fn determine(ctx,p){
    let m=model(ctx);if p.run!=m.run||p.revision!=m.revision{return;}
    if m.refuse{m.refuse=false;put(ctx,m,"The Host kept the old selection.");return;}
    m.selected=if p.node==""{()}else{p.node};
    put(ctx,m,if p.node==""{"Closed the inspector."}else{"Selected "+p.node});
}
fn send(ctx,e){
    let m=model(ctx);m.ticket+=1;m.now+=1000;m.revision+=1;
    if m.calls.len>=60{m.calls.shift();}
    m.calls.push(#{id:"request/"+m.ticket.to_string(),from:"lead",to:"search/资料",label:"Search the draft",at_ms:m.now,status:"pending"});
    m.nodes[3].status="working";put(ctx,m,"Published a request.");
}
fn reply(fail,ctx,e){
    let m=model(ctx);
    for j in 0..m.calls.len{
        let i=m.calls.len-j-1;
        if m.calls[i].status!="pending"||m.calls[i].to!="search/资料"{continue;}
        m.calls[i].status=if fail{"error"}else{"ok"};m.calls[i].elapsed_ms=625;
        m.nodes[3].status=if fail{"error"}else{"done"};
        m.now+=625;m.revision+=1;put(ctx,m,if fail{"Published an error reply."}else{"Published a successful reply."});return;
    }
}
fn cool(ctx,e){let m=model(ctx);m.now+=65000;put(ctx,m,"The heat window has elapsed.");}
fn action(key,text,callback){button::Button(#{key:key,text:text,size:"sm",variant:"ghost",on_click:callback})}
fn demo(ctx,props){
    let m=model(ctx);
    column([graph::AgentGraph(#{key:"graph",title:"Preview communication",run:m.run,revision:m.revision,now_ms:m.now,nodes:m.nodes,edges:m.edges,
            calls:m.calls,selected:m.selected,notice:m.notice,on_select:Fn("determine")}),
        row([action("send","Send search request",Fn("send")),action("reply","Return search reply",Fn("reply").curry(false)),
            action("fail","Return search error",Fn("reply").curry(true)),action("heat","Advance past heat window",Fn("cool")),
            action("reject","Refuse next selection",Fn("refuse")),action("reset","Reset communication",Fn("reset"))])
            .with_key("demo-controls").with_style(style().min_width(px(0)).gap(px(5)).flex_wrap())
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(4))))
}
