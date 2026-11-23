// Local, deterministic demonstration. Neither model requests nor microphone access occur.
import "patterns/writing_assistant" as writer;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/writing_demo","export":"WritingDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/writing_assistant","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},events:#{},slots:#{},parts:["root"]},
    render:Fn("demo_view")
});
fn WritingDemo(props){render_component("gallery/writing_demo",props)}
fn initial(serial){
    #{conversation:"writing/"+serial.to_string(),serial:serial,revision:0,seq:0,draft:"",messages:[],
        phase:"idle",speech:#{state:"idle",lease:"",preview:""},pending:(),answer:(),take:(),decline:false,notice:""}
}
fn model(ctx){let m=ctx.get_state("model");if m==(){initial(1)}else{m}}
fn current(m,p){p.conversation==m.conversation&&p.revision==m.revision}
fn save(ctx,m){ctx.set_state("model",m);}
fn changed(ctx,p){
    let m=model(ctx);if !current(m,p)||m.pending!=()||m.take!=(){return;}
    m.draft=p.text;save(ctx,m);
}
fn send(ctx,p){
    let m=model(ctx);if !current(m,p)||m.pending!=()||m.answer!=()||m.take!=(){return;}
    m.seq+=1;m.revision+=1;m.phase="waiting";m.notice="";
    m.pending=#{conversation:m.conversation,seq:m.seq,prompt:p.text,reject:m.decline,source:p.source};
    m.decline=false;save(ctx,m);
}
fn accepted(ctx,p){
    let m=model(ctx);if m.pending!=p{return;}
    m.pending=();m.revision+=1;
    if p.reject{m.phase="failed";m.notice="The demo Host declined the message. Draft retained.";save(ctx,m);return;}
    let order=p.seq.to_string();
    m.messages.push(#{id:"prompt/"+order,role:"user",text:p.prompt});m.draft="";m.phase="waiting";
    m.answer=#{conversation:m.conversation,key:"answer/"+order,step:0};
    save(ctx,m);
}
fn writing_tick(ctx,answer){
    let m=model(ctx);if m.answer!=answer{return;}
    let phrases=["Here’s a first draft. ","Thanks for the update — let’s keep the deadline visible.\n\n",
        "Please flag anything that might slip by Thursday. ","We can adjust the plan before Friday."];
    if answer.step==0{
        m.messages.push(#{id:answer.key,role:"assistant",text:phrases[0]});
    }else{
        let entries=m.messages;let last=entries.len-1;let item=entries[last];
        if item.id!=answer.key{return;}
        item.text+=phrases[answer.step];entries[last]=item;m.messages=entries;
    }
    m.revision+=1;
    if answer.step+1==phrases.len{m.phase="idle";m.answer=();}
    else{m.phase="streaming";m.answer.step+=1;}
    save(ctx,m);
}
fn reset(ctx,p){
    let m=model(ctx);if current(m,p){save(ctx,initial(m.serial+1));}
}
fn decline(ctx,p){
    let m=model(ctx);m.decline=true;m.notice="The next message will be declined.";save(ctx,m);
}
fn dictate(ctx,p){
    let m=model(ctx);if !current(m,p)||m.pending!=()||m.answer!=(){return;}
    if p.action=="start"{
        if m.take!=()||p.lease!=m.speech.lease{return;}
        m.seq+=1;let lease="take/"+m.seq.to_string();
        m.take=#{conversation:m.conversation,lease:lease,step:0};m.speech=#{state:"listening",lease:lease,preview:""};
        m.notice="";
    }else{
        if m.take==()||m.take.lease!=p.lease{return;}
        if m.speech.preview!=""{m.draft=m.speech.preview;}
        m.take=();m.speech=#{state:"idle",lease:"",preview:""};
        m.notice="Review the simulated dictation before sending.";
    }
    m.revision+=1;save(ctx,m);
}
fn dictate_tick(ctx,take){
    let m=model(ctx);if m.take!=take{return;}
    let stages=["Write a friendly","Write a friendly reminder","Write a friendly reminder that the report is due Friday."];
    m.speech.preview=stages[take.step];
    if take.step+1==stages.len{
        m.draft=m.speech.preview;m.take=();m.speech=#{state:"idle",lease:"",preview:""};
        m.notice="Review the simulated dictation before sending.";
    }else{m.take.step+=1;}
    save(ctx,m);
}
fn demo_view(ctx,props){
    let m=model(ctx);
    if m.pending!=(){timeout("ask",310,false,Fn("accepted"),m.pending);}
    if m.answer!=(){timeout("stream",if m.answer.step==0{460}else{370},false,Fn("writing_tick"),m.answer);}
    if m.take!=(){timeout("dictation",290,false,Fn("dictate_tick"),m.take);}
    let panel=writer::WritingAssistant(#{key:"writer",conversation:m.conversation,revision:m.revision,
        messages:m.messages,draft:m.draft,draft_locked:m.pending!=(),phase:m.phase,message:m.notice,speech:m.speech,
        suggestions:[#{id:"email/1",label:"Polish my email"},#{id:"shorter",label:"Make it shorter"},
            #{id:"tone",label:"Fix the tone"},#{id:"reply",label:"Draft a reply"}],
        on_change:Fn("changed"),on_send:Fn("send"),on_reset:Fn("reset"),on_dictate:Fn("dictate")});
    column([
        panel,
        text("Dictation is simulated. No microphone or model is used.").with_style(style().typography("caption").text_color(theme_color("text_muted"))),
        button::Button(#{key:"decline",text:"Decline next message",size:"sm",variant:"secondary",on_click:Fn("decline")}),
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).items_start().gap(px(8))))
}
