/* gpui-rhai
{"id":"patterns/breakdown_chart","export":"BreakdownChart","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/select","patterns/agent_tree","patterns/compact_breakdown"],"capabilities":{},"assets":[],"tokens":["border","breakdown_chart.c1","breakdown_chart.c2","breakdown_chart.c3","breakdown_chart.c4","breakdown_chart.c5","breakdown_chart.c6","breakdown_chart.c7","breakdown_chart.grid","breakdown_chart.muted","breakdown_chart.rest","breakdown_chart.zero","focus_ring","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","typography.body","typography.caption","typography.label"],"environment":[]}
*/
// One signed measure by group and segment, with a linked summary table.
// Example: BreakdownChart(#{key:"work",dataset:"workload",records:[#{team:"api"}],dimensions:[#{id:"team",label:"Team",levels:[#{id:"api",label:"API"}]}],measures:[#{id:"count",label:"Tasks",unit:"tasks"}]})
import "components/select" as select;
import "patterns/agent_tree" as ids;
import "patterns/compact_breakdown" as compact;

define_component(#{
    metadata:#{id:"patterns/breakdown_chart","export":"BreakdownChart",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/select","patterns/agent_tree","patterns/compact_breakdown"],capabilities:#{},assets:[],tokens:["border","breakdown_chart.c1","breakdown_chart.c2","breakdown_chart.c3","breakdown_chart.c4","breakdown_chart.c5","breakdown_chart.c6","breakdown_chart.c7","breakdown_chart.grid","breakdown_chart.muted","breakdown_chart.rest","breakdown_chart.zero","focus_ring","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","typography.body","typography.caption","typography.label"],environment:[]},
    schema:#{
        props:#{
            key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable component key."},
            dataset:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable dataset identity. A different identity starts fresh local choices, visibility and attention."},
            title:#{schema:#{type:"string"},required:false,sensitive:false,"default":#{type:"string",value:"Workload"},doc:"The card title."},
            records:#{schema:#{type:"array",max_items:1024,items:#{type:"ui_value"}},required:true,sensitive:true,doc:"Row maps keyed by dimension or measure field. Missing dimensions do not match a bucket. A missing sum field contributes zero; present measures must be finite numbers."},
            dimensions:#{schema:#{type:"array",max_items:8,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Dimension ID and default source field."},
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Name in the selectors and table."},
                field:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Read a different record field than this dimension's ID."},
                levels:#{schema:#{type:"array",max_items:12,items:#{type:"object",allow_unknown:false,fields:#{
                    id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business value in a record, in display order."},
                    label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Displayed category label; it need not equal the business ID."},
                    ink:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional theme ink token when used as a segment."},
                    glyph:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional short decorative glyph for slice labels and rows."}
                }}},required:true,sensitive:false,doc:"Ordered categories to present, including categories with no observations."}
            }}},required:true,sensitive:false,doc:"Dimensions available to group and segment the records."},
            measures:#{schema:#{type:"array",max_items:8,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable measure ID."},
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Name in the measure selector and table."},
                unit:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Optional unit text after the total; use an empty string for no unit."},
                field:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Numeric field to sum. Omit it to count matching records."}
            }}},required:true,sensitive:false,doc:"Measures selectable by the reader."},
            default_measure:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initial measure ID; defaults to the first."},
            default_slice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initial group dimension; defaults to the first."},
            default_segment:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initial segment dimension; empty string means no segmentation. If omitted, use the next dimension that differs from the slice."}
        },
        state:#{fields:#{
            choice:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
            visibility:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
            attention:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
            opened:#{schema:#{type:"ui_value"},"default":#{type:"null"}}
        }},
        events:#{},slots:#{},
        parts:["root","heading","heading_narrow","title","total","unit","panel","controls","picker","picker_label","chart","plot_frame","plot","gridline","zero","axis","tick","column","bar","segment","track","x_axis","x_active","tooltip","tooltip_title","tooltip_item","tooltip_value","swatch","table","table_header","table_row","row_active","cell","cell_header","cell_total","cell_muted","footer","footer_narrow","legend","legend_narrow","legend_chip","legend_chip_narrow","legend_hidden","legend_hint","legend_reset","empty"]
    },
    render:Fn("render_chart")
});
fn BreakdownChart(props){render_component("patterns/breakdown_chart",props)}
fn get(p,k,d){if k in p&&p[k]!=(){p[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn unit(n){theme_length("space.unit").times(n)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn ink(ctx,parts,value,base){text(value).with_style(ctx.component_style(parts,base.min_width(px(0))))}
fn tx(ctx,key,word){ctx.t_or("breakdown_chart."+key,word)}
fn checked_label(x){x!=""&&utf8_byte_count(x)<=160}
fn options(rows){
    let output=[];
    for item in rows{output.push(#{value:item.id,label:item.label});}
    output
}
fn lookup(records){
    let out=#{};
    for x in records{
        if !ids::valid(x.id)||!checked_label(x.label)||x.id in out{throw "BreakdownChart choices need distinct IDs and bounded labels";}
        out[x.id]=x;
    }
    out
}
fn state_for(ctx,field,p){
    let v=ctx.get_state(field);
    if v!=()&&v.dataset==p.dataset{v}else{#{dataset:p.dataset}}
}
fn choice(ctx,p,dims,measures){
    let old=state_for(ctx,"choice",p);
    let measure=get(old,"measure",get(p,"default_measure",p.measures[0].id));
    if !(measure in measures){measure=p.measures[0].id;}
    let slice=get(old,"slice",get(p,"default_slice",p.dimensions[0].id));
    if !(slice in dims){slice=p.dimensions[0].id;}
    let fallback="";
    for d in p.dimensions{if d.id!=slice{fallback=d.id;break;}}
    let segment=get(old,"segment",get(p,"default_segment",fallback));
    if segment==slice||!(segment in dims){segment="";}
    #{dataset:p.dataset,measure:measure,slice:slice,segment:segment}
}
fn select_value(cfg,field,ctx,value){
    if value==(){return;}
    let old=cfg.segment;
    cfg[field]=value;
    if cfg.slice==cfg.segment{cfg.segment="";}
    if cfg.segment!=old{ctx.set_state("visibility",());}
    ctx.set_state("choice",cfg);
}
fn set_open(dataset,picker,ctx,value){
    let current=ctx.get_state("opened");
    if value{ctx.set_state("opened",#{dataset:dataset,picker:picker});}
    else if current!=()&&current.dataset==dataset&&current.picker==picker{ctx.set_state("opened",());}
}
fn picker(ctx,p,cfg,key,title,choices){
    let active=state_for(ctx,"opened",p);
    column([ink(ctx,"picker_label",title,style().typography("caption").text_color(theme_color("text_muted"))),
        select::Select(#{key:key,label:title,options:choices,value:cfg[key],open:get(active,"picker","")==key,
            query:"",size:"sm",width:relative(1.0),on_change:Fn("select_value").curry(cfg,key),
            on_open_change:Fn("set_open").curry(p.dataset,key)})])
        .with_key(key).with_style(ctx.component_style("picker",style().gap(sp(1)).flex_grow().flex_basis(px(0)).min_width(px(0))))
}
fn controls(ctx,p,cfg){
    let parts=[#{value:"",label:tx(ctx,"none","None")}];
    for d in p.dimensions{if d.id!=cfg.slice{parts.push(#{value:d.id,label:d.label});}}
    row([picker(ctx,p,cfg,"measure",tx(ctx,"measure","Measure"),options(p.measures)),
        picker(ctx,p,cfg,"slice",tx(ctx,"slice","Slice"),options(p.dimensions)),
        picker(ctx,p,cfg,"segment",tx(ctx,"segment","Segment"),parts)])
        .with_key("controls").with_style(ctx.component_style("controls",style().min_width(px(0)).gap(sp(2)).padding_x(sp(4)).padding_top(sp(3))))
}
fn visibility(ctx,cfg,segments){
    let old=ctx.get_state("visibility");let hidden=[];
    if old!=()&&old.dataset==cfg.dataset&&old.segment==cfg.segment{
        for s in segments{if s.id in old.hidden{hidden.push(s.id);}}
    }
    if hidden.len==segments.len{return [];}
    hidden
}
fn toggle(ticket,key,ctx,event){
    let current=ctx.get_state("visibility");let hidden=[];
    if current!=()&&current.dataset==ticket.dataset&&current.segment==ticket.segment{
        for s in ticket.ids{if s in current.hidden{hidden.push(s);}}
    }
    if key!=""{
        let n=hidden.index_of(key);
        if n>=0{hidden.remove(n);}
        else if hidden.len+1<ticket.ids.len{hidden.push(key);}
    }else{hidden=[];}
    ctx.set_state("visibility",#{dataset:ticket.dataset,segment:ticket.segment,hidden:hidden});
    handled()
}
fn bucket(record,field){
    let v=get(record,field,());let tag=type_of(v);
    if v!=()&&(tag=="string"||tag=="i64"||tag=="f64"){v.to_string()}else{""}
}
fn reading(record,field){
    if field==""{return 1.0;}
    let v=get(record,field,());if v==(){return 0.0;}
    let tag=type_of(v);
    if tag!="i64"&&tag!="f64"{throw "BreakdownChart sum fields must be numeric";}
    let n=compact::decimal(v);
    if !(abs(n)<=1000000000000.0){throw "BreakdownChart sum fields must be finite and within ±1e12";}
    n
}
fn step_size(span){
    if span<=0.0{return 1.0;}
    let raw=max(span/5.0,0.000000000001);let size=10.0**log(raw,10.0).floor();
    for k in [1.0,2.0,2.5,5.0,10.0]{if size*k>=raw{return size*k;}}
    size*10.0
}
fn model(p,slice,segment,measure,hidden,segments){
    let slices=lookup(slice.levels);let orders=lookup(segments);let raw=#{};
    for s in slice.levels{raw[s.id]=#{};}
    let sf=get(slice,"field",slice.id);let wf=if segment==(){""}else{get(segment,"field",segment.id)};
    let nf=get(measure,"field","");
    // Each accepted record contributes once to a single cell.
    for record in p.records{
        if type_of(record)!="map"{throw "BreakdownChart records must be row maps";}
        let a=bucket(record,sf);if !(a in slices){continue;}
        let b=if segment==(){"all"}else{bucket(record,wf)};
        if !(b in orders){continue;}
        let cell=raw[a];cell[b]=get(cell,b,0.0)+reading(record,nf);raw[a]=cell;
    }
    let by_id=#{};let grand=0.0;let positive=0.0;let negative=0.0;let keys=[];
    for a in slice.levels{
        let cells=raw[a.id];let high=0.0;let low=0.0;
        for s in segments{
            if s.id in hidden{continue;}
            let v=get(cells,s.id,0.0);if v>=0.0{high+=v;}else{low+=v;}
        }
        let total=high+low;grand+=total;positive=max(positive,high);negative=min(negative,low);
        by_id[a.id]=#{item:a,cells:cells,total:total};keys.push(a.id);
    }
    let step=step_size(positive-negative);
    let hi=if negative==0.0{5}else{max(1,(positive/step).ceiling().to_int())};
    let lo=if negative==0.0{0}else{(negative/step).floor().to_int()};
    let ticks=[];
    for k in lo..hi+1{ticks.push(k.to_float()*step);}
    #{grand:grand,by_id:by_id,keys:keys,segments:segments,segmented:segment!=(),hidden:hidden,high:hi.to_float()*step,low:lo.to_float()*step,ticks:ticks}
}
fn value_at(n,m){max(0.0,min(1.0,(m.high-n)/(m.high-m.low)))}
fn amount(ctx,value){
    if value>0.0&&value<0.05{return "<"+ctx.format_number(0.1);}
    if value<0.0&&value> -0.05{return "−<"+ctx.format_number(0.1);}
    ctx.format_number(value,#{max_fraction_digits:1})
}
fn tick_amount(ctx,value){
    let a=abs(value);
    if a>=1000000000.0{return amount(ctx,value/1000000000.0)+"B";}
    if a>=1000000.0{return amount(ctx,value/1000000.0)+"M";}
    if a>=1000.0{return amount(ctx,value/1000.0)+"k";}
    amount(ctx,value)
}
fn tone(level,index){
    theme_color(get(level,"ink","breakdown_chart.c"+(1+index%7).to_string()))
}
fn set_attention(scope,field,id,ctx,inside){compact::attend(scope,field,id,ctx,inside);}
fn read_attention(ctx,scope,m){
    let a=compact::attention(ctx,scope);
    for key in [a.bar,a.row,a.focus]{if key in m.by_id{return key;}}
    ""
}
fn focus_chart(scope,key,ctx,event){
    ctx.focus("bar:"+key);compact::attend(scope,"focus",key,ctx,true);handled()
}
fn clear_attention(ctx,event){ctx.set_state("attention",());handled()}
fn rect_style(){style().absolute().top(px(0)).inset_start(px(0)).width(relative(1.0)).height(relative(1.0))}
fn plot_span(){style().absolute().top(px(0)).bottom(unit(6)).width(relative(1.0))}
fn segment_stack(ctx,m,row_data,active){
    let above=0.0;let below=0.0;let marks=[];
    if active{
        marks.push(box([]).with_key("track").accessibility_hidden(true)
            .with_style(ctx.component_style("track",rect_style().background(theme_color("breakdown_chart.rest")).radius(theme_radius("sm")))));
    }
    let zero=value_at(0.0,m);
    for i in 0..m.segments.len{
        let s=m.segments[i];let v=if s.id in m.hidden{0.0}else{get(row_data.cells,s.id,0.0)};
        let from=if v>=0.0{above}else{below};let to=from+v;
        if v>=0.0{above=to;}else{below=to;}
        let start=value_at(max(from,to),m);let end=value_at(min(from,to),m);
        // Keep zero-height marks mounted for a continuous layout transition.
        let mark=style().absolute().top(relative(start)).height(relative(max(0.0,end-start)))
            .inset_start(px(0)).width(relative(1.0)).background(tone(s,i));
        if from!=0.0&&end-start>=0.01{
            mark=mark.border_color(theme_color("surface_raised"));
            mark=if v>0.0{mark.border_bottom(px(1))}else{mark.border_top(px(1))};
        }
        marks.push(box([]).with_key("segment:"+s.id).accessibility_hidden(true)
            .with_style(ctx.component_style("segment",mark))
            .layout_motion(520,ease()));
    }
    stack(marks).with_key("stack").accessibility_hidden(true)
        .with_style(ctx.component_style("bar",style().relative().width(unit(3)).max_width(relative(1.0))
            .height(relative(1.0)).radius(theme_radius("sm")).overflow_hidden()
            .transform(#{origin_y:zero})))
        .motion(motion_transition("scale_y",0.0,1.0,#{duration_ms:520,easing:ease(),intent:"decorative"}))
}
fn tick_layer(ctx,m){
    let lines=[];
    for i in 0..m.ticks.len{
        let value=m.ticks[i];let pos=value_at(value,m);
        let style_part=if value==0.0{["gridline","zero"]}else{["gridline"]};
        lines.push(box([]).with_key("grid:"+i.to_string()).accessibility_hidden(true).with_style(ctx.component_style(style_part,
            style().absolute().top(relative(pos)).width(relative(1.0)).height(sp(0.25)).background(theme_color("breakdown_chart.grid")))));
    }
    stack(lines).with_key("grid").accessibility_hidden(true).with_style(plot_span())
}
fn swatch(ctx,color){
    box([]).accessibility_hidden(true).with_style(ctx.component_style("swatch",style().width(sp(2)).height(sp(2))
        .radius(theme_radius("pill")).flex_shrink(false).background(color)))
}
fn category(ctx,item,part,style_base){
    let word=get(item,"glyph","");let content=[];
    if word!=""{content.push(text(word).accessibility_hidden(true).with_style(style().typography("caption").flex_shrink(false)));}
    content.push(ink(ctx,part,item.label,style().typography("caption").whitespace_nowrap().text_ellipsis()));
    row(content).with_style(style_base.gap(sp(1)).min_width(px(0)))
}
fn description(ctx,m,d,measure){
    let value=amount(ctx,d.total)+if measure.unit!=""{" "+measure.unit}else{""};
    let read=d.item.label+": "+value;
    for s in m.segments{
        if m.segmented&&!(s.id in m.hidden){read+="; "+s.label+": "+amount(ctx,get(d.cells,s.id,0.0));}
    }
    read
}
fn compare_amount(a,b){if a.value==b.value{a.slot-b.slot}else if a.value>b.value{-1}else{1}}
fn tooltip(ctx,m,d,measure,index,plot_width,rtl){
    let stats=[];
    for i in 0..m.segments.len{
        let s=m.segments[i];let v=get(d.cells,s.id,0.0);
        if v!=0.0&&!(s.id in m.hidden){stats.push(#{slot:i,value:v});}
    }
    stats.sort(Fn("compare_amount"));
    let children=[row([category(ctx,d.item,"tooltip_title",style().flex_grow()),
        ink(ctx,"tooltip_value",amount(ctx,d.total),style().typography("caption").font_weight(600))])
        .with_style(style().gap(sp(2)).justify_between().min_width(px(0)))];
    if m.segmented{
        for cell in stats{
            let s=m.segments[cell.slot];
            children.push(row([swatch(ctx,tone(s,cell.slot)),ink(ctx,"tooltip_item",s.label,style().typography("caption").flex_grow().text_ellipsis()),
                ink(ctx,"tooltip_value",amount(ctx,cell.value),style().typography("caption").flex_shrink(false))])
                .with_key(s.id).with_style(style().min_width(px(0)).gap(sp(1.5)).items_center()));
        }
    }
    let ratio=(index.to_float()+0.5)/m.keys.len.to_float();if rtl{ratio=1.0-ratio;}
    let width=min(1.0,176.0/max(1.0,plot_width));let gap=min(0.09,16.0/max(1.0,plot_width));
    let left=if ratio<0.5{ratio+gap}else{ratio-gap-width};
    left=max(0.0,min(1.0-width,left));
    column(children).with_key("tooltip").accessibility_role("status").accessibility_label(d.item.label+" breakdown")
        .accessibility_description(description(ctx,m,d,measure))
        .with_style(ctx.component_style("tooltip",style().absolute().top(px(0)).left(relative(left))
            .width(relative(width)).min_width(px(0)).gap(sp(1.5)).padding(sp(2.5)).radius(theme_radius("lg"))
            .background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border"))))
        .layout_motion(220,ease())
}
fn plot(ctx,m,measure,scope,active){
    let reference=element_ref("plot");
    let right=ctx.text_direction()=="rtl";
    let columns=[];
    let last=m.keys.len-1;
    for i in 0..m.keys.len{
        let key=m.keys[i];let d=m.by_id[key];let on=active==key;
        let press=Fn("focus_chart").curry(scope,key);
        let bar=row([segment_stack(ctx,m,d,on)]).with_key("bar").with_ref(element_ref("bar:"+key))
            .tab_stop(true).accessibility_role("button").accessibility_label(d.item.label+" bar")
            .accessibility_description(description(ctx,m,d,measure))
            .on_hover_change(Fn("set_attention").curry(scope,"bar",key))
            .on_focus_within_change(Fn("set_attention").curry(scope,"focus",key))
            .on_click(press).on_key_value("plain+enter",press,()).on_key_value("plain+space",press,())
            .on_key_value("plain+left",Fn("focus_chart").curry(scope,m.keys[max(0,i-1)]),())
            .on_key_value("plain+right",Fn("focus_chart").curry(scope,m.keys[min(last,i+1)]),())
            .on_key_value("plain+home",Fn("focus_chart").curry(scope,m.keys[0]),())
            .on_key_value("plain+end",Fn("focus_chart").curry(scope,m.keys[last]),())
            .on_key_value("plain+escape",Fn("clear_attention"),())
            .with_style(ctx.component_style("column",style().relative().flex_grow().width(relative(1.0)).min_height(px(0)).justify_center().items_center()
                .cursor_crosshair().focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))
                .opacity(if active==""||on{1.0}else{0.35}))).paint_transition(160,"ease_out");
        let x=category(ctx,if on{d.item}else{#{label:d.item.label}},if on{["x_axis","x_active"]}else{["x_axis"]},
            style().height(unit(6)).width(relative(1.0)).justify_center().items_center().padding_x(sp(0.5))).accessibility_hidden(true);
        columns.push(column([bar,x]).with_key("category:"+key).with_style(style().min_width(px(0)).flex_basis(px(0)).flex_grow().height(relative(1.0))));
    }
    let frame=[tick_layer(ctx,m),row(columns).with_key("columns").with_style(rect_style())];
    if active in m.by_id{
        let size=ctx.element_size(reference);
        frame.push(tooltip(ctx,m,m.by_id[active],measure,m.keys.index_of(active),if size==(){486.0}else{size.width},right));
    }
    let ticks=[];
    for i in 0..m.ticks.len{
        let v=m.ticks[i];
        ticks.push(ink(ctx,"tick",tick_amount(ctx,v),style().absolute().top(relative(value_at(v,m))).margin_top(offset_px(-8))
            .width(relative(1.0)).typography("caption").text_end().text_color(theme_color("text_muted")).whitespace_nowrap()));
    }
    let axis=stack([stack(ticks).with_style(plot_span())]).with_key("value-axis").accessibility_hidden(true)
        .with_style(ctx.component_style("axis",style().relative().width(unit(11)).height(relative(1.0)).flex_shrink(false)));
    let pair=row([stack(frame).with_key("plot").with_ref(reference).accessibility_role("group").accessibility_label(tx(ctx,"plot","Breakdown plot"))
            .with_style(ctx.component_style("plot",style().relative().height(relative(1.0)).min_width(px(0)).flex_grow().flex_basis(px(0)))),
        axis]).with_key("plot-frame").with_style(ctx.component_style("plot_frame",style().min_width(px(0)).height(unit(56)).gap(sp(1))));
    column([pair]).with_key("chart").with_style(ctx.component_style("chart",style().min_width(px(0)).padding_x(sp(4))
        .padding_top(sp(5)).padding_bottom(sp(2))))
}
fn table_cell(ctx,value,index,muted,head,color){
    let first=index==1;let total=index==2;
    let parts=["cell"];if head{parts.push("cell_header");}if total{parts.push("cell_total");}if muted{parts.push("cell_muted");}
    let base=style().justify_center().flex_grow().flex_basis(px(0))
        .min_width(unit(if first{21}else{22})).padding_x(sp(2));
    let text_style=style().typography("caption").min_width(px(0)).text_ellipsis().whitespace_nowrap().font_feature("tnum",1);
    let word=ink(ctx,parts,value,if first{text_style.text_start()}else{text_style.text_end()});
    let contents=if color==(){word}else{row([swatch(ctx,color),word]).with_style(style().items_center().justify_end().gap(sp(1.5)).min_width(px(0)))};
    let cell=column([contents]).with_style(base);
    if head{cell.accessibility_column_header(value,index)}
    else if first{cell.accessibility_role("rowheader").accessibility_label(value).accessibility_column_index(index)}
    else{cell.accessibility_table_cell(value,index)}
}
fn breakdown_table(ctx,m,slice,measure,scope,active,segmented){
    let header=[table_cell(ctx,slice.label,1,false,true,()),table_cell(ctx,measure.label,2,false,true,())];
    if segmented{for i in 0..m.segments.len{let s=m.segments[i];header.push(table_cell(ctx,s.label,i+3,s.id in m.hidden,true,tone(s,i)));}}
    let base=style().height(unit(9)).width(relative(1.0)).min_width(px(0))
        .border_top(sp(0.25)).border_color(theme_color("border"));
    let lines=[row(header).with_key("header").accessibility_role("row").accessibility_row_index(1).with_style(ctx.component_style("table_header",base))];
    for j in 0..m.keys.len{
        let key=m.keys[j];
        let d=m.by_id[key];
        let title=get(d.item,"glyph","");if title!=""{title+=" ";}title+=d.item.label;
        let cells=[table_cell(ctx,title,1,false,false,()),table_cell(ctx,amount(ctx,d.total),2,false,false,())];
        if segmented{
            for i in 0..m.segments.len{let s=m.segments[i];cells.push(table_cell(ctx,amount(ctx,get(d.cells,s.id,0.0)),i+3,s.id in m.hidden,false,()));}
        }
        lines.push(row(cells).with_key("row:"+key).tab_stop(true).accessibility_role("row")
            .accessibility_label(d.item.label).accessibility_description(description(ctx,m,d,measure)).accessibility_row_index(j+2)
            .on_hover_change(Fn("set_attention").curry(scope,"row",key))
            .on_focus_within_change(Fn("set_attention").curry(scope,"focus",key))
            .on_key_value("plain+escape",Fn("clear_attention"),())
            .with_style(ctx.component_style(if active==key{["table_row","row_active"]}else{["table_row"]},base.focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))))
            .paint_transition(160,"ease_out"));
    }
    let tracks=21+22*(header.len-1);
    let sheet=column(lines).accessibility_role("table").accessibility_label(tx(ctx,"table","Breakdown detail"))
        .accessibility_row_count(lines.len).accessibility_column_count(header.len)
        .with_style(style().min_width(unit(tracks)).width(relative(1.0)));
    column([sheet]).with_key("table").with_style(ctx.component_style("table",style().min_width(px(0)).width(relative(1.0)).overflow_x_scroll()));
}
fn reveal_legend(key,ctx,inside){if inside{ctx.scroll_into_view(key);}}
fn legend_button(ctx,parts,key,name,children,base,action){
    row(children).with_key(key).accessibility_role("button").accessibility_label(name)
        .tab_stop(true).on_click(action).on_key_value("plain+enter",action,()).on_key_value("plain+space",action,())
        .with_style(ctx.component_style(parts,base.items_center().height(unit(7)).cursor_pointer()
            .typography("caption").hover(style().background(theme_color("surface_hover")))
            .focus_ring(px(2),offset_px(2),theme_color("focus_ring")))).paint_transition(160,"ease_out")
}
fn legend(ctx,cfg,m,segmented,clean,narrow){
    let controls=[];
    let footer=[];
    if segmented{
        let keys=[];for s in m.segments{keys.push(s.id);}
        let ticket=#{dataset:cfg.dataset,segment:cfg.segment,ids:keys};
        for i in 0..m.segments.len{
            let s=m.segments[i];let shown=!(s.id in m.hidden);
            let key="legend:"+s.id;
            let parts=["legend_chip"];
            if narrow{parts.push("legend_chip_narrow");}
            if !shown{parts.push("legend_hidden");}
            let label=text(if shown{[span(s.label)]}else{[span(s.label).strikethrough()]})
                .with_style(style().min_width(px(0)).whitespace_nowrap().text_ellipsis());
            let chip=legend_button(ctx,parts,key,s.label,[swatch(ctx,tone(s,i)).with_style(style().flex_shrink(false)),label],
                style().gap(sp(1.5)).padding_x(sp(2.25)).min_width(px(0)).flex_shrink(false)
                    .max_width(if clean{unit(55)}else{relative(1.0)}).opacity(if shown{1.0}else{0.45}),
                Fn("toggle").curry(ticket,s.id)).accessibility_pressed(shown);
            if clean{
                chip=chip.with_ref(element_ref(key)).on_focus_within_change(Fn("reveal_legend").curry(key));
            }
            controls.push(chip);
        }
        let strip=row(controls).with_key("legend").accessibility_role("group").accessibility_label(tx(ctx,"legend","Segments"))
            .scrollbars("hidden","hidden")
            .with_style(ctx.component_style(if narrow{["legend","legend_narrow"]}else{["legend"]},
                style().min_width(px(0)).flex_grow().gap(sp(0.5))
                    .merge(if clean{style().flex_nowrap().overflow_x_scroll()}else{style().flex_wrap()})));
        footer.push(strip);
        if m.hidden.len>0{
            let name=tx(ctx,"show_all","Show all");
            footer.push(legend_button(ctx,"legend_reset","show-all",name,[text(name).with_style(style().whitespace_nowrap())],
                style().padding_x(sp(2.5)).flex_shrink(false).font_weight(500),Fn("toggle").curry(ticket,"")));
        }
    }else{
        footer.push(ink(ctx,"legend_hint",tx(ctx,"hint","Pick a segment to split the bars"),
            style().padding_start(sp(2)).typography("caption").text_color(theme_color("text_muted"))));
    }
    row(footer).with_key("footer").with_style(ctx.component_style(if narrow{["footer","footer_narrow"]}else{["footer"]},
        style().min_width(px(0)).min_height(unit(11)).justify_between().items_center().gap(sp(2.5)).padding(sp(1.5))))
}
fn heading(ctx,p,value,measure,narrow){
    row([ink(ctx,"title",p.title,style().typography("body").font_weight(500).flex_basis(px(120)).flex_grow().text_ellipsis().whitespace_nowrap())
            .accessibility_role("heading").accessibility_level(2),
        row([ink(ctx,"total",amount(ctx,value),style().typography("body").font_weight(600).font_feature("tnum",1).whitespace_nowrap()),
            ink(ctx,"unit",measure,style().typography("caption").text_color(theme_color("breakdown_chart.muted"))).with_key("unit")])
            .with_key("total").accessibility_role("status").accessibility_label(tx(ctx,"visible_total","Visible total")+": "+amount(ctx,value)
                +if measure!=""{" "+measure}else{""}).with_style(style().gap_x(sp(1)).items_baseline().flex_wrap().max_width(relative(1.0)).flex_shrink(false))])
        .with_key("heading").with_style(ctx.component_style(if narrow{["heading","heading_narrow"]}else{["heading"]},
            style().gap_x(px(12)).gap_y(px(2)).padding_x(sp(3)).padding_y(sp(2)).justify_between().items_baseline().flex_wrap()))
}
fn render_chart(ctx,p){
    if !ids::valid(p.dataset){throw "BreakdownChart requires a stable dataset ID";}
    let base=style().min_width(px(0)).width(relative(1.0)).max_width(px(560)).radius(theme_radius("lg")).background(theme_color("surface_raised"));
    let root=element_ref("card");
    let theme=ctx.theme_variant();let clean=theme!=()&&theme.family=="Clean";
    let narrow=clean&&ctx.element_width_class(root,[420])==0;
    if p.dimensions.len==0||p.measures.len==0{
        return column([heading(ctx,p,0.0,"",narrow),ink(ctx,"empty",tx(ctx,"empty","No grouping available"),
            style().padding(sp(4)).typography("caption"))]).with_key(p.key).with_ref(root).with_style(ctx.component_style("root",base));
    }
    let dims=lookup(p.dimensions);let measures=lookup(p.measures);
    let cfg=choice(ctx,p,dims,measures);let side=dims[cfg.slice];let measure=measures[cfg.measure];
    let segment=if cfg.segment==""{()}else{dims[cfg.segment]};
    let segments=if segment==(){[#{id:"all",label:tx(ctx,"all","All")}]}else{segment.levels};
    let hidden=visibility(ctx,cfg,segments);
    let m=model(p,side,segment,measure,hidden,segments);
    let scope=#{dataset:p.dataset,slice:cfg.slice};
    let active=read_attention(ctx,scope,m);
    let content=[controls(ctx,p,cfg)];
    if m.keys.len>0&&segments.len>0{
        content.push(plot(ctx,m,measure,scope,active));
        content.push(breakdown_table(ctx,m,side,measure,scope,active,segment!=()));
    }else{
        content.push(ink(ctx,"empty",tx(ctx,"empty","No grouping available"),style().padding(sp(4)).typography("caption")));
    }
    let panel=column(content).with_key("panel").with_style(ctx.component_style("panel",
        style().min_width(px(0)).width(relative(1.0)).background(theme_color("surface_raised")).radius(theme_radius("lg"))))
        .layout_size_motion(cfg.slice+":"+m.keys.len.to_string(),"height",420,ease());
    let foot=legend(ctx,cfg,m,segment!=(),clean,narrow);
    column([heading(ctx,p,m.grand,measure.unit,narrow),panel,foot]).with_key(p.key).with_ref(root).accessibility_role("group").accessibility_label(p.title)
        .with_style(ctx.component_style("root",base))
}
