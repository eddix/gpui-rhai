/* gpui-rhai
{"id":"patterns/compact_breakdown","export":"CompactBreakdown","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["patterns/agent_tree"],"capabilities":{},"assets":[],"tokens":["accent","border","compact_breakdown.c1","compact_breakdown.c2","compact_breakdown.c3","compact_breakdown.c4","compact_breakdown.c5","compact_breakdown.card","compact_breakdown.up","compact_breakdown.up_bg","compact_breakdown.down","compact_breakdown.down_bg","design.name","danger","focus_ring","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","success","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.metadata"],"environment":[]}
*/
// A total split into stable parts, with ranked detail and a controlled period.
// Example: CompactBreakdown(#{key:"cost",series:"usage",revision:0,items:[#{id:"api",label:"API calls",value:84.0}]})
import "patterns/agent_tree" as ids;

define_component(#{
    metadata:#{id:"patterns/compact_breakdown","export":"CompactBreakdown",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_tree"],capabilities:#{},
        tokens:["accent","border","compact_breakdown.c1","compact_breakdown.c2","compact_breakdown.c3","compact_breakdown.c4","compact_breakdown.c5","compact_breakdown.card","compact_breakdown.up","compact_breakdown.up_bg","compact_breakdown.down","compact_breakdown.down_bg","design.name","danger","focus_ring","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","success","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.metadata"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable component key."},
        series:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable category namespace. Change it when starting an unrelated breakdown."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision of the accepted snapshot."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Heading. Defaults to Breakdown."},
        total_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Caption when no category is being inspected."},
        empty_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Description when no categories are available."},
        items:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique category ID stable across accepted periods."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Human-readable category label."},
            value:#{schema:#{type:"number",min:0.0,max:1000000000000.0},required:true,sensitive:true,doc:"Finite nonnegative amount."},
            color:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:16777215}},required:false,sensitive:false,doc:"Optional RGB24 override for this category."},
        }}},required:true,sensitive:true,doc:"Accepted categories, not the result of a pending period proposal."},
        periods:#{schema:#{type:"optional",value:#{type:"array",max_items:6,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique period ID."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible period label."},
        }}}},required:false,sensitive:false,doc:"Optional period switch choices."},
        period:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Host-accepted period, required when choices are provided."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Ignore new period proposals while preserving keyboard focus."},
        on_period_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Handle a period proposal without mutating the accepted snapshot in the component."},
        delta:#{schema:#{type:"optional",value:#{type:"number",min:-1000000.0,max:1000000.0}},required:false,sensitive:true,doc:"Signed percent change measured against the previous period."},
        prefix:#{schema:#{type:"string"},"default":#{type:"string",value:""},required:false,sensitive:false,doc:"Optional number prefix, e.g. a currency symbol; at most 64 UTF-8 bytes."},
        suffix:#{schema:#{type:"string"},"default":#{type:"string",value:""},required:false,sensitive:false,doc:"Optional unit suffix; at most 64 UTF-8 bytes."},
        decimals:#{schema:#{type:"integer",min:0,max:6},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Maximum fraction digits for amounts."},
        show_footer:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Show the largest category or the supplied footer."},
        footer:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional custom footer in place of the largest category."},
    },state:#{fields:#{
        history:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        attention:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{period_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
        series:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted series."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision."},
        from:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted period at the time of the request."},
        to:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Requested period."},
    }},doc:"A proposal; accept by passing new period and data, or keep the original snapshot."}},slots:#{},
    parts:["root","panel","panel_cozy","head","head_narrow","title","periods","periods_cozy","period","period_tight","period_active","period_fill",
        "figure","total","delta","delta_up","delta_down","caption","bar","segment","segment_start","segment_end","list","row","row_active","dot","name","amount","share",
        "empty","footer","footer_cozy","footer_text"],effects:["history","clean_attention"]},render:Fn("render_breakdown")
});

