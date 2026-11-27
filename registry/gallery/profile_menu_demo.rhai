import "patterns/profile_menu" as account;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/profile_menu_demo","export":"ProfileMenuDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/profile_menu","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn ProfileMenuDemo(p){render_component("gallery/profile_menu_demo",p)}
fn state(ctx){
    let s=ctx.get_state("demo");
    if s==(){#{done:2,refuse:false,custom:false,narrow:false,note:""}}else{s}
}
fn select(ctx,event){
    let s=state(ctx);s.note="Requested "+event.id;ctx.set_state("demo",s);
}
fn next(ctx,event){
    let s=state(ctx);
    if s.done!=event.done||event.total!=5{return;}
    if s.refuse{s.refuse=false;s.note="The Host kept the existing setup";}
    else {s.done+=1;s.note=if s.done==5{"Profile setup complete"}else{"The Host accepted the next step"};}
    ctx.set_state("demo",s);
}
fn control(key,ctx,event){
    let s=state(ctx);
    switch key{
        "refuse"=>{s.refuse=true;s.note="The next step will be refused";},
        "custom"=>{s.custom=!s.custom;},
        "narrow"=>{s.narrow=!s.narrow;},
        _=>{ctx.set_state("demo",());return;}
    }
    ctx.set_state("demo",s);
}
fn control_button(key,label){
    button::Button(#{key:key,text:label,size:"xs",variant:"ghost",on_click:Fn("control").curry(key)})
}
fn extended(){
    let out=[#{id:"settings",label:"Preferences"},#{id:"locked",label:"Workspace admin",disabled:true}];
    for i in 1..13{
        out.push(#{id:"saved/"+i.to_string(),label:"Saved profile "+i.to_string()});
    }
    out.push(#{id:"detail/账户",label:"Account details",badge:"New"});
    out
}
fn render_demo(ctx,p){
    let s=state(ctx);
    let steps=["Verify the backup email","Review privacy options","Choose a profile picture","Add a recovery contact","Set a passphrase"];
    let card=account::ProfileMenu(#{key:"account",name:"Ada Lovelace",email:"ada@example.test",
        items:if s.custom{extended()}else{()},setup:#{done:s.done,total:5,next:if s.done<5{steps[s.done]}else{()}},
        on_select:Fn("select"),on_continue:Fn("next")});
    column([row([control_button("refuse","Refuse next step"),
        control_button("custom",if s.custom{"Default actions"}else{"Long action list"}),
        control_button("narrow",if s.narrow{"Restore width"}else{"Narrow card"}),
        control_button("reset","Reset profile")]).with_style(style().min_width(px(0)).gap(px(6)).flex_wrap()),
        column([card]).with_key("view").with_style(style().width(if s.narrow{px(208)}else{relative(1.0)})
            .max_width(relative(1.0))),
        text(s.note).accessibility_role("status").accessibility_label(s.note)
            .with_style(style().typography("caption").text_color(theme_color("text_muted")))])
        .with_style(ctx.component_style("root",style().min_width(px(0)).gap(px(10))))
}
