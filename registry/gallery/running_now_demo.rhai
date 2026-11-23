import "patterns/running_now" as run;
import "components/button" as btn;
define_component(#{
    metadata:#{id:"gallery/running_now_demo","export":"RunningNowDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/running_now","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        events:#{},slots:#{},parts:["root"],timers:["settle","retire"]},render:Fn("demo")
});
fn RunningNowDemo(p){render_component("gallery/running_now_demo",p)}
fn sample(ctx){
    let v=ctx.get_state("sample");if v!=(){return v;}
    #{revision:0,open:false,changes:#{},pending:(),reject:false,empty:false,retire_armed:false,retiring:false,removed:false,note:""}
}
fn opened(ctx,value){
    let s=sample(ctx);s.open=value;
    if value&&s.retire_armed{s.retire_armed=false;s.retiring=true;}
    ctx.set_state("sample",s);
}
fn retire(ctx,event){
    let s=sample(ctx);
    if s.retiring{s.retiring=false;s.removed=true;s.revision+=1;s.pending=();ctx.set_state("sample",s);}
}
fn action(ctx,p){
    let s=sample(ctx);if s.pending!=()||p.revision!=s.revision{return;}
    s.pending=#{id:p.id,kind:p.kind,revision:p.revision,refused:s.reject};
    s.reject=false;s.note="Waiting for the Host…";ctx.set_state("sample",s);
}
fn settle(ctx,request){
    let s=sample(ctx);if s.pending!=request||request.revision!=s.revision{return;}
    s.pending=();
    if request.refused{s.note="The Host refused the change.";}
    else{
        s.changes[request.id]=if request.kind=="pause"{"paused"}else{"running"};
        s.revision+=1;s.note="Accepted "+request.kind+" for "+request.id;
    }
    ctx.set_state("sample",s);
}
fn opened_task(ctx,p){let s=sample(ctx);s.note="Open task: "+p.id;ctx.set_state("sample",s);}
fn opened_all(ctx,p){let s=sample(ctx);s.note="Open full task list";ctx.set_state("sample",s);}
fn change(key,ctx,event){
    let s=sample(ctx);
    if key=="reject"{s.reject=!s.reject;s.note=if s.reject{"The next request will be refused."}else{""};}
    else if key=="empty"{s.empty=!s.empty;s.revision+=1;s.pending=();}
    else if key=="complete"{s.changes["index"]="done";s.revision+=1;s.pending=();}
    else if key=="retire"{s.retire_armed=true;}
    else{s=();}ctx.set_state("sample",s);
}
fn tool(key,label){btn::Button(#{key:key,text:label,variant:"ghost",size:"sm",on_click:Fn("change").curry(key)})}
fn tasks(s){
    if s.empty{return [];}
    let list=[
        #{id:"index",title:"Index documents",step:"Embedding the latest files",done:2,total:4,status:"running"},
        #{id:"report",title:"Prepare report",step:"Waiting for approval",done:1,total:3,status:"needs_you"},
        #{id:"tests",title:"Check integrations",step:"Running smoke tests",done:1,total:5,status:"running"},
        #{id:"cleanup",title:"Clean up artifacts",step:"Scanning output",done:4,total:5,status:"paused"},
        #{id:"export",title:"Export data",step:"",done:3,total:3,status:"done"}
    ];
    for i in 0..list.len{
        let t=list[i];if t.id in s.changes{t.status=s.changes[t.id];list[i]=t;}
    }
    if s.removed{list.remove(0);}
    list
}
fn demo(ctx,p){
    let s=sample(ctx);
    if s.pending!=(){timeout("settle",300,false,Fn("settle"),s.pending);}
    if s.retiring{timeout("retire",420,false,Fn("retire"),());}
    let chip=run::RunningNow(#{key:"running",board:"workspace",revision:s.revision,tasks:tasks(s),open:s.open,align:"start",
        pending:if s.pending==(){[]}else{[s.pending.id]},on_open_change:Fn("opened"),
        on_action:Fn("action"),on_open_task:Fn("opened_task"),on_open_all:Fn("opened_all")});
    column([row([chip]).with_style(style().min_width(px(0))),
        row([tool("reject",if s.reject{"Accept next change"}else{"Refuse next change"}),
            tool("complete","Finish indexing"),tool("empty",if s.empty{"Restore tasks"}else{"Empty list"}),
            tool("retire","Remove first task on next open"),tool("reset","Reset board")])
            .with_style(style().flex_wrap().gap(px(6))),
        text(s.note).accessibility_role("status").accessibility_label(s.note).with_style(style().typography("caption").text_color(theme_color("text_muted")))
    ]).with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(10))))
}
