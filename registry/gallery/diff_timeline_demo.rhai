import "patterns/diff_timeline" as timeline;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/diff_timeline_demo","export":"DiffTimelineDemo","version":"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/diff_timeline","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn DiffTimelineDemo(p){render_component("gallery/diff_timeline_demo",p)}
fn state(ctx){
    let old=ctx.get_state("demo");
    if old==(){#{run:0,time:58.0,live:true,narrow:false,empty:false}}else{old}
}
fn details(){
    // Supplied out of order on purpose; equal timestamps preserve input order.
    [#{id:"api",at:35,file:"src/api.rs",added:85,removed:12,diff:[#{kind:"add",text:"pub struct RequestWindow {"},#{kind:"ctx",text:"    offset: u32,"}]},
     #{id:"discard",at:6,file:"src/lib.rs",added:0,removed:38,diff:[#{kind:"del",text:"pub mod obsolete_adapter;"}]},
     #{id:"config",at:14,file:"config/default.toml",added:4,removed:9,diff:[#{kind:"del",text:"buffer_size = 256"},#{kind:"add",text:"window_size = 512"}]},
     #{id:"api-route",at:35,file:"src/api.rs",added:8,removed:0,diff:[#{kind:"add",text:"route.bind(window());"}]},
     #{id:"preview",at:52,file:"assets/preview.png",added:0,removed:0},
     #{id:"docs",at:71,file:"docs/Guide.md",added:41,removed:4,diff:[#{kind:"add",text:"Open the request window."}]},
     #{id:"rhai",at:84,file:"examples/账户.rhai",added:22,removed:3},
     #{id:"bench",at:92,file:"notes/timings.csv",added:7,removed:1},
     #{id:"tests",at:104,file:"tests/window_tests.rs",added:23,removed:0},
     #{id:"cleanup",at:112,file:"src/lib.rs",added:12,removed:5,diff:[#{kind:"add",text:"pub use api::RequestWindow;"}]}]
}
fn control(key,ctx,event){
    let s=state(ctx);
    switch key{
        "early"=>{s.time=16.0;s.live=true;s.empty=false;},
        "advance"=>{s.time=min(120.0,s.time+24.0);s.live=s.time<120.0;s.empty=false;},
        "all"=>{s.time=120.0;s.live=false;s.empty=false;},
        "narrow"=>{s.narrow=!s.narrow;},
        "empty"=>{s.empty=!s.empty;s.run+=1;},
        _=>{s.run+=1;s.time=58.0;s.live=true;s.empty=false;}
    }
    ctx.set_state("demo",s);
}
fn btn(key,words){button::Button(#{key:key,text:words,size:"xs",variant:"ghost",on_click:Fn("control").curry(key)})}
fn render_demo(ctx,p){
    let s=state(ctx);
    let card=timeline::DiffTimeline(#{key:"diff",run:"patch:"+s.run.to_string(),agent:#{id:"fixture/ada",name:"Patch agent"},
        context:"PR #214",edits:if s.empty{[]}else{details()},duration:120,elapsed:s.time,live:s.live,max_rows:4});
    column([row([btn("early","Early edits"),btn("advance","Advance playback"),btn("all","Show full run"),
        btn("narrow",if s.narrow{"Wide timeline"}else{"Narrow timeline"}),btn("empty",if s.empty{"Restore edits"}else{"Empty ledger"}),
        btn("reset","New run")]).with_style(style().min_width(px(0)).flex_wrap().gap(px(6))),
        column([card]).with_key("sized").with_style(style().width(if s.narrow{px(350)}else{relative(1.0)})
            .max_width(if s.narrow{relative(1.0)}else{px(700)}))])
        .with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(10))))
}
