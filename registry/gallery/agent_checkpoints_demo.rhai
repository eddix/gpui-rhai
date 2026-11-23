// This Host fixture adds a new attempt without rewriting the earlier checkpoints.
import "patterns/agent_checkpoints" as history;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/agent_checkpoints_demo","export":"AgentCheckpointsDemo","version":"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_checkpoints","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn AgentCheckpointsDemo(props){render_component("gallery/agent_checkpoints_demo",props)}
fn initial(serial){
    #{serial:serial,run:"sample/history/"+serial.to_string(),revision:0,head:"review/retry",notice:(),pending:(),decline:false,rows:[
        #{id:"start",title:"Read the brief",elapsed_ms:9000,files:2,cost_cents:3},
        #{id:"prototype/one",title:"Build preview",parent:"start",elapsed_ms:33000,files:4,cost_cents:12},
        #{id:"review/one",title:"Review the first preview",parent:"prototype/one",status:"failed",elapsed_ms:51000,cost_cents:2},
        #{id:"prototype/候補",title:"Build preview",parent:"start",elapsed_ms:15000,files:3,cost_cents:8},
        #{id:"review/retry",title:"Check the revised preview",parent:"prototype/候補",status:"running",elapsed_ms:27000,cost_cents:1}
    ]}
}
fn model(ctx){let s=ctx.get_state("model");if s==(){initial(1)}else{s}}
fn save(ctx,m){ctx.set_state("model",m);}
fn report(ctx,m,text){m.notice=text;save(ctx,m);}
fn reset(ctx,p){save(ctx,initial(model(ctx).serial+1));}
fn decline(ctx,p){let m=model(ctx);m.decline=true;report(ctx,m,"The next rewind will be declined.");}
fn index(rows,id){for i in 0..rows.len{if rows[i].id==id{return i;}}-1}
fn begin(ctx,p){
    let m=model(ctx);
    if p.run!=m.run||p.revision!=m.revision||p.head!=m.head||p.checkpoint==m.head||m.pending!=()||index(m.rows,p.checkpoint)<0||m.rows.len>=96{return;}
    let ticket=p;ticket.refuse=m.decline;m.decline=false;m.pending=ticket;
    report(ctx,m,"Waiting for the demo Host…");
}
fn complete(ctx,p){
    let m=model(ctx);
    if p!=m.pending||p.run!=m.run||p.revision!=m.revision||p.head!=m.head{return;}
    m.pending=();
    if p.refuse{report(ctx,m,"The demo Host declined the rewind.");return;}
    let old=index(m.rows,m.head);
    if old>=0&&"status" in m.rows[old]&&m.rows[old].status=="running"{m.rows[old].status="stopped";}
    m.revision+=1;m.head="attempt/"+m.revision.to_string();
    m.rows.push(#{id:m.head,title:"Resume from checkpoint",parent:p.checkpoint,status:"running",elapsed_ms:0});
    report(ctx,m,"Started a new attempt from "+p.checkpoint);
}
fn finish(ctx,p){
    let m=model(ctx);let i=index(m.rows,m.head);
    if m.pending!=()||i<0||!("status" in m.rows[i])||m.rows[i].status!="running"{return;}
    m.rows[i].status="done";m.revision+=1;report(ctx,m,"Finished the current attempt.");
}
fn control(key,label,callback,disabled){
    button::Button(#{key:key,text:label,size:"sm",variant:"ghost",on_click:callback,disabled:disabled})
}
fn demo(ctx,props){
    let m=model(ctx);if m.pending!=(){timeout("reply",400,false,Fn("complete"),m.pending);}
    let i=index(m.rows,m.head);
    let running=i>=0&&"status" in m.rows[i]&&m.rows[i].status=="running";
    column([history::AgentCheckpoints(#{key:"history",run:m.run,revision:m.revision,task:"Prepare a release preview",
            head:m.head,checkpoints:m.rows,busy:m.pending!=(),notice:m.notice,on_rewind:Fn("begin")}),
        row([control("decline","Decline next rewind",Fn("decline"),m.pending!=()),
            control("finish","Finish current attempt",Fn("finish"),!running||m.pending!=()),
            control("reset","Reset history",Fn("reset"),false)]).with_key("tools").with_style(style().min_width(px(0)).gap(px(5)).flex_wrap())
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(4))))
}
