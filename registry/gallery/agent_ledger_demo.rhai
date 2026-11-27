// Accepted snapshots for the interactive ledger demonstration.
import "patterns/agent_ledger" as ledger;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_ledger_demo","export":"AgentLedgerDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_ledger","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentLedgerDemo(props){render_component("gallery/agent_ledger_demo",props)}
fn initial(serial){
    let now=1762218200000;
    #{serial:serial,run:"ledger/"+serial.to_string(),revision:0,now:now,point:(),pending:(),refuse:false,notice:(),always:false,added:0,
        entries:[
            #{id:"price/定价",action:"Changed price",target:"Pro plan",system:"Billing",at_ms:now-850000,change:#{before:"$29",after:"$34"}},
            #{id:"email",action:"Sent launch email",target:"Subscribers",system:"Mail",at_ms:now-220000,detail:"1,432 recipients",
                reversible:false,depends_on:["price/定价"]},
            #{id:"page",action:"Updated pricing page",system:"Website",at_ms:now-490000,change:#{before:"Old price",after:"New price"},
                depends_on:["price/定价"]},
            #{id:"cdn",action:"Refreshed previews",target:"Product cards",system:"Delivery",at_ms:now-85000,detail:"Publishing cache refreshed",
                depends_on:["page","email"]},
            #{id:"quote",action:"Updated quote template",system:"Sales",at_ms:now-340000,detail:"Annual pricing block",
                depends_on:["price/定价"]},
            #{id:"notes",action:"Recorded release notes",system:"Workspace",at_ms:now-18000,detail:"Draft for the launch review"},
        ],undone_batches:[#{root:"quote",changes:["quote"]}]}
}
fn model(ctx){let m=ctx.get_state("sample");if m==(){initial(1)}else{m}}
fn save(ctx,m,notice){m.notice=notice;ctx.set_state("sample",m);}
fn reset(ctx,e){ctx.set_state("sample",initial(model(ctx).serial+1));}
fn toggle(ctx,e){let m=model(ctx);m.always=!m.always;ctx.set_state("sample",m);}
fn preview(ctx,e){let m=model(ctx);m.point=if m.point==(){"price/定价"}else{()};ctx.set_state("sample",m);}
fn refuse(ctx,e){let m=model(ctx);m.refuse=true;save(ctx,m,"The next ledger action will be refused.");}
fn note(ctx,e){
    let m=model(ctx);if m.entries.len>=64{return;}
    m.added+=1;m.revision+=1;m.now+=1000;
    m.entries.push(#{id:"new/"+m.added.to_string(),action:"Logged a follow-up",target:"#"+m.added.to_string(),
        system:"Workspace",at_ms:m.now,detail:"Added after the snapshot"});
    save(ctx,m,"The Host recorded another change.");
}
fn begin(ctx,p){
    let m=model(ctx);
    if m.pending!=()||p.run!=m.run||p.revision!=m.revision{return;}
    let g=ledger::graph(m);if !(p.anchor in g.entries){return;}
    let proof=#{run:m.run,revision:m.revision,on_action:Fn("begin")};
    if ledger::proposal(proof,g,ledger::batch_state(m,g),g.entries[p.anchor])!=p{return;}
    m.pending=#{request:p,refuse:m.refuse};m.refuse=false;m.point=();
    save(ctx,m,"Waiting for the demo Host…");
}
fn settle_request(ctx,p){
    let m=model(ctx);
    if m.pending==()||m.pending!=p{return;}
    let request=p.request;m.pending=();
    if m.run!=request.run||m.revision!=request.revision{save(ctx,m,"The ledger changed before this request settled.");return;}
    if p.refuse{save(ctx,m,"The Host refused the ledger action.");return;}
    let kind=request.kind;
    if kind=="undo"{m.undone_batches.push(#{root:request.root,changes:request.changes});}
    else{
        let remaining=[];for b in m.undone_batches{if b.root!=request.root{remaining.push(b);}}m.undone_batches=remaining;
    }
    m.revision+=1;
    save(ctx,m,if kind=="undo"{"Undone: "+request.changes.len.to_string()+" changes"}else{"Restored: "+request.changes.len.to_string()+" changes"});
}
fn control(key,label,callback){
    button::Button(#{key:key,text:label,size:"sm",variant:"ghost",on_click:callback})
}
fn demo(ctx,props){
    let m=model(ctx);
    if m.pending!=(){timeout("settle",460,false,Fn("settle_request"),m.pending);}
    column([ledger::AgentLedger(#{key:"surface",run:m.run,revision:m.revision,now_ms:m.now,entries:m.entries,
            agent:"Release assistant",undone_batches:m.undone_batches,point_at:m.point,show_actions:m.always,busy:m.pending!=(),
            on_action:Fn("begin"),notice:m.notice}),
        row([control("preview",if m.point==(){"Preview price cascade"}else{"Clear cascade preview"},Fn("preview")),
            control("show",if m.always{"Hide idle commands"}else{"Show ledger commands"},Fn("toggle")),
            control("refuse","Refuse next ledger action",Fn("refuse")),control("new","Record follow-up",Fn("note")),
            control("reset","Reset ledger",Fn("reset"))]).with_key("controls").with_style(style().min_width(px(0)).flex_wrap())
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(4))))
}
