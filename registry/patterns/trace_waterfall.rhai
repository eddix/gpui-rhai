/* gpui-rhai
{"id":"patterns/trace_waterfall","export":"TraceWaterfall","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/spinner","components/icon_button"],"capabilities":{},"assets":[],"tokens":["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","text_primary","design.name","trace.heading_case","typography.body","typography.caption","typography.code","typography.label","trace.tray","trace.card","trace.muted","trace.text","trace.grid","trace.guide","trace.hover","trace.selected","trace.well","trace.accent","trace.now","trace.agent_bg","trace.agent_edge","trace.agent_ink","trace.agent_mark","trace.llm_bg","trace.llm_edge","trace.llm_ink","trace.llm_mark","trace.tool_bg","trace.tool_edge","trace.tool_ink","trace.tool_mark","trace.error_bg","trace.error_edge","trace.error_ink","trace.error_mark","trace.live_bg","trace.live_ink","trace.done_bg","trace.done_ink","trace.slow_bg","trace.slow_ink"],"environment":[]}
*/
// Time-based execution tree with exact rollups, a shared clock and span details.
// Example: TraceWaterfall(#{key:"run",session:"preview",spans:[#{id:"root",name:"Review",kind:"agent",status:"done",start_ms:0,end_ms:820}]})
import "components/spinner" as spin;
import "components/icon_button" as btn;

