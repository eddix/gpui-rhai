/* gpui-rhai
{"id":"patterns/kpi_chart","export":"KpiChart","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["patterns/agent_tree"],"capabilities":{},"assets":[],"tokens":["border","compact_breakdown.c4","compact_breakdown.c5","focus_ring","kpi_chart.first","kpi_chart.second","kpi_chart.third","kpi_chart.grid","kpi_chart.guide","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_raised","text.danger","text.success","text_muted","text_primary","typography.body","typography.caption","typography.display"],"environment":[]}
*/
// Multi-series KPI card with a shared sample focus and honest gaps.
// Example: KpiChart(#{key:"latency",metric:"service/latency",revision:0,title:"Latency",value:"42 ms",samples:[#{id:"now",label:"Now"}],series:[#{id:"p50",label:"Median",values:[42]}]})
import "patterns/agent_tree" as ids;

define_component(#{
    metadata:#{id:"patterns/kpi_chart","export":"KpiChart",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/agent_tree"],capabilities:#{},assets:[],tokens:["border","compact_breakdown.c4","compact_breakdown.c5","focus_ring","kpi_chart.first","kpi_chart.second","kpi_chart.third","kpi_chart.grid","kpi_chart.guide","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_raised","text.danger","text.success","text_muted","text_primary","typography.body","typography.caption","typography.display"],environment:[]},
    schema:#{
        props:#{
            key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable visual instance key."},
            metric:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of the measured metric. Change it when replacing the dataset rather than updating it."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted data revision. A new revision draws the new series using the current Motion policy."},
            title:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Metric title, also used for the accessible plot name."},
            value:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Already formatted headline value."},
            delta:#{schema:#{type:"optional",value:#{type:"number",min:-1000000000000,max:1000000000000}},required:false,sensitive:false,doc:"Signed change against a prior period. Zero shows no delta."},
            format:#{schema:#{type:"string",allowed:["number","percent_points","percent_fraction"]},required:false,sensitive:false,"default":#{type:"string",value:"number"},doc:"Numeric readout representation; percent points use 0–100 and fractions use 0–1."},
            prefix:#{schema:#{type:"string"},required:false,sensitive:false,"default":#{type:"string",value:""},doc:"App-defined prefix for numeric axes, readouts and change."},
            suffix:#{schema:#{type:"string"},required:false,sensitive:false,"default":#{type:"string",value:""},doc:"App-defined suffix for readouts and change. Percent modes provide % by default."},
            footer:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional card footnote."},
            empty_text:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Alternative message when no reading exists."},
            default_sample:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Sample highlighted when neither the pointer nor the keyboard is reading the plot."},
            samples:#{schema:#{type:"array",max_items:96,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable sample ID; label may repeat."},
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Short label for the horizontal axis."},
                detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Long label for the tooltip and accessible readout."}
            }}},required:true,sensitive:false,doc:"Samples in chronological order. Every series is aligned to this order."},
            series:#{schema:#{type:"array",max_items:6,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable series ID."},
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Series label displayed in the sample readout."},
                ink:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional theme color token for the series."},
                values:#{schema:#{type:"array",max_items:96,items:#{type:"optional",value:#{type:"number",min:-1000000000000,max:1000000000000}}},required:true,sensitive:false,doc:"One number or () per sample. Missing readings break the line; zero remains a reading."}
            }}},required:true,sensitive:false,doc:"Bounded series for one KPI card; use Chart with native datasets for larger workloads."},
        },
        state:#{fields:#{
            hover:#{schema:#{type:"optional",value:#{type:"ui_value"}},"default":#{type:"null"}},
            keyboard:#{schema:#{type:"optional",value:#{type:"ui_value"}},"default":#{type:"null"}},
        }},
        events:#{},slots:#{},
        parts:["root","body","body_narrow","title","headline","value","delta","delta_up","delta_down","chart","chart_narrow","plot","plot_narrow","y_axis","y_axis_narrow","x_axis","tick","empty","tooltip","tooltip_title","tooltip_row","tooltip_label","tooltip_value","swatch","footer","footer_narrow"]
    },
    render:Fn("render_kpi")
});
fn KpiChart(props){render_component("patterns/kpi_chart",props)}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn u(n){theme_length("space.unit").times(n)}
fn get(map,key,otherwise){if key in map&&map[key]!=(){map[key]}else{otherwise}}
fn label(ctx,part,value,base){text(value).with_style(ctx.component_style(part,base.min_width(px(0))))}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn decimal(n){n.to_float()}
fn amount(ctx,p,n,axis){
    let value=if p.format=="percent_fraction"{decimal(n)*100.0}else{decimal(n)};
    let ending=if p.format=="number"{if axis{""}else{p.suffix}}else{get(p,"suffix","")};
    if p.format!="number"&&ending==""{ending="%";}
    let a=abs(value);let scale=1.0;let mark="";
    if p.format=="number"{
        if a>=1000000000.0{scale=1000000000.0;mark="B";}
        else if a>=1000000.0{scale=1000000.0;mark="M";}
        else if a>=1000.0{scale=1000.0;mark="k";}
    }
    let f=if scale!=1.0{1}else if a>0.0&&a<1.0{6}else{2};
    p.prefix+ctx.format_number(value/scale,#{max_fraction_digits:f})+mark+ending
}
fn nice_edge(v){
    if v<=0.0{return 2.0;}
    let half=max(v/2.0,0.000000000001);let exponent=log(half,10.0).floor();
    let scale=10.0**exponent;
    for step in [1.0,1.5,2.0,2.5,3.0,4.0,5.0,6.0,8.0,10.0]{
        if step*scale>=half{return 2.0*step*scale;}
    }
    20.0*scale
}
fn position(i,n){if n<=1{0.5}else{i.to_float()/(n-1).to_float()}}
fn physical(i,n,rtl){let x=position(i,n);if rtl{1.0-x}else{x}}
fn model(p){
    if !ids::valid(p.metric){throw "KpiChart requires a stable metric ID";}
    let keys=[];let seen=#{};let lines=#{};let lo=0.0;let hi=0.0;let have=false;
    for s in p.samples{
        if !ids::valid(s.id)||s.id in seen||s.label==""||utf8_byte_count(s.label)>128||utf8_byte_count(get(s,"detail",""))>256{
            throw "KpiChart samples need distinct IDs and bounded labels";
        }
        seen[s.id]=true;keys.push(s.id);
    }
    for s in p.series{
        if !ids::valid(s.id)||s.id in lines||s.label==""||utf8_byte_count(s.label)>128||s.values.len!=keys.len{
            throw "KpiChart series must have distinct IDs and one reading per sample";
        }
        lines[s.id]=true;
        for v in s.values{
            if v!=(){have=true;lo=min(lo,decimal(v));hi=max(hi,decimal(v));}
        }
    }
    let percent=if p.format=="percent_fraction"{1.0}else if p.format=="percent_points"{100.0}else{0.0};
    let top=if hi==0.0&&percent>0.0{percent}else{nice_edge(max(hi,percent/10.0))};
    let floor=if lo<0.0{-nice_edge(-lo)}else{0.0};
    #{keys:keys,low:floor,high:top,available:have}
}
fn color_for(s,i){
    let keys=["kpi_chart.first","kpi_chart.second","kpi_chart.third","compact_breakdown.c4","compact_breakdown.c5","text_muted"];
    theme_color(get(s,"ink",keys[i]))
}
fn y_coord(value,m,height){height*(m.high-decimal(value))/(m.high-m.low)}
fn curve_layer(p,m,rtl,plot){
    let paths=[];let isolated=[];
    for i in 0..p.series.len{
        let s=p.series[i];let ink=color_for(s,i);let points=[];
        for k in 0..s.values.len{
            let v=s.values[k];
            if v==(){continue;}
            let x=plot.width*physical(k,s.values.len,rtl);let y=y_coord(v,m,plot.height);
            let left=k>0&&s.values[k-1]!=();
            let right=k+1<s.values.len&&s.values[k+1]!=();
            if left||right{points.push(if left{path_line(x,y)}else{path_move(x,y)});}
            else{isolated.push(canvas_circle("alone:"+i.to_string()+":"+k.to_string(),x,y,2.25,ink));}
        }
        if points.len>0{
            paths.push(canvas_stroke_path("curve:"+i.to_string(),points,2.0,ink).stroke_cap("round").stroke_join("round"));
        }
    }
    let full=style().absolute().top(px(0)).left(px(0)).width(relative(1.0)).height(relative(1.0));
    let change=p.metric+":"+p.revision.to_string();
    let rendered=[canvas(canvas_scene(paths).view_box(0,0,plot.width,plot.height)).with_key("curves").accessibility_hidden(true)
        .with_style(full).motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:1100,easing:ease(),intent:"decorative"})).motion_on_change(change)];
    if isolated.len>0{
        rendered.push(canvas(canvas_scene(isolated).view_box(0,0,plot.width,plot.height)).with_key("isolated").accessibility_hidden(true)
            .with_style(full).motion(motion_transition("opacity",0.0,1.0,#{duration_ms:220,easing:ease(),intent:"decorative"})).motion_on_change(change));
    }
    rendered
}
fn pointer(metric,id,ctx,inside){
    let next=#{metric:metric,id:id};
    let old=ctx.get_state("hover");
    if inside{if old!=next{ctx.set_state("hover",next);}}
    else if old==next{ctx.set_state("hover",());}
}
fn current(ctx,field,p){
    let v=ctx.get_state(field);
    if v!=()&&v.metric==p.metric{v.id}else{""}
}
fn set_keyboard(metric,id,ctx){
    let value=if id==""{()}else{#{metric:metric,id:id}};
    if ctx.get_state("keyboard")!=value{ctx.set_state("keyboard",value);}
}
fn inspect_sample(metric,id,ctx,event){
    ctx.focus("plot");
    set_keyboard(metric,id,ctx);
    handled()
}
fn move_key(metric,keys,pinned,step,ctx,event){
    let hovered=ctx.get_state("hover");
    let old=ctx.get_state("keyboard");
    let id=if hovered!=()&&hovered.metric==metric&&hovered.id in keys{hovered.id}
        else if old!=()&&old.metric==metric&&old.id in keys{old.id}else{pinned};
    let active=keys.index_of(id);
    let index=if step==-200{0}else if step==200{keys.len-1}
        else if active<0{if step>0{0}else{keys.len-1}}else{max(0,min(keys.len-1,active+step))};
    ctx.set_state("hover",());
    set_keyboard(metric,keys[index],ctx);
    handled()
}
fn dismiss(ctx,event){
    ctx.set_state("hover",());ctx.set_state("keyboard",());handled()
}
fn blur(ctx,inside){if !inside{ctx.set_state("keyboard",());}}
fn rank(a,b){
    if a.value==b.value{return a.order-b.order;}
    if a.value==(){return 1;}if b.value==(){return -1;}
    if a.value>b.value{-1}else{1}
}
fn tooltip(ctx,p,index,x,plot){
    let target=p.samples[index];let values=[];
    for i in 0..p.series.len{
        let s=p.series[i];values.push(#{order:i,value:s.values[index]});
    }
    values.sort(Fn("rank"));
    let title=get(target,"detail",target.label);
    let description="";
    let content=[label(ctx,"tooltip_title",title,style().typography("caption").font_weight(600).text_ellipsis().whitespace_nowrap()).with_key("title")];
    for v in values{
        let s=p.series[v.order];let read=if v.value==(){ctx.t_or("kpi_chart.missing","—")}else{amount(ctx,p,v.value,false)};
        if description!=""{description+="; ";}
        description+=s.label+": "+(if v.value==(){ctx.t_or("kpi_chart.no_reading","No reading")}else{read});
        let chip=box([]).accessibility_hidden(true)
            .with_style(ctx.component_style("swatch",style().width(sp(2)).height(sp(2)).radius(theme_radius("pill")).flex_shrink(false).background(color_for(s,v.order))));
        content.push(row([chip,label(ctx,"tooltip_label",s.label,style().typography("caption").flex_grow().whitespace_nowrap().text_ellipsis()),
            label(ctx,"tooltip_value",read,style().typography("caption").whitespace_nowrap().flex_shrink(false).font_feature("tnum",1))])
            .with_key("series:"+s.id)
            .with_style(ctx.component_style("tooltip_row",style().justify_between().gap(sp(1.5)).items_center().min_width(px(0)))));
    }
    // The preferred side flips across the guide. Clamp when that side cannot
    // fit the readout, using the same measured plot that owns the hit strips.
    let width=min(1.0,176.0/plot.width);
    let gap=min(0.08,14.0/plot.width);
    let preferred=if x<=0.5{x+gap}else{x-gap-width};
    let left=max(0.0,min(1.0-width,preferred));
    let base=style().absolute().top(offset_px(-8)).left(relative(left)).min_width(px(0)).width(relative(width)).max_width(relative(1.0-left)).gap(sp(1))
        .padding(sp(2.5)).radius(theme_radius("lg")).background(theme_color("surface_raised"))
        .border(px(1)).border_color(theme_color("border"));
    column(content).with_key("readout").accessibility_role("status").accessibility_label(title).accessibility_description(description)
        .with_style(ctx.component_style("tooltip",base))
        .layout_motion(220,ease())
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:160,easing:ease(),intent:"decorative"}))
}
fn plot(ctx,p,m,active,narrow){
    let frame=[];let count=m.keys.len;
    let plot_ref=element_ref("plot");
    // The retained Canvas view-box is contain-fit. Drive it with the *real*
    // plot dimensions so all layers and the native pointer regions share a scale.
    let measured=ctx.element_size(plot_ref);
    let dims=if measured==(){#{width:360.0,height:116.0}}else{#{width:max(1.0,measured.width),height:max(1.0,measured.height)}};
    let w=dims.width;let h=dims.height;
    let rtl=ctx.text_direction()=="rtl";
    let stroke=theme_color("kpi_chart.grid");
    let grid=[canvas_line("top",0,0.5,w,0.5,1,stroke),canvas_line("center",0,h/2.0,w,h/2.0,1,stroke),canvas_line("base",0,h-0.5,w,h-0.5,1,stroke)];
    let full=style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0));
    frame.push(canvas(canvas_scene(grid).view_box(0,0,w,h)).with_key("grid").accessibility_hidden(true).with_style(full));
    if m.available{
        for path in curve_layer(p,m,rtl,dims){frame.push(path);}
        if active>=0{
            let ratio=physical(active,count,rtl);let x=ratio*w;
            let marks=[canvas_line("guide",x,0,x,h,1,theme_color("kpi_chart.guide"))];
            for i in 0..p.series.len{
                let s=p.series[i];let v=s.values[active];
                if v!=(){
                    let y=y_coord(v,m,h);
                    marks.push(canvas_circle("rim:"+i.to_string(),x,y,4.25,theme_color("surface_raised")));
                    marks.push(canvas_circle("point:"+i.to_string(),x,y,3.1,color_for(s,i)));
                }
            }
            frame.push(canvas(canvas_scene(marks).view_box(0,0,w,h)).with_key("marker").accessibility_hidden(true).with_style(full));
        }
        if active>=0{frame.push(tooltip(ctx,p,active,physical(active,count,rtl),dims));}
        for i in 0..count{
            let sample=p.samples[i];
            let start=if i==0{0.0}else{(position(i-1,count)+position(i,count))/2.0};
            let end=if i==count-1{1.0}else{(position(i,count)+position(i+1,count))/2.0};
            frame.push(box([]).with_key("hit:"+sample.id).accessibility_hidden(true)
                .on_hover_change(Fn("pointer").curry(p.metric,sample.id)).on_click(Fn("inspect_sample").curry(p.metric,sample.id))
                .with_style(style().absolute().top(px(0)).inset_start(relative(start)).width(relative(end-start))
                    .height(relative(1.0)).cursor_crosshair()));
        }
    }else{
        let words=get(p,"empty_text",ctx.t_or("kpi_chart.empty","No data yet"));
        frame.push(row([label(ctx,"empty",words,style().typography("caption").padding_x(sp(2)).background(theme_color("surface_raised"))
            .text_color(theme_color("text_muted")))]).with_key("empty").with_style(full.items_center().justify_center()));
    }
    let base=style().relative().min_width(px(0)).height(u(29))
        .radius(theme_radius("sm")).focus_ring(px(2),px(2),theme_color("focus_ring"));
    let root=stack(frame).with_key("plot").with_ref(plot_ref).accessibility_role("group")
        .accessibility_label(p.title+" "+ctx.t_or("kpi_chart.plot","chart"))
        .with_style(ctx.component_style(if narrow{["plot","plot_narrow"]}else{["plot"]},base));
    if m.available{
        let pinned=get(p,"default_sample","");
        // The SDK maps physical arrows to logical key bindings before delivery.
        root=root.tab_stop(true).accessibility_description(ctx.t_or("kpi_chart.keys","Use the arrow keys to read each sample."))
            .on_key_value("plain+left",Fn("move_key").curry(p.metric,m.keys,pinned,-1),())
            .on_key_value("plain+right",Fn("move_key").curry(p.metric,m.keys,pinned,1),())
            .on_key_value("plain+home",Fn("move_key").curry(p.metric,m.keys,pinned,-200),())
            .on_key_value("plain+end",Fn("move_key").curry(p.metric,m.keys,pinned,200),())
            .on_key_value("plain+escape",Fn("dismiss"),()).on_focus_within_change(Fn("blur"));
    }
    root
}
fn chart(ctx,p,m,active,narrow){
    let ink=style().typography("caption").text_color(theme_color("text_muted")).font_feature("tnum",1);
    let y=[];
    if m.available{
        let endpoints=[m.high,(m.high+m.low)/2.0,m.low];
        for i in 0..3{
            y.push(row([label(ctx,"tick",amount(ctx,p,endpoints[i],true),ink.whitespace_nowrap())])
                .with_key("tick:"+i.to_string())
                .with_style(style().absolute().top(relative(i.to_float()/2.0)).margin_top(offset_px(-8))
                    .height(sp(4)).width(relative(1.0)).items_center().justify_end()));
        }
    }
    let yaxis=column(y).with_key("y-axis").accessibility_hidden(true)
        .with_style(ctx.component_style(if narrow{["y_axis","y_axis_narrow"]}else{["y_axis"]},style().relative().height(u(29)).width(u(11)).flex_shrink(false)));
    let xlabels=[];
    if m.available{
        let n=m.keys.len;
        if n==1{xlabels.push(label(ctx,"tick",p.samples[0].label,ink.text_center()).with_key("single").with_style(style().width(relative(1.0))));}
        else{
            let mid=n/2;let center=position(mid,n);
            let center_width=min(0.32,0.7*min(center,1.0-center));
            let start_width=if n==2{0.48}else{min(0.32,center-center_width/2.0)};
            let end_width=if n==2{0.48}else{min(0.32,1.0-center-center_width/2.0)};
            let keys=if n==2{[0,n-1]}else{[0,mid,n-1]};
            for i in 0..keys.len{
                let item=p.samples[keys[i]];
                let base=ink.absolute().top(px(0)).whitespace_nowrap().text_ellipsis();
                let spot=if i==0{base.inset_start(px(0)).width(relative(start_width)).text_start()}
                    else if i==keys.len-1{base.inset_end(px(0)).width(relative(end_width)).text_end()}
                    else{base.inset_start(relative(center-center_width/2.0)).width(relative(center_width)).text_center()};
                xlabels.push(label(ctx,"tick",item.label,spot).with_key("x:"+item.id));
            }
        }
    }
    // Labels inherit logical direction, and the same direction is applied in
    // the projected paths, marker, tooltip and pointer sampling above.
    let xaxis=row(xlabels).with_key("x-axis").accessibility_hidden(true)
        .with_style(ctx.component_style("x_axis",style().relative().height(sp(5)).width(relative(1.0)).min_width(px(0))));
    row([yaxis,column([plot(ctx,p,m,active,narrow),xaxis]).with_key("plot-column")
        .with_style(style().flex_basis(px(0)).flex_grow().min_width(px(0)).gap(sp(2)))])
        .with_key("chart").with_style(ctx.component_style(if narrow{["chart","chart_narrow"]}else{["chart"]},style().width(relative(1.0)).min_width(px(0))
            .gap(sp(3)).margin_top(sp(5)).items_start()))
}
fn render_kpi(ctx,p){
    let m=model(p);
    let card_ref=element_ref("card");
    let narrow=ctx.element_width_class(card_ref,[360])==0;
    let active=current(ctx,"hover",p);let key=current(ctx,"keyboard",p);
    if !(active in m.keys){active=key;}
    if !(active in m.keys){active=get(p,"default_sample","");}
    let active=m.keys.index_of(active);
    let values=[label(ctx,"value",p.value,style().typography("display").font_weight(500).font_feature("tnum",1)).with_key("value")];
    let delta=get(p,"delta",0.0);
    if delta!=0.0{
        let up=delta>0.0;
        values.push(label(ctx,["delta",if up{"delta_up"}else{"delta_down"}],
            (if up{"+"}else{"−"})+amount(ctx,p,abs(delta),false),style().typography("caption").flex_shrink(false).whitespace_nowrap()
            .text_color(theme_color(if up{"text.success"}else{"text.danger"}))).with_key("delta"));
    }
    let children=[column([
        label(ctx,"title",p.title,style().typography("body").text_color(theme_color("text_muted")).whitespace_nowrap().text_ellipsis())
            .with_key("title").accessibility_role("heading").accessibility_level(2),
        row(values).with_key("headline").with_style(ctx.component_style("headline",style().items_center().flex_wrap().gap(sp(2)).min_width(px(0)))),
        chart(ctx,p,m,active,narrow),
    ]).with_key("body").with_style(ctx.component_style(if narrow{["body","body_narrow"]}else{["body"]},style().padding(sp(4)).gap(sp(1.5)).min_width(px(0))))];
    let footer=get(p,"footer","");
    if footer!=""{
        children.push(label(ctx,if narrow{["footer","footer_narrow"]}else{["footer"]},footer,style().typography("caption").text_color(theme_color("text_muted"))
            .padding_x(sp(4)).padding_bottom(sp(4))).with_key("footer"));
    }
    column(children).with_key(p.key).with_ref(card_ref).accessibility_role("group").accessibility_label(p.title)
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(460)).min_width(px(0))
            .background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border")).radius(theme_radius("lg"))))
}
