import "patterns/fan_out_grid" as grid;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/fan_out_grid_demo","export":"FanOutGridDemo","version":"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/fan_out_grid","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn FanOutGridDemo(p){render_component("gallery/fan_out_grid_demo",p)}
fn state(ctx){
    let s=ctx.get_state("demo");
    if s!=(){s}else{#{run:0,rev:0,finish:false,narrow:false,empty:false,controlled:false,pin:"",reject:false,retry:false,notice:""}}
}
fn job(s){"index/demo:"+s.run.to_string()}
fn choose(ctx,p){
    let s=state(ctx);
    if p.run!=job(s)||p.revision!=s.rev{return;}
    if s.reject{s.reject=false;s.notice="The Host kept the pinned task";}
    else{
        if s.controlled{s.pin=p.id;}
        s.notice=if p.id==""{"Cleared task at revision "+p.revision.to_string()}else{"Pinned "+p.id+" at revision "+p.revision.to_string()};
    }
    ctx.set_state("demo",s);
}
fn retry(ctx,p){
    let s=state(ctx);
    if p.run!=job(s)||p.revision!=s.rev{return;}
    s.rev+=1;s.retry=true;s.finish=false;
    s.notice="Retry accepted: "+p.ids[0]+", "+p.ids[1];
    ctx.set_state("demo",s);
}
fn control(op,ctx,event){
    let s=state(ctx);
    switch op{
        "finish"=>{s.finish=!s.finish;s.rev+=1;s.notice=if s.finish{"Job settled"}else{"Job running"};},
        "control"=>{s.controlled=!s.controlled;s.pin="";s.reject=false;},
        "reject"=>{s.reject=true;s.notice="The next inspection is declined";},
        "narrow"=>{s.narrow=!s.narrow;},
        "empty"=>{s.empty=!s.empty;s.run+=1;s.pin="";},
        _=>{s.run+=1;s.rev=0;s.pin="";s.retry=false;s.finish=false;s.notice="Opened a fresh job";}
    }
    ctx.set_state("demo",s);
}
fn btn(op,label){button::Button(#{key:op,text:label,variant:"ghost",size:"xs",on_click:Fn("control").curry(op)})}
fn tasks(s){
    let out=[];
    for i in 0..144{
        let err=i==2||i==5;let id=if i==2{"docs/索引"}else if i==5{"notes:2"}else{"task:"+i.to_string()};
        let k=if err{
            if s.retry{if s.finish{"done"}else{"retrying"}}else{"failed"}
        }else if s.finish||i%12<5{"done"}else if i%12<8{"running"}else{"queued"};
        out.push(#{id:id,label:"Task "+i.to_string(),status:k,worker:if k=="queued"{""}else{"Worker "+(i%8+1).to_string()},
            start_ms:if k=="queued"{()}else{i*20},end_ms:if k=="done"||k=="failed"{3600+i*55}else{()},
            cost_micros:if k=="queued"{()}else{1000+100*i},attempts:if err&&s.retry{2}else{1},
            result:if k=="done"{"Index updated"}else{""},error:if err{"Source manifest timed out"}else{""}});
    }
    out
}
fn render_demo(ctx,p){
    let s=state(ctx);
    let props=#{key:"batch",run:job(s),revision:s.rev,job:"Index 144 sources",now_ms:13700,workers:8,
        tasks:if s.empty{[]}else{tasks(s)},on_select:Fn("choose"),on_retry:Fn("retry")};
    if s.controlled{props.pinned=s.pin;}
    let buttons=[btn("finish",if s.finish{"Running job"}else{"Settle job"}),
        btn("control",if s.controlled{"Local inspection"}else{"Host-controlled inspection"}),btn("reject","Refuse next pin"),
        btn("narrow",if s.narrow{"Wide field"}else{"Narrow field"}),btn("empty",if s.empty{"Restore tasks"}else{"Empty field"}),btn("reset","New job")];
    column([row(buttons).with_style(style().min_width(px(0)).gap(px(6)).flex_wrap()),
        column([grid::FanOutGrid(props)]).with_key("size")
            .with_style(style().width(if s.narrow{px(350)}else{relative(1.0)}).max_width(if s.narrow{relative(1.0)}else{px(760)})),
        text(s.notice).accessibility_role("status").accessibility_label(s.notice)
            .with_style(style().typography("caption").text_color(theme_color("text_muted")))])
        .with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(14))))
}
