import "patterns/context_window" as budget;
import "components/button" as btn;
define_component(#{
    metadata:#{id:"gallery/context_window_demo","export":"ContextWindowDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/context_window","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},events:#{},parts:["root"],timers:["decide"]},render:Fn("demo")
});
fn ContextWindowDemo(p){render_component("gallery/context_window_demo",p)}
fn initial(){
    #{revision:0,limit:100000,chat:39000,files:[
        #{id:"design",label:"Architecture notes",tokens:19000},
        #{id:"changes",label:"Change log",tokens:9000}
      ],pending:(),reject:false,narrow:false,saved:(),compacted:(),note:""}
}
fn sample(ctx){let s=ctx.get_state("sample");if s==(){initial()}else{s}}
fn toolbar(key,ctx,event){
    let s=sample(ctx);
    switch key{
        "reject"=>{s.reject=!s.reject;s.note=if s.reject{"The next proposal will be refused"}else{""};},
        "tight"=>{s.limit=if s.limit==70000{100000}else{70000};s.revision+=1;s.pending=();s.saved=();},
        "none"=>{s.limit=if s.limit==0{100000}else{0};s.revision+=1;s.pending=();s.saved=();},
        "narrow"=>{s.narrow=!s.narrow;},
        _=>{s=();}
    }
    ctx.set_state("sample",s);
}
fn propose(ctx,p){
    let s=sample(ctx);if s.pending!=()||s.revision!=p.revision{return;}
    s.pending=#{receipt:p,refused:s.reject};s.reject=false;s.note="Waiting for context approval";
    ctx.set_state("sample",s);
}
fn decide(ctx,proposal){
    let s=sample(ctx);let p=proposal.receipt;
    if s.pending!=proposal||s.revision!=p.revision{return;}
    s.pending=();
    if proposal.refused{s.note="Context change refused";}
    else{
        if p.kind=="remove"{
            let remaining=[];
            for item in s.files{if item.id!=p.item{remaining.push(item);}}
            s.files=remaining;s.saved=();
        }else{
            s.saved=s.chat-12000;s.chat=12000;s.compacted=#{part:"chat",tokens:s.chat};
        }
        s.revision+=1;s.note="Accepted "+p.kind+" at revision "+s.revision.to_string();
    }
    ctx.set_state("sample",s);
}
fn show_action(ctx,p){
    let s=sample(ctx);s.note="New conversation requested";ctx.set_state("sample",s);
}
fn control(key,words){btn::Button(#{key:key,text:words,size:"xs",variant:"ghost",on_click:Fn("toolbar").curry(key)})}
fn demo(ctx,p){
    let s=sample(ctx);
    if s.pending!=(){
        timeout("decide",if s.pending.receipt.kind=="compact"{950}else{330},false,Fn("decide"),s.pending);
    }
    let receipt=if s.pending==(){()}else{s.pending.receipt};
    let card=budget::ContextWindow(#{key:"capacity",context:"chat",revision:s.revision,limit:s.limit,model:"Document assistant",
        parts:[#{id:"system",label:"System prompt",tokens:8000,locked:true},#{id:"tools",label:"Tools",tokens:11000,locked:true},
            #{id:"files/资料",label:"Files",tokens:99000,items:s.files},#{id:"chat",label:"Conversation",tokens:s.chat,compact:true},
            #{id:"memory",label:"Memory",tokens:5000,locked:true}],
        pending:if receipt==(){()}else{#{kind:receipt.kind,part:receipt.part,item:receipt.item}},
        saved_tokens:s.saved,compacted:s.compacted,on_request:Fn("propose"),
        action_label:"New chat",on_action:Fn("show_action")});
    column([row([control("reject",if s.reject{"Accept next change"}else{"Refuse next change"}),
       control("tight",if s.limit==70000{"Restore limit"}else{"Lower limit"}),
       control("none",if s.limit==0{"Use limit"}else{"Remove limit"}),
       control("narrow",if s.narrow{"Wide card"}else{"Narrow card"}),control("reset","Reset context")])
       .with_style(style().flex_wrap().gap(px(5))),
       column([card]).with_style(if s.narrow{style().width(px(260)).max_width(relative(1.0))}else{style().width(relative(1.0))}),
       text(s.note).accessibility_role("status").accessibility_label(s.note).with_style(style().typography("caption").text_color(theme_color("text_muted")))])
       .with_style(ctx.component_style("root",style().gap(px(10)).min_width(px(0))))
}
