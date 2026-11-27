import "patterns/plan_vs_actual" as plan;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/plan_vs_actual_demo","export":"PlanVsActualDemo","version":"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/plan_vs_actual","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn PlanVsActualDemo(p){render_component("gallery/plan_vs_actual_demo",p)}
fn state(ctx){
    let old=ctx.get_state("demo");
    if old!=(){old}else{#{turn:0,revision:0,now:78000,finished:false,narrow:false,empty:false,controlled:false,pin:"",reject:false,notice:""}}
}
fn run(s){"rollout:"+s.turn.to_string()}
fn inspected(ctx,p){
    let s=state(ctx);if p.run!=run(s)||p.revision!=s.revision{return;}
    if s.reject{s.reject=false;s.notice="The Host kept the previous inspection";}
    else{
        if s.controlled{s.pin=p.id;}
        s.notice=if p.id==""{"Inspection cleared"}else{"Pin accepted: "+p.id};
    }
    ctx.set_state("demo",s);
}
fn control(op,ctx,e){
    let s=state(ctx);
    switch op{
        "clock"=>{s.now+=75000;s.revision+=1;},
        "finish"=>{s.finished=!s.finished;s.revision+=1;s.now=if s.finished{max(109000,s.now)}else{78000};},
        "control"=>{s.controlled=!s.controlled;s.pin="";s.reject=false;},
        "reject"=>{s.reject=true;s.notice="The next pin will be declined";},
        "narrow"=>{s.narrow=!s.narrow;},
        "empty"=>{s.empty=!s.empty;s.turn+=1;s.pin="";},
        _=>{s.turn+=1;s.revision=0;s.pin="";s.now=78000;s.finished=false;s.notice="New rollout";}
    }
    ctx.set_state("demo",s);
}
fn btn(op,title){button::Button(#{key:op,text:title,variant:"ghost",size:"xs",on_click:Fn("control").curry(op)})}
fn tasks(s){
    [
      #{id:"brief",title:"Read the brief",planned_ms:20000,start_ms:0,end_ms:19000,spent_micros:21000,planned_micros:30000},
      #{id:"ui/构建",title:"Build the layout",planned_ms:40000,depends_on:["brief"],start_ms:19000,
        end_ms:if s.finished{58000}else{()},spent_micros:if s.finished{185000}else{256000},planned_micros:220000},
      #{id:"assets",title:"Prepare assets",planned_ms:20000,depends_on:["brief"],start_ms:19000,end_ms:43000,spent_micros:-12000,planned_micros:45000},
      #{id:"review",title:"Review draft",planned_ms:30000,depends_on:["ui/构建","assets"],start_ms:if s.finished{58000}else{()},
        end_ms:if s.finished{83000}else{()},spent_micros:if s.finished{61000}else{()},planned_micros:80000},
      #{id:"publish",title:"Handoff",planned_ms:30000,depends_on:["review"],start_ms:if s.finished{83000}else{()},
        end_ms:if s.finished{109000}else{()},spent_micros:if s.finished{16000}else{()},planned_micros:50000}
    ]
}
fn render_demo(ctx,p){
    let s=state(ctx);
    let cfg=#{key:"schedule",run:run(s),revision:s.revision,now_ms:s.now,task:"Prepare the pricing page",
        steps:if s.empty{[]}else{tasks(s)},on_inspect:Fn("inspected")};
    if s.controlled{cfg.pinned=s.pin;}
    let controls=[btn("clock","Advance clock"),btn("finish",if s.finished{"Running plan"}else{"Finish earlier"}),
        btn("control",if s.controlled{"Local inspection"}else{"Host-controlled inspection"}),btn("reject","Refuse next pin"),
        btn("narrow",if s.narrow{"Wide timeline"}else{"Narrow timeline"}),btn("empty",if s.empty{"Restore steps"}else{"Empty plan"}),btn("reset","New rollout")];
    column([row(controls).with_style(style().min_width(px(0)).gap(px(6)).flex_wrap()),
        column([plan::PlanVsActual(cfg)]).with_key("sizing").with_style(style().width(if s.narrow{px(390)}else{relative(1.0)}).max_width(if s.narrow{relative(1.0)}else{px(850)})),
        text(s.notice).accessibility_role("status").accessibility_label(s.notice).with_style(style().typography("caption").text_color(theme_color("text_muted")))])
        .with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(14))))
}
