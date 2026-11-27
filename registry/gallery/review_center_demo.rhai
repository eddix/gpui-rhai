// Local review queue: receipts, proposals, file snapshots and sources are only examples.
import "patterns/review_center" as center;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/review_center_demo","export":"ReviewCenterDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/review_center","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn ReviewCenterDemo(props){render_component("gallery/review_center_demo",props)}
fn initial(serial){
    let n=serial.to_string();
    #{serial:serial,sequence:0,receipt:(),removed:(),feedback:(),decline:false,ticket:(),now:4000000,reviewed:3,items:[
        #{id:"release/"+n,revision:0,title:"Prepare the rollout",agent:"Release assistant",model:"Local Planner",kind:"Code change",
            risk:"high",confidence:0.66,created_ms:2240000,reasoning:"Keep the initial rollout small until observability is in place.",
            files:[
                #{id:"config",name:"rollout.toml",before:"enabled = false\nchannel = \"stable\"\nworkers = 2\n",
                    after:"enabled = true\nchannel = \"canary\"\nworkers = 2\n",added:2,removed:2},
                #{id:"notes",name:"release-notes.md",before:"Deploy to all users.\n",after:"Pilot with the canary group.\n",added:1,removed:1},
            ],sources:[#{id:"plan/runbook",label:"Rollout runbook"}]},
        #{id:"letter/"+n,revision:0,title:"Reply to Jordan",agent:"Support assistant",kind:"Email",risk:"medium",confidence:0.91,created_ms:3600000,
            reasoning:"Confirm the billing adjustment in plain language before sending a response.",
            body_label:"Proposed reply",body:["Hi Jordan, we expect the refund in ",#{text:"two business days",
                note:"The settlement has not reached the payment provider yet.",source:"Settlement batch"}, " after the provider accepts it."],
            fields:[#{id:"audience",label:"Recipient",value:"Jordan"},#{id:"tone",label:"Tone",value:"Direct"}],
            sources:[#{id:"support/ticket",label:"Billing notes"}]},
        #{id:"backup/"+n,revision:0,title:"Plan the backup",agent:"Operations assistant",kind:"Plan",risk:"low",
            confidence:0.62,created_ms:3000000,reasoning:"Compare the plan with the storage policy before approving.",
            steps:[
                #{title:"Inspect the retention policy",tool:"Read policy",detail:"Use the locally available settings."},
                #{title:["Stage a copy in ",#{text:"the archive",note:"Check available storage before starting.",source:"Storage report"}],tool:"Stage files"},
                #{title:"Confirm the next checkpoint"},
            ]},
    ]}
}
fn state(ctx){let m=ctx.get_state("model");if m==(){initial(1)}else{m}}
fn save(ctx,m){ctx.set_state("model",m);}
fn decline(ctx,p){let m=state(ctx);m.decline=true;save(ctx,m);}
fn reset(ctx,p){save(ctx,initial(state(ctx).serial+1));}
fn clock(ctx,p){let m=state(ctx);m.now+=60000;save(ctx,m);}
fn decide(ctx,p){
    let m=state(ctx);if m.ticket!=(){return;}
    let data=m.items;
    for i in 0..data.len{
        let row=data[i];if row.id!=p.request||row.revision!=p.revision{continue;}
        row.busy=true;row.notice="Waiting for the demo Host…";data[i]=row;
        let ticket=p;ticket.declined=m.decline;ticket.serial=m.serial;
        m.ticket=ticket;m.items=data;m.decline=false;m.feedback=();
        save(ctx,m);return;
    }
}
fn finish(ctx,p){
    let m=state(ctx);if m.ticket!=p||m.serial!=p.serial{return;}
    let data=m.items;m.ticket=();
    for i in 0..data.len{
        let row=data[i];if row.id!=p.request||row.revision!=p.revision{continue;}
        row.busy=false;
        if p.declined{
            row.notice="The demo Host declined this proposal.";data[i]=row;
        }else{
            row.notice=();m.removed=row;m.sequence+=1;
            let outcome=if p.action=="approve"{"approved"}else{"rejected"};
            m.receipt=#{id:"review/"+m.serial.to_string()+"/"+m.sequence.to_string(),request:row.id,revision:row.revision,
                outcome:outcome,title:row.title,expires_ms:m.now+30000};
            m.reviewed+=1;data.remove(i);
            if p.action=="reject"{m.feedback="Recorded rejection: "+p.reason;}
            else if p.body!=()||p.fields.len>0{
                let details=[];for id in p.fields.keys(){details.push(id+": "+p.fields[id]);}
                if p.body!=(){details.push("reply: "+p.body);}
                let summary="Accepted edits";for line in details{summary+=" · "+line;}
                m.feedback=summary;
            }
        }
        break;
    }
    m.items=data;save(ctx,m);
}
fn undo(ctx,p){
    let m=state(ctx);let receipt=m.receipt;
    if receipt==()||m.removed==()||m.now>=receipt.expires_ms||p.receipt!=receipt.id||p.request!=receipt.request||p.revision!=receipt.revision{return;}
    let row=m.removed;row.revision+=1;let data=m.items;data.push(row);m.items=data;
    m.removed=();m.receipt=();m.reviewed-=1;
    m.feedback="Restored "+row.title;save(ctx,m);
}
fn skip(ctx,p){let m=state(ctx);m.feedback="Skipped without resolving";save(ctx,m);}
fn source(ctx,p){let m=state(ctx);m.feedback="Requested source: "+p.source;save(ctx,m);}
fn demo(ctx,props){
    let m=state(ctx);if m.ticket!=(){timeout("decide",480,false,Fn("finish"),m.ticket);}
    let nodes=[center::ReviewCenter(#{key:"reviews",items:m.items,now_ms:m.now,reviewed:m.reviewed,receipt:m.receipt,
        on_decide:Fn("decide"),on_undo:Fn("undo"),on_skip:Fn("skip"),on_source:Fn("source")}),
        row([
            button::Button(#{key:"refuse",text:if m.decline{"Next decision will be declined"}else{"Decline next decision"},variant:"secondary",size:"sm",
                disabled:m.decline,on_click:Fn("decline")}),
            button::Button(#{key:"clock",text:"Advance demo clock",variant:"ghost",size:"sm",on_click:Fn("clock")}),
            button::Button(#{key:"reset",text:"Reset review demo",variant:"ghost",size:"sm",on_click:Fn("reset")}),
        ]).with_key("demo-actions").with_style(style().gap(px(5)).min_width(px(0)).flex_wrap())
    ];
    if m.feedback!=(){nodes.push(text(m.feedback).with_key("feedback").accessibility_role("status"));}
    column(nodes).with_key(props.key).with_style(ctx.component_style("root",style().gap(px(9)).min_width(px(0))))
}
