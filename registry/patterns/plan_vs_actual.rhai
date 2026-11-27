/* gpui-rhai
{"id":"patterns/plan_vs_actual","export":"PlanVsActual","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"assets":[],"tokens":["border","focus_ring","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","plan_vs_actual.actual","plan_vs_actual.actual_edge","plan_vs_actual.actual_mark","plan_vs_actual.ahead","plan_vs_actual.ahead_fill","plan_vs_actual.ahead_gap","plan_vs_actual.card","plan_vs_actual.forecast","plan_vs_actual.forecast_edge","plan_vs_actual.ghost","plan_vs_actual.grid","plan_vs_actual.late","plan_vs_actual.late_edge","plan_vs_actual.late_fill","plan_vs_actual.late_gap","plan_vs_actual.link","plan_vs_actual.live","plan_vs_actual.live_line","plan_vs_actual.muted","plan_vs_actual.quiet","plan_vs_actual.text","plan_vs_actual.wash"],"environment":[]}
*/
// Compare a dependency plan to observed work without starting or modifying tasks.
// Example: PlanVsActual(#{key:"run",run:"pricing",now_ms:18000,steps:[#{id:"layout",title:"Build layout",planned_ms:14000,start_ms:0}]})
define_component(#{
    metadata:#{id:"patterns/plan_vs_actual","export":"PlanVsActual",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:[],capabilities:#{},assets:[],tokens:["border","focus_ring","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","plan_vs_actual.actual","plan_vs_actual.actual_edge","plan_vs_actual.actual_mark","plan_vs_actual.ahead","plan_vs_actual.ahead_fill","plan_vs_actual.ahead_gap","plan_vs_actual.card","plan_vs_actual.forecast","plan_vs_actual.forecast_edge","plan_vs_actual.ghost","plan_vs_actual.grid","plan_vs_actual.late","plan_vs_actual.late_edge","plan_vs_actual.late_fill","plan_vs_actual.late_gap","plan_vs_actual.link","plan_vs_actual.live","plan_vs_actual.live_line","plan_vs_actual.muted","plan_vs_actual.quiet","plan_vs_actual.text","plan_vs_actual.wash"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable instance key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of a single run. Changing it clears ephemeral attention."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted revision returned with inspection proposals."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Defaults to Plan vs actual."},
        task:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional shared task label."},
        now_ms:#{schema:#{type:"integer",min:0,max:31536000000},required:true,sensitive:false,doc:"Host-owned elapsed milliseconds; no script timer is created."},
        steps:#{schema:#{type:"array",max_items:96,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable step ID."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Visible step name."},
            planned_ms:#{schema:#{type:"integer",min:0,max:31536000000},required:true,sensitive:false,doc:"Planned step duration."},
            depends_on:#{schema:#{type:"array",max_items:32,items:#{type:"string"}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Distinct prerequisite step IDs. Input does not have to be topologically ordered."},
            start_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:31536000000}},required:false,sensitive:false,doc:"Actual start, relative to run start. Omit for waiting steps."},
            end_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:31536000000}},required:false,sensitive:false,doc:"Recorded finish. When only the finish is known, the display infers a non-negative start from the planned duration."},
            spent_micros:#{schema:#{type:"optional",value:#{type:"integer",min:-1000000000000,max:1000000000000}},required:false,sensitive:true,doc:"Observed signed cost in currency micro-units."},
            planned_micros:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:true,doc:"Optional budget in currency micro-units. A known zero is not missing."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative mark beside the step."}
        }}},required:true,sensitive:true,doc:"Accepted snapshot in display order; the steps form an acyclic dependency graph."},
        currency:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Display prefix, defaults to $."},
        pinned:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Controlled inspection ID, or empty to leave unpinned."},
        on_inspect:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called for a deliberate pin or unpin, not passive hover."}
    },state:#{fields:#{attention:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
    events:#{inspect:#{payload:#{type:"object",allow_unknown:false,fields:#{
        run:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
        id:#{schema:#{type:"string"},required:true,sensitive:false}
    }},doc:"Inspection proposal. Empty ID clears the pin."}},slots:#{},
    parts:["root","head","head_narrow","title","task","mark","totals","totals_narrow","metric","metric_label","metric_value","verdict","axis","axis_label","scaffold",
        "rail","track","row","row_hot","row_dim","state","step","step_icon","actual","forecast","over","plan","sheen","duration","delta",
        "guide","now_dot","tick","finish_tag","wire","wire_dot","legend","legend_narrow","legend_mark","detail","detail_title","detail_line","empty"],effects:["sync_run"]},
    render:Fn("render_run")
});

