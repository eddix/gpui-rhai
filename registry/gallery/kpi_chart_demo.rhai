import "patterns/kpi_chart" as kpi;
import "components/button" as btn;

define_component(#{
    metadata:#{id:"gallery/kpi_chart_demo","export":"KpiChartDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/kpi_chart","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{empty:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            narrow:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            shifted:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            rate:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0}}}},
        events:#{},slots:#{},parts:["root"]},render:Fn("demo")
});
fn KpiChartDemo(p){render_component("gallery/kpi_chart_demo",p)}
fn toggle(key,ctx,event){
    ctx.set_state(key,!ctx.get_state(key));
    if key!="narrow"{ctx.set_state("revision",ctx.get_state("revision")+1);}
}
fn control(key,name){btn::Button(#{key:key,text:name,size:"sm",variant:"ghost",on_click:Fn("toggle").curry(key)})}
fn demo(ctx,p){
    let empty=ctx.get_state("empty");let narrow=ctx.get_state("narrow");
    let shifted=ctx.get_state("shifted");let rate=ctx.get_state("rate");
    let days=[
        #{id:"mon",label:"Mar 4",detail:"Monday, Mar 4"},#{id:"mar:五",label:"Mar 5",detail:"Tuesday, Mar 5"},
        #{id:"wed",label:"Mar 6",detail:"Wednesday, Mar 6"},#{id:"thu",label:"Mar 7",detail:"Thursday, Mar 7"},
        #{id:"fri",label:"Mar 8",detail:"Friday, Mar 8"},#{id:"sat",label:"Mar 9",detail:"Saturday, Mar 9"},
        #{id:"sun",label:"Mar 10",detail:"Sunday, Mar 10"}
    ];
    let first=if rate{[0.04,0.03,0.06,0.06,0.045,0.08,0.07]}else{[3200,4800,3700,5400,6200,5800,9600]};
    let second=if rate{[0.01,0.02,(),0.028,0.027,0.04,0.05]}else{[1900,2300,(),3200,4200,3600,5400]};
    let third=if rate{[0.03,0.04,0.022,0.032,0.04,0.048,0.05]}else{[620,750,870,(),800,2800,3700]};
    let samples=[];let a=[];let b=[];let c=[];
    for i in (if shifted{2}else{0})..days.len{
        samples.push(days[i]);a.push(if empty{()}else{first[i]});
        b.push(if empty{()}else{second[i]});c.push(if empty{()}else{third[i]});
    }
    let card=kpi::KpiChart(#{key:"metric-card",metric:if rate{"conversion"}else{"revenue"},revision:ctx.get_state("revision"),
        title:if rate{"Conversion"}else{"Revenue"},value:if empty{"—"}else if rate{"7.0%"}else{"$18.7k"},
        format:if rate{"percent_fraction"}else{"number"},prefix:if rate{""}else{"$"},
        delta:if empty{0.0}else if rate{-0.01}else{1780.0},samples:samples,
        series:[#{id:"direct",label:"Direct",values:a},#{id:"partners/合作",label:"Partners",values:b},
            #{id:"marketplace",label:"Marketplace",values:c}],
        footer:"Compared with the previous week",empty_text:"No recorded readings"});
    let shell=if narrow{style().width(px(306)).max_width(relative(1.0))}else{style().width(relative(1.0))};
    column([column([card]).with_style(shell),
        row([control("empty",if empty{"Restore readings"}else{"Clear readings"}),
            control("narrow",if narrow{"Wide card"}else{"Narrow card"}),control("shifted",if shifted{"Restore days"}else{"Drop first two days"}),
            control("rate",if rate{"Show revenue"}else{"Show conversion"})]).with_style(style().flex_wrap().gap(px(6))),
    ]).with_style(ctx.component_style("root",style().gap(px(10)).min_width(px(0))))
}
