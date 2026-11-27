/* gpui-rhai
{"id":"patterns/usage_meter","export":"UsageMeter","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["patterns/compact_breakdown","patterns/agent_tree","components/button"],"capabilities":{},"assets":[],"tokens":["accent","border","compact_breakdown.c1","compact_breakdown.c2","compact_breakdown.c3","compact_breakdown.c4","compact_breakdown.c5","focus_ring","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.metadata","usage_meter.accent","usage_meter.fill","usage_meter.full","usage_meter.full_text","usage_meter.spark","usage_meter.track","usage_meter.warn","usage_meter.warn_text"],"environment":[]}
*/
// Present accepted usage, period history and capped or unlimited plan limits.
// Example: UsageMeter(#{key:"usage",series:"account",revision:0,total:1000,used:[#{id:"api",label:"API",value:240}]})
import "patterns/compact_breakdown" as breakdown;
import "patterns/agent_tree" as ids;
import "components/button" as button;

define_component(#{
    metadata:#{id:"patterns/usage_meter","export":"UsageMeter",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/compact_breakdown","patterns/agent_tree","components/button"],capabilities:#{},
        tokens:["accent","border","compact_breakdown.c1","compact_breakdown.c2","compact_breakdown.c3","compact_breakdown.c4","compact_breakdown.c5","focus_ring","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.metadata","usage_meter.accent","usage_meter.fill","usage_meter.full","usage_meter.full_text","usage_meter.spark","usage_meter.track","usage_meter.warn","usage_meter.warn_text"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable component key."},
        series:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of the account or usage series."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision of the accepted usage snapshot."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Heading. Defaults to Usage."},
        plan:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional plan name near the reset date."},
        unit:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Unit of the main quota. Defaults to credits."},
        total:#{schema:#{type:"number",min:0.0,max:1000000000000.0},required:true,sensitive:true,doc:"Accepted finite nonnegative capacity. Usage above zero with zero capacity is over the limit."},
        used:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable category identity across periods."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Visible category label."},
            value:#{schema:#{type:"number",min:0.0,max:1000000000000.0},required:true,sensitive:true,doc:"Accepted amount from this category."},
            color:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:16777215}},required:false,sensitive:false,doc:"Custom RGB24 paint. Defaults to a stable palette slot."},
        }}},required:true,sensitive:true,doc:"At most 32 usage categories. The Host accepts a complete snapshot, never partial period previews."},
        periods:#{schema:#{type:"optional",value:#{type:"array",max_items:6,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique period ID."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible period label."},
        }}}},required:false,sensitive:false,doc:"Optional controlled period choices."},
        period:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Current accepted period; required with period choices."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Suppress repeated proposals without changing the selected period."},
        on_period_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a period proposal; the application accepts or refuses it."},
        warn_at:#{schema:#{type:"number",min:0.01,max:1.0},"default":#{type:"float",value:0.8},required:false,sensitive:false,doc:"Warn at or above this fraction of quota, below the limit."},
        reset_at_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:253402300799999}},required:false,sensitive:true,doc:"Reset instant in Unix milliseconds."},
        now_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:253402300799999}},required:false,sensitive:false,doc:"Inject an accepted clock for reproducible rendering. Otherwise the view clock refreshes hourly when a reset exists."},
        daily:#{schema:#{type:"optional",value:#{type:"array",max_items:120,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable day identity, not a display label."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Day label for hover, keyboard focus and accessibility."},
            value:#{schema:#{type:"optional",value:#{type:"number",min:0.0,max:1000000000000.0}},required:false,sensitive:true,doc:"Omit for a future day. A present zero is an observed day."},
        }}}},required:false,sensitive:true,doc:"Chronological sparkline data. No data is fetched by this component."},
        limits:#{schema:#{type:"optional",value:#{type:"array",max_items:12,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique quota key."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Quota label."},
            used:#{schema:#{type:"number",min:0.0,max:1000000000000.0},required:true,sensitive:true,doc:"Accepted usage for this limit."},
            total:#{schema:#{type:"optional",value:#{type:"number",min:0.0,max:1000000000000.0}},required:false,sensitive:true,doc:"Omit for an unlimited resource; zero is a real finite capacity."},
            unit:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional quota-specific unit."},
            decimals:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:6}},required:false,sensitive:false,doc:"Maximum displayed fraction digits. Defaults to 1 for fractional amounts."},
        }}}},required:false,sensitive:true,doc:"Additional capped or unlimited resources, in caller order."},
        upgrade_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Action label below the warning threshold."},
        top_up_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Action label near and at the limit."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a versioned upgrade or top-up request; no purchase is initiated by this component."},
    },state:#{fields:#{
        history:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        attention:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        day:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        clock:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        period_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            series:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted usage series."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted usage revision."},
            from:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Active period at proposal time."},
            to:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Requested period."},
        }},doc:"Ask the Host for a different period. Keep all prior values until the Host publishes the new snapshot."},
        action:#{payload:#{type:"object",allow_unknown:false,fields:#{
            series:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted usage series."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted usage revision."},
            period:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted period, or empty without a period selector."},
            state:#{schema:#{type:"string",allowed:["ok","warn","limit"]},required:true,sensitive:false,doc:"Condition of the main quota."},
            kind:#{schema:#{type:"string",allowed:["upgrade","top_up"]},required:true,sensitive:false,doc:"User intent; the Host decides whether and how to handle it."},
        }},doc:"An explicit upgrade or top-up request, not a purchase or quota mutation."},
    },slots:#{},parts:["root","panel","panel_cozy","head","head_narrow","title","plan","periods","periods_cozy","period","period_tight","period_active","period_fill",
        "figure","readout","total","muted","pct","pct_value","pct_warn","pct_limit","dot","dot_limit","bar","segment","segment_start","segment_cut","hint","hint_name","hint_amount","hint_unit","legend","legend_label","row","row_active","swatch",
        "amount","share","empty","daily","daily_header","spark","spark_few","spark_dense","day_fill","day_future","day_active",
        "limits","limit","limit_head","limit_value","limit_readout","limit_readout_cozy","limit_track","limit_fill","limit_warn","limit_full","limit_unlimited",
        "footer","footer_cozy","status","strong","action","action_strong"],
    effects:["history","clean_attention"]},render:Fn("render_usage")
});

