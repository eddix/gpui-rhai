import "patterns/parallel_attempts" as attempts;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/parallel_attempts_demo","export":"ParallelAttemptsDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/parallel_attempts","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn ParallelAttemptsDemo(p){render_component("gallery/parallel_attempts_demo",p)}
fn state(ctx){
    let s=ctx.get_state("demo");
    if s!=(){s}else{#{run:0,revision:0,done:false,progress:0.24,narrow:false,empty:false,controlled:false,chosen:"",refuse:false,notice:""}}
}
fn run(s){"test-race/"+s.run.to_string()}
fn select(ctx,event){
    let s=state(ctx);
    if event.run!=run(s)||event.revision!=s.revision{return;}
    if s.refuse{s.refuse=false;s.notice="The Host kept its selection";}
    else{
        if s.controlled{s.chosen=event.id;}
        s.notice="Selected "+event.id+" at revision "+event.revision.to_string();
    }
    ctx.set_state("demo",s);
}
fn toggle(op,ctx,event){
    let s=state(ctx);
    switch op{
        "grow"=>{s.progress=min(0.96,s.progress+0.24);s.revision+=1;},
        "finish"=>{s.done=!s.done;s.revision+=1;s.notice=if s.done{"Results committed"}else{"Back to live trials"};},
        "controlled"=>{s.controlled=!s.controlled;s.chosen="";s.refuse=false;},
        "reject"=>{s.refuse=true;s.notice="The next proposal will be refused";},
        "empty"=>{s.empty=!s.empty;s.run+=1;s.revision=0;},
        "narrow"=>{s.narrow=!s.narrow;},
        _=>{s.run+=1;s.revision=0;s.chosen="";s.notice="New attempt set";}
    }
    ctx.set_state("demo",s);
}
fn btn(key,name){
    button::Button(#{key:key,text:name,variant:"ghost",size:"xs",on_click:Fn("toggle").curry(key)})
}
fn model_mark(path){
    svg("<svg viewBox='0 0 16 16' stroke='currentColor' fill='none' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><path d='"+path+"'/></svg>")
        .with_style(style().width(px(15)).height(px(15)))
}
fn sample(s){
    [
        #{id:"api/界",name:"Attempt A",approach:"Mock the payment API",model:"Atlas Mini",icon:model_mark("M2.5 13l5.5-10 5.5 10M5 9h6"),
            status:if s.done{"done"}else{"running"},progress:s.progress,step:"Running tests: 12 of 20",
            note:"All 20 runs green",score:92,cost_micros:129000,duration_ms:4700},
        #{id:"trace",name:"Attempt B",approach:"Trace the fixture race",model:"Vista 4",icon:model_mark("M2.5 4.5L8 12l5.5-7.5"),
            status:if s.done{"done"}else{"running"},progress:min(0.94,s.progress+0.29),
            step:"Checking browser timings",note:"19 of 20 runs green",score:87,cost_micros:182000,duration_ms:13830},
        #{id:"wait",name:"Attempt C",approach:"Add a fixed wait",model:"Nova",icon:model_mark("M3.5 12V4l9 8V4"),
            status:"failed",progress:0.37,note:"Retry did not resolve the race",score:99,cost_micros:7000,duration_ms:6250}
    ]
}
fn render_demo(ctx,p){
    let s=state(ctx);
    let props=#{key:"race",run:run(s),revision:s.revision,task:"Fix the flaky checkout test",
        attempts:if s.empty{[]}else{sample(s)},on_pick:Fn("select")};
    if s.controlled{props.picked=s.chosen;}
    let controls=[btn("grow","Advance progress"),btn("finish",if s.done{"Show live trials"}else{"Finish trials"}),
        btn("controlled",if s.controlled{"Local pick"}else{"Host-controlled pick"}),btn("reject","Refuse next pick"),
        btn("narrow",if s.narrow{"Wide card"}else{"Narrow card"}),btn("empty",if s.empty{"Restore attempts"}else{"Empty attempts"}),btn("reset","New run")];
    column([row(controls).with_style(style().min_width(px(0)).gap(px(6)).flex_wrap()),
        column([attempts::ParallelAttempts(props)]).with_key("sized")
            .with_style(style().width(if s.narrow{px(370)}else{relative(1.0)}).max_width(if s.narrow{relative(1.0)}else{px(780)})),
        text(s.notice).accessibility_role("status").accessibility_label(s.notice)
            .with_style(style().typography("caption").text_color(theme_color("text_muted")))])
        .with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(14))))
}
