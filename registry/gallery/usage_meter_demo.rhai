import "patterns/usage_meter" as usage;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/usage_meter_demo","export":"UsageMeterDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/usage_meter","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn UsageMeterDemo(props){render_component("gallery/usage_meter_demo",props)}
fn state(ctx){
    let m=ctx.get_state("model");
    if m==(){#{period:"month",revision:0,pending:(),refuse:false,narrow:false,notice:""}}else{m}
}
fn sample(period){
    switch period{
        "week"=>#{total:1000,used:[#{id:"compute",label:"Compute",value:280},#{id:"search",label:"Search",value:150},#{id:"storage",label:"Indexing",value:70}]},
        "quarter"=>#{total:1000,used:[#{id:"compute",label:"Compute",value:640},#{id:"storage",label:"Indexing",value:240},
            #{id:"media",label:"Media",value:380},#{id:"search",label:"Search",value:80}]},
        _=>#{total:1000,used:[#{id:"compute",label:"Compute",value:460},#{id:"search",label:"Search",value:196},
            #{id:"storage",label:"Indexing",value:144},#{id:"media",label:"Media",value:68}]},
    }
}
fn days(period){
    let points=if period=="week"{[38,59,0,110,76,(),()]}
        else if period=="quarter"{[68,140,84,109,210,122,300,169,119,104,265,194,145,(),()]}
        else{[38,59,0,110,76,122,63,118,91,68,80,109,60,77,130,208,157,121,87,198,64,(),(),(),(),(),(),(),(),()]};
    let out=[];for i in 0..points.len{out.push(#{id:"day-"+i.to_string(),label:"Day "+(i+1).to_string(),value:points[i]});}out
}
fn request(ctx,p){
    let m=state(ctx);
    if m.pending!=()||p.series!="example-usage"||p.revision!=m.revision||p.from!=m.period||!(p.to in ["week","month","quarter"]){return;}
    m.pending=#{request:p,refuse:m.refuse};m.refuse=false;m.notice="Waiting for usage snapshot…";
    ctx.set_state("model",m);
}
fn settle(ctx,p){
    let m=state(ctx);
    if m.pending!=p||m.revision!=p.request.revision{return;}
    m.pending=();
    if p.refuse{m.notice="The Host kept the current usage snapshot.";}
    else{m.period=p.request.to;m.revision+=1;m.notice="Accepted usage: "+m.period;}
    ctx.set_state("model",m);
}
fn action(ctx,p){
    let m=state(ctx);
    if m.pending!=()||p.series!="example-usage"||p.revision!=m.revision||p.period!=m.period{return;}
    m.notice=if p.kind=="upgrade"{"Upgrade requested"}else{"Top-up requested"};
    ctx.set_state("model",m);
}
fn refuse(ctx,e){let m=state(ctx);m.refuse=true;m.notice="The next period will be refused.";ctx.set_state("model",m);}
fn narrow(ctx,e){let m=state(ctx);m.narrow=!m.narrow;ctx.set_state("model",m);}
fn demo(ctx,p){
    let m=state(ctx);
    if m.pending!=(){timeout("settle",300,false,Fn("settle"),m.pending);}
    let data=sample(m.period);
    let card=usage::UsageMeter(#{key:"usage-card",series:"example-usage",revision:m.revision,title:"Workspace usage",plan:"Pro plan",
        total:data.total,used:data.used,period:m.period,on_period_change:Fn("request"),on_action:Fn("action"),busy:m.pending!=(),
        periods:[#{id:"week",label:"Week"},#{id:"month",label:"Month"},#{id:"quarter",label:"Quarter"}],
        now_ms:1762128000000,reset_at_ms:1762905600000,daily:days(m.period),
        limits:[#{id:"seats",label:"Seats",used:if m.period=="week"{3}else{8},total:10},
            #{id:"storage",label:"Storage",used:if m.period=="quarter"{12.5}else{5.4},total:10,unit:"GB"},
            #{id:"projects",label:"Projects",used:12}]});
    column([
        column([card]).with_key("width").with_style(style().width(px(if m.narrow{278}else{460})).max_width(relative(1.0)).min_width(px(0))),
        row([button::Button(#{key:"narrow",text:if m.narrow{"Wide usage card"}else{"Narrow usage card"},size:"sm",variant:"ghost",on_click:Fn("narrow")}),
            button::Button(#{key:"refuse",text:"Refuse next snapshot",size:"sm",variant:"ghost",on_click:Fn("refuse")})])
            .with_key("controls").with_style(style().min_width(px(0)).flex_wrap()),
        text(m.notice).with_key("receipt").accessibility_role("status").with_style(style().typography("caption").text_color(theme_color("text_muted"))),
    ]).with_key(p.key).with_style(ctx.component_style("root",style().gap(px(12)).min_width(px(0))))
}
