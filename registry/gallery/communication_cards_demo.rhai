import "patterns/notification_stack" as notices;
import "patterns/away_summary" as away;
import "components/button" as btn;

define_component(#{
    metadata:#{id:"gallery/communication_cards_demo","export":"CommunicationCardsDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/notification_stack","patterns/away_summary","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},mode:#{schema:#{type:"string",allowed:["notices","away"]},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn CommunicationCardsDemo(p){render_component("gallery/communication_cards_demo",p)}
fn get(p,k,d){if k in p&&p[k]!=(){p[k]}else{d}}
fn model(ctx){
    let s=ctx.get_state("model");if s!=(){return s;}
    #{revision:0,front:0,pending:(),refuse:false,finished:#{},collapsed:false,narrow:false,clock:false,note:""}
}
fn start(kind,ctx,e){
    let s=model(ctx);
    if s.pending!=()||s.revision!=e.revision{return;}
    s.pending=#{kind:kind,ticket:e,refused:s.refuse};s.refuse=false;s.note="Waiting for the Host…";
    ctx.set_state("model",s)
}
fn handled_action(ctx,e){
    let s=model(ctx);
    if s.revision==e.revision&&s.pending==(){
        s.note="Host action: "+e.action;ctx.set_state("model",s);
    }
}
fn settle(ctx,p){
    let s=model(ctx);
    if s.pending!=p||s.revision!=p.ticket.revision{return;}
    s.pending=();
    if p.refused{s.note="The Host kept the current snapshot.";}
    else{
        if p.kind=="dismiss"{s.front+=1;s.note="Dismissed: "+p.ticket.id;}
        else{s.finished[p.ticket.id]=true;s.note="Accepted update: "+p.ticket.id;}
        s.revision+=1;
    }
    ctx.set_state("model",s);
}
fn fold(ctx,e){
    let s=model(ctx);if s.revision!=e.revision{return;}
    s.collapsed=e.collapsed;ctx.set_state("model",s);
}
fn refuse(ctx,e){let s=model(ctx);s.refuse=true;s.note="Next request will be refused.";ctx.set_state("model",s);}
fn narrow(ctx,e){let s=model(ctx);s.narrow=!s.narrow;ctx.set_state("model",s);}
fn clock(ctx,e){let s=model(ctx);s.clock=!s.clock;ctx.set_state("model",s);}
fn reset(ctx,e){ctx.set_state("model",());}
fn control(k,label,cb){btn::Button(#{key:k,text:label,size:"sm",variant:"ghost",on_click:cb})}
fn queue(s){
    let list=[
        #{id:"release",badge:"Team news",title:"Release ready",text:"The new workspace is available. Review the changes when you have a moment.",pattern:"orbit",action:#{id:"notes",label:"Release notes",external:true}},
        #{id:"billing",badge:"Account",title:"Billing moved",text:"Invoices and usage now live together under Settings. Your existing plan and billing date are unchanged, and old invoices are still available.",pattern:"grid",action:#{id:"billing",label:"Open account"}},
        #{id:"keys",badge:"Getting started",title:"Shortcuts are ready",text:"Open the command menu to find available shortcuts.",pattern:"rays"},
        #{id:"fourth",badge:"Workspace",title:"Background maintenance",text:"This notice is not mounted while it is behind the stack.",pattern:"grid"},
    ];
    let out=[];for i in s.front..list.len{out.push(list[i]);}out
}
fn changes(s){
    let rows=[
        #{id:"deployment",title:"Prepare release",kind:"done",detail:"Staging checks completed."},
        #{id:"metadata",title:"Choose metadata",kind:"needs_you",detail:"Waiting for you to choose a title."},
        #{id:"import",title:"Import archive",kind:"failed",detail:"A file could not be read."},
        #{id:"pipeline",title:"Migrate pipeline",kind:"needs_you",detail:"Pick the target queue."},
        #{id:"docs",title:"Regenerate docs",kind:"done",detail:"Updated 12 pages."},
        #{id:"reports",title:"Update reports",kind:"done",detail:"Weekly rollup is ready."},
    ];
    for i in 0..rows.len{
        let item=rows[i];
        if item.id in s.finished{item.kind="done";item.detail="The Host finished the task.";rows[i]=item;}
    }
    rows
}
fn demo(ctx,p){
    let s=model(ctx);
    if s.pending!=(){timeout("settle",300,false,Fn("settle"),s.pending);}
    let card=if p.mode=="notices"{
        notices::NotificationStack(#{key:"stack",stack:"workspace-news",revision:s.revision,notices:queue(s),busy:s.pending!=(),
            on_dismiss:Fn("start").curry("dismiss"),on_action:Fn("handled_action")})
    }else{
        let waiting=if s.pending==(){[]}else{[s.pending.ticket.id]};
        away::AwaySummary(#{key:"summary",session:"shift-a",revision:s.revision,since_ms:1763631600000,now_ms:1763639640000,
            time_style:if s.clock{"clock"}else{"duration"},time_zone:"UTC",items:changes(s),limit:3,collapsed:s.collapsed,pending:waiting,
            on_action:Fn("start").curry("task"),on_collapsed_change:Fn("fold")})
    };
    let actions=[control("refused","Refuse next request",Fn("refuse")),control("width",if s.narrow{"Wide card"}else{"Narrow card"},Fn("narrow"))];
    if p.mode=="away"{actions.push(control("clock",if s.clock{"Show duration"}else{"Show clock time"},Fn("clock")));}
    else{actions.push(control("reset","Reset stack",Fn("reset")));}
    let wide=if p.mode=="notices"{380}else{600};
    column([
        column([card]).with_key("width").with_style(style().width(px(if s.narrow{316}else{wide})).max_width(relative(1.0)).min_width(px(0))),
        row(actions).with_key("controls").with_style(style().min_width(px(0)).flex_wrap()),
        text(s.note).with_key("receipt").accessibility_role("status").with_style(style().typography("caption").text_color(theme_color("text_muted"))),
    ]).with_key(p.key).with_style(ctx.component_style("root",style().gap(px(12)).min_width(px(0))))
}