define_component(#{
    metadata:#{id:"patterns/trace_waterfall","export":"TraceWaterfall",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/spinner","components/icon_button"],capabilities:#{},assets:[],tokens:["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","text_primary","design.name","trace.heading_case","typography.body","typography.caption","typography.code","typography.label","trace.tray","trace.card","trace.muted","trace.text","trace.grid","trace.guide","trace.hover","trace.selected","trace.well","trace.accent","trace.now","trace.agent_bg","trace.agent_edge","trace.agent_ink","trace.agent_mark","trace.llm_bg","trace.llm_edge","trace.llm_ink","trace.llm_mark","trace.tool_bg","trace.tool_edge","trace.tool_ink","trace.tool_mark","trace.error_bg","trace.error_edge","trace.error_ink","trace.error_mark","trace.live_bg","trace.live_ink","trace.done_bg","trace.done_ink","trace.slow_bg","trace.slow_ink"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of this run; changing it resets local folding and selection."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted Host revision carried in interaction events."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Overrides the trace heading; it does not name or identify a span."},
        currency:#{schema:#{type:"string"},"default":#{type:"string",value:"USD"},required:false,sensitive:false,doc:"Currency code for integer cost_micros; not a conversion."},
        spans:#{schema:#{type:"array",max_items:128,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique span ID, stable across timing updates, folds and inspection."},
            parent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Parent ID; exactly one span has no parent."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible span name in the tree and inspector."},
            kind:#{schema:#{type:"string",allowed:["agent","subagent","llm","tool"]},required:true,sensitive:false,doc:"Span category; model and tool calls contribute to call and slowest-call reporting."},
            status:#{schema:#{type:"string",allowed:["running","done","failed"]},required:true,sensitive:false,doc:"Host-reported lifecycle status of this span."},
            start_ms:#{schema:#{type:"integer",min:0,max:31536000000},required:true,sensitive:false,doc:"Elapsed milliseconds since the run began."},
            end_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:31536000000}},required:false,sensitive:false,doc:"Elapsed milliseconds at completion. Missing running ends follow the clock."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional private explanatory text for this span."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional model label shown with the span metrics."},
            icon:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Short custom mark replacing the kind swatch."},
            tokens_in:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Input tokens attributable to this span itself; do not include its descendants here."},
            tokens_out:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Output tokens attributable to this span itself; do not include its descendants here."},
            cost_micros:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"This span's own cost; descendants are added to their parents."},
            input:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional input text displayed only for inspection, not replay."},
            output:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional read-only output text supplied by the Host."},
            error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional private failure detail; the status field determines whether the span is failed."}
        }}},required:true,sensitive:true,doc:"Bounded flat tree. Parent may occur before or after its children. Siblings retain input order."},
        now_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:31536000000}},required:false,sensitive:false,doc:"Deterministic supplied elapsed time; takes precedence over started_at_ms."},
        started_at_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Epoch time for the wall-clock live mode."},
        min_range_ms:#{schema:#{type:"integer",min:1000,max:31536000000},"default":#{type:"integer",value:10000},required:false,sensitive:false,doc:"Minimum displayed elapsed-time range; longer spans can extend it."},
        max_height:#{schema:#{type:"integer",min:96,max:1800},"default":#{type:"integer",value:460},required:false,sensitive:false,doc:"Scrollable tree viewport height cap in logical pixels."},
        mark_slowest:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Marks the longest finished model or tool call even when folding changes visible rows."},
        collapsed:#{schema:#{type:"optional",value:#{type:"array",max_items:128,items:#{type:"string"}}},required:false,sensitive:false,doc:"Controlled collapsed span IDs when supplied."},
        default_collapsed:#{schema:#{type:"array",max_items:128,items:#{type:"string"}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Initial set of folded spans when disclosure is local."},
        open_id:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Controlled inspector ID. An explicit null closes it."},
        default_open_id:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initial inspected span when inspector selection is local."},
        on_collapsed_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the requested fold plus the complete proposed collapsed ID list and run identity."},
        on_open_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a span ID or null with the current session and revision; private span text is omitted."}
    }, state:#{fields:#{
        ui:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:false},
        clock:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:false}
    }},events:#{
        collapsed_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            session:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer"},required:true,sensitive:false},
            id:#{schema:#{type:"string"},required:true,sensitive:false},collapsed:#{schema:#{type:"array",max_items:128,items:#{type:"string"}},required:true,sensitive:false}
        }},doc:"Disclosure proposal with the new complete collapsed ID list."},
        open_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            session:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer"},required:true,sensitive:false},
            id:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:false}
        }},doc:"Requested span ID or null; never includes private span text."}
    },slots:#{},parts:["root","head","spark","title","task","indicator","status","head_status","head_live","head_done","head_error","card","stats","stats_narrow","stat","stat_rule","stat_label","stat_label_tiny","stat_value","stat_value_narrow","stat_error","axis","axis_caption","axis_label","tick",
       "grid","rule","now","now_dot","rows","row","row_open","names","guides","guide","chevron","tile","mark","name","subtitle","span_label_flow","count","error_glyph",
       "lane","bar","bar_narrow","shine","fragment","readout","readout_tag","slow","details","detail_head","detail_title","detail_subtitle","detail_close","detail_stats","detail_stat_label","detail_stat_value",
       "detail_error","documents","document","empty"],effects:["focus"]},render:Fn("view_trace")
});
fn TraceWaterfall(p){render_component("patterns/trace_waterfall",p)}
fn get(m,k,d){if m!=()&&k in m&&m[k]!=(){m[k]}else{d}}
fn u(n){theme_length("space.unit").times(n)}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn ink(name){let k="trace."+name;theme_color(k)}
fn tx(ctx,k,v){ctx.t_or("trace."+k,v)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn subtle(){style().typography("caption").min_width(px(0)).text_color(ink("muted"))}
fn line(){subtle().whitespace_nowrap().text_ellipsis()}
fn node(ctx,n,k,base){n.with_style(ctx.component_style(k,base))}
fn label(ctx,k,v,base){node(ctx,text(v),k,base)}
fn option_name(ctx,k){
    switch k{"agent"=>tx(ctx,"agent","Agent run"),"subagent"=>tx(ctx,"subagent","Sub-agent"),"llm"=>tx(ctx,"llm","LLM call"),_=>tx(ctx,"tool","Tool call")}
}
fn status_name(ctx,k){
    switch k{"running"=>tx(ctx,"running","Running"),"done"=>tx(ctx,"done","Done"),_=>tx(ctx,"failed","Failed")}
}
fn duration(ctx,ms){
    if ms<1000{return ctx.format_number(ms)+" ms";}
    if ms<10000{return ctx.format_number(ms.to_float()/1000.0,#{min_fraction_digits:1,max_fraction_digits:1})+" s";}
    let s=(ms+500)/1000;
    if s<60{return ctx.format_number(s)+" s";}
    if s<3600{return (s/60).to_string()+"m "+(s%60).to_string()+"s";}
    if s<86400{return (s/3600).to_string()+"h "+(s/60%60).to_string()+"m";}
    (s/86400).to_string()+"d "+(s/3600%24).to_string()+"h"
}
fn axis_time(ctx,ms){
    let sec=ms/1000;
    if sec<60{return ctx.format_number(sec)+"s";}
    if sec<3600{return (sec/60).to_string()+"m"+if sec%60==0{""}else{" "+(sec%60).to_string()+"s"};}
    (sec/3600).to_string()+"h"+if sec/60%60==0{""}else{" "+(sec/60%60).to_string()+"m"}
}
fn price(ctx,p,amount){
    let digits=if amount==0{0}else if amount<10000{3}else{2};
    let out=ctx.format_number(amount.to_float()/1000000.0,#{min_fraction_digits:digits,max_fraction_digits:digits});
    (if amount>0&&amount<1000{"<"}else{""})+(if p.currency=="USD"{"$"}else{p.currency+" "})+
        if amount>0&&amount<1000{ctx.format_number(0.001,#{min_fraction_digits:3,max_fraction_digits:3})}else{out}
}
fn tokens(ctx,n){
    if n<1000{return ctx.format_number(n);}
    if n<1000000{return ctx.format_number(n.to_float()/1000.0,#{max_fraction_digits:1})+"k";}
    ctx.format_number(n.to_float()/1000000.0,#{max_fraction_digits:1})+"M"
}
fn glyph(w){
    let d=switch w{"spark"=>"M2 9h2l2-5 3 8 2-4h3","closed"=>"m6 4 4 4-4 4","closed_rtl"=>"m10 4-4 4 4 4","expanded"=>"m4 6 4 4 4-4",
        "failed"=>"M8 2 14 13H2Z M8 6v3m0 2h.01",_=>"m4 4 8 8m0-8-8 8"};
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.35' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
        .accessibility_hidden(true).with_style(style().width(u(3.5)).height(u(3.5)).flex_shrink(false))
}
fn apply_key(n,cb){n.on_click(cb).on_key_value("plain+enter",cb,()).on_key_value("plain+space",cb,())}
fn wheel(ctx,session){ctx.set_state("clock",#{session:session,current:ctx.now()});}
fn now_in(ctx,p){
    if get(p,"now_ms",())!=(){return p.now_ms;}
    let last=0;let live=false;
    for s in p.spans{last=max(last,get(s,"end_ms",s.start_ms));if s.status=="running"{live=true;}}
    let started=get(p,"started_at_ms",());
    if started==()||!live{return last;}
    interval("trace-clock",250,false,Fn("wheel"),p.session);
    let q=ctx.get_state("clock");let t=if q!=()&&q.session==p.session{q.current}else{ctx.now()};
    min(31536000000,max(0,t-started))
}
fn defaults(p){#{session:p.session,cursor:"",open:get(p,"default_open_id",""),folded:p.default_collapsed}}
fn get_ui(ctx,p){let s=ctx.get_state("ui");if s!=()&&s.session==p.session{s}else{defaults(p)}}
fn move(key,ctx,e){ctx.focus(key);ctx.scroll_into_view(key);handled()}
fn focused(seed,id,ctx,inside){
    if inside{
        let s=state_for(ctx,seed);
        if s.cursor!=id{s.cursor=id;ctx.set_state("ui",s);}
    }
}
fn state_for(ctx,seed){let s=ctx.get_state("ui");if s!=()&&s.session==seed.session{s}else{seed}}
fn toggle(seed,t,id,fold,co,ctx,e){
    let next=[];let held=false;
    for value in fold{if value==id{held=true;}else{next.push(value);}}
    if !held{next.push(id);}
    let s=state_for(ctx,seed);
    s.cursor=id;if !co{s.folded=next;}
    ctx.set_state("ui",s);ctx.focus(ref_for(id));ctx.emit("collapsed_change",#{session:t.session,revision:t.revision,id:id,collapsed:next});
    handled()
}
fn open_detail(seed,t,controlled,id,focus,ctx,e){
    let s=state_for(ctx,seed);
    if !controlled{s.open=id;ctx.set_state("ui",s);}
    if focus!=""{ctx.focus(ref_for(focus));}
    ctx.emit("open_change",#{session:t.session,revision:t.revision,id:if id==""{()}else{id}});
    handled()
}
fn disclosure(seed,t,controlled,id,ctx,shown){if !shown{open_detail(seed,t,controlled,"",id,ctx,());}}
fn noop(ctx,e){}
fn repair(ctx,v){
    let s=ctx.get_state("ui");
    if s!=()&&s.session==v.session{
        if v.key!=""&&s.cursor==v.was{s.cursor=v.key;ctx.focus(ref_for(v.key));}
        if v.prune!=""&&s.open==v.prune{s.open="";}
        ctx.set_state("ui",s);
    }
}
fn ref_for(id){"trace-row:"+id}
fn kind(s){if s.status=="failed"{"error"}else if s.kind=="subagent"{"agent"}else{s.kind}}
fn chip(ctx,c,msg,k,bits){
    bits.push(text(msg).with_key("chip-text"));
    let base=line().items_center().gap(sp(0.75)).min_height(u(4.5)).padding_x(sp(1.75)).radius(theme_radius("lg"))
        .background(ink(k+"_bg")).text_color(ink(k+"_ink")).font_weight(550).flex_shrink(false);
    node(ctx,row(bits),c,base)
}
fn caption_label(ctx,part,k,upper){
    label(ctx,part,if upper{k.to_upper()}else{k},line().typography("label")).accessibility_role("text").accessibility_label(k)
}
fn metric(ctx,m,k,v,is_error,detail){
    let head_parts=if detail{["detail_stat_label"]}else if m.tiny{["stat_label","stat_label_tiny"]}else{["stat_label"]};
    let head=caption_label(ctx,head_parts,k,m.upper_labels).with_key(if detail{"detail-stat-label"}else{"stat-label"});
    let parts=if detail{["detail_stat_value"]}else if m.narrow{["stat_value","stat_value_narrow"]}else{["stat_value"]};
    if is_error{parts.push("stat_error");}
    let value=label(ctx,parts,v,style().typography("body").min_width(px(0))
        .whitespace_nowrap().text_ellipsis().font_feature("tnum",1).font_weight(550).text_color(if is_error{ink("error_ink")}else{theme_color("text_primary")})).with_key("stat-value");
    node(ctx,column([head,value]).accessibility_role("group").accessibility_label(k),"stat",style().min_width(px(0)).gap(sp(0.5)))
}
fn header(ctx,p,m){
    let title=get(p,"title",tx(ctx,"title","Trace waterfall"));let d=m.data;
    let bits=[node(ctx,glyph("spark"),"spark",style().text_color(ink("accent")).flex_shrink(false)),
        label(ctx,"title",title,style().typography("body").min_width(px(0)).font_weight(600).whitespace_nowrap().text_ellipsis())
            .accessibility_role("heading").accessibility_level(2)];
    let is_live=d.live;let root=if d.root==(){()}else{p.spans[d.root]};
    if !m.narrow&&root!=(){bits.push(label(ctx,"task",get(root,"input",root.name),line().flex_grow()).with_key("task-caption"));}
    else{bits.push(box([]).with_style(style().flex_grow()));}
    if root!=(){
        let tone=if is_live{"live"}else if root.status=="failed"{"error"}else{"done"};
        let pulse=if !is_live{[]}else{[node(ctx,box([]).with_key("pulse"),"indicator",style().width(u(1.25)).height(u(1.25)).flex_shrink(false)
            .radius(theme_radius("lg")).background(ink("accent"))).motion(motion_transition("opacity",0.4,1.0,#{duration_ms:720,iterations:"infinite",autoreverse:true,intent:"decorative"}))
            .accessibility_hidden(true)]};
        let text=if is_live{tx(ctx,"live","Live")}else{status_name(ctx,root.status)};
        bits.push(chip(ctx,"status",text,tone,pulse).with_key("head-status").with_style(ctx.component_style(["head_status","head_"+tone],style()))
            .accessibility_role("status").accessibility_label(text));
    }
    node(ctx,row(bits).with_key("head"),"head",style().min_width(px(0)).padding(sp(2.5)).gap(sp(2)).items_center())
}
fn summary(ctx,p,m){
    let d=m.data;
    let fields=[
        [tx(ctx,"duration","Duration"),duration(ctx,if d.root==(){0}else{d.rows[d.root].duration}),false],
        [tx(ctx,"cost","Cost"),price(ctx,p,d.cost),false],
        [tx(ctx,"tokens","Tokens"),tokens(ctx,d.tokens_in+d.tokens_out),false],
        [tx(ctx,"llms","LLM calls"),ctx.format_number(d.llms),false],
        [tx(ctx,"tools","Tool calls"),ctx.format_number(d.tools),false],
        [tx(ctx,"errors","Errors"),ctx.format_number(d.errors),d.errors>0]];
    let count=if m.narrow{3}else{6};let pieces=[];
    for i in 0..fields.len{
        let f=fields[i];let cell=metric(ctx,m,f[0],f[1],f[2],false).with_key("metric:"+i.to_string());
        let first=i%count==0;let last=i%count==count-1;
        let regular=if m.tiny&&m.clean{2.5}else if m.narrow{3}else{4};
        let start=if first{if m.tiny&&m.clean{3.5}else if m.narrow{4}else{5.5}}else{regular};
        let end=if last{if m.narrow{2}else{2.5}}else{regular};
        let insets=style().padding_start(u(start)).padding_end(u(end));
        cell=cell.with_style(if first{insets}else{ctx.component_style("stat_rule",
            insets.border_start(px(1)).border_color(ink("grid")))});
        pieces.push(cell);
    }
    node(ctx,box(pieces).with_key("stats").accessibility_role("group").accessibility_label(tx(ctx,"totals","Trace totals")),
        if m.narrow{["stats","stats_narrow"]}else{["stats"]},
        style().min_width(px(0)).gap_y(if m.narrow{sp(3)}else{px(0)}).padding_y(sp(3)).grid().grid_cols(count))
}
// Offset spacers and bars all sample the same native fraction channel, with
// identical duration and easing. No separate clock owns the ticks or the grid.
fn fraction(n,f,base,ms){
    n.with_style(base.width(relative(f))).motion(motion_transition("width_fraction",f,f,#{duration_ms:ms,easing:ease(),intent:"feedback"}))
}
fn origin(m,time){time.to_float()/m.data.range.to_float()}
fn pos(ctx,m,children,time,key,row_height){
    let offset=fraction(box([]).with_key("offset"),origin(m,time),style().height(px(0)).flex_shrink(false),720);
    row([offset]+children).with_key(key).with_style(style().absolute().inset_start(px(0)).top(px(0)).width(relative(1.0)).height(row_height).items_center())
}
fn frame(ctx,m,head,track,key,base){
    if m.narrow{column([head,track]).with_key(key).with_style(base)}
    else{row([head,track]).with_key(key).with_style(base)}
}
fn stem(m,bit){
    bit.with_style(style().min_width(px(0)).width(if m.narrow{relative(1.0)}else{u(59)}).flex_shrink(false))
}
fn axis(ctx,m){
    let labels=[];let h=u(7.5);
    for t in m.data.ticks{
        let text=label(ctx,"tick",axis_time(ctx,t),line().font_feature("tnum",1).padding_start(sp(1))).with_key("tick-label");
        let bits=if m.narrow{[node(ctx,box([]),"rule",style().width(px(1)).height(h).flex_shrink(false).background(ink("grid"))),text]}else{[text]};
        labels.push(pos(ctx,m,bits,t,"tick:"+t.to_string(),h));
    }
    if m.data.live{
        let dot=node(ctx,box([]),"now_dot",style().width(u(1.25)).height(u(1.25)).radius(theme_radius("lg")).background(ink("accent")));
        let aligned=row([dot]).with_style(style().height(h).items_end());
        labels.push(pos(ctx,m,[aligned],m.data.now,"axis-now",h));
    }
    let track=box(labels).with_key("axis-track").with_ref(m.track).with_style(style().relative().min_width(px(0)).flex_grow().height(h).overflow_hidden());
    // The rail and time track still allocate the same space. The caption is
    // chart chrome, independently anchored over the rail, not a row or column
    // heading in the trace's summary content flow.
    let bits=[frame(ctx,m,stem(m,box([])),track,"axis-flow",
        ctx.component_style("axis",style().width(relative(1.0)).min_width(px(0)).padding_x(sp(2.5)).items_center()))];
    if !m.narrow{
        let caption=caption_label(ctx,"axis_label",tx(ctx,"span","Span"),m.upper_labels).with_key("axis-label");
        bits.push(node(ctx,row([caption]).with_key("axis-caption"),"axis_caption",style()
            .absolute().inset_start(u(4)).top(px(0)).width(u(55)).height(h).items_center().clip()));
    }
    stack(bits).with_key("axis").with_style(style().width(relative(1.0)).min_width(px(0))).accessibility_hidden(true)
}
fn grid(ctx,m){
    let marks=[];let h=relative(1.0);
    for t in m.data.ticks{
        let v=node(ctx,box([]),"rule",style().width(px(1)).height(h).flex_shrink(false).background(ink(if t==0{"guide"}else{"grid"})));
        marks.push(pos(ctx,m,[v],t,"rule:"+t.to_string(),h));
    }
    if m.data.live{
        let v=node(ctx,box([]),"now",style().width(px(1)).height(h).flex_shrink(false).background(ink("now")));
        marks.push(pos(ctx,m,[v],m.data.now,"now-line",h));
    }
    let layer=box(marks).with_style(style().relative().min_width(px(0)).flex_grow().height(h));
    let bits=if m.narrow{[layer]}else{[stem(m,box([])),layer]};
    node(ctx,row(bits).with_key("grid").accessibility_hidden(true),"grid",style().absolute().width(relative(1.0)).height(h)
        .inset_start(px(0)).top(px(0)).padding_x(sp(2.5)))
}
fn bar(ctx,s,m,r){
    let color=kind(s);let parts=[];
    if s.status=="running"&&r.duration>0{
        let glint=node(ctx,box([]).with_key("shine"),"shine",style().absolute().top(px(0)).inset_start(px(0)).height(relative(1.0)).width(relative(0.27))
            .background(alpha(ink("card"),0.6)));
        parts.push(glint.motion(motion_transition("translate_x_fraction",if m.rtl{1.0}else{-1.0},if m.rtl{-3.9}else{3.9},
            #{duration_ms:1500,iterations:"infinite",intent:"decorative"})));
    }
    let parts_key=if m.narrow{["bar","bar_narrow"]}else{["bar"]};
    let body=ctx.component_style(parts_key,style().height(u(if m.narrow{2.5}else{4})).min_width(px(2)).flex_shrink(false).radius(theme_radius("sm")).relative().clip()
        .border(px(1)).border_color(ink(color+"_edge")).background(ink(color+"_bg")).opacity(if r.folded{0.75}else{1.0}));
    fraction(row(parts).with_key("bar"),origin(m,r.duration),body,720)
}
fn reading(ctx,p,m,r){duration(ctx,r.duration)+(if r.cost>0&&!m.tiny{" · "+price(ctx,p,r.cost)}else{""})}
fn readout(ctx,p,m,s,r,slow,side){
    let value=reading(ctx,p,m,r);let inside=side=="inside";let inline=side=="inline";
    let fore=if s.status=="failed"||inside{ink(kind(s)+"_ink")}else{ink("text")};
    let bits=[label(ctx,"readout",value,line().typography("label").text_color(fore).font_feature("tnum",1)).with_key("readout-label")];
    if slow{
        if m.tiny{bits.push(box([]).with_style(style().width(u(1.5)).height(u(1.5)).flex_shrink(false)
            .radius(theme_radius("lg")).background(ink("slow_ink"))));}
        else{bits.push(chip(ctx,"slow",tx(ctx,"slowest","Slowest"),"slow",[]).with_key("slow-badge"));}
    }
    if bits.len==2&&(inside||side=="before"){bits=[bits[1],bits[0]];}
    let base=style().items_center().min_width(px(0)).gap_x(sp(1.5));
    if inline{return row(bits).with_style(base.margin_start(sp(2)));}
    let tag=base.min_height(u(4.5)).padding_x(sp(1)).margin_start(if side=="after"{sp(0.5)}else{px(0)});
    let tag=if inside||m.open==s.id{tag}else{tag.background(ink("card"))};
    node(ctx,row(bits).with_key("duration-tag"),"readout_tag",tag)
}
fn lane(ctx,p,m,s,r,slow){
    let h=relative(1.0);
    let bits=[pos(ctx,m,[bar(ctx,s,m,r)],r.start,"segment",h)];
    for i in r.marks{
        let q=m.data.rows[i];let color=kind(p.spans[i]);
        let dot=fraction(box([]).with_key("fragment"),origin(m,q.duration),
            ctx.component_style("fragment",style().height(u(0.75)).min_width(px(1)).flex_shrink(false).radius(theme_radius("sm"))
                .background(ink(color+"_mark")).opacity(if r.folded{0.9}else{0.24})),720);
        bits.push(pos(ctx,m,[dot],q.start,"fragment:"+p.spans[i].id,relative(1.0)));
    }
    if !m.narrow{
        let need=(grapheme_count(reading(ctx,p,m,r)).to_float()*6.3+if slow{64.0}else{12.0});
        let room=((need/m.track_width)*m.data.range.to_float()).round().to_int();
        let after=r.end+room<=m.data.range;
        let inside=!after&&(r.duration>=room||r.start<room);
        let at=if after{r.end}else if inside{max(r.start,r.end-room)}else{max(0,r.start-room)};
        let side=if after{"after"}else if inside{"inside"}else{"before"};
        bits.push(pos(ctx,m,[readout(ctx,p,m,s,r,slow,side)],at,"duration",h));
    }
    node(ctx,box(bits).with_key("lane").accessibility_hidden(true),"lane",style().relative().min_width(px(0)).flex_grow().height(u(if m.narrow{3}else{8}))
        .width(if m.narrow{relative(1.0)}else{auto()}).overflow_hidden())
}
fn span_name(ctx,p,m,s,r,slow,toggle){
    let bits=[];
    for depth in 0..min(if m.narrow{6}else{8},r.depth){
        let rule=node(ctx,box([]),"guide",style().height(relative(1.0)).width(px(1)).background(ink("guide")));
        bits.push(row([rule]).with_key("guide:"+depth.to_string()).with_style(style().width(u(if m.narrow{2.5}else{3.25}))
            .height(relative(1.0)).items_start().flex_shrink(false)).accessibility_hidden(true));
    }
    let knob=style().width(u(4.5)).height(u(5.5)).items_center().justify_center().flex_shrink(false);
    if r.first_child!=(){
        let caption=if r.folded{tx(ctx,"expand","Expand")}else{tx(ctx,"collapse","Collapse")};
        let g=if !r.folded{"expanded"}else if m.rtl{"closed_rtl"}else{"closed"};
        bits.push(node(ctx,apply_key(row([glyph(g)]).with_key("fold").accessibility_role("button").accessibility_label(caption+" "+s.name).tab_stop(false),toggle),
            "chevron",knob.radius(theme_radius("sm")).cursor_pointer().text_color(ink("muted")).hover(style().background(ink("well")))));
    }else{bits.push(box([]).with_style(knob));}
    bits.push(swatch(ctx,s));
    let words=[label(ctx,"name",s.name,line().text_color(theme_color("text_primary")).font_weight(if r.first_child==(){400}else{500}))];
    let sub=get(s,"detail","");
    if sub!=""&&!m.tiny{words.push(label(ctx,"subtitle",sub,line().typography("label")).with_key("span-model-caption"));}
    bits.push(node(ctx,row(words).with_key("span-labels"),"span_label_flow",style().min_width(px(0)).gap_x(sp(1.5)).items_baseline().flex_grow()));
    if r.folded{bits.push(label(ctx,"count","+"+ctx.format_number(r.descendants),line().padding_x(sp(1)).radius(theme_radius("lg")).background(ink("well")).flex_shrink(false)));}
    if s.status=="failed"{bits.push(node(ctx,row([glyph("failed")]),"error_glyph",style().text_color(ink("error_ink")).flex_shrink(false)));}
    else if s.status=="running"&&r.first_child==(){bits.push(spin::Spinner(#{key:"running",label:"Running",size:"xs"}).accessibility_hidden(true));}
    if m.narrow{bits.push(readout(ctx,p,m,s,r,slow,"inline").with_style(style().flex_shrink(false)));}
    node(ctx,row(bits),"names",style().min_width(px(0)).width(if m.narrow{relative(1.0)}else{u(59)}).height(u(if m.narrow{7.5}else{8}))
        .flex_shrink(false).items_center().gap(sp(0.75)))
}
fn swatch(ctx,s){
    let custom=get(s,"icon","");
    let contents=if custom==""{[node(ctx,box([]),"mark",style().width(u(2)).height(u(2)).radius(theme_radius("sm")).background(ink(kind(s)+"_mark")))]}
        else{[text(custom).with_style(line())]};
    node(ctx,row(contents).accessibility_hidden(true),"tile",style().width(u(5)).height(u(5)).flex_shrink(false).items_center().justify_center()
        .background(ink("card")).radius(theme_radius("sm")))
}
fn details(ctx,p,m,s,r,close){
    let caption=get(s,"model",get(s,"detail",""));
    let words=column([label(ctx,"detail_title",s.name,line().text_color(theme_color("text_primary")).font_weight(600)),
        label(ctx,"detail_subtitle",option_name(ctx,s.kind)+(if caption==""{""}else{" · "+caption}),line())])
        .with_style(style().min_width(px(0)).flex_grow());
    let tone=if s.status=="failed"{"error"}else if s.status=="running"{"live"}else{"done"};
    let kill=btn::IconButton(#{key:"close",label:tx(ctx,"close","Close details"),icon:glyph("close"),variant:"ghost",size:"xs",on_click:close});
    let head=node(ctx,row([swatch(ctx,s),words,chip(ctx,"status",status_name(ctx,s.status),tone,[]),kill]).with_key("details-head"),
        "detail_head",style().min_width(px(0)).items_center().gap(sp(1.5)));
    let stats=[metric(ctx,m,tx(ctx,"duration","Duration"),duration(ctx,r.duration),false,true),
        metric(ctx,m,tx(ctx,"started","Started"),"+"+duration(ctx,r.start),false,true)];
    if r.cost!=0{stats.push(metric(ctx,m,tx(ctx,"cost","Cost"),price(ctx,p,r.cost),false,true));}
    if r.tokens_in!=0{stats.push(metric(ctx,m,tx(ctx,"tokens_in","Tokens in"),tokens(ctx,r.tokens_in),false,true));}
    if r.tokens_out!=0{stats.push(metric(ctx,m,tx(ctx,"tokens_out","Tokens out"),tokens(ctx,r.tokens_out),false,true));}
    if r.descendants>0{stats.push(metric(ctx,m,tx(ctx,"count","Spans"),ctx.format_number(r.descendants),false,true));}
    let body=[head,node(ctx,box(stats),"detail_stats",style().grid().grid_cols(if m.narrow{2}else{3}).gap(sp(1.5)).padding(sp(2)).background(ink("well"))
        .min_width(px(0)).radius(theme_radius("sm")))];
    if get(s,"error","")!=""{body.push(label(ctx,"detail_error",s.error,subtle().text_color(ink("error_ink"))
        .min_width(px(0)).padding(sp(2)).background(ink("error_bg")).radius(theme_radius("sm"))));}
    for name in ["input","output"]{
        let data=get(s,name,"");
        if data==""{continue;}
        let field=if name=="input"{tx(ctx,"input","Input")}else{tx(ctx,"output","Output")};
        let value=label(ctx,"document",data,style().typography(if s.kind=="tool"{"code"}else{"caption"}).text_color(theme_color("text_primary"))
            .min_width(px(0)).padding(sp(2)).background(ink("well")).radius(theme_radius("sm")));
        body.push(node(ctx,column([caption_label(ctx,"detail_stat_label",field,m.upper_labels),value]).with_key(name),"documents",style().gap(sp(1)).min_width(px(0))));
    }
    node(ctx,column(body).with_key("details").accessibility_role("dialog").accessibility_label(s.name+" "+tx(ctx,"details","details"))
        .on_key_value("plain+escape",close,()).scrollbars("hidden","auto"),"details",style().width(u(80)).max_height(u(106))
        .min_width(px(0)).overflow_y_scroll().gap(sp(2.5)).padding(sp(3)).radius(theme_radius("lg")).background(ink("card")))
        .enter_motion(motion_transition("opacity",0.0,1.0,#{duration_ms:220,intent:"decorative"}))
}
fn render_row(ctx,p,m,i){
    let order=m.data.visible;let k=order[i];let r=m.data.rows[k];let s=p.spans[k];let on=m.open==s.id;let slow=p.mark_slowest&&m.data.slowest==k;
    let fold=Fn("toggle").curry(m.seed,m.ticket,s.id,m.folded,m.control_fold);
    let click=Fn("open_detail").curry(m.seed,m.ticket,m.control_open,if on{""}else{s.id},"");
    let close=Fn("open_detail").curry(m.seed,m.ticket,m.control_open,"",s.id);
    let parts=if on{["row","row_open"]}else{["row"]};
    let base=ctx.component_style(parts,style().min_width(px(0)).padding_x(sp(2.5)).items_center().min_height(u(8))
        .background(if on{ink("selected")}else{rgba(0x00000000)}).hover(style().background(ink("hover")))
        .cursor_pointer().focus_ring(px(2),offset_px(-2),theme_color("focus_ring")));
    let name=span_name(ctx,p,m,s,r,slow,fold);
    let track=lane(ctx,p,m,s,r,slow);
    let n=frame(ctx,m,name,track,"row-node",if m.narrow{base.padding_y(sp(0.75))}else{base})
        .with_ref(element_ref(ref_for(s.id))).tab_stop(m.active==s.id).accessibility_role("treeitem").accessibility_label(s.name)
        .accessibility_level(r.depth+1).accessibility_selected(on)
        .accessibility_description(option_name(ctx,s.kind)+" · "+status_name(ctx,s.status)+" · "+duration(ctx,r.duration)+
            (if r.cost==0{""}else{" · "+price(ctx,p,r.cost)})+(if slow{" · "+tx(ctx,"slowest","Slowest")}else{""}))
        .on_focus_within_change(Fn("focused").curry(m.seed,s.id)).paint_transition(180,"ease_out");
    let n=apply_key(n,click)
        .on_key_value("plain+up",Fn("move").curry(ref_for(p.spans[order[max(i-1,0)]].id)),())
        .on_key_value("plain+down",Fn("move").curry(ref_for(p.spans[order[min(i+1,order.len-1)]].id)),())
        .on_key_value("plain+home",Fn("move").curry(ref_for(p.spans[order[0]].id)),())
        .on_key_value("plain+end",Fn("move").curry(ref_for(p.spans[order[order.len-1]].id)),());
    if m.open!=""{n=n.on_key_value("plain+escape",close,());}
    let forward=if m.rtl{"plain+left"}else{"plain+right"};let back=if m.rtl{"plain+right"}else{"plain+left"};
    if r.first_child!=(){
        n=n.accessibility_expanded(!r.folded);
        if r.folded{n=n.on_key_value(forward,fold,());}
        else{n=n.on_key_value(back,fold,()).on_key_value(forward,Fn("move").curry(ref_for(p.spans[r.first_child].id)),());}
    }
    if (r.first_child==()||r.folded)&&r.parent!=(){n=n.on_key_value(back,Fn("move").curry(ref_for(p.spans[r.parent].id)),());}
    let panel=if on{details(ctx,p,m,s,r,close)}else{box([])};
    overlay(n,panel,#{id:"trace-detail:"+s.id,kind:"popover",open:on,placement:"bottom",align:if m.narrow{"center"}else{"end"},gap:5.0,
        modal:false,initial_focus:"first",activate_on_trigger:false,dismiss_on_escape:true,dismiss_on_outside:true}).with_key("span:"+s.id)
        .on_open_change(Fn("disclosure").curry(m.seed,m.ticket,m.control_open,s.id))
}
fn view_trace(ctx,p){
    if p.session==""||utf8_byte_count(p.session)>96{throw "Trace waterfall requires a stable session";}
    let s=get_ui(ctx,p);let seed=defaults(p);let known=#{};
    for i in 0..p.spans.len{let t=p.spans[i];if t.name==""{throw "Trace span requires a name";}known[t.id]=i;}
    let controlled_fold=get(p,"collapsed",())!=();let controlled_open="open_id" in p;
    let folded=if controlled_fold{p.collapsed}else{s.folded};
    folded=folded.filter(|k| k in known);
    let old_open=if controlled_open{get(p,"open_id","")}else{s.open};
    let shell=element_ref("trace");let track=element_ref("time-track");
    let width=ctx.element_width_class(shell,[400,560]);let size=ctx.element_size(track);
    let track_width=if size==(){600.0}else{max(1.0,size.width)};
    let fit=min(8,max(3,(track_width/96.0).round().to_int()));
    let d=trace_waterfall_projection(p.spans,folded,now_in(ctx,p),p.min_range_ms,fit);
    let active=if s.cursor in known{known[s.cursor]}else{d.root};
    let restore=active!=()&&d.rows[active].hidden_by!=();
    if restore{active=d.rows[active].hidden_by;}
    let active=if active==(){""}else{p.spans[active].id};
    let open=if old_open in known&&d.rows[known[old_open]].hidden_by==(){old_open}else{""};
    let prune=if !controlled_open&&old_open!=""&&open==""{old_open}else{""};
    if restore||prune!=""{effect("focus",#{key:if restore{active}else{""},was:s.cursor,session:p.session,prune:prune},Fn("repair"),Fn("noop"));}
    let m=#{narrow:width!=()&&width<2,tiny:width==0,rtl:ctx.text_direction()=="rtl",data:d,active:active,open:open,seed:seed,track:track,
        clean:ctx.theme_token("design.name")=="clean",upper_labels:ctx.theme_token("trace.heading_case")=="upper",
        track_width:track_width,
        control_fold:controlled_fold,control_open:controlled_open,folded:folded,ticket:#{session:p.session,revision:p.revision}};
    let rows=[];
    if d.visible.len>0&&!m.narrow{rows.push(grid(ctx,m));}
    for i in 0..d.visible.len{rows.push(render_row(ctx,p,m,i));}
    if rows.len==0{rows.push(label(ctx,"empty",tx(ctx,"empty","No spans yet"),subtle().text_center().padding(sp(6))));}
    let viewport=node(ctx,column([column(rows).with_key("tree").with_style(style().relative().min_width(px(0)))
        .accessibility_role("tree").accessibility_label(get(p,"title",tx(ctx,"title","Trace waterfall")))]).with_key("viewport").scrollbars("hidden","auto"),
        "rows",style().max_height(px(p.max_height)).min_width(px(0)).overflow_y_scroll());
    let card=node(ctx,column([summary(ctx,p,m),axis(ctx,m),viewport]).with_key("card"),"card",
        style().min_width(px(0)).radius(theme_radius("lg")).background(ink("card")));
    node(ctx,column([header(ctx,p,m),card]).with_key(p.key).with_ref(shell),"root",
        style().width(relative(1.0)).min_width(px(0)).padding(sp(1)).radius(theme_radius("lg")).background(ink("tray")))
}
