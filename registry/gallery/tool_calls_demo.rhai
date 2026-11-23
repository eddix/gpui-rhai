import "patterns/tool_call_card" as card;
import "patterns/tool_call_group" as group;
import "components/button" as button;
define_component(#{
    metadata:#{id:"gallery/tool_calls_demo","export":"ToolCallsDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/tool_call_card","patterns/tool_call_group","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false},
        mode:#{schema:#{type:"string",allowed:["card","group"]},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},events:#{},parts:["root"],timers:["decide"]},render:Fn("demo")
});
fn ToolCallsDemo(p){render_component("gallery/tool_calls_demo",p)}
fn t0(){1763631600000}
fn sample(ctx){
    let s=ctx.get_state("sample");
    if s==(){#{revision:0,phase:"initial",now:t0()+1200,reject:false,pending:(),note:"",narrow:false}}else{s}
}
fn toolbar_action(key,ctx,event){
    let s=sample(ctx);
    switch key{
        "error"=>{s.phase="error";s.revision+=1;s.pending=();s.note="Published failure";},
        "finish"=>{s.phase="complete";s.revision+=1;s.pending=();s.note="Published result";},
        "clock"=>{s.now+=750;},
        "refuse"=>{s.reject=!s.reject;s.note=if s.reject{"The next retry will be refused"}else{""};},
        "narrow"=>{s.narrow=!s.narrow;},
        _=>{s=();}
    }
    ctx.set_state("sample",s);
}
fn retry(ctx,p){
    let s=sample(ctx);
    if s.pending!=()||s.revision!=p.revision{return;}
    s.pending=#{receipt:p,refused:s.reject};s.reject=false;s.note="Waiting for retry approval";
    ctx.set_state("sample",s);
}
fn decide(ctx,request){
    let s=sample(ctx);let p=request.receipt;
    if s.pending!=request||s.revision!=p.revision{return;}
    s.pending=();
    if request.refused{s.note="Retry request refused";}
    else{s.phase="retry";s.revision+=1;s.note="Accepted retry: "+p.id;}
    ctx.set_state("sample",s);
}
fn copied(ctx,p){
    let s=sample(ctx);s.note="Copied "+p.section+" for "+p.id;ctx.set_state("sample",s);
}
fn tool(id,name){button::Button(#{key:id,text:name,size:"xs",variant:"ghost",on_click:Fn("toolbar_action").curry(id)})}
fn long_result(){
    let rows=[];
    for i in 0..34{rows.push(#{page:i+1,stage:"indexed",ref:"document-"+(i+1).to_string()});}
    #{documents:rows, indexed:34}
}
fn call_sample(s){
    let phase=s.phase;
    let status=if phase=="error"{"error"}else if phase=="retry"{"running"}else{"success"};
    let c=#{id:"index/task",name:"read_index",summary:"Resolve the current document index",
        status:status,icon:"database",arguments:#{folder:"documentation",recursive:true}};
    if status=="success"{c.result=long_result();c.duration_ms=840;}
    else if status=="error"{c.error="Cannot load the index. Try again after the cache warms.";c.duration_ms=420;}
    else{c.started_ms=t0();}
    c
}
fn group_sample(s){
    let running=s.phase=="initial";let cache=s.phase=="retry";
    [#{id:"search/task",name:"search_docs",icon:"search",summary:"Find related topics",
        arguments:#{query:"native layout"},result:if running{()}else{#{matches:3}},
        status:if running{"running"}else{"success"},started_ms:t0(),duration_ms:if running{()}else{1240}},
     #{id:"meta/task",name:"read_metadata",icon:"file",summary:"Read title and labels",status:"success",arguments:#{page:2},result:#{title:"Overview"},duration_ms:180},
     #{id:"cache/task",name:"load_cache",icon:"tool",summary:"Load the local cache",status:if cache{"running"}else{"error"},
        error:if cache{()}else{"Cache not ready."},started_ms:t0(),duration_ms:if cache{()}else{240}}]
}
fn demo(ctx,p){
    let s=sample(ctx);
    if s.pending!=(){timeout("decide",300,false,Fn("decide"),s.pending);}
    let widget=if p.mode=="card"{
        card::ToolCallCard(#{key:"single",session:"inspect",revision:s.revision,entry:call_sample(s),pending:s.pending!=(),
            now_ms:s.now,max_code_height:128,on_retry:Fn("retry"),on_copy:Fn("copied")})
    }else{
        group::ToolCallGroup(#{key:"stack",session:"batch",revision:s.revision,calls:group_sample(s),now_ms:s.now,
            pending:if s.pending==(){[]}else{[s.pending.receipt.id]},on_retry:Fn("retry"),on_copy:Fn("copied")})
    };
    column([row([tool("error","Publish failure"),tool("finish","Publish result"),tool("clock","Advance clock"),
        tool("refuse",if s.reject{"Accept next retry"}else{"Refuse next retry"}),
        tool("narrow",if s.narrow{"Wide card"}else{"Narrow card"}),tool("reset","Reset fixture")])
        .with_style(style().flex_wrap().gap(px(4))),
        column([widget]).with_style(if s.narrow{style().width(px(340)).max_width(relative(1.0))}else{style().width(relative(1.0))}),
        text(s.note).accessibility_role("status").accessibility_label(s.note).with_style(style().typography("caption")
          .text_color(theme_color("text_muted")))])
      .with_style(ctx.component_style("root",style().gap(px(10)).min_width(px(0))))
}
