// Local fixture: effects and receipts are sample data; no external actions are run.
import "patterns/decision_tree" as tree;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/decision_tree_demo","export":"DecisionTreeDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/decision_tree","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        events:#{},slots:#{},parts:["root"]},
    render:Fn("demo")
});
fn DecisionTreeDemo(props){render_component("gallery/decision_tree_demo",props)}
fn initial(serial){
    let id=serial.to_string();
    #{serial:serial,reject:false,ticket:(),items:[
        #{id:"support/"+id,revision:0,agent:"Billing assistant",model:"Local review model",risk:"high",
            question:"How should we handle three duplicate subscriptions?",
            context:"The original subscriptions are still active.",
            options:[
                #{id:"refund",label:"Cancel the newer ones",recommended:true,
                    effects:[["Refund ",#{text:"about $147",note:"Two renewals are still processing; the total can change.",source:"Billing ledger · draft"}], "Keep the older plans active"]},
                #{id:"contact",label:"Contact customers first",effects:["Let customers choose which renewal to keep"],
                    children:[
                        #{id:"email",label:"Email subscribers",effects:["Send a draft for manual review"]},
                        #{id:"helpdesk",label:"Queue in support",effects:["Open a review item for each customer"]},
                    ]},
                #{id:"wait",label:"Wait for settlement",effects:["Recheck the ledger tomorrow"]},
            ]},
        #{id:"launch/"+id,revision:0,agent:"Release assistant",risk:"medium",question:"Who should see the release summary first?",
            options:[#{id:"team",label:"Send to the project team",effects:["Keep the preview private"]},
                #{id:"everyone",label:"Prepare a public draft",effects:["No messages are sent from this demo"]}]},
    ]}
}
fn state(ctx){let m=ctx.get_state("model");if m==(){initial(1)}else{m}}
fn save(ctx,m){ctx.set_state("model",m);}
fn reject_next(ctx,p){let m=state(ctx);m.reject=true;save(ctx,m);}
fn reset(ctx,p){save(ctx,initial(state(ctx).serial+1));}
fn finish(ctx,p){
    let m=state(ctx);if m.ticket!=p{return;}
    let rows=m.items;
    for i in 0..rows.len{
        let item=rows[i];if item.id!=p.request||item.revision!=p.revision{continue;}
        item.busy=false;
        if p.reject{item.notice="The demo Host declined this proposal.";}
        else{
            item.revision+=1;
            item.receipt=#{option:p.option,message:if p.option==""{"Instruction recorded: "+p.text}else{"The demo Host recorded the selected branch."}};
        }
        rows[i]=item;
    }
    m.items=rows;m.ticket=();save(ctx,m);
}
fn choose(ctx,p){
    let m=state(ctx);if m.ticket!=(){return;}
    let rows=m.items;
    for i in 0..rows.len{
        let item=rows[i];if item.id!=p.request||item.revision!=p.revision||"receipt" in item{continue;}
        let ticket=p;ticket.reject=m.reject;m.reject=false;m.ticket=ticket;
        item.busy=true;item.notice="Waiting for the demo Host…";rows[i]=item;m.items=rows;
        save(ctx,m);return;
    }
}
fn demo(ctx,props){
    let m=state(ctx);
    if m.ticket!=(){timeout("finish",500,false,Fn("finish"),m.ticket);}
    column([tree::DecisionTree(#{key:"tree",decisions:m.items,on_choose:Fn("choose")}),
        row([
            button::Button(#{key:"decline",text:if m.reject{"Next proposal will be declined"}else{"Decline next proposal"},size:"sm",variant:"secondary",
                disabled:m.reject,on_click:Fn("reject_next")}),
            button::Button(#{key:"reset",text:"Reset decision demo",size:"sm",variant:"ghost",on_click:Fn("reset")}),
        ]).with_key("actions").with_style(style().min_width(px(0)).gap(theme_spacing("xs")).flex_wrap())
    ]).with_key(props.key).with_style(ctx.component_style("root",style().gap(theme_spacing("md")).min_width(px(0))))
}
