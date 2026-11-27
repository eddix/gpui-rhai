// An accepted, versioned snapshot and delayed commands for the Gallery.
import "patterns/agent_recap" as recap;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_recap_demo","export":"AgentRecapDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_recap","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentRecapDemo(props){render_component("gallery/agent_recap_demo",props)}
fn start(){1762137600000}
fn event(id,t,kind,title,detail){
    #{id:id,at_ms:start()+t*60000,kind:kind,title:title,detail:detail,state:"open"}
}
fn initial(serial){
    let done=event("indexed",32,"done","Indexed the support queue","Support assistant");done.count=38;
    let release=event("release/签收",102,"needs","Publish the draft release","Preview assistant needs your sign-off");
    release.action="Approve";release.done_label="Approved";
    let report=event("report",345,"done","Prepared the morning report","Research assistant");report.count=12;
    #{run:"recap/"+serial.to_string(),serial:serial,revision:0,replay:0,board:true,
        events:[event("pipeline",225,"failed","Preview sync failed","Storage connection timed out"),done,release,
            event("review",266,"needs","Review the ledger export","Check the subtotal before continuing"),report],
        pending:(),refuse:false,notice:(),inspected:""}
}
fn model(ctx){let m=ctx.get_state("sample");if m==(){initial(1)}else{m}}
fn index(m,id){for i in 0..m.events.len{if m.events[i].id==id{return i;}}-1}
fn save(ctx,m,notice){m.notice=notice;ctx.set_state("sample",m);}
fn request(ctx,p){
    let m=model(ctx);
    if m.run!=p.run||m.revision!=p.revision||m.pending!=(){return;}
    let i=index(m,p.event);if i<0{return;}let e=m.events[i];
    if p.action=="review"&&e.kind=="needs"&&e.state=="open"{save(ctx,m,"Review requested: "+e.title);return;}
    if p.action=="undo"&&((e.kind=="done"&&e.state=="open")||e.state=="resolved"){
        e.state=if e.kind=="done"{"undone"}else{"open"};m.events[i]=e;m.revision+=1;
        save(ctx,m,"Undone: "+e.title);return;
    }
    if p.action=="redo"&&e.kind=="done"&&e.state=="undone"{
        e.state="open";m.events[i]=e;m.revision+=1;save(ctx,m,"Redone: "+e.title);return;
    }
    if p.action=="primary"&&e.kind!="done"&&e.state=="open"{
        e.state="pending";m.events[i]=e;m.revision+=1;
        m.pending=#{run:m.run,revision:m.revision,event:e.id,refuse:m.refuse};m.refuse=false;
        save(ctx,m,"Waiting for the demo Host…");
    }
}
fn finish(ctx,p){
    let m=model(ctx);if p!=m.pending||p.run!=m.run{return;}
    let i=index(m,p.event);if i<0{return;}
    m.pending=();let e=m.events[i];e.state=if p.refuse{"open"}else{"resolved"};
    m.events[i]=e;m.revision+=1;save(ctx,m,if p.refuse{"Not accepted: "+e.title}else{"Accepted: "+e.title});
}
fn inspect(ctx,p){
    let m=model(ctx);if p.run==m.run&&p.revision==m.revision&&p.event!=m.inspected{
        m.inspected=p.event;ctx.set_state("sample",m);
    }
}
fn replay(ctx,e){let m=model(ctx);m.replay+=1;save(ctx,m,"Replaying the activity.");}
fn refuse(ctx,e){let m=model(ctx);m.refuse=true;save(ctx,m,"The next command will not be accepted.");}
fn change_board(ctx,e){let m=model(ctx);m.board=!m.board;ctx.set_state("sample",m);}
fn reset(ctx,e){ctx.set_state("sample",initial(model(ctx).serial+1));}
fn control(key,label,handler){
    button::Button(#{key:key,text:label,variant:"ghost",size:"sm",on_click:handler})
}
fn demo(ctx,props){
    let m=model(ctx);
    if m.pending!=(){timeout("settle",460,false,Fn("finish"),m.pending);}
    let children=[recap::AgentRecap(#{key:"surface",run:m.run,revision:m.revision,since_ms:start(),until_ms:start()+450*60000,
        events:m.events,limit:2,replay_serial:m.replay,board:m.board,notice:m.notice,on_action:Fn("request"),on_inspect:Fn("inspect")}),
        row([control("replay","Replay recap",Fn("replay")),control("refuse","Reject next recap action",Fn("refuse")),
            control("board",if m.board{"Show summary only"}else{"Show recap board"},Fn("change_board")),
            control("reset","Reset recap",Fn("reset"))]).with_key("controls").with_style(style().min_width(px(0)).flex_wrap())];
    if m.inspected!=""{children.push(text("Inspected event: "+m.inspected).with_key("inspect-result").accessibility_role("status"));}
    column(children).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.unit").times(2))))
}