fn PlanVsActual(p){render_component("patterns/plan_vs_actual",p)}
fn get(m,k,d){if k in m&&m[k]!=(){m[k]}else{d}}
fn u(n){theme_length("space.unit").times(n)}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn tx(ctx,k,fallback){ctx.t_or("plan_vs_actual."+k,fallback)}
fn paint(k){let token="plan_vs_actual."+k;theme_color(token)}
fn ease(){cubic_bezier(0.32,0.72,0.24,1.0)}
fn cap(){style().typography("caption").text_color(paint("muted")).min_width(px(0))}
fn label(ctx,part,t,s){text(t).with_style(ctx.component_style(part,s))}
fn pair(n){if n<10{"0"+n.to_string()}else{n.to_string()}}
fn clock(n){
    let s=(max(0,n)+500)/1000;
    if s>=86400{return (s/86400).to_string()+"d "+((s%86400)/3600).to_string()+"h";}
    if s>=3600{return (s/3600).to_string()+":"+pair((s/60)%60)+":"+pair(s%60);}
    (s/60).to_string()+":"+pair(s%60)
}
fn difference(n){
    let s=(abs(n)+500)/1000;
    (if n<0{"−"}else{"+"})+if s<60{s.to_string()+"s"}else{clock(abs(n))}
}
fn money(ctx,p,n){
    if n==(){return "—";}
    let prefix=get(p,"currency","$");let v=abs(n);let sign=if n<0{"−"}else{""};
    if v>0&&v<5000{return sign+"<"+prefix+"0.01";}
    sign+prefix+ctx.format_number(v.to_float()/1000000.0,#{min_fraction_digits:2,max_fraction_digits:2})
}
fn svg_path(ctx,d,tone){
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
        .accessibility_hidden(true).with_style(ctx.component_style("state",style().width(u(4)).height(u(4)).flex_shrink(false).text_color(paint(tone))))
}
fn state_icon(ctx,st){
    if st=="done"{return svg_path(ctx,"M3 8.5 6.2 11.5 13 4.8","muted");}
    if st=="waiting"{return svg_path(ctx,"M8 3a5 5 0 1 0 0 10A5 5 0 0 0 8 3m0 2.6v3l1.8 1","quiet");}
    svg_path(ctx,"M8 2a6 6 0 0 1 6 6M8 14a6 6 0 0 1-4.2-1.8","live").with_key("running")
        .motion(motion_transition("rotate",0.0,360.0,#{duration_ms:900,iterations:"infinite",intent:"decorative"}))
}
fn local(ctx,id){
    let s=ctx.get_state("attention");
    if s!=()&&s.run==id{s}else{#{run:id,hover:"",focus:"",cursor:"",pinned:"",muted:false}}
}
fn sync_run(ctx,data){
    let s=ctx.get_state("attention");
    if s!=()&&s.run!=data.run{ctx.set_state("attention",local(ctx,data.run));}
}
fn idle(ctx,data){}
fn attend(run,field,id,ctx,inside){
    let s=local(ctx,run);
    if inside{
        if s[field]==id{return;}
        s[field]=id;s.muted=false;
        if field=="focus"{s.cursor=id;}
    }else if s[field]==id{s[field]="";}else{return;}
    ctx.set_state("attention",s);
}
fn pin(t,id,ctx,e){
    let s=local(ctx,t.run);
    let next=if id==t.pinned{""}else{id};
    if !t.controlled{s.pinned=next;}
    s.muted=false;ctx.set_state("attention",s);
    ctx.emit("inspect",#{run:t.run,revision:t.revision,id:next});handled()
}
fn close(t,ctx,e){
    let s=local(ctx,t.run);
    if t.pinned==""&&s.focus==""&&s.hover==""{return ();}
    s.hover="";s.muted=true;
    if !t.controlled{s.pinned="";}
    ctx.set_state("attention",s);
    if t.pinned!=""{ctx.emit("inspect",#{run:t.run,revision:t.revision,id:""});}
    handled()
}
fn row_ref(i){"step:"+i.to_string()}
fn seek(run,i,ctx,e){
    let s=local(ctx,run);s.hover="";s.muted=false;ctx.set_state("attention",s);
    let key=row_ref(i);ctx.focus(key);
    let b=ctx.element_bounds(key);
    if b!=()&&b.clip!=()&&(b.visual.y<b.clip.y||b.visual.y+b.visual.height>b.clip.y+b.clip.height){
        ctx.scroll_into_view(key);
    }
    handled()
}
fn fmt_status(ctx,st){switch st{
    "done"=>tx(ctx,"done","Done"),"running"=>tx(ctx,"running","Running"),_=>tx(ctx,"waiting","Waiting")
}}
fn verdict(ctx,p,m){
    let n=m.data.late_by;
    if abs(n)<1500{return tx(ctx,"on","On plan");}
    if n>0{return difference(n)+" "+if m.data.finished{tx(ctx,"late","late")}else{tx(ctx,"behind","behind")};}
    difference(n).sub_string(1)+" "+if m.data.finished{tx(ctx,"early","early")}else{tx(ctx,"ahead","ahead")}
}
fn timed(ctx,words,v,ref,alert){
    let atomic=cap().whitespace_nowrap();
    row([label(ctx,"metric_label",words,atomic),
        label(ctx,"metric_value",v,atomic.text_color(if alert{paint("late")}else{theme_color("text_primary")}).font_weight(500).font_feature("tnum",1)),
        label(ctx,"metric_label",ref,atomic)])
        .with_style(ctx.component_style("metric",style().items_baseline().gap_x(u(1.5)).flex_shrink(false)))
}
fn head(ctx,p,m){
    let title=get(p,"title",tx(ctx,"title","Plan vs actual"));
    let mark=svg("<svg viewBox='0 0 16 16' fill='currentColor'><rect x='1.5' y='2.5' width='7' height='3' rx='1.5'/><rect x='5' y='6.5' width='8' height='3' rx='1.5' opacity='.6'/><rect x='8.5' y='10.5' width='6' height='3' rx='1.5' opacity='.32'/></svg>")
        .accessibility_hidden(true).with_style(ctx.component_style("mark",style().width(u(4)).height(u(4)).flex_shrink(false).text_color(paint("live"))));
    let title_node=label(ctx,"title",title,style().typography("body").font_weight(500).min_width(px(0)).whitespace_nowrap().text_ellipsis())
        .accessibility_role("heading").accessibility_level(2);
    let left=[mark,title_node];
    let task=get(p,"task","");
    if !m.narrow&&task!=""{
        left.push(label(ctx,"task",task,cap().whitespace_nowrap().text_ellipsis().flex_grow().flex_basis(px(0))).with_key("task"));
    }
    let elapsed=clock(if m.data.finished{m.data.eta}else{p.now_ms});
    let baseline=clock(m.data.planned_end);
    let figures=[timed(ctx,tx(ctx,"time","Time"),elapsed,"/ "+baseline,!m.data.finished&&p.now_ms>m.data.planned_end)];
    if m.data.spent!=()||m.data.budget!=(){
        let total=money(ctx,p,m.data.spent);let budget=if m.data.budget==(){""}else{"/ "+money(ctx,p,m.data.budget)};
        figures.push(timed(ctx,tx(ctx,"cost","Cost"),total,budget,m.data.spent!=()&&m.data.budget!=()&&m.data.spent>m.data.budget));
    }
    let desc=verdict(ctx,p,m);
    let state=if m.data.late_by>=1500{"late"}else if m.data.late_by<=-1500{"ahead"}else{"on"};
    let result=label(ctx,"verdict",desc,cap().padding_x(sp(2)).radius(theme_radius("sm")).flex_shrink(false).whitespace_nowrap()
        .background(if state=="on"{paint("wash")}else{paint(state+"_fill")})
        .text_color(if state=="on"{paint("text")}else{paint(state)}))
        .with_key("verdict").accessibility_role("status").accessibility_label(desc);
    let stats_base=style().min_width(px(0)).items_center().flex_shrink(false).gap_x(u(3.5)).margin_end(u(1));
    let stats=if m.narrow {
        row(figures).with_key("totals").with_style(ctx.component_style(["totals","totals_narrow"],
            stats_base.width(relative(1.0)).gap_x(u(4)).gap_y(u(1)).margin_end(px(0)).flex_wrap()))
    }else{row(figures).with_key("totals").with_style(ctx.component_style("totals",stats_base))};
    let start=row(left).with_key("title").with_style(style().min_width(px(0)).items_center().gap_x(u(2)).flex_grow().flex_basis(px(0)));
    let items=if m.narrow{[start,result,stats]}else{[start,stats,result]};
    let base=style().min_width(px(0)).gap_x(u(2)).gap_y(u(1.5)).items_center()
        .padding_start(u(4)).padding_end(u(3.5)).padding_y(u(3))
        .border_bottom(px(0.5)).border_color(theme_color("border"));
    row(items).with_key("head").with_style(if m.narrow{ctx.component_style(["head","head_narrow"],
        base.padding_start(u(3)).padding_end(u(3)).padding_y(u(2.5)).flex_wrap())}else{ctx.component_style("head",base)})
}
fn frac(node,n,base){
    node.with_style(base.width(relative(n)).flex_shrink(false))
        .motion(motion_transition("width_fraction",n,n,#{duration_ms:700,easing:ease(),intent:"decorative"}))
}
fn fraction(n,m){max(0.0,min(1.0,n.to_float()/m.data.range.to_float()))}
fn spacer(value){frac(box([]).with_key("offset"),value,style().height(px(0)))}
fn layer_style(){style().absolute().inset_start(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0))}
fn placed(key,x,kids,base){
    let out=[spacer(x)];for node in kids{out.push(node);}
    row(out).with_key(key).with_style(base)
}
fn rail(ctx,m,children){
    row(children).with_style(ctx.component_style("rail",style().width(u(if m.narrow{30}else{47})).min_width(px(0))
        .items_center().gap_x(sp(2)).flex_shrink(false)))
}
fn scaffold(ctx,left,right,s){
    row([left,right]).with_style(ctx.component_style("scaffold",style().gap_x(sp(3.5)).padding_x(sp(4)).min_width(px(0)))).with_style(s)
}
fn track(ctx,nodes){
    box(nodes).with_style(ctx.component_style("track",style().relative().height(relative(1.0)).flex_grow().flex_basis(px(0)).min_width(px(0))))
}
fn sheen(ctx,tone){
    let clear=rgba(0x00000000);let ink=paint("card");
    let sweep=style().absolute().top(px(0)).inset_start(px(0)).width(u(15)).height(relative(1.0))
        .layered_gradient(layered_gradient([#{angle:90,stops:[#{offset:0.0,color:clear},#{offset:0.5,color:ink},#{offset:1.0,color:clear}]}])).opacity(0.7);
    box([]).with_key("sweep").with_style(ctx.component_style("sheen",sweep))
        .motion(motion_transition("translate_x_fraction",-1.0,12.0,#{duration_ms:1500,iterations:"infinite",intent:"decorative"}))
}
fn bar(ctx,m,key,a,b,kind,hot,animated){
    let start=fraction(a,m);let width=fraction(b,m)-start;
    if width<=0.0{return ();}
    let is_plan=kind=="plan";
    let base=style().height(u(3.5)).min_width(px(0)).relative().radius(theme_radius("sm"))
        .border(px(if is_plan{1.0}else{0.5})).border_color(if is_plan{paint("ghost")}else if hot&&kind=="actual"{paint("actual_mark")}
            else if kind=="over"{paint("late_edge")}else{paint(kind+"_edge")});
    if is_plan{base=base.border_dashed();}
    else{
        base=base.background(paint(if kind=="over"{"late_fill"}else{kind})).clip();
    }
    let children=if animated{[sheen(ctx,kind)]}else{[]};
    let rect=frac(box(children).with_key("band"),width,ctx.component_style(kind,base));
    placed(key,start,[rect],layer_style().items_center()).accessibility_hidden(true)
}
fn tag(ctx,p,m,step,r,hot){
    let waiting=r.status=="waiting";
    let main=clock(if waiting{step.planned_ms}else{r.elapsed});
    let suffix=if (r.status=="done"||r.over)&&abs(r.delta)>=1500{difference(r.delta)}else{""};
    let cost=if hot&&get(step,"spent_micros",())!=(){money(ctx,p,step.spent_micros)}else{""};
    let children=[label(ctx,"duration",main,cap().text_color(if waiting{paint("quiet")}else{theme_color("text_primary")}).font_feature("tnum",1))];
    if suffix!=""{
        let k=if r.delta>0{"late"}else{"ahead"};
        children.push(label(ctx,"delta",suffix,cap().font_weight(500).padding_x(sp(1)).radius(theme_radius("sm"))
            .text_color(paint(k)).background(paint(k+"_fill"))));
    }
    if cost!=""&&!m.narrow{
        children.push(label(ctx,"duration","·",cap().text_color(paint("quiet")).flex_shrink(false)));
        children.push(label(ctx,"duration",cost,cap().text_color(paint("text")).whitespace_nowrap().text_ellipsis()));
    }
    let count=grapheme_count(main+suffix+if m.narrow{""}else{cost});
    let tag_width=7.0*count.to_float()+if suffix==""{12.0}else{22.0};
    let w=max(1.0,m.width);
    let right=fraction(max(r.end,r.plan_end),m)*w+4.0;
    let left=fraction(min(r.start,r.plan_start),m)*w-4.0-tag_width;
    let x=if right+tag_width<=w{right}else if left>=0.0{left}else{max(0.0,w-tag_width)};
    let body=row(children).with_key("readout").with_style(ctx.component_style("duration",
        style().min_width(px(0)).max_width(px(max(1.0,w))).gap_x(sp(1)).padding_x(sp(1)).items_center()
            .background(if hot{rgba(0x00000000)}else{paint("card")})));
    placed("readout",x/w,[body],layer_style().items_center()).accessibility_hidden(true)
}
fn reference_title(p,i){p.steps[i].title}
fn names(p,ids){
    let result="";
    for i in ids{if result!=""{result+=", ";}result+=reference_title(p,i);}
    if result==""{"—"}else{result}
}
fn summary(ctx,p,s,r){
    let status=fmt_status(ctx,r.status);
    let clock_words=if r.status=="waiting"{tx(ctx,"starts","Starts in")+" "+clock(max(0,r.start-p.now_ms))}
        else{tx(ctx,"elapsed","Elapsed")+" "+clock(r.elapsed)};
    let cost=if get(s,"spent_micros",())==(){""}else{" · "+tx(ctx,"cost","Cost")+" "+money(ctx,p,s.spent_micros)};
    status+" · "+clock_words+" · "+tx(ctx,"plan","Plan")+" "+clock(s.planned_ms)+cost+
        " · "+tx(ctx,"deps","Waits for")+": "+names(p,r.deps)+" · "+tx(ctx,"next","Unblocks")+": "+names(p,r.next)
}
fn row_step(ctx,p,m,i){
    let s=p.steps[i];let r=m.data.rows[i];let hot=m.hot==i;
    let rang=[state_icon(ctx,r.status)];
    let icon=get(s,"icon",());
    if icon!=()&&!m.narrow{
        rang.push(box([icon]).accessibility_hidden(true).with_style(ctx.component_style("step_icon",
            style().width(u(5)).height(u(5)).flex_shrink(false).radius(theme_radius("sm")).clip())));
    }
    rang.push(label(ctx,"step",s.title,style().typography("body").min_width(px(0)).whitespace_nowrap().text_ellipsis()
        .text_color(if r.status=="waiting"{paint("text")}else{theme_color("text_primary")})));
    let layers=[];
    if r.status=="waiting"{layers.push(bar(ctx,m,"forecast",r.start,r.end,"forecast",hot,false));}
    else{
        if r.end>r.observed_end{layers.push(bar(ctx,m,"forecast",r.observed_end,r.end,"forecast",hot,false));}
        let base_end=if r.over{r.start+s.planned_ms}else{r.observed_end};
        layers.push(bar(ctx,m,"actual",r.start,base_end,"actual",hot,r.status=="running"&&!r.over));
        if r.over{layers.push(bar(ctx,m,"over",base_end,r.observed_end,"over",hot,r.status=="running"));}
    }
    layers.push(bar(ctx,m,"plan",r.plan_start,r.plan_end,"plan",hot,false));
    layers.push(tag(ctx,p,m,s,r,hot));
    let children=[];for layer in layers{if layer!=(){children.push(layer);}}
    let state=["row"];if hot{state.push("row_hot");}
    if m.hot>=0&&!hot&&!(i in m.near){state.push("row_dim");}
    let rail_base=style().height(u(8)).items_center().focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))
        .hover(style().background(paint("wash")));
    let pick=Fn("pin").curry(m.ticket,s.id);
    scaffold(ctx,rail(ctx,m,rang),track(ctx,children),ctx.component_style(state,rail_base)).with_key("step:"+s.id).with_ref(element_ref(row_ref(i)))
        .accessibility_role("button").accessibility_label(s.title).accessibility_description(summary(ctx,p,s,r)).accessibility_pressed(s.id==m.pinned)
        .tab_stop(m.cursor==i).on_hover_change(Fn("attend").curry(p.run,"hover",s.id))
        .on_focus_within_change(Fn("attend").curry(p.run,"focus",s.id)).on_click(pick)
        .on_key_value("plain+enter",pick,()).on_key_value("plain+space",pick,())
        .on_key_value("plain+escape",Fn("close").curry(m.ticket),()).on_key_value("plain+up",Fn("seek").curry(p.run,max(0,i-1)),())
        .on_key_value("plain+down",Fn("seek").curry(p.run,min(p.steps.len-1,i+1)),())
        .on_key_value("plain+home",Fn("seek").curry(p.run,0),())
        .on_key_value("plain+end",Fn("seek").curry(p.run,p.steps.len-1),()).paint_transition(180,"ease_out")
}
fn vertical(ctx,m,key,value,tone,dashed,start){
    let line=style().height(relative(1.0)).width(px(1)).flex_shrink(false);
    if dashed{line=line.border_left(px(1)).border_color(paint(tone)).border_dashed();}
    else{line=line.background(paint(tone));}
    placed(key,fraction(value,m),[box([]).with_key("line").with_style(ctx.component_style("guide",line))],
        layer_style().top(if start{u(8)}else{px(0)}).bottom(px(0)).height(auto()))
}
fn guides(ctx,p,m,reference){
    let shapes=[];
    if abs(m.data.late_by)>=1500{
        let a=fraction(min(m.data.eta,m.data.planned_end),m);
        let b=fraction(max(m.data.eta,m.data.planned_end),m);
        let bg=if m.data.late_by>0{paint("late_gap")}else{paint("ahead_gap")};
        let rect=frac(box([]).with_key("wash"),b-a,style().height(relative(1.0)).background(bg));
        shapes.push(placed("variance",a,[rect],layer_style().top(u(8)).bottom(px(0)).height(auto())));
    }
    for i in 0..m.data.ticks.len{
        let t=m.data.ticks[i];let key=t.to_string();
        shapes.push(vertical(ctx,m,"tick:"+key,t,"grid",false,false));
        if !m.narrow||i%2==0{
            let end=max(0.0,1.0-34.0/max(32.0,m.width));
            shapes.push(placed("tick-label:"+key,min(fraction(t,m)+0.008,end),[label(ctx,"tick",clock(t),cap().font_feature("tnum",1))],
                layer_style().padding_top(sp(1.5))));
        }
    }
    shapes.push(vertical(ctx,m,"plan",m.data.planned_end,"ghost",true,true));
    if abs(m.data.late_by)>=1500{
        shapes.push(vertical(ctx,m,"eta",m.data.eta,if m.data.late_by>0{"late_edge"}else{"ahead"},false,true));
    }
    if !m.data.finished{
        shapes.push(vertical(ctx,m,"now",p.now_ms,"live_line",false,true));
        let marker=box([]).with_key("dot").with_style(ctx.component_style("now_dot",style().width(u(1.25)).height(u(1.25))
            .radius(u(10)).margin_start(offset_px(-2.0)).margin_top(offset_px(-2.5)).flex_shrink(false).background(paint("live"))));
        shapes.push(placed("now-dot",fraction(p.now_ms,m),[marker],layer_style().top(u(8)).height(u(3)).items_start()));
    }
    let plot=track(ctx,shapes).with_key("plot").with_ref(reference);
    scaffold(ctx,rail(ctx,m,[]),plot,layer_style()).with_key("guides").accessibility_hidden(true)
}
fn wire(ctx,m,a,b,key,y0,y1){
    let left=min(a,b);let len=abs(b-a);let top=min(y0,y1);let h=max(0.5,abs(y1-y0));
    let ink=paint("link");
    let line_style=ctx.component_style("wire",style().width(px(1)).height(relative(1.0)).flex_shrink(false).background(ink));
    let vertical=placed("stem:"+key,a,[box([]).with_key("v").with_style(line_style)],
        layer_style().top(px(top)).height(px(h)));
    let horizontal=placed("line:"+key,left,[frac(box([]).with_key("h"),len,style().height(px(1)).background(ink))],
        layer_style().top(px(y1)).height(px(1)));
    let out=[vertical,horizontal];
    for side in ["out","in"]{
        let mark=box([]).with_key("tip").with_style(ctx.component_style("wire_dot",style().width(u(1.25)).height(u(1.25)).flex_shrink(false)
            .border(px(1)).border_color(ink).background(if side=="out"{paint("card")}else{ink})));
        out.push(placed("dot:"+side+":"+key,if side=="out"{a}else{b},[mark],
            layer_style().top(offset_px((if side=="out"{y0}else{y1})-2.0)).height(u(1.25))));
    }
    out
}
fn link_overlay(ctx,m,stage){
    let plot=ctx.element_bounds(stage);
    if plot==(){return ();}
    let edges=[];let target=m.data.rows[m.hot];
    for dep in target.deps{edges.push([dep,m.hot]);}
    for next in target.next{edges.push([m.hot,next]);}
    let paths=[];
    for e in edges{
        let a=ctx.element_bounds(row_ref(e[0]));let b=ctx.element_bounds(row_ref(e[1]));
        if a==()||b==(){continue;}
        let r=m.data.rows[e[0]];let next=m.data.rows[e[1]];
        let y0=max(0.0,a.layout.y-plot.layout.y+a.layout.height*0.5+if e[0]<e[1]{7.0}else{-7.0});
        let y1=max(0.0,b.layout.y-plot.layout.y+b.layout.height*0.5);
        let x1=fraction(next.start,m);
        let hi=fraction(r.end,m);let lo=fraction(r.start,m);
        let x0=max(lo,hi-min((hi-lo)*0.5,10.0/max(1.0,m.width)));
        paths+=wire(ctx,m,x0,x1,e[0].to_string()+":"+e[1].to_string(),y0,y1);
    }
    scaffold(ctx,rail(ctx,m,[]),track(ctx,paths),layer_style()).with_key("dependencies").accessibility_hidden(true)
}
fn finish_badge(ctx,key,words,tone,at,width){
    let style=cap().height(u(5)).min_width(px(0)).whitespace_nowrap().text_ellipsis();
    style=if tone=="plan"{style.background(paint("wash")).text_color(paint("text"))}else{style.background(paint(tone+"_fill")).text_color(paint(tone))};
    placed(key,at/width,[label(ctx,"finish_tag",words,style)],layer_style().padding_top(sp(1.25)).items_start())
}
fn finish_badges(ctx,m){
    if m.narrow||m.width<110.0{return [];}
    let width=max(1.0,m.width);
    let plan=m.data.planned_end;let eta=m.data.eta;
    let ptxt=tx(ctx,"plan","Plan")+" "+clock(plan);
    let ftxt=(if m.data.finished{tx(ctx,"done","Done")}else{tx(ctx,"eta","ETA")})+" "+clock(eta);
    let pw=grapheme_count(ptxt).to_float()*7.0+12.0;
    let fw=grapheme_count(ftxt).to_float()*7.0+12.0;
    if abs(m.data.late_by)<1500{
        let x=max(0.0,min(width-pw,fraction(plan,m)*width-pw*0.5));
        return [finish_badge(ctx,"plan-badge",ptxt,"plan",x,width)];
    }
    if pw+fw+4.0>width{return [];}
    let late=eta>plan;let left=if late{pw}else{fw};let right=if late{fw}else{pw};
    let a=max(0.0,min(width-left-right-4.0,fraction(min(eta,plan),m)*width-left));
    let b=max(a+left+4.0,min(width-right,fraction(max(eta,plan),m)*width));
    [finish_badge(ctx,"plan-badge",ptxt,"plan",if late{a}else{b},width),
        finish_badge(ctx,"eta-badge",ftxt,if late{"late"}else{"ahead"},if late{b}else{a},width)]
}
fn legend(ctx,p,m){
    let children=[];
    for entry in [["plan",tx(ctx,"plan","Plan")],["actual",tx(ctx,"actual","Actual")],["over",tx(ctx,"over","Over plan")]]{
        let colour=switch entry[0]{"plan"=>paint("ghost"),"actual"=>paint("actual_edge"),_=>paint("late_edge")};
        let base=style().width(u(3.5)).height(u(2)).radius(theme_radius("sm")).border(px(0.5)).border_color(colour);
        if entry[0]=="plan"{base=base.border_dashed();}else{base=base.background(if entry[0]=="actual"{paint("actual")}else{paint("late_fill")});}
        children.push(row([box([]).with_style(ctx.component_style("legend_mark",base)),label(ctx,"tick",entry[1],cap())])
            .with_style(style().items_center().gap(sp(1.5))));
    }
    let key=if m.narrow{row([])}else{row(children).with_style(style().items_center().gap_x(sp(2)))};
    let lane=if p.steps.len==0{[]}else{finish_badges(ctx,m)};
    let foot=scaffold(ctx,rail(ctx,m,[key]),track(ctx,lane),ctx.component_style(if m.narrow{["legend","legend_narrow"]}else{["legend"]},style().min_width(px(0))
        .height(u(if m.narrow{4}else{10})).items_center().border_top(px(1)).border_color(theme_color("border")))).with_key("legend");
    if m.hot<0{return column([foot]).with_key("foot-and-inspection");}
    let step=p.steps[m.hot];let r=m.data.rows[m.hot];
    let text=[label(ctx,"detail_title",step.title,cap().font_weight(600)),
        label(ctx,"detail_line",tx(ctx,"deps","Waits for")+": "+names(p,r.deps),cap()),
        label(ctx,"detail_line",tx(ctx,"next","Unblocks")+": "+names(p,r.next),cap())];
    if get(step,"spent_micros",())!=(){text.push(label(ctx,"detail_line",tx(ctx,"cost","Cost")+" "+money(ctx,p,step.spent_micros),cap()));}
    let detail=row(text).with_key("detail").accessibility_role("status").accessibility_label(step.title+" · "+summary(ctx,p,step,r))
        .with_style(ctx.component_style("detail",style().min_width(px(0)).padding(sp(2.75)).items_center().gap(sp(2)).flex_wrap()
            .border_top(px(1)).border_color(theme_color("border"))));
    column([foot,detail]).with_key("foot-and-inspection")
}
fn breakpoint_probe(reference){
    // Measure just the authored responsive length in the inherited scale.
    // This shell has no layout, scroll, input, or semantic contribution.
    box([box([]).with_key("breakpoint-width").with_ref(reference)
        .with_style(style().width(u(140)).height(px(0)).flex_shrink(false))])
        .with_key("breakpoint-measure").accessibility_hidden(true)
        .with_style(style().absolute().inset_start(px(0)).top(px(0)).width(px(0)).height(px(0)).overflow_hidden().invisible())
}
fn render_run(ctx,p){
    if p.run==""||utf8_byte_count(p.run)>96||utf8_byte_count(get(p,"currency","$"))>12{throw "PlanVsActual needs a bounded run identifier and currency prefix";}
    let data=plan_vs_actual_projection(p.steps,p.now_ms);let indices=#{};
    for i in 0..p.steps.len{
        let s=p.steps[i];if s.title==""||utf8_byte_count(s.title)>512{throw "Plan step needs a nonempty bounded title";}
        indices[s.id]=i;
    }
    let shell=element_ref("plan-vs-actual");let stage=element_ref("stage");let plot=element_ref("plot");
    let w=ctx.element_size(plot);
    let s=local(ctx,p.run);let controlled=get(p,"pinned",())!=();
    let pin=get(p,"pinned",s.pinned);if !(pin in indices){pin="";}
    let hovered=if s.hover in indices{s.hover}else if s.focus in indices{s.focus}else{""};
    let active=if pin!=""{pin}else if s.muted{""}else{hovered};
    let hot=get(indices,active,-1);
    let near=if hot<0{[]}else{data.rows[hot].deps+data.rows[hot].next};
    let breakpoint=element_ref("responsive-breakpoint");
    let measure=ctx.element_size(breakpoint);
    let narrow=measure==()||measure.width<=0.0||ctx.element_width_class(shell,[measure.width])!=1;
    let m=#{data:data,width:if w==(){420.0}else{w.width},narrow:narrow,hot:hot,near:near,
        cursor:get(indices,s.cursor,0),pinned:pin,ticket:#{run:p.run,revision:p.revision,pinned:pin,controlled:controlled}};
    effect("sync_run",#{run:p.run},Fn("sync_run"),Fn("idle"));
    let body=[];
    if p.steps.len==0{body.push(label(ctx,"empty",tx(ctx,"empty","No scheduled steps"),cap().padding(sp(4))));}
    else{
        body.push(guides(ctx,p,m,plot));
        body.push(scaffold(ctx,rail(ctx,m,[label(ctx,"axis_label",tx(ctx,"step","STEP"),cap().font_weight(500))]),track(ctx,[]),
            ctx.component_style("axis",style().height(u(8)).items_center().border_bottom(px(1)).border_color(theme_color("border")))).with_key("axis"));
        for i in 0..p.steps.len{body.push(row_step(ctx,p,m,i));}
        if hot>=0{
            let wires=link_overlay(ctx,m,stage);if wires!=(){body.push(wires);}
        }
    }
    let graph=column(body).with_key("stage").with_ref(stage).accessibility_role("group").accessibility_label(tx(ctx,"timeline","Schedule timeline"))
        .with_style(style().relative().min_width(px(0)));
    column([head(ctx,p,m),graph,legend(ctx,p,m),breakpoint_probe(breakpoint)]).with_key("plan-vs-actual").with_ref(shell)
        .with_style(ctx.component_style("root",style().relative().width(relative(1.0)).min_width(px(0)).background(paint("card"))))
}
