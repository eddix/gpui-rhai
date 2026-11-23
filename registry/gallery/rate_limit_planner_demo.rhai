import "patterns/rate_limit_planner" as plan;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/rate_limit_planner_demo","export":"RateLimitPlannerDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/rate_limit_planner","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn RateLimitPlannerDemo(p){render_component("gallery/rate_limit_planner_demo",p)}
fn seed(){[
    #{id:"index",name:"Index notes",at:405,peak:18000,rise:24,half_life:44},
    #{id:"review",name:"Review guides",at:510,peak:27000,rise:30,half_life:62,days:"weekdays"},
    #{id:"digest",name:"Draft digest",at:525,peak:22500,rise:36,half_life:68,days:"weekdays"},
    #{id:"library",name:"Update library",at:870,peak:18000,rise:34,half_life:70},
    #{id:"archive",name:"Archive history",at:670,peak:13000,rise:30,half_life:50,days:"weekend"}
]}
fn state(ctx){
    let s=ctx.get_state("model");
    if s!=(){s}else{#{run:0,revision:0,runs:seed(),day:"weekdays",now:435,measured:true,selected:"review",narrow:false,deny:false,pending:(),notice:""}}
}
fn id(s){"week-a:"+s.run.to_string()}
fn selected(ctx,value){
    let s=state(ctx);
    if value!=s.selected{s.selected=value;ctx.set_state("model",s);}
}
fn request(ctx,event){
    let s=state(ctx);
    if event.session!=id(s)||event.revision!=s.revision||s.pending!=(){return;}
    if s.deny{s.deny=false;s.notice="Schedule proposal refused";s.revision+=1;}
    else{s.runs=event.runs;s.revision+=1;s.notice="Schedule accepted: "+event.operation;}
    ctx.set_state("model",s);
}
fn action(op,ctx,e){
    let s=state(ctx);
    switch op{
        "narrow"=>{s.narrow=!s.narrow;},
        "day"=>{s.day=if s.day=="weekdays"{"weekend"}else{"weekdays"};s.revision+=1;s.notice="Showing "+s.day;},
        "measured"=>{s.measured=!s.measured;s.revision+=1;},
        "clock"=>{s.now=if s.now==435{1425}else{435};s.revision+=1;s.notice=if s.now==1425{"Planner clock advanced"}else{"Planner clock reset"};},
        "deny"=>{s.deny=true;s.notice="The next schedule proposal will be refused";},
        _=>{s.run+=1;s.revision=0;s.runs=seed();s.selected="review";s.day="weekdays";s.now=435;s.notice="Planner reset";s.deny=false;},
    }
    ctx.set_state("model",s);
}
fn b(op,name){button::Button(#{key:op,text:name,size:"xs",variant:"ghost",on_click:Fn("action").curry(op)})}
fn measured(s){
    let values=[];
    if s.measured{
        for i in 0..(s.now-360){
            let v=if i<43{700}else{min(16000,(i-43)*(i-43)*22)};
            values.push(if i%16==0{()}else{v});
        }
    }
    values
}
fn render_demo(ctx,p){
    let s=state(ctx);
    let tools=row([b("narrow",if s.narrow{"Wide planner"}else{"Narrow planner"}),b("day",if s.day=="weekdays"{"Show weekend"}else{"Show weekdays"}),
        b("measured",if s.measured{"Hide measured load"}else{"Show measured load"}),b("clock",if s.now==435{"Show completed runs"}else{"Show running window"}),
        b("deny","Refuse next schedule proposal"),b("reset","Reset planner")])
        .with_style(style().min_width(px(0)).flex_wrap().gap(theme_length("space.spacing_unit")));
    let board=plan::RateLimitPlanner(#{key:"case",session:id(s),revision:s.revision,runs:s.runs,now:s.now,day:s.day,limit:40000,
        actual:measured(s),checkpoint:#{at:1380,label:"Nightly batch"},selected:s.selected,on_runs_change:Fn("request"),on_selected_change:Fn("selected")});
    let width=column([board]).with_key("sized").with_style(style().min_width(px(0)).max_width(if s.narrow{px(446)}else{px(940)}).width(relative(1.0)));
    let caption=text(s.notice).accessibility_role("status").accessibility_label(s.notice)
        .with_style(style().typography("caption").text_color(theme_color("text_muted")));
    column([tools,width,caption]).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(3))))
}
