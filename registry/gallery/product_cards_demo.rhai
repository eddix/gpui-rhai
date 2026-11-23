import "patterns/change_summary" as summary;
import "patterns/tips_panel" as tips;
import "components/button" as btn;
import "components/kbd" as kbd;

define_component(#{
    metadata:#{id:"gallery/product_cards_demo","export":"ProductCardsDemo",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/change_summary","patterns/tips_panel","components/button","components/kbd"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},
                    mode:#{schema:#{type:"string",allowed:["changes","tips"]},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn ProductCardsDemo(p){render_component("gallery/product_cards_demo",p)}
fn model(ctx){
    let m=ctx.get_state("model");
    if m==(){#{narrow:false,week:0,revision:0,accepted:#{},pending:(),refuse:false,closed:false,notice:""}}else{m}
}
fn button(key,label,callback){btn::Button(#{key:key,text:label,size:"sm",variant:"ghost",on_click:callback})}
fn size_toggle(ctx,e){let m=model(ctx);m.narrow=!m.narrow;ctx.set_state("model",m);}
fn compare(ctx,e){let m=model(ctx);m.week=(m.week+1)%3;ctx.set_state("model",m);}
fn refusal(ctx,e){let m=model(ctx);m.refuse=true;m.notice="Next tip request will be refused.";ctx.set_state("model",m);}
fn reopen(ctx,e){let m=model(ctx);m.closed=false;m.notice="Tips reopened.";ctx.set_state("model",m);}
fn candidate(ctx,p){
    let m=model(ctx);if m.closed||m.pending!=()||p.series!="workspace-intro"||p.revision!=m.revision||p.id in m.accepted{return;}
    m.pending=#{ticket:p,refuse:m.refuse};m.refuse=false;m.notice="Waiting for the Host…";
    ctx.set_state("model",m);
}
fn settle(ctx,p){
    let m=model(ctx);if m.pending!=p||m.revision!=p.ticket.revision{return;}
    m.pending=();
    if p.refuse{m.notice="The Host declined this tip action.";}
    else{m.accepted[p.ticket.id]=true;m.revision+=1;m.notice="Completed: "+p.ticket.id;}
    ctx.set_state("model",m);
}
fn close(ctx,p){
    let m=model(ctx);if p.series!="workspace-intro"||p.revision!=m.revision{return;}
    m.pending=();m.closed=true;m.notice="Tips closed by the Host.";ctx.set_state("model",m);
}
fn table(m){
    if m.week==2{return [];}
    let latency=#{id:"latency",label:"Request latency",change:if m.week==0{"Down 14 ms to 62 ms"}else{"Up 6 ms to 68 ms"},tone:if m.week==0{"good"}else{"bad"},direction:if m.week==0{"down"}else{"up"}};
    let deploy=#{id:"deploy",label:"Deploy frequency",change:"Up to 6 deployments per day",tone:"good"};
    let repeat=#{id:"mobile",label:"Request latency",change:"Unchanged at 110 ms on mobile",tone:"neutral"};
    let acceptance=#{id:"acceptance",label:"跨区域发布成功率",change:"从 97.4% 上升至 99.1%，连续 12 次验证均成功。",tone:"good"};
    if m.week==0{[latency,deploy,repeat,acceptance]}else{[acceptance,repeat,latency,deploy]}
}
fn onboarding(m){
    let keys=row([text("Open commands with"),kbd::Kbd(#{key:"command-key",keys:["mod","K"]})])
        .with_style(style().min_width(px(0)).items_center().flex_wrap().gap(px(4)));
    let out=[
        #{id:"commands",title:"Find any command",description:"Open commands with the platform shortcut.",detail:keys,action:"Try commands",primary:true,done_label:"Commands enabled"},
        #{id:"theme",title:"Choose a theme",description:"Make a design yours without changing the workspace or losing the tab you are working on.",action:"Try themes"},
        #{id:"keys",title:"Navigate with keys",description:"Use Tab to move across controls and Space to activate the current action.",action:"See shortcuts"},
        #{id:"reviews",title:"Review a change",description:"Keep the original state until a proposal is accepted.",action:"Open review"},
        #{id:"history",title:"Read the history",description:"Walk through the last accepted snapshot.",action:"Read history"},
        #{id:"quota",title:"Check your quota",description:"See what is used and what is still available.",action:"View quota"},
        #{id:"search",title:"Search the workspace",description:"Find a file or open a recently modified report.",action:"Try search"},
        #{id:"trace",title:"Inspect a trace",description:"Locate the slow step in a run.",action:"Open trace"},
        #{id:"files",title:"Browse your files",description:"Keep the heading and footer in reach while reading a longer list.",action:"Open files"},
    ];
    for i in 0..out.len{let t=out[i];t.done=t.id in m.accepted;out[i]=t;}
    out
}
fn demo(ctx,p){
    let m=model(ctx);
    let content=[];let controls=[];
    if p.mode=="changes"{
        content.push(summary::ChangeSummary(#{key:"summary",period:["This week","Last week","No comparison"][m.week],changes:table(m)}));
        controls.push(button("compare","Change comparison",Fn("compare")));
    }else{
        if !m.closed{
            if m.pending!=(){timeout("settle",320,false,Fn("settle"),m.pending);}
            content.push(tips::TipsPanel(#{key:"tips",series:"workspace-intro",revision:m.revision,tips:onboarding(m),
                pending:if m.pending==(){()}else{m.pending.ticket.id},max_height:240,on_action:Fn("candidate"),on_close:Fn("close")}));
            controls.push(button("refuse","Refuse next tip",Fn("refusal")));
        }else{controls.push(button("reopen","Reopen tips",Fn("reopen")));}
    }
    controls.push(button("size",if m.narrow{"Wide card"}else{"Narrow card"},Fn("size_toggle")));
    let wide=if p.mode=="changes"{480}else{700};
    column([
        column(content).with_key("width").with_style(style().width(px(if m.narrow{324}else{wide})).max_width(relative(1.0)).min_width(px(0))),
        row(controls).with_key("controls").with_style(style().min_width(px(0)).flex_wrap()),
        text(m.notice).with_key("notice").accessibility_role("status").with_style(style().typography("caption").text_color(theme_color("text_muted"))),
    ]).with_key(p.key).with_style(ctx.component_style("root",style().gap(px(12)).min_width(px(0))))
}
