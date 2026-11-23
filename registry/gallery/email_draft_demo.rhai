import "patterns/email_draft" as draft;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/email_draft_demo","export":"EmailDraftDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/email_draft","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        slots:#{},parts:["root"],events:#{},timers:["resolve"]},render:Fn("render_demo")
});
fn EmailDraftDemo(p){render_component("gallery/email_draft_demo",p)}
fn prior(ctx){
    let s=ctx.get_state("demo");
    if s==(){#{number:0,revision:0,phase:"draft",request:(),receipt:(),refuse:false,narrow:false,empty:false,renamed:false,note:""}}else{s}
}
fn draft_id(s){"renewal:"+s.number.to_string()}
fn valid(s,p){p.draft==draft_id(s)&&s.revision==p.revision}
fn roster(s){
    [#{id:"jonas",name:"Jonas Weber",email:"jonas@example.test"},#{id:"lena",name:"Lena Hart",email:"lena@example.test"},
        #{id:"finance/采购",name:if s.renamed{"Finance operations"}else{"Finance team"},email:"billing@example.test"}]
}
fn send(ctx,p){
    let s=prior(ctx);
    if !valid(s,p)||s.phase!="draft"||s.empty||p.recipient==""{return;}
    let destination="";
    for c in roster(s){if c.id==p.recipient{destination=c.name;break;}}
    if destination==""{return;}
    s.phase="sending";s.request=p;s.receipt=#{recipient_name:destination,attachment_label:if p.attachment==""{()}else{"Renewal-quote.pdf"}};
    s.revision+=1;s.note="The Host is checking this message";
    ctx.set_state("demo",s);
}
fn resolve(ctx,ticket){
    let s=prior(ctx);
    if !valid(s,ticket)||s.phase!="sending"||s.request==(){return;}
    s.phase=if s.refuse{"draft"}else{"sent"};s.revision+=1;
    // These are local fixtures, never transport commands. Do not echo message text.
    s.note=if s.refuse{"The Host declined the send"}else{
        "Simulated send to "+s.request.recipient+(if s.request.attachment==""{" without a file"}else{" with a file"})
    };
    if s.refuse{s.receipt=();}
    s.refuse=false;s.request=();ctx.set_state("demo",s);
}
fn discard(ctx,p){
    let s=prior(ctx);
    if !valid(s,p)||s.phase!="draft"{return;}
    s.phase="discarded";s.revision+=1;s.note="The Host discarded the draft";ctx.set_state("demo",s);
}
fn restore(ctx,p){
    let s=prior(ctx);
    if !valid(s,p)||!(s.phase in ["discarded","sent"]){return;}
    s.phase="draft";s.revision+=1;s.receipt=();s.note="The Host restored the draft";ctx.set_state("demo",s);
}
fn link(ctx,p){
    let s=prior(ctx);
    if valid(s,p){s.note="Requested "+p.id+" in the Host";ctx.set_state("demo",s);}
}
fn control(key,ctx,event){
    let s=prior(ctx);
    switch key{
        "refuse"=>{s.refuse=true;s.note="The next send will be declined";},
        "empty"=>{s.empty=!s.empty;s.revision+=1;},
        "narrow"=>{s.narrow=!s.narrow;},
        "rename"=>{s.renamed=true;s.revision+=1;s.note="The Host updated the address book";},
        _=>{s.number+=1;s.revision+=1;s.phase="draft";s.request=();s.receipt=();s.refuse=false;s.empty=false;s.renamed=false;s.note="New local draft";}
    }
    ctx.set_state("demo",s);
}
fn control_btn(key,text,disabled){button::Button(#{key:key,text:text,size:"xs",variant:"ghost",disabled:disabled,on_click:Fn("control").curry(key)})}
fn render_demo(ctx,p){
    let s=prior(ctx);
    if s.phase=="sending"{timeout("resolve",780,false,Fn("resolve"),#{draft:draft_id(s),revision:s.revision});}
    let tones=[#{id:"friendly",label:"Friendly",body:"Hi {first}, a heads-up that your Growth plan renews on October 9 for $8,900 a year. If you'd like to add seats or switch to monthly billing, you can review the quote here:"},
        #{id:"short",label:"Short",body:"Hi {first}, your Growth plan renews on October 9 ($8,900/year). Quote for changes:"},
        #{id:"formal",label:"Formal",body:"Dear {name}, your Growth subscription is scheduled to renew on October 9 at $8,900 per year. Please review the quote if you would like to adjust seats or billing."}];
    let card=draft::EmailDraft(#{key:"email",draft:draft_id(s),revision:s.revision,phase:s.phase,reference:"SUB-1182",account:"Northwind Logistics",
        amount:"$8,900",status:"Renews in 5 days",agent:"Renewal agent",contacts:if s.empty{[]}else{roster(s)},tones:tones,receipt:if s.phase=="sent"{s.receipt}else{()},
        attachment:#{id:"renewal-quote",label:"Renewal-quote.pdf"},link:#{id:"review-quote",label:"Review quote"},
        on_send:Fn("send"),on_discard:Fn("discard"),on_undo:Fn("restore"),on_link:Fn("link")});
    column([row([control_btn("refuse","Refuse next send",s.phase!="draft"||s.refuse),
       control_btn("empty",if s.empty{"Restore recipients"}else{"Remove recipients"},s.phase!="draft"),
       control_btn("rename","Rename saved contact",s.phase!="sent"||s.renamed),
       control_btn("narrow",if s.narrow{"Wide draft"}else{"Narrow draft"},false),control_btn("reset","New draft",false)])
       .with_style(style().min_width(px(0)).gap(px(6)).flex_wrap()),
       column([card]).with_key("sized").with_style(style().width(if s.narrow{px(320)}else{relative(1.0)}).max_width(relative(1.0))),
       text(s.note).accessibility_role("status").accessibility_label(s.note).with_style(style().typography("caption").text_color(theme_color("text_muted")))])
       .with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(10))))
}