fn CompactBreakdown(props){render_component("patterns/compact_breakdown",props)}
fn get(m,k,d){if k in m && m[k]!=(){m[k]}else{d}}
fn decimal(n){if type_of(n)=="i64"{n.to_float()}else{n}}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn unit(n){theme_length("space.unit").times(n)}
fn px_ref(key,id){key+":"+id}
fn item_key(series,id){series.len.to_string()+":"+series+":"+id}
fn order(a,b){if a.value==b.value{a.rank-b.rank}else if a.value>b.value{-1}else{1}}
fn share_of(amount,total){if total>0.0{100.0*amount/total}else{0.0}}
fn label(ctx,s,p,base){text(s).with_style(ctx.component_style(p,base.min_width(px(0)).whitespace_nowrap().text_ellipsis()))}
fn amount(ctx,x,props,part,key,lanes,projection,index){
    let base=style().min_width(px(0)).typography(if part=="total"{"display"}else{"body"}).text_color(theme_color("text_primary"))
        .font_feature("tnum",1).whitespace_nowrap().text_ellipsis();
    if part=="amount"{base=base.flex_shrink(false).max_width(relative(0.70));}
    let node=ctx.number_text(x,props.decimals,props.prefix,props.suffix,false).with_key(key)
        .with_style(ctx.component_style(part,base));
    if lanes==(){node}else{node.sample_numbers(lanes,projection,index)}
}
fn ratio(ctx,pct,key,lanes,index){
    ctx.number_text(pct,0,"","%",true).with_key(key)
        .with_style(ctx.component_style("share",style().typography("metadata").font_feature("tnum",1).width(unit(8.5))
            .flex_shrink(false).text_end().text_color(theme_color("text_muted")).whitespace_nowrap()))
        .sample_numbers(lanes,"percent",index)
}
fn format_percent(ctx,pct){
    if pct>0.0&&pct<1.0{return "<"+ctx.format_number(1)+"%";}
    if pct>99.0&&pct<100.0{return ">"+ctx.format_number(99)+"%";}
    ctx.format_number(pct,#{max_fraction_digits:0})+"%"
}
fn snapshots(ctx,props){
    if !ids::valid(props.series){throw "CompactBreakdown requires a stable series ID";}
    let by_id=#{};let total=0.0;
    for entry in props.items{
        if !ids::valid(entry.id)||entry.id in by_id{throw "Breakdown categories require distinct valid IDs";}
        if entry.label==""||utf8_byte_count(entry.label)>512{throw "Breakdown labels must be nonempty and at most 512 bytes";}
        let e=entry;e.value=decimal(e.value);by_id[e.id]=e;total+=e.value;
    }
    let h=ctx.get_state("history");
    if h==()||h.series!=props.series||!("next_ref" in h){h=#{series:props.series,ids:[],next:0,next_ref:0};}
    let known=#{};
    for k in h.ids{known[k.id]=true;}
    for e in props.items{
        if !(e.id in known){
            h.ids.push(#{id:e.id,slot:h.next,ref:"part:"+h.next_ref.to_string()});
            h.next_ref+=1;h.next=min(4,h.next+1);
        }
    }
    // Live slots are never dropped. Keep only the most recent bounded window
    // of inactive slots to allow ordinary period changes to glide from zero.
    while h.ids.len>64{
        let drop=0;while h.ids[drop].id in by_id{drop+=1;}h.ids.remove(drop);
    }
    effect("history",h,Fn("remember"),Fn("noop"));
    let ranked=[];
    for i in 0..h.ids.len{
        let slot=h.ids[i];if !(slot.id in by_id){continue;}
        let e=by_id[slot.id];
        e.rank=i;e.share=share_of(e.value,total);e.slot=slot.slot;by_id[e.id]=e;ranked.push(e);
    }
    ranked.sort(Fn("order"));
    #{history:h,ranked:ranked,items:by_id,total:total}
}
fn noop(ctx,p){}
fn remember(ctx,p){if ctx.get_state("history")!=p{ctx.set_state("history",p);}}
fn attention(ctx,s){
    let a=ctx.get_state("attention");if a!=()&&a.series==s{return a;}
    #{series:s,row:"",bar:"",focus:""}
}
fn attend(series,field,id,ctx,inside){
    let a=attention(ctx,series);let old=a[field];
    let next=if inside{id}else if old==id{""}else{old};
    if old!=next{a[field]=next;ctx.set_state("attention",a);}
}
fn clean_attention(ctx,p){
    let a=attention(ctx,p.series);let changed=false;
    for field in ["row","bar","focus"]{if a[field]!=""&&!(a[field] in p.ids){a[field]="";changed=true;}}
    if changed{ctx.set_state("attention",a);}
}
fn inspect(ctx,props,data){
    let a=attention(ctx,props.series);
    for k in [a.bar,a.row,a.focus]{if k in data.items{return k;}}
    ""
}
fn tone(e,slot){
    if e!=()&&get(e,"color",())!=(){return rgb(e.color);}
    switch slot{
        0=>theme_color("compact_breakdown.c1"),1=>theme_color("compact_breakdown.c2"),
        2=>theme_color("compact_breakdown.c3"),3=>theme_color("compact_breakdown.c4"),_=>theme_color("compact_breakdown.c5"),
    }
}
fn propose(p,ctx,event){ctx.emit("period_change",p);handled()}
fn move_period(p,ref,busy,ctx,event){ctx.focus(ref);if !busy && p.from!=p.to{ctx.emit("period_change",p);}handled()}
fn request(props,to){#{series:props.series,revision:props.revision,from:props.period,to:to}}
fn switch_period(ctx,props,narrow,cozy,scope){
    let periods=get(props,"periods",[]);let set=#{};
    for i in 0..periods.len{
        let p=periods[i];
        if !ids::valid(p.id)||p.id in set||p.label==""||utf8_byte_count(p.label)>128{throw "Period choices need distinct valid IDs and labels";}
        set[p.id]=i;
    }
    if periods.len==0{
        if get(props,"period",())!=(){throw "The accepted period must belong to the period choices";}
        return ();
    }
    if !(get(props,"period","") in set){throw "The accepted period must belong to the period choices";}
    let out=[];let enabled=get(props,"on_period_change",())!=();let rtl=ctx.text_direction()=="rtl";
    for i in 0..periods.len{
        let p=periods[i];let selected=p.id==props.period;
        let parts=["period"];if narrow{parts.push("period_tight");}if selected{parts.push("period_active");}
        let content=[];
        if selected{
            content.push(column([]).with_key("selected-fill").accessibility_hidden(true)
                .with_style(ctx.component_style("period_fill",style().absolute().left(px(1)).right(px(1)).top(px(1)).bottom(px(1))
                    .radius(px(5)).background(theme_color("surface_raised"))))
                .shared_layout(scope,"period").layout_motion(360,ease()));
        }
        content.push(text(p.label).with_style(style().min_width(px(0)).whitespace_nowrap().text_ellipsis()));
        let n=row(content)
            .with_key(px_ref("period",p.id)).with_ref(element_ref(px_ref("period",i.to_string())))
            .accessibility_role("radio").accessibility_label(p.label).accessibility_checked(selected)
            .tab_stop(enabled&&selected)
            .with_style(ctx.component_style(parts,style().items_center().justify_center().min_width(px(0))
                .min_height(unit(6.5)).padding_x(sp(if narrow{1.5}else{2.0})).typography("caption").cursor_pointer()
                .border(px(1)).border_color(rgba(0x00000000)).focus(style().border_color(theme_color("focus_ring")))
                .text_color(theme_color(if selected{"text_primary"}else{"text_muted"}))))
            .paint_transition(150,"ease_out");
        if enabled{
            if !props.busy&&!selected{n=n.on_click(Fn("propose").curry(request(props,p.id)))
                .on_key_value("plain+enter",Fn("propose").curry(request(props,p.id)),())
                .on_key_value("plain+space",Fn("propose").curry(request(props,p.id)),());}
            let next=periods[(i+1)%periods.len].id;let prev=periods[(i+periods.len-1)%periods.len].id;
            for dest in [["plain+left",if rtl{next}else{prev}],["plain+right",if rtl{prev}else{next}],["plain+home",periods[0].id],["plain+end",periods[periods.len-1].id]]{
                n=n.on_key_value(dest[0],Fn("move_period").curry(request(props,dest[1]),px_ref("period",set[dest[1]].to_string()),props.busy),());
            }
        }
        out.push(n);
    }
    row(out).with_key("periods").accessibility_role("radiogroup").accessibility_label(ctx.t_or("compact_breakdown.period","Period"))
        .with_style(ctx.component_style(if cozy{["periods","periods_cozy"]}else{["periods"]},style().min_width(px(0)).max_width(relative(1.0))))
}
fn color_bar(ctx,props,model,active,lanes,refs){
    let bars=[];let cut=0.0005;
    let remaining=0;for id in model.items.keys(){if model.items[id].share/100.0>cut{remaining+=1;}}
    let first=true;
    for i in 0..model.history.ids.len{
        let slot=model.history.ids[i];
        let e=get(model.items,slot.id,());let on=active==slot.id;
        let f=if e==(){0.0}else{e.share/100.0};
        let parts=["segment"];
        if f>cut{
            remaining-=1;
            if first{parts.push("segment_start");first=false;}
            if remaining==0{parts.push("segment_end");}
        }
        let ink=column([]).with_key("fill").with_style(ctx.component_style(parts,style().background(tone(e,slot.slot))
            .width(relative(1.0)).height(relative(1.0)).opacity(if active==""||on{1.0}else{0.27})))
            .paint_transition(160,"ease_out")
            .motion(motion_transition("scale_y",if on{1.5}else{1.0},if on{1.5}else{1.0},#{duration_ms:240,easing:ease(),intent:"feedback"}));
        let base=style().height(relative(1.0)).min_width(unit(0.5)).flex_basis(px(0));
        let base=if f>cut{base.flex_grow(f*100.0)}else{base.hidden()};
        let n=column([ink]).with_key(item_key(props.series,slot.id)).accessibility_hidden(true)
            .with_ref(refs[i]).number_motion_source()
            .with_style(base)
            .motion(motion_transition("number_value",0.0,if e==(){0.0}else{e.value},#{duration_ms:620,easing:ease()}))
            .motion(motion_transition("number_weight",1.0,if active==""||on{1.0}else{0.0},#{duration_ms:350,easing:ease()}))
            .sample_flex_share(lanes,i,cut);
        if e!=()&&f>0.0{n=n.on_hover_change(Fn("attend").curry(props.series,"bar",e.id));}
        bars.push(n);
    }
    row(bars).with_key("distribution").with_style(ctx.component_style("bar",style().height(unit(2)).min_width(px(0))
        .width(relative(1.0)).gap_x(unit(0.5))))
}
fn ranked_rows(ctx,props,model,active,narrow,lanes){
    let rows=[];
    for e in model.ranked{
        let hot=e.id==active;
        let swatch=column([]).with_key("dot").accessibility_hidden(true)
            .with_style(ctx.component_style("dot",style().width(px(8)).height(px(8)).radius(px(2)).flex_shrink(false).background(tone(e,e.slot))));
        let content=[swatch,label(ctx,e.label,"name",style().flex_grow().typography("body")).with_key("name"),
            amount(ctx,e.value,props,"amount","value",lanes,"amount",e.rank)];
        if !narrow{content.push(ratio(ctx,e.share,"share",lanes,e.rank));}
        let accepted=ctx.format_number(e.value,#{max_fraction_digits:props.decimals});
        let summary=ctx.t_or("compact_breakdown.row","{label}, {value}, {share}",#{label:e.label,value:props.prefix+accepted+props.suffix,share:format_percent(ctx,e.share)});
        rows.push(row(content).with_key("item:"+item_key(props.series,e.id)).tab_stop(true).accessibility_role("listitem")
            .accessibility_label(summary).on_hover_change(Fn("attend").curry(props.series,"row",e.id))
            .on_focus_within_change(Fn("attend").curry(props.series,"focus",e.id))
            .with_style(ctx.component_style(if hot{["row","row_active"]}else{["row"]},
                style().items_center().gap(sp(2.25)).min_width(px(0)).min_height(unit(7.5)).padding_x(sp(2)).radius(theme_radius("sm"))
                    .opacity(if hot||active==""{1.0}else{0.45}).border(px(1)).border_color(rgba(0x00000000))
                    .focus(style().border_color(theme_color("focus_ring")))))
            .paint_transition(160,"ease_out").layout_motion(560,ease()));
    }
    if rows.len==0{rows.push(label(ctx,get(props,"empty_label",ctx.t_or("compact_breakdown.empty","No categories")),"empty",
        style().typography("caption").text_color(theme_color("text_muted"))));}
    column(rows).with_key("ranking").accessibility_role("list").with_style(ctx.component_style("list",style().min_width(px(0))))
}
fn delta(ctx,d){
    let up=d>=0.0;
    let clean=ctx.theme_variant()!=()&&ctx.theme_token("design.name")=="clean";
    let stroke=if clean{"1.3"}else{"1.6"};
    let arrow=if up{"M2 8 5.5 4.5 8 7l4-4m-3 0h3v3"}else{"M2 4l3.5 3.5L8 5l4 4m-3 0h3V6"};
    let mark=svg(`<svg viewBox='0 0 14 12' fill='none' stroke='currentColor' stroke-width='${stroke}' stroke-linecap='round' stroke-linejoin='round'><path d='${arrow}'/></svg>`)
        .accessibility_hidden(true).with_style(style().width(unit(3)).height(unit(3)).flex_shrink(false));
    let s=(if d>0.0{"+"}else{""})+ctx.format_number(d,#{max_fraction_digits:1})+"%";
    let timing=#{duration_ms:380,easing:ease(),intent:"decorative"};
    row([mark,text(s)]).with_key("delta").with_style(ctx.component_style(["delta",if up{"delta_up"}else{"delta_down"}],
        style().items_center().gap(sp(0.5)).min_height(unit(5)).padding_start(sp(1)).padding_end(sp(1.5))
            .isolated_opacity(true).radius(theme_radius("pill")).typography("caption").font_feature("tnum",1)
            .text_color(theme_color(if up{"compact_breakdown.up"}else{"compact_breakdown.down"}))
            .background(theme_color(if up{"compact_breakdown.up_bg"}else{"compact_breakdown.down_bg"}))))
        .motion(motion_transition("opacity",0.0,1.0,timing))
        .motion(motion_transition("scale_x",0.85,1.0,timing))
        .motion(motion_transition("scale_y",0.85,1.0,timing))
}
fn summary_caption(ctx,props,selected){
    let ink=style().typography("caption").text_color(theme_color("text_muted")).font_feature("tnum",1);
    if selected==(){
        return label(ctx,get(props,"total_label",ctx.t_or("compact_breakdown.total","Total")),"caption",ink).with_key("caption");
    }
    let key="compact_breakdown.caption";let fallback="{label} · {share} of total";
    let share=format_percent(ctx,selected.share);
    let pattern=ctx.t_or(key,fallback);
    let parts=pattern.split("{label}");
    // Locales choose where the name appears. If a custom translation doesn't
    // have one name slot, preserve its ordinary single-line formatting.
    if parts.len!=2{
        return label(ctx,ctx.t_or(key,fallback,#{label:selected.label,share:share}),"caption",ink).with_key("caption");
    }
    let before=parts[0];before.replace("{share}",share);
    let after=parts[1];after.replace("{share}",share);
    let fixed=style().flex_shrink(false).whitespace_nowrap();
    let runs=[];
    if before!=""{runs.push(text(before).with_key("before").with_style(fixed).accessibility_hidden(true));}
    runs.push(text(selected.label).with_key("name").with_style(style().min_width(px(0)).flex_shrink(true)
        .whitespace_nowrap().text_ellipsis()).accessibility_hidden(true));
    if after!=""{runs.push(text(after).with_key("after").with_style(fixed).accessibility_hidden(true));}
    row(runs).with_key("caption").accessibility_role("text").accessibility_label(before+selected.label+after)
        .with_style(ctx.component_style("caption",ink.min_width(px(0)).items_center().overflow_hidden()))
}
fn render_breakdown(ctx,props){
    let model=snapshots(ctx,props);let active=inspect(ctx,props,model);
    effect("clean_attention",#{series:props.series,ids:model.items.keys()},Fn("clean_attention"),Fn("noop"));
    let root_ref=element_ref("breakdown");
    let width=ctx.element_width_class(root_ref,[251,321]);let narrow=width==0;let cozy=width!=2;
    let title=get(props,"title",ctx.t_or("compact_breakdown.title","Breakdown"));
    let head=[label(ctx,title,"title",style().typography("body").font_weight(600)).with_key("title").accessibility_role("heading").accessibility_level(2)];
    let switcher=switch_period(ctx,props,narrow,cozy,root_ref.semantic_id);if switcher!=(){head.push(switcher);}
    let heading=if narrow&&switcher!=(){
        column(head).with_style(ctx.component_style(["head","head_narrow"],style().min_width(px(0)).items_start().gap(sp(1.5))))
    }else{
        row(head).with_style(ctx.component_style("head",style().min_width(px(0)).items_center().justify_between().gap(sp(2.5))))
    };
    let selected=get(model.items,active,());
    let number=if selected==(){model.total}else{selected.value};
    let refs=[];for slot in model.history.ids{refs.push(element_ref(slot.ref));}
    let lanes=if refs.len==0{()}else{motion_numbers(refs)};
    let fig=[amount(ctx,number,props,"total","figure:"+props.series,lanes,"weighted_sum",0)];
    if selected==()&&get(props,"delta",())!=(){fig.push(delta(ctx,decimal(props.delta)));}
    let panel=column([
        heading.with_key("heading"),
        row(fig).with_key("figure").with_style(ctx.component_style("figure",style().min_width(px(0)).items_center().gap(sp(2)))),
        summary_caption(ctx,props,selected),
        color_bar(ctx,props,model,active,lanes,refs),ranked_rows(ctx,props,model,active,narrow,lanes),
    ]).with_key("panel").with_style(ctx.component_style(if cozy{["panel","panel_cozy"]}else{["panel"]},
        style().width(relative(1.0)).min_width(px(0)).padding(sp(3)).radius(theme_radius("lg"))
            .background(theme_color("compact_breakdown.card")).border(px(1)).border_color(theme_color("border"))));
    let body=[panel];let footer=get(props,"footer",());
    if props.show_footer&&(footer!=()||model.total>0.0){
        if footer==(){
            let largest=model.ranked[0];
            footer=label(ctx,ctx.t_or("compact_breakdown.largest","Largest: {label} · {share}",#{label:largest.label,share:format_percent(ctx,largest.share)}),
                "footer_text",style().typography("caption").text_color(theme_color("text_muted")));
        }
        body.push(row([footer]).with_key("footer").with_style(ctx.component_style(if cozy{["footer","footer_cozy"]}else{["footer"]},
            style().width(relative(1.0)).min_width(px(0)).items_center().min_height(unit(8.5)).padding_x(sp(2.5)))));
    }
    column(body).with_key(props.key).with_ref(root_ref).accessibility_role("group").accessibility_label(title)
        .with_style(ctx.component_style("root",style().gap(px(0)).width(relative(1.0)).max_width(px(360)).min_width(px(0)).overflow_hidden()))
}
