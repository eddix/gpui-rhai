// Two views of the same in-memory decision contract. No external work is run.
import "patterns/decision_card" as cards;
import "patterns/decision_inbox" as inbox;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/decisions_demo","export":"DecisionsDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/decision_card","patterns/decision_inbox","components/button"],capabilities:#{}},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        mode:#{schema:#{type:"string",allowed:["card","inbox"]},required:true,sensitive:false},
    },state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},events:#{},slots:#{},parts:["root"]},
    render:Fn("demo_view")
});
fn DecisionsDemo(props){render_component("gallery/decisions_demo",props)}
fn initial(serial){
    let prefix="round/"+serial.to_string()+"/";
    let rows=[
        #{id:prefix+"review",revision:0,kind:"decision",question:"How should we review the rollout?",
            task:"Release review",options:[#{id:"review",title:"Review first",summary:"Read both summaries before accepting.",recommended:true},
                #{id:"accept",title:"Accept summary",summary:"Keep the current text."},
                #{id:"wait",title:"Wait for checks",summary:"Come back when the checks finish."}],facts:[],
            collapsed:false,phase:"pending",receipt:(),notice:()},
        #{id:prefix+"layout",revision:0,kind:"pick",question:"Which layout should the overview use?",
            task:"Overview layout",options:[
                #{id:"split",title:"Split view",recommended:true,blocks:[#{x:8,y:11,width:104,height:11,tone:"ink"},
                    #{x:8,y:30,width:38,height:42,tone:"soft"},#{x:52,y:30,width:60,height:42,tone:"mid"}]},
                #{id:"stack",title:"Stacked view",blocks:[#{x:8,y:9,width:104,height:11,tone:"ink"},
                    #{x:8,y:26,width:104,height:19,tone:"mid"},#{x:8,y:49,width:104,height:22,tone:"soft"}]},
                #{id:"compact",title:"Compact list",blocks:[#{x:10,y:9,width:100,height:12,tone:"ink"},#{x:10,y:25,width:100,height:12,tone:"soft"},
                    #{x:10,y:41,width:100,height:12,tone:"mid"},#{x:10,y:57,width:100,height:12,tone:"soft"}]}],facts:[],
            collapsed:true,phase:"pending",receipt:(),notice:()},
        #{id:prefix+"note",revision:0,kind:"approval",question:"Record the final note?",task:"Publishing draft",options:[],
            facts:[#{label:"Document",value:"Release notes"},#{label:"Visibility",value:"Team only"}],
            collapsed:true,phase:"pending",receipt:(),notice:()},
    ];
    #{serial:serial,rows:rows,queue:#{},reveal:0,reject:false,outcome:""}
}
fn model(ctx){let value=ctx.get_state("model");if value==(){initial(1)}else{value}}
fn save(ctx,value){ctx.set_state("model",value);}
fn resolved_label(row,p){
    if p.kind=="custom"{return "Custom answer recorded";}
    if p.kind=="approve"{return "Approved";}
    if p.kind=="decline"{return "Declined";}
    if p.kind=="option"{
        for option in row.options{if option.id==p.option{return option.title;}}
    }
    ""
}
fn proposed(ctx,p){
    let m=model(ctx);let rows=m.rows;
    for i in 0..rows.len{
        let row=rows[i];
        if row.id!=p.request||row.revision!=p.revision||row.phase!="pending"{continue;}
        if "choice" in p{
            let ix=if p.choice=="primary"{0}else{1};
            p.kind=if row.kind=="approval"{if ix==0{"approve"}else{"decline"}}else{"option"};
            p.option=if row.kind=="approval"{""}else{row.options[ix].id};
        }
        let label=resolved_label(row,p);
        if label==""{return;}
        let ticket=#{request:p.request,revision:p.revision,label:label,reject:m.reject};
        m.reject=false;m.queue[p.request]=ticket;row.phase="busy";row.notice=();rows[i]=row;m.rows=rows;
        m.outcome="";save(ctx,m);return;
    }
}
fn receive(ctx,p){
    let m=model(ctx);if !(p.request in m.queue)||m.queue[p.request]!=p{return;}
    let rows=m.rows;
    for i in 0..rows.len{
        let row=rows[i];
        if row.id!=p.request||row.revision!=p.revision||row.phase!="busy"{continue;}
        if p.reject{
            row.phase="pending";row.notice="The demo Host rejected this answer.";
        }else{
            row.phase="resolved";row.revision+=1;row.receipt=p.label;
            m.outcome="Recorded: "+p.label;
        }
        rows[i]=row;
    }
    m.rows=rows;let queue=m.queue;queue.remove(p.request);m.queue=queue;save(ctx,m);
}
fn retire(ctx,p){
    let m=model(ctx);let next=[];
    for row in m.rows{
        if row.id!=p.request||row.revision!=p.revision||row.phase!="resolved"{next.push(row);}
    }
    m.rows=next;save(ctx,m);
}
fn refresh(ctx,event){
    let m=model(ctx);let rows=m.rows;if rows.len==0{return;}
    let item=rows[0];if item.phase=="resolved"{return;}
    item.revision+=1;item.question="How should we review the revised rollout?";item.notice="This is a new revision.";
    item.phase="pending";rows[0]=item;m.rows=rows;let queue=m.queue;queue.remove(item.id);m.queue=queue;save(ctx,m);
}
fn reset(ctx,event){let m=model(ctx);save(ctx,initial(m.serial+1));}
fn reject(ctx,event){let m=model(ctx);m.reject=true;m.outcome="The next answer will be rejected.";save(ctx,m);}
fn reveal(ctx,event){let m=model(ctx);m.reveal+=1;save(ctx,m);}
fn later(ctx,p){let m=model(ctx);m.outcome="Deferred for now; still pending.";save(ctx,m);}
fn control(key,label,action){button::Button(#{key:key,text:label,size:"sm",variant:"secondary",on_click:action})}
fn demo_view(ctx,props){
    let m=model(ctx);let advanced=[];let quick=[];
    for row in m.rows{
        if row.phase=="busy"&&row.id in m.queue{
            timeout("accept-"+row.kind,480,false,Fn("receive"),m.queue[row.id]);
        }
        if row.phase=="resolved"{
            timeout("retire-"+row.kind,1100,false,Fn("retire"),#{request:row.id,revision:row.revision});
        }else{
            advanced.push(#{id:row.id,revision:row.revision,kind:row.kind,question:row.question,task:row.task,options:row.options,
                facts:row.facts,busy:row.phase=="busy",notice:row.notice});
        }
        let primary=if row.kind=="approval"{"Approve"}else{row.options[0].title};
        let secondary=if row.kind=="approval"{"Decline"}else{row.options[1].title};
        quick.push(#{id:row.id,revision:row.revision,title:row.question,detail:row.task,
            primary:primary,secondary:secondary,collapsed:row.collapsed,phase:row.phase,receipt:row.receipt,notice:row.notice});
    }
    let controls=[control("reject","Decline next answer",Fn("reject")),control("reset","Reset decisions",Fn("reset")),
        control("refresh","Revise first decision",Fn("refresh"))];
    let demo=if props.mode=="card"{
        controls.push(control("reveal","Reveal decisions",Fn("reveal")));
        cards::DecisionCard(#{key:"deck",decisions:advanced,reveal:m.reveal,on_answer:Fn("proposed"),on_later:Fn("later")})
    }else{
        inbox::DecisionInbox(#{key:"inbox",decisions:quick,on_answer:Fn("proposed")})
    };
    let children=[demo,row(controls).with_style(style().min_width(px(0)).gap(px(6)).flex_wrap())];
    if m.outcome!=""{children.push(text(m.outcome).with_key("outcome").accessibility_role("status")
        .with_style(style().typography("caption").text_color(theme_color("text_muted"))));}
    column(children).with_key(props.key).audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).items_start().gap(px(8)).width(relative(1.0))))
}
