import "patterns/memory_panel" as memory;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/memory_panel_demo","export":"MemoryPanelDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/memory_panel","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{demo:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
        slots:#{},parts:["root"],events:#{}},render:Fn("render_demo")
});
fn MemoryPanelDemo(p){render_component("gallery/memory_panel_demo",p)}
fn seed(){[
    #{id:"rust",text:"Builds Rust tools",category:"Work",source:"From a chat",date:"Mar 7"},
    #{id:"tea",text:"Prefers tea",category:"Preferences",source:"Added by you",date:"Apr 4",paused:true},
    #{id:"orbit",text:"Shipping Orbit",category:"Projects",source:"From a chat",date:"Apr 11"}
]}
fn model(ctx){
    let s=ctx.get_state("demo");
    if s!=(){s}else{#{workspace:0,revision:0,memories:seed(),narrow:false,denied:false,enabled:true,notice:""}}
}
fn workspace(s){"workspace:"+s.workspace.to_string()}
fn change(ctx,e){
    let s=model(ctx);if e.workspace!=workspace(s)||e.revision!=s.revision{return;}
    if s.denied{s.denied=false;s.notice="The Host refused the change";}
    else{s.memories=e.memories;s.notice="The Host accepted: "+e.kind;}
    s.revision+=1;ctx.set_state("demo",s);
}
fn enabled(ctx,e){
    let s=model(ctx);if e.workspace!=workspace(s)||e.revision!=s.revision{return;}
    s.enabled=e.enabled;s.notice=if s.enabled{"Memory enabled by Host"}else{"Memory disabled by Host"};ctx.set_state("demo",s);
}
fn action(ctx,e){let s=model(ctx);s.notice="Memory management requested";ctx.set_state("demo",s);}
fn tool(op,ctx,e){
    let s=model(ctx);switch op{
        "narrow"=>{s.narrow=!s.narrow;},
        "deny"=>{s.denied=true;s.notice="The next change will be refused";},
        _=>{s.workspace+=1;s.revision=0;s.memories=seed();s.denied=false;s.enabled=true;s.notice="Workspace reset";},
    }
    ctx.set_state("demo",s);
}
fn btn(key,name){button::Button(#{key:key,text:name,variant:"ghost",size:"xs",on_click:Fn("tool").curry(key)})}
fn render_demo(ctx,p){
    let s=model(ctx);
    let controls=row([btn("deny","Refuse next change"),btn("narrow",if s.narrow{"Wide memory"}else{"Narrow memory"}),
        btn("reset","Reset workspace")]).with_style(style().flex_wrap().gap(theme_length("space.spacing_unit")));
    let panel=memory::MemoryPanel(#{key:"case",workspace:workspace(s),revision:s.revision,enabled:s.enabled,
        memories:s.memories,categories:["Work","Preferences","Projects"],on_change:Fn("change"),on_enabled_change:Fn("enabled"),
        action_label:"Manage",on_action:Fn("action")});
    let sized=column([panel]).with_key("sized").with_style(style().width(if s.narrow{px(300)}else{relative(1.0)}).max_width(relative(1.0)).min_width(px(0)));
    let status=text(s.notice).accessibility_role("status").accessibility_label(s.notice)
        .with_style(style().typography("caption").text_color(theme_color("text_muted")));
    column([controls,sized,status]).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(3))))
}
