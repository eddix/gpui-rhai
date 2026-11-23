// Host snapshots for a live, roving verification feed in Gallery.
import "patterns/agent_findings" as findings;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_findings_demo","export":"AgentFindingsDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_findings","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root","status"]},render:Fn("demo")
});
fn AgentFindingsDemo(props){render_component("gallery/agent_findings_demo",props)}
fn storage(){#{id:"storage/检查",name:"Storage review",model:"Index assistant",step:"Reading the cache manifest",done:7,total:20}}
fn initial(serial){
    let now=1762218200000;
    #{serial:serial,run:"verification/"+serial.to_string(),revision:0,now:now,pending:(),notice:"",next:0,
        agents:[storage(),
            #{id:"layout",name:"Layout review",model:"Preview assistant",step:"Measuring narrow cards",done:14,total:20},
            #{id:"schema",name:"Schema review",done:12,total:12}],
        rows:[
            #{id:"cache/older",agent:"storage/检查",title:"Document the empty cache",severity:"note",at_ms:now-165000},
            #{id:"cutoff",agent:"layout",title:"Table footer clips at 150%",severity:"warning",location:"table.rhai:86",at_ms:now-24000},
            #{id:"path/边界",agent:"storage/检查",title:"Cache key can collide",severity:"critical",location:"index.rs:120",at_ms:now-12000}]}
}
fn model(ctx){let m=ctx.get_state("sample");if m==(){initial(1)}else{m}}
fn save(ctx,m,notice){m.notice=notice;ctx.set_state("sample",m);}
fn reset(ctx,e){ctx.set_state("sample",initial(model(ctx).serial+1));}
fn schedule(ctx,e){
    let m=model(ctx);if m.pending!=(){return;}
    m.pending=#{run:m.run,sequence:m.next+1};
    save(ctx,m,"Waiting for a new finding…");
}
fn publish(ctx,p){
    let m=model(ctx);
    if m.pending==()||m.run!=p.run||m.pending.sequence!=p.sequence{return;}
    m.pending=();m.next=p.sequence;m.now+=2000;m.revision+=1;
    let added=if m.next==1{"Missing focus indicator"}else{"Follow-up finding "+m.next.to_string()};
    m.rows.push(#{id:"revision/"+m.next.to_string(),agent:"layout",title:added,severity:"warning",
        location:"form.rhai:"+m.next.to_string(),at_ms:m.now});
    if m.rows.len>24{m.rows.shift();}
    save(ctx,m,"A new finding arrived.");
}
fn advance(ctx,e){
    let m=model(ctx);m.now+=15000;m.revision+=1;
    for i in 0..m.agents.len{
        if m.agents[i].id=="layout"{
            m.agents[i].done=min(m.agents[i].total,m.agents[i].done+2);
            m.agents[i].step="Checking the footer and focus outline";
        }
    }
    save(ctx,m,"Verification progress updated.");
}
fn change_team(ctx,e){
    let m=model(ctx);m.revision+=1;
    if m.agents[0].id=="storage/检查"{m.agents.remove(0);save(ctx,m,"Storage review left the run.");}
    else{m.agents.insert(0,storage());save(ctx,m,"Storage review rejoined the run.");}
}
fn open(ctx,p){
    let m=model(ctx);
    if m.run!=p.run||m.revision!=p.revision{return;}
    for item in m.rows{
        if item.id==p.finding&&item.agent==p.agent{save(ctx,m,"Opened "+p.finding+" at revision "+p.revision.to_string());return;}
    }
}
fn action_button(key,label,callback){
    button::Button(#{key:key,text:label,size:"sm",variant:"ghost",on_click:callback})
}
fn demo(ctx,props){
    let m=model(ctx);
    if m.pending!=(){timeout("publish",460,false,Fn("publish"),m.pending);}
    let members=m.agents[0].id=="storage/检查";
    let children=[findings::AgentFindings(#{key:"surface",run:m.run,revision:m.revision,now_ms:m.now,
            agents:m.agents,findings:m.rows,on_open:Fn("open")}),
        row([action_button("publish","Schedule finding",Fn("schedule")),action_button("checks","Advance checks",Fn("advance")),
            action_button("team",if members{"Remove Storage review"}else{"Restore Storage review"},Fn("change_team")),
            action_button("reset","Reset findings",Fn("reset"))]).with_key("controls").with_style(style().min_width(px(0)).flex_wrap())];
    if m.notice!=""{children.push(text(m.notice).with_key("notice").accessibility_role("status")
        .with_style(ctx.component_style("status",style().typography("caption"))));}
    column(children).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.unit").times(2))))
}