fn UsageMeter(props){render_component("patterns/usage_meter",props)}
fn get(m,k,d){breakdown::get(m,k,d)}
fn sp(n){breakdown::sp(n)}
fn u(n){breakdown::unit(n)}
fn ease(){breakdown::ease()}
fn dec(n){breakdown::decimal(n)}
fn tx(ctx,k,f){ctx.t_or("usage_meter."+k,f)}
fn tx(ctx,k,f,p){ctx.t_or("usage_meter."+k,f,p)}
fn typography(ctx,s,part){breakdown::label(ctx,s,part,style().typography("caption").text_color(theme_color("text_muted")))}
fn numeric(ctx,x,d,suffix,share,key,part,group,projection,idx){
    let role=if part=="total"{"display"}else if part=="share"{"metadata"}else{"body"};
    sampled_number(ctx,x,d,suffix,share,key,part,role,group,projection,idx)
}
fn sampled_number(ctx,x,d,suffix,share,key,parts,role,group,projection,idx){
    let n=ctx.number_text(x,d,"",suffix,share).with_key(key)
        .with_style(ctx.component_style(parts,style().typography(role)
            .font_feature("tnum",1).min_width(px(0)).whitespace_nowrap().text_ellipsis()));
    if group==(){n}else{n.sample_numbers(group,projection,idx)}
}
fn quota(used,total,warn){
    if used>0.0&&used>=total{"limit"}else if total>0.0&&used/total>=warn{"warn"}else{"ok"}
}
fn percent(used,total){if total>0.0{100.0*(used/total)}else{0.0}}
fn names(ctx,state){
    if state=="limit"{tx(ctx,"limit","Limit reached")}else if state=="warn"{tx(ctx,"warn","Running low")}else{tx(ctx,"ok","Within limit")}
}
fn validate_rows(rows){
    let seen=#{};
    for r in rows{
        if !ids::valid(r.id)||r.id in seen||r.label==""||utf8_byte_count(r.label)>240{throw "UsageMeter needs unique IDs and bounded, nonempty labels";}
        seen[r.id]=true;
    }
}
fn tick(ctx,arg){ctx.set_state("clock",ctx.now());}
fn resets(ctx,p){
    let at=get(p,"reset_at_ms",());if at==(){return "";}
    let now=get(p,"now_ms",());
    if now==(){
        interval("reset-clock",3600000,false,Fn("tick"),());
        now=ctx.get_state("clock");if now==(){now=ctx.now();}
    }
    let delta=max(0,at-now);let days=(delta+86399999)/86400000;
    if days==0{return tx(ctx,"today","Resets today");}
    if days==1{return tx(ctx,"tomorrow","Resets tomorrow");}
    if days>60{
        let date=time_parts(at,"UTC").date;
        try{return tx(ctx,"reset_date","Resets {date}",#{date:ctx.format_date(date,"medium")});}
        catch{return tx(ctx,"reset_date","Resets {date}",#{date:date});}
    }
    tx(ctx,"reset_days","Resets in {n} days",#{n:ctx.format_number(days)})
}
fn color(e,slot){breakdown::tone(e,slot)}
fn swatch(ctx,e){
    box([]).accessibility_hidden(true).with_style(ctx.component_style("swatch",style().width(px(8)).height(px(8)).radius(px(2)).flex_shrink(false)
        .background(color(e,e.slot))))
}
fn dot(ctx,state){
    box([]).accessibility_hidden(true).with_style(ctx.component_style(if state=="limit"{["dot","dot_limit"]}else{["dot"]},
        style().width(px(6)).height(px(6)).radius(theme_radius("pill")).flex_shrink(false).background(theme_color("usage_meter.warn"))))
}
fn caption(ctx,e,unit){
    tx(ctx,"part_label","{name}, {amount} {unit}, {share}",#{name:e.label,amount:ctx.format_number(e.value,#{max_fraction_digits:1}),unit:unit,share:breakdown::format_percent(ctx,e.share)})
}
fn pick_category(series,id,ctx,event){ctx.focus("usage-track");breakdown::attend(series,"focus",id,ctx,true);handled()}
fn clear_category(ctx,p){
    let a=breakdown::attention(ctx,p.series);a.focus="";ctx.set_state("attention",a);handled()
}
fn jump_category(series,ids,delta,ctx,event){
    let a=breakdown::attention(ctx,series);let i=ids.index_of(a.focus);
    let next=if i<0{if delta<0{ids.len-1}else{0}}else{(i+ids.len+delta)%ids.len};
    pick_category(series,ids[next],ctx,());
}
fn blur_category(series,ctx,inside){if !inside{clear_category(ctx,#{series:series});}}
fn usage_bar(ctx,p,model,active,refs,group){
    let slots=[];let ids=[];let starts=true;
    let base=max(p.capacity,model.total);
    for i in 0..model.history.ids.len{
        let slot=model.history.ids[i];let e=get(model.items,slot.id,());let value=if e==(){0.0}else{e.value};
        let width=if base==0.0{0.0}else{value/base};
        let parts=if starts{["segment","segment_start"]}else{["segment"]};
        let ink=box([]).with_key("fill").with_style(ctx.component_style(parts,style().height(relative(1.0)).absolute()
            .inset_start(px(0)).inset_end(px(0)).top(px(0)).background(color(e,slot.slot))));
        let slit=box([]).with_key("slit").with_style(ctx.component_style("segment_cut",style()
            .absolute().inset_end(px(0)).top(px(0)).height(relative(1.0)).width(u(0.5)).background(theme_color("surface_raised"))));
        if value>0.0{starts=false;}
        let n=column([ink,slit]).with_key(breakdown::item_key(p.series,slot.id)).with_ref(refs[i]).number_motion_source().accessibility_hidden(true)
            .with_style(style().relative().height(relative(1.0)).width(relative(width)).min_width(px(0))
                .flex_shrink(false).overflow_hidden().isolated_opacity(true)
                .opacity(if active==""||active==slot.id{1.0}else{0.28}))
            .paint_transition(200,"ease_out",#{group_effects:true})
            .motion(motion_transition("number_value",0.0,value,#{duration_ms:650,easing:ease()})).sample_numbers(group,"width_fraction",i);
        if e!=()&&value>0.0{
            n=n.on_hover_change(Fn("hover_category").curry(p.series,"bar",e.id)).on_click(Fn("pick_category").curry(p.series,e.id));
            ids.push(e.id);
        }
        slots.push(n);
    }
    let n=row(slots).with_key("usage-track").with_ref(element_ref("usage-track")).accessibility_role("progressbar").accessibility_label(tx(ctx,"used","Credits used"))
        .accessibility_value(min(p.capacity,model.total)).accessibility_value_min(0.0).accessibility_value_max(p.capacity)
        .accessibility_description(tx(ctx,"used_of","{used} of {capacity} {unit} used",#{used:ctx.format_number(model.total),capacity:ctx.format_number(p.capacity),unit:p.unit}))
        .tab_stop(ids.len>0)
        .with_style(ctx.component_style("bar",style().height(px(10)).width(relative(1.0)).min_width(px(0))
            .radius(px(5)).overflow_hidden().background(theme_color("usage_meter.track"))
            .focus_ring(px(2),offset_px(-1),theme_color("focus_ring"))));
    if ids.len>0{
        n=n.on_key_value("plain+left",Fn("jump_category").curry(p.series,ids,-1),())
            .on_key_value("plain+right",Fn("jump_category").curry(p.series,ids,1),())
            .on_key_value("plain+escape",Fn("clear_category"),#{series:p.series})
            .on_focus_within_change(Fn("blur_category").curry(p.series));
    }
    n
}
fn hover_category(series,source,id,ctx,inside){breakdown::attend(series,source,id,ctx,inside);}
fn repair_attention(ctx,p){breakdown::clean_attention(ctx,p);}
fn category_hint(ctx,p,model,e,group){
    let base=max(p.capacity,model.total);
    let anchor=0.5;
    if e.value>0.0&&base>0.0{
        let leading=0.0;
        for i in 0..e.rank{
            let previous=get(model.items,model.history.ids[i].id,());
            if previous!=(){leading+=previous.value;}
        }
        anchor=min(1.0,max(0.0,(leading+e.value*0.5)/base));
    }
    let name=row([swatch(ctx,e),typography(ctx,e.label,"hint_name")]).with_key("hint-name")
        .with_style(style().items_center().min_width(px(0)).gap_x(sp(1.5)));
    let value=row([numeric(ctx,e.value,1,"",false,"hint-amount","hint_amount",group,"amount",e.rank),
        typography(ctx,p.unit,"hint_unit")]).with_key("hint-value")
        .with_style(style().items_baseline().min_width(px(0)).gap_x(sp(1)));
    let tip=column([name,value]).with_key("category-detail").accessibility_role("status").accessibility_label(caption(ctx,e,p.unit))
        .with_style(ctx.component_style("hint",style().min_width(px(0)).max_width(relative(1.0)).gap_y(u(0.25)).padding_x(sp(2.5)).padding_y(sp(1.5))
            .background(theme_color("surface_hover")).radius(theme_radius("sm"))));
    box([tip]).with_key("category-anchor")
        .with_style(style().flex_col().items_start().width(relative(1.0)).min_width(px(0)).absolute()
            .bottom(relative(1.0)).inset_start(px(0)).margin_bottom(u(1.5)))
        .sample_strip_hint(group,e.rank,anchor)
}
fn legend(ctx,p,model,active,narrow,group){
    let rows=[];
    for source in p.used{
        let e=model.items[source.id];
        let hot=active==e.id;let parts=if hot{["row","row_active"]}else{["row"]};
        let amount=numeric(ctx,e.value,1,"",false,"amount","amount",group,"amount",e.rank);
        let items=[swatch(ctx,e),breakdown::label(ctx,e.label,"legend_label",style().typography("body").flex_grow()).with_key("label"),amount];
        if !narrow{
            items.push(numeric(ctx,e.share,0,"%",true,"share","share",group,"percent",e.rank).with_style(style().width(u(10)).flex_shrink(false).text_end()));
        }
        rows.push(row(items).with_key("row:"+e.id).accessibility_role("listitem").accessibility_label(caption(ctx,e,p.unit))
            .tab_stop(true).on_hover_change(Fn("hover_category").curry(p.series,"row",e.id))
            .on_focus_within_change(Fn("hover_category").curry(p.series,"focus",e.id))
            .with_style(ctx.component_style(parts,style().items_center().min_height(u(7.5)).padding_x(sp(2)).min_width(px(0)).gap(sp(2.5))
                .opacity(if active==""||hot{1.0}else{0.45}).radius(theme_radius("sm"))
                .border(px(1)).border_color(rgba(0x00000000)).focus(style().border_color(theme_color("focus_ring")))))
            .paint_transition(160,"ease_out").layout_motion(420,ease()));
    }
    if rows.len==0{rows.push(typography(ctx,tx(ctx,"empty","No usage in this period"),"empty"));}
    column(rows).with_key("legend").accessibility_role("list").accessibility_label(tx(ctx,"by_category","By category"))
        .with_style(ctx.component_style("legend",style().margin_top(sp(2.5)).min_width(px(0))))
}
fn set_day(token,id,ctx,event){
    let next=#{token:token,id:id};
    if ctx.get_state("day")!=next{ctx.set_state("day",next);}
    handled()
}
fn pick_day(token,id,ctx,event){ctx.focus("daily-track");set_day(token,id,ctx,event)}
fn hover_day(token,id,ctx,inside){
    if inside{set_day(token,id,ctx,());}
    else {
        let d=ctx.get_state("day");
        if d!=()&&d.token==token&&d.id==id{set_day(token,"",ctx,());}
    }
}
fn move_day(token,ids,delta,ctx,event){
    let d=ctx.get_state("day");let from=if d==()||d.token!=token{-1}else{ids.index_of(d.id)};
    if delta==0{return set_day(token,"",ctx,());}
    let i=if from<0{if delta>0{0}else{ids.len-1}}else{max(0,min(ids.len-1,from+delta))};
    set_day(token,ids[i],ctx,())
}
fn blur_day(token,ctx,inside){if !inside{set_day(token,"",ctx,());}}
fn daily(ctx,p,days){
    if days.len==0{return ();}
    let token=#{series:p.series,period:get(p,"period",""),revision:p.revision};let d=ctx.get_state("day");
    let active=if d!=()&&d.token==token{d.id}else{""};
    let axes=[];let tips=[];let selected=();let top=1.0;let sum=0.0;let n=0;
    for day in days{
        let v=get(day,"value",());
        if v!=(){let value=dec(v);top=max(top,value);sum+=value;n+=1;}
        if day.id==active{selected=day;}
    }
    let mean=if n==0{0.0}else{sum/n.to_float()};
    for day in days{
        tips.push(day.id);
        let v=get(day,"value",());let future=v==();let hot=day.id==active;
        let parts=["day_fill"];if future{parts.push("day_future");}if hot{parts.push("day_active");}
        let bar=style().width(relative(1.0)).radius(px(2)).background(theme_color(if future{"usage_meter.track"}else{"usage_meter.spark"}))
            .opacity(if active==""||hot{1.0}else{0.60});
        if !future{bar=bar.height(relative(1.0)).transform(#{origin_y:1.0,scale_y:max(0.04,dec(v)/top)});}
        else {bar=bar.height(px(2));}
        let fill=box([]).with_key("fill").accessibility_hidden(true)
            .with_style(ctx.component_style(parts,bar)).paint_transition(160,"ease_out");
        if !future{
            fill=fill.motion(motion_transition("scale_y",0.0,max(0.04,dec(v)/top),#{duration_ms:500,easing:ease()}));
        }
        let read=if future{tx(ctx,"future","Not yet recorded")}else{ctx.format_number(dec(v))+" "+p.unit};
        axes.push(column([fill]).with_key("day:"+day.id).accessibility_role("listitem").accessibility_label(day.label+" · "+read)
            .on_hover_change(Fn("hover_day").curry(token,day.id)).on_click(Fn("pick_day").curry(token,day.id))
            .with_style(style().flex_grow().flex_basis(px(0)).min_width(px(0)).height(relative(1.0)).justify_end()));
    }
    let read=if selected==(){
        if n==0{tx(ctx,"no_daily","No recorded days")}else{tx(ctx,"avg","Avg {n} / day",#{n:ctx.format_number(mean,#{max_fraction_digits:0})})}
    }else{
        let v=get(selected,"value",());
        selected.label+" · "+(if v==(){tx(ctx,"future","Not yet recorded")}else{ctx.format_number(dec(v))+" "+p.unit})
    };
    let rtl=if ctx.text_direction()=="rtl"{-1}else{1};
    let chart=row(axes).with_key("days").with_ref(element_ref("daily-track")).tab_stop(true).accessibility_role("list").accessibility_label(tx(ctx,"per_day","Per day"))
        .accessibility_description(tx(ctx,"daily_keys","Use arrow keys to inspect a day."))
        .on_key_value("plain+left",Fn("move_day").curry(token,tips,-rtl),()).on_key_value("plain+right",Fn("move_day").curry(token,tips,rtl),())
        .on_key_value("plain+home",Fn("set_day").curry(token,tips[0]),()).on_key_value("plain+end",Fn("set_day").curry(token,tips[tips.len-1]),())
        .on_key_value("plain+escape",Fn("set_day").curry(token,""),()).on_focus_within_change(Fn("blur_day").curry(token))
        .with_style(ctx.component_style(if days.len<=14{["spark","spark_few"]}else if days.len>45{["spark","spark_dense"]}else{["spark"]},
            style().height(u(8.5)).width(relative(1.0)).min_width(px(0)).gap(sp(0.5)).items_end()
                .border(px(1)).border_color(rgba(0x00000000)).focus(style().border_color(theme_color("focus_ring")))));
    column([
        row([typography(ctx,tx(ctx,"per_day","Per day"),"muted"),
            typography(ctx,read,"muted").with_key("readout").accessibility_role("status")])
            .with_key("daily-head").with_style(ctx.component_style("daily_header",style().min_width(px(0)).gap(sp(3)).justify_between())),
        chart,
    ]).with_key("daily").with_style(ctx.component_style("daily",style().margin_top(sp(3)).gap(sp(2)).min_width(px(0))))
}
fn plan_limits(ctx,p,limits,cols,cozy){
    if limits.len==0{return ();}
    let cells=[];
    for i in 0..limits.len{
        let l=limits[i];let used=dec(l.used);let total=get(l,"total",());
        let unit=get(l,"unit","");let unlimited=total==();if !unlimited{total=dec(total);}
        let state=if unlimited{"ok"}else{quota(used,total,p.warn_at)};
        let digits=get(l,"decimals",if used%1.0!=0.0||(!unlimited&&total%1.0!=0.0){1}else{0});
        let ref=element_ref("limit:"+i.to_string());let group=motion_numbers([ref]);
        if !unlimited{group=group.with_capacity(total);}
        let parts=if cozy{["limit_readout","limit_readout_cozy"]}else{["limit_readout"]};
        let shown=sampled_number(ctx,used,digits,"",false,"limit-used",["amount"]+parts,"body",group,"amount",0);
        let of=if unlimited{tx(ctx,"unlimited","Unlimited")}else{
            tx(ctx,"of","of {total}",#{total:ctx.format_number(total,#{max_fraction_digits:digits})+(if unit==""{""}else{" "+unit})})
        };
        let denominator=breakdown::label(ctx,of,["muted"]+parts,
            style().typography("body").font_weight(400).text_color(theme_color("text_muted")));
        let value=row([shown,denominator]).with_key("limit-read")
            .with_style(ctx.component_style("limit_value",style().min_width(px(0)).gap(sp(1)).items_center().flex_wrap()));
        let f=if unlimited{0.0}else if total==0.0{if used>0.0{1.0}else{0.0}}else{min(1.0,used/total)};
        let parts=["limit_fill"];if state=="warn"{parts.push("limit_warn");}if state=="limit"{parts.push("limit_full");}
        let fill=box([]).with_key("limit-value-source").with_ref(ref).number_motion_source().accessibility_hidden(true)
            .motion(motion_transition("number_value",0.0,used,#{duration_ms:650,easing:ease()}))
            .with_style(ctx.component_style(parts,style().height(relative(1.0)).width(relative(f)).min_width(px(0))
                .background(theme_color("usage_meter.accent"))));
        if !unlimited{fill=fill.sample_numbers(group,"width_fraction",0);}
        let track_nodes=[fill];
        if unlimited{track_nodes.push(stripes());}
        let track=row(track_nodes).with_key("limit-track").with_style(ctx.component_style(if unlimited{["limit_track","limit_unlimited"]}else{["limit_track"]},
            style().width(relative(1.0)).height(px(4)).radius(px(2)).overflow_hidden().background(theme_color("usage_meter.track"))));
        if !unlimited{
            track=track.accessibility_role("progressbar").accessibility_label(l.label)
                .accessibility_value_min(0.0).accessibility_value_max(total).accessibility_value(min(used,total))
                .accessibility_description(names(ctx,state));
        }
        let heading=[typography(ctx,l.label,"muted")];if state!="ok"{heading.push(dot(ctx,state));}
        cells.push(column([row(heading).with_key("limit-head").with_style(ctx.component_style("limit_head",style().min_width(px(0)).items_center().gap(sp(1.5)))),
            value,track]).with_key("limit:"+l.id).accessibility_role("listitem").accessibility_label(l.label)
            .accessibility_description(of+" · "+names(ctx,state))
            .with_style(ctx.component_style("limit",style().min_width(px(0)).height(relative(1.0)))));
    }
    let rows=[];let i=0;
    while i<cells.len{
        let line=[];
        for j in 0..cols{
            if i+j<cells.len{line.push(cells[i+j].with_style(style().flex_grow().flex_basis(px(0))));}
            else{line.push(box([]).accessibility_hidden(true).with_style(style().flex_grow().flex_basis(px(0))));}
        }
        rows.push(row(line).with_key("limit-line:"+i.to_string()).with_style(style().min_width(px(0)).gap(sp(4.5))));
        i+=cols;
    }
    column(rows).with_key("limits").accessibility_role("list").accessibility_label(tx(ctx,"plan_limits","Plan limits"))
        .with_style(ctx.component_style("limits",style().margin_top(sp(4)).padding_top(sp(3.5)).gap(sp(3)).min_width(px(0))
            .border_top(px(1)).border_color(theme_color("border"))))
}
fn stripes(){
    svg("<svg viewBox='0 0 144 4' preserveAspectRatio='none'><defs><pattern id='quota-stripes' width='6' height='4' patternUnits='userSpaceOnUse'><path d='M-2 4 2 0M4 4 8 0' fill='none' stroke='currentColor' stroke-width='2.5'/></pattern></defs><rect width='144' height='4' fill='url(#quota-stripes)'/></svg>")
        .with_key("unlimited-track-mark").accessibility_hidden(true).with_style(style().width(relative(1.0)).height(relative(1.0))
            .text_color(theme_color("usage_meter.track")))
}
fn action(ticket,ctx,event){ctx.emit("action",ticket);handled()}
fn footer(ctx,p,model,group,state,cozy){
    let status=[];
    if state!="ok"{
        status.push(dot(ctx,state));
        status.push(typography(ctx,names(ctx,state),"strong"));
    }
    if state!="limit"{
        let fallback=max(0.0,p.capacity-model.total);
        status.push(numeric(ctx,fallback,0," "+p.unit+" "+tx(ctx,"left","left"),false,"remaining","muted",group,"remaining",0));
    }else if model.total>p.capacity{
        let over=ctx.format_number(model.total-p.capacity);
        status.push(typography(ctx,tx(ctx,"over","{n} over",#{n:over}),"muted"));
    }
    let body=[row(status).with_key("status").accessibility_role("status")
        .accessibility_label(names(ctx,state))
        .with_style(ctx.component_style("status",style().min_width(px(0)).gap(sp(1.5)).items_center()))];
    if get(p,"on_action",())!=(){
        let label=if state=="ok"{get(p,"upgrade_label",tx(ctx,"upgrade","Upgrade"))}else{get(p,"top_up_label",tx(ctx,"top_up","Add credits"))};
        let ticket=#{series:p.series,revision:p.revision,period:get(p,"period",""),state:state,kind:if state=="ok"{"upgrade"}else{"top_up"}};
        body.push(button::Button(#{key:"usage-action",text:label,variant:if state=="ok"{"secondary"}else{"primary"},size:"sm",
            disabled:p.busy,on_click:Fn("action").curry(ticket)})
            .with_style(ctx.component_style(if state=="ok"{["action"]}else{["action","action_strong"]},style().flex_shrink(false).max_width(relative(1.0)))));
    }
    row(body).with_key("footer").with_style(ctx.component_style(if cozy{["footer","footer_cozy"]}else{["footer"]},style()
        .items_center().justify_between().flex_wrap().min_width(px(0)).gap(sp(2)).min_height(u(11))))
}
fn render_usage(ctx,p){
    let days=get(p,"daily",[]);let limits=get(p,"limits",[]);validate_rows(days);validate_rows(limits);
    p.unit=get(p,"unit",tx(ctx,"credits","credits"));if utf8_byte_count(p.unit)>48{throw "Usage unit is limited to 48 bytes";}
    p.capacity=dec(p.total);p.warn_at=dec(p.warn_at);
    let model=breakdown::snapshots(ctx,#{series:p.series,items:p.used});
    let active=breakdown::inspect(ctx,p,model);
    effect("clean_attention",#{series:p.series,ids:model.items.keys()},Fn("repair_attention"),Fn("noop"));
    let root=element_ref("usage");
    let wc=ctx.element_width_class(root,[301,401]);let narrow=wc==0;let cozy=wc!=2;
    let state=quota(model.total,p.capacity,p.warn_at);
    let refs=[];for slot in model.history.ids{refs.push(element_ref(slot.ref));}
    let group=if refs.len==0{()}else{motion_numbers(refs).with_capacity(p.capacity)};
    let title=get(p,"title",tx(ctx,"title","Usage"));let plan=get(p,"plan","");let reset=resets(ctx,p);
    let header=[breakdown::label(ctx,title,"title",style().typography("body").font_weight(600)).with_key("title").accessibility_role("heading").accessibility_level(2)];
    if plan!=""||reset!=""{header.push(typography(ctx,plan+(if plan!=""&&reset!=""{" · "}else{""})+reset,"plan"));}
    let headings=[column(header).with_key("header-title").with_style(style().min_width(px(0)))];
    let switcher=breakdown::switch_period(ctx,p,narrow,cozy,root.semantic_id);if switcher!=(){headings.push(switcher);}
    let head=if narrow&&switcher!=(){column(headings)}else{row(headings)};
    let panel=[head.with_key("head").with_style(ctx.component_style(if narrow{["head","head_narrow"]}else{["head"]},
        style().items_start().justify_between().min_width(px(0)).gap(sp(3))))];
    let pct=[ ];if state!="ok"{pct.push(dot(ctx,state));}
    if p.capacity==0.0&&model.total>0.0{
        pct.push(breakdown::label(ctx,tx(ctx,"over_quota","Over quota"),"pct_value",style().typography("body")));
    }
    else{
        pct.push(numeric(ctx,percent(model.total,p.capacity),0,"%",false,"capacity-percent","pct_value",group,"capacity_percent",0));
    }
    panel.push(row([
        row([numeric(ctx,model.total,0,"",false,"used","total",group,"sum",0),
            typography(ctx,tx(ctx,"of","of {total}",#{total:ctx.format_number(p.capacity)+" "+p.unit}),"muted")])
            .with_style(ctx.component_style("readout",style().min_width(px(0)).gap_x(sp(1.5)).flex_wrap().items_baseline())),
        row(pct).with_key("quota-pct").with_style(ctx.component_style(if state=="ok"{["pct"]}else{["pct","pct_"+state]},style().items_center().gap(sp(1.5)).flex_shrink(false))),
    ]).with_key("figure").with_style(ctx.component_style("figure",style().items_baseline().justify_between().flex_wrap().gap(sp(3)).min_width(px(0)))));
    let distribution=[usage_bar(ctx,p,model,active,refs,group)];
    if active!=""{distribution.push(category_hint(ctx,p,model,model.items[active],group));}
    panel.push(column(distribution).with_key("distribution").with_style(style().relative().margin_top(sp(2.5)).min_width(px(0))));
    panel.push(legend(ctx,p,model,active,narrow,group));
    let spark=daily(ctx,p,days);if spark!=(){panel.push(spark);}
    let limit=plan_limits(ctx,p,limits,if narrow{1}else if cozy{min(2,limits.len)}else{min(3,limits.len)},cozy);
    if limit!=(){panel.push(limit);}
    column([
        column(panel).with_key("panel").with_style(ctx.component_style(if cozy{["panel","panel_cozy"]}else{["panel"]},
            style().width(relative(1.0)).min_width(px(0)).padding(sp(4)).radius(theme_radius("lg")).background(theme_color("surface_raised")))),
        footer(ctx,p,model,group,state,cozy),
    ]).with_key(p.key).with_ref(root).accessibility_role("group").accessibility_label(title)
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(460)).min_width(px(0)).overflow_hidden()))
}
fn noop(ctx,arg){}
