import "patterns/collaboration_stream" as stream;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/collaboration_stream_demo","export":"CollaborationStreamDemo","version":"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/collaboration_stream","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn CollaborationStreamDemo(p){render_component("gallery/collaboration_stream_demo",p)}
fn sample(){
    [
        #{id:"brief",from:"you",to:["all"],kind:"ask",text:"Check the release guide before we share it.",at:"14:01"},
        #{id:"start",from:"atlas",to:["scribe"],kind:"spawn",text:"Scribe joined the review"},
        #{id:"plan",from:"atlas",to:["scribe"],kind:"handoff",text:"Compare the installation steps with the current CLI help.",at:"14:02"},
        #{id:"verify",from:"scribe",to:["you"],kind:"share",text:"The fresh install and theme switch both worked.",at:"14:04",
            attachments:[#{id:"log",label:"Verification log",kind:"tool",meta:"2 checks",preview:"install: passed\ntheme switch: passed"}],
            facts:[#{label:"Checks",value:"2 passed"},#{label:"Elapsed",value:"14 s"}],
            items:[#{label:"Clean install",detail:"fresh profile"},#{label:"Theme switch",detail:"Modern and Clean"}]},
        #{id:"folder",from:"scribe",to:["atlas"],kind:"share",text:"I left notes and draft examples in the review folder.",at:"14:07",
            attachments:[#{id:"notes",label:"review/",meta:"3 entries",preview:"drafts/\nnotes.md\nchecks.txt"},
                #{id:"checks",label:"checks.csv",meta:"2 results",preview:"step,result\nInstallation,passed\nThemes,\"Modern, Clean\""}]},
        #{id:"confirm",from:"atlas",to:["you"],kind:"question",text:"Should we keep the quick-start example?",at:"14:10",reply_to:"brief",
            facts:[#{label:"Status",value:"Decision needed"}]}
    ]
}
fn state(c){
    let v=c.get_state("sample");
    if v!=(){v}else{#{run:0,revision:0,count:0,live:true,narrow:false,typing:false,more_agents:false,draft:"",sent:[],pending:(),deny:false,notice:""}}
}
fn session(v){"editorial:"+v.run.to_string()}
fn op(which,c,e){
    let v=state(c);
    switch which{
        "append"=>{v.count+=1;v.revision+=1;v.notice="Agent message appended";},
        "narrow"=>{v.narrow=!v.narrow;},
        "live"=>{v.live=!v.live;},
        "typing"=>{v.typing=!v.typing;},
        "deny"=>{v.deny=true;v.notice="The next send will be declined";},
        "recipients"=>{
            if v.pending==(){
                v.more_agents=true;
                v.draft="@Atlas @Scribe @Interface @Verification @Runtime please review";
                v.notice="Five-recipient draft loaded";
            }
        },
        _=>{v.run+=1;v.revision=0;v.count=0;v.draft="";v.pending=();v.sent=[];v.typing=false;v.more_agents=false;v.notice="Conversation reset";},
    }
    c.set_state("sample",v);
}
fn changed(c,e){let v=state(c);v.draft=e.text;c.set_state("sample",v);}
fn send(c,e){
    let v=state(c);
    if v.pending!=()||e.session!=session(v)||e.revision!=v.revision{return;}
    v.pending=e;v.notice="Message proposed";c.set_state("sample",v);
}
fn accept(c,e){
    let v=state(c);
    if v.pending==()||e.session!=session(v){return;}
    if v.deny{v.notice="Send declined by Host";v.deny=false;}
    else{
        v.sent.push(#{id:"sent:"+v.sent.len.to_string(),from:"you",to:e.to,kind:"reply",text:e.text,reply_to:e.reply_to,at:"14:12"});
        v.draft="";v.revision+=1;v.notice=if e.reply_to==(){ "Message accepted by Host: "+e.to[0] }else{"Reply accepted by Host: "+e.to[0]};
    }
    v.pending=();c.set_state("sample",v);
}
fn control(op,title){button::Button(#{key:op,text:title,size:"xs",variant:"ghost",on_click:Fn("op").curry(op)})}
fn render_demo(c,p){
    let v=state(c);if v.pending!=(){timeout("decide",460,false,Fn("accept"),v.pending);}
    let messages=sample();
    for i in 0..min(v.count,32){
        messages.push(#{id:"more:"+i.to_string(),from:"scribe",to:["you"],kind:"reply",
            text:"Review update "+(i+1).to_string()+": the example works in a clean workspace.",at:"14:11"});
    }
    messages+=v.sent;
    if v.typing{messages.push(#{id:"typing",from:"scribe",to:["all"],kind:"reply",typing:true,text:""});}
    let agents=[#{id:"atlas",name:"Atlas",model:"Planner"},#{id:"scribe",name:"Scribe",model:"Writer"}];
    if v.more_agents{
        agents+=[#{id:"interface",name:"Interface",model:"Designer"},
            #{id:"quality",name:"Verification",model:"Verifier"},
            #{id:"runtime",name:"Runtime",model:"Operator"}];
    }
    let data=stream::CollaborationStream(#{key:"activity",session:session(v),revision:v.revision,task:"Release guide",agents:agents,messages:messages,
        live:v.live,max_height:405,draft:v.draft,pending:v.pending!=(),on_change:Fn("changed"),on_send:Fn("send")});
    let controls=row([control("append","Append message"),control("narrow",if v.narrow{"Wide stream"}else{"Narrow stream"}),
        control("live",if v.live{"Stop live mode"}else{"Start live mode"}),control("typing",if v.typing{"Stop typing"}else{"Show typing"}),
        control("deny","Refuse next send"),control("recipients","Load five-recipient draft"),control("reset","Reset conversation")])
        .with_style(style().flex_wrap().gap(theme_length("space.spacing_unit")));
    let narrow_width=if v.more_agents{px(220)}else{px(360)};
    let sized=column([data]).with_key("sized").with_style(style().min_width(px(0)).width(relative(1.0)).max_width(if v.narrow{narrow_width}else{px(790)}));
    let notice=text(v.notice).accessibility_role("status").accessibility_label(v.notice).with_style(style().typography("caption").text_color(theme_color("text_muted")));
    column([controls,sized,notice]).with_style(c.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2))))
}
