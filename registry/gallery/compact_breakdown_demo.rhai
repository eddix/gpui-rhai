// Change the period using the original, unchanged snapshot while the Host decides.
import "patterns/compact_breakdown" as breakdown;
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/compact_breakdown_demo","export":"CompactBreakdownDemo","version":"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/compact_breakdown","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{model:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn CompactBreakdownDemo(props){render_component("gallery/compact_breakdown_demo",props)}
fn sample(ctx){
    let m=ctx.get_state("model");
    if m!=(){return m;}
    #{period:"month",revision:0,pending:(),refuse:false,narrow:false,notice:""}
}
fn categories(period){
    if period=="week"{return [
        #{id:"inference",label:"Inference",value:642.5},#{id:"storage",label:"Storage",value:720.0},
        #{id:"compute",label:"Background compute",value:212.5},#{id:"transfer",label:"Transfer",value:4.0},
    ];}
    if period=="year"{return [
        #{id:"storage",label:"Storage",value:48764.5},#{id:"inference",label:"Inference",value:99720.25},
        #{id:"compute",label:"Background compute",value:19970.25},#{id:"support",label:"Support",value:18000.0},
    ];}
    [#{id:"inference",label:"Inference",value:8420.0},#{id:"compute",label:"Background compute",value:3140.0},
        #{id:"storage",label:"Storage",value:1740.0},#{id:"transfer",label:"Transfer",value:620.0},
        #{id:"archive",label:"Archive",value:40.0}]
}
fn request(ctx,p){
    let m=sample(ctx);
    if m.pending!=()||p.series!="costs"||p.revision!=m.revision||p.from!=m.period||!(p.to in ["week","month","year"]){return;}
    m.pending=#{request:p,refuse:m.refuse};m.refuse=false;m.notice="Waiting for the Host…";
    ctx.set_state("model",m);
}
fn settle(ctx,p){
    let m=sample(ctx);
    if m.pending!=p||m.revision!=p.request.revision{return;}
    m.pending=();
    if p.refuse{m.notice="The Host refused the period change.";}
    else{m.period=p.request.to;m.revision+=1;m.notice="Accepted period: "+m.period;}
    ctx.set_state("model",m);
}
fn refuse(ctx,e){let m=sample(ctx);m.refuse=true;m.notice="The next proposal will be refused.";ctx.set_state("model",m);}
fn narrow(ctx,e){let m=sample(ctx);m.narrow=!m.narrow;ctx.set_state("model",m);}
fn demo(ctx,props){
    let m=sample(ctx);
    if m.pending!=(){timeout("settle",300,false,Fn("settle"),m.pending);}
    let data=categories(m.period);
    let card=breakdown::CompactBreakdown(#{key:"cost-card",series:"costs",revision:m.revision,title:"Infrastructure cost",
        total_label:"Total spending",prefix:"$",decimals:1,items:data,period:m.period,
        periods:[#{id:"week",label:"Week"},#{id:"month",label:"Month"},#{id:"year",label:"Year"}],
        delta:if m.period=="week"{-6.8}else if m.period=="year"{9.1}else{12.4},busy:m.pending!=(),on_period_change:Fn("request")});
    column([
        column([card]).with_key("card-width").with_style(style().width(px(if m.narrow{238}else{360})).max_width(relative(1.0)).min_width(px(0))),
        row([button::Button(#{key:"narrow",text:if m.narrow{"Wide card"}else{"Narrow card"},size:"sm",variant:"ghost",on_click:Fn("narrow")}),
            button::Button(#{key:"refuse",text:"Refuse next period",size:"sm",variant:"ghost",on_click:Fn("refuse")})])
            .with_key("controls").with_style(style().min_width(px(0)).flex_wrap()),
        text(m.notice).with_key("receipt").accessibility_role("status").with_style(style().typography("caption").text_color(theme_color("text_muted"))),
    ]).with_key(props.key).with_style(ctx.component_style("root",style().gap(px(12)).min_width(px(0))))
}
