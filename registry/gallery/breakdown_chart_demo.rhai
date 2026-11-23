import "patterns/breakdown_chart" as chart;
import "components/button" as btn;
define_component(#{
    metadata:#{id:"gallery/breakdown_chart_demo","export":"BreakdownChartDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/breakdown_chart","components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{narrow:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            dense:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            corrected:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
            empty:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}},
        events:#{},slots:#{},parts:["root"]},
    render:Fn("demo")
});
fn BreakdownChartDemo(p){render_component("gallery/breakdown_chart_demo",p)}
fn flip(key,ctx,event){ctx.set_state(key,!ctx.get_state(key));}
fn button(key,name){btn::Button(#{key:key,text:name,size:"sm",variant:"ghost",on_click:Fn("flip").curry(key)})}
fn demo(ctx,p){
    let narrow=ctx.get_state("narrow");let corrected=ctx.get_state("corrected");let empty=ctx.get_state("empty");
    let dense=ctx.get_state("dense");
    let data=[
        #{area:"api",source:"auto",phase:"done",effort:8},#{area:"api",source:"auto",phase:"done",effort:4},
        #{area:"api",source:"human",phase:"open",effort:6},#{area:"api",source:"queue",phase:"open",effort:-3},
        #{area:"ops",source:"auto",phase:"done",effort:2},#{area:"ops",source:"human",phase:"open",effort:-7},
        #{area:"ops",source:"queue",phase:"open",effort:3},#{area:"web",source:"human",phase:"done",effort:11},
        #{area:"web",source:"auto",phase:"open",effort:7}
    ];
    if corrected{data.push(#{area:"api",source:"auto",phase:"done",effort:-12});}
    if empty{data=[];}
    let sources=[#{id:"auto",label:"Automated"},#{id:"human",label:"Manual"},#{id:"queue",label:"Queued"}];
    if dense{
        sources.push(#{id:"remote",label:"Remote synchronization"});
        sources.push(#{id:"backfill",label:"Historical backfill"});
        sources.push(#{id:"review",label:"Independent verification"});
    }
    let card=chart::BreakdownChart(#{key:"workload",dataset:"delivery",title:"Weekly workload",records:data,
        dimensions:[#{id:"area",label:"Area",levels:[#{id:"api",label:"API"},#{id:"ops",label:"Operations"},
            #{id:"web",label:"Web"},#{id:"support",label:"Support"}]},
            #{id:"source",label:"Source",levels:sources},
            #{id:"phase",label:"Stage",levels:[#{id:"done",label:"Done"},#{id:"open",label:"Open"}]}],
        measures:[#{id:"count",label:"Task count",unit:"tasks"},
            #{id:"effort",label:"Effort",field:"effort",unit:"points"}]});
    column([column([card]).with_style(if narrow{style().width(px(318)).max_width(relative(1.0))}else{style().width(relative(1.0))}),
        row([button("narrow",if narrow{"Wide card"}else{"Narrow card"}),
            button("dense",if dense{"Compact legend"}else{"Dense legend"}),
            button("corrected",if corrected{"Undo correction"}else{"Post a correction"}),
            button("empty",if empty{"Restore workload"}else{"Empty workload"})]).with_style(style().gap(px(6)).flex_wrap())])
        .with_style(ctx.component_style("root",style().gap(px(8)).min_width(px(0))))
}
