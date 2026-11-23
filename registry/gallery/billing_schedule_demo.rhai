import "patterns/billing_schedule" as billing;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/billing_schedule_demo","export":"BillingScheduleDemo","version":"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/billing_schedule","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn BillingScheduleDemo(props){render_component("gallery/billing_schedule_demo",props)}
fn seed(){#{phase:"Build-out phase",start:"2026-01-01",duration:"milestone",lines:[
    #{id:"seed/one",kind:"product",name:"Set-up fee",repeat:"once",timing:"advance",price_minor:499900}]}}
fn state(ctx){
    let s=ctx.get_state("demo");
    if s!=(){s}else{#{run:0,revision:0,draft:seed(),controlled:false,refuse:false,narrow:false,notice:""}}
}
fn id(s){"contract/demo:"+s.run.to_string()}
fn edited(ctx,p){
    let s=state(ctx);
    if p.schedule!=id(s)||p.revision!=s.revision{return;}
    if s.controlled&&s.refuse{s.refuse=false;s.notice="The Host kept the previous quote";}
    else{s.draft=p.draft;s.revision+=1;s.notice="Accepted "+p.action;}
    ctx.set_state("demo",s);
}
fn control(op,ctx,event){
    let s=state(ctx);
    switch op{
        "control"=>{s.controlled=!s.controlled;s.refuse=false;s.run+=1;s.revision=0;},
        "refuse"=>{s.refuse=true;s.notice="The next edit will be declined";},
        "narrow"=>{s.narrow=!s.narrow;},
        _=>{s.run+=1;s.revision=0;s.draft=seed();s.refuse=false;s.notice="Quote reset";}
    }
    ctx.set_state("demo",s);
}
fn btn(op,label){button::Button(#{key:op,text:label,variant:"ghost",size:"xs",on_click:Fn("control").curry(op)})}
fn render_demo(ctx,p){
    let s=state(ctx);
    let catalog=[
        #{id:"platform",name:"Platform licence",repeat:"monthly",price_minor:120000},
        #{id:"support",name:"Premium support",repeat:"monthly",price_minor:35000},
        #{id:"seats",name:"Extra seats (10)",repeat:"monthly",price_minor:24000},
        #{id:"workshop",name:"Onboarding workshop",repeat:"once",price_minor:150000}];
    column([row([btn("control",if s.controlled{"Local quote"}else{"Host-controlled quote"}),btn("refuse","Refuse next edit"),
        btn("narrow",if s.narrow{"Wide schedule"}else{"Narrow schedule"}),btn("reset","Reset quote")])
        .with_style(style().gap(px(6)).min_width(px(0)).flex_wrap()),
        column([billing::BillingSchedule(#{key:"phase",schedule:id(s),revision:s.revision,draft:s.draft,catalog:catalog,
            controlled:s.controlled,on_change:Fn("edited")})]).with_key("surface")
            .with_style(style().width(if s.narrow{px(348)}else{relative(1.0)}).max_width(if s.narrow{relative(1.0)}else{px(560)})),
        text(s.notice).accessibility_role("status").accessibility_label(s.notice).with_style(style().typography("caption").text_color(theme_color("text_muted")))])
        .with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(14))))
}
