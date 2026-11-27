// Fixture-only dispatch: the user moves sample records, with no job execution.
import "patterns/delegate_board" as board;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/delegate_board_demo","export":"DelegateBoardDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/delegate_board","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn DelegateBoardDemo(props){render_component("gallery/delegate_board_demo",props)}
fn base(generation){
    #{generation:generation,board:"demo/dispatch/"+generation.to_string(),revision:0,mode:"manual",now:1735689600000,
        ticket:(),decline:false,notice:(),replacement:(),incoming:0,proposals:0,agents:[
            #{id:"plan",name:"Planner",model:"Planning assistant",current_minutes:6,rate_cents:6},
            #{id:"build",name:"Builder",model:"Implementation assistant",rate_cents:8},
            #{id:"verify",name:"Verifier",model:"Verification assistant",accepting:false,rate_cents:4},
        ], tasks:[
            #{id:"constraints",title:"Review constraints",estimate:8,agent:"plan"},
            #{id:"notes/v1",title:"Draft release notes",estimate:7,tools:[#{label:"Text"},#{label:"Git"}]},
            #{id:"fixtures",title:"Prepare fixtures",estimate:4,tools:[#{label:"Rhai"}]},
            #{id:"keyboard",title:"Check keyboard paths",estimate:11,tools:[#{label:"Gallery"}]},
        ]}
}
fn state(ctx){let m=ctx.get_state("model");if m==(){base(1)}else{m}}
fn save(ctx,m){ctx.set_state("model",m);}
fn reset(ctx,p){save(ctx,base(state(ctx).generation+1));}
fn decline(ctx,p){let m=state(ctx);m.decline=true;m.notice="The next assignment will be declined.";save(ctx,m);}
fn change_mode(ctx,p){
    let m=state(ctx);if p.board!=m.board||p.revision!=m.revision||m.ticket!=()||p.mode==m.mode{return;}
    m.mode=p.mode;m.revision+=1;m.notice=if p.mode=="auto"{"Automatic mode accepted."}else{"Manual mode accepted."};save(ctx,m);
}
fn next_task(ctx,p){
    let m=state(ctx);let tasks=m.tasks;m.incoming+=1;
    tasks.push(#{id:"incoming-"+m.incoming.to_string(),title:"Inspect night build "+m.incoming.to_string(),estimate:3,tools:[#{label:"Build"}]});
    m.tasks=tasks;m.revision+=1;m.ticket=();m.notice="New work arrived.";save(ctx,m);
}
fn arm_replacement(ctx,p){
    let m=state(ctx);m.replacement=#{board:m.board,revision:m.revision};m.notice="A fresh snapshot is scheduled.";save(ctx,m);
}
fn replace(ctx,p){
    let m=state(ctx);if m.replacement!=p{return;}
    m.replacement=();
    if m.board==p.board&&m.revision==p.revision{
        m.revision+=1;m.ticket=();m.notice="Replaced the accepted snapshot.";
    }
    save(ctx,m);
}
fn begin(ctx,p){
    let m=state(ctx);if p.board!=m.board||p.revision!=m.revision||m.ticket!=(){return;}
    let t=p;t.decline=m.decline;m.ticket=t;m.decline=false;m.proposals+=1;m.notice="Waiting for the demo Host…";save(ctx,m);
}
fn owner(t){if "agent" in t&&t.agent!=(){t.agent}else{""}}
fn validate(m,ticket){
    let owners=#{};
    for row in m.tasks{owners[row.id]=owner(row);}
    let available=#{};
    for a in m.agents{if !("accepting" in a)||a.accepting{available[a.id]=true;}}
    let moves=#{};
    for item in ticket.moves{
        if item.task in moves||!(item.task in owners)||owners[item.task]!=item.from||item.from==item.agent||
            (item.agent!=""&&!(item.agent in available)){return ();}
        moves[item.task]=item;
    }
    moves
}
fn first_in_queue(tasks,agent){
    for task in tasks{if owner(task)==agent{return task.id;}}
    ""
}
fn finish(ctx,p){
    let m=state(ctx);if p!=m.ticket||p.board!=m.board||p.revision!=m.revision{return;}
    m.ticket=();let moves=validate(m,p);
    if p.decline||moves==(){m.notice="The demo Host declined this assignment.";save(ctx,m);return;}
    let data=[];let changed=#{};
    for row in m.tasks{
        if row.id in moves{
            let item=row;let next=moves[row.id];
            item.agent=if next.agent==""{()}else{next.agent};changed[row.id]=item;
        }else{data.push(row);}
    }
    for item in p.moves{data.push(changed[item.task]);}
    let agents=m.agents;
    for i in 0..agents.len{
        let agent=agents[i];
        if "current_minutes" in agent&&first_in_queue(m.tasks,agent.id)!=first_in_queue(data,agent.id){
            agent.current_minutes=();agents[i]=agent;
        }
    }
    m.agents=agents;
    m.tasks=data;m.revision+=1;
    m.notice="Accepted "+p.kind+" assignment: "+p.moves.len.to_string();
    save(ctx,m);
}
fn demo(ctx,props){
    let m=state(ctx);
    if m.ticket!=(){timeout("assignment",480,false,Fn("finish"),m.ticket);}
    if m.replacement!=(){timeout("snapshot",620,false,Fn("replace"),m.replacement);}
    let suggestion=();for t in m.tasks{if t.id=="notes/v1"&&owner(t)==""{suggestion=#{task:t.id,agent:"build"};}}
    column([board::DelegateBoard(#{key:"board",board:m.board,revision:m.revision,mode:m.mode,now_ms:m.now,agents:m.agents,tasks:m.tasks,
        suggestion:suggestion,busy:m.ticket!=(),notice:m.notice,on_assign:Fn("begin"),on_mode_change:Fn("change_mode")}),
        row([
            button::Button(#{key:"decline",text:if m.decline{"Next assignment will be declined"}else{"Decline next assignment"},variant:"secondary",size:"sm",
                disabled:m.decline,on_click:Fn("decline")}),
            button::Button(#{key:"add",text:"Add sample task",variant:"ghost",size:"sm",on_click:Fn("next_task")}),
            button::Button(#{key:"replace",text:"Schedule new snapshot",variant:"ghost",size:"sm",on_click:Fn("arm_replacement")}),
            button::Button(#{key:"reset",text:"Reset board demo",variant:"ghost",size:"sm",on_click:Fn("reset")}),
        ]).with_key("demo-actions").with_style(style().min_width(px(0)).gap(px(5)).flex_wrap()),
        text("Assignment proposals: "+m.proposals.to_string()).with_key("proposals").accessibility_role("status"),
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(8))))
}
