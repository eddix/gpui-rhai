/* gpui-rhai
{"id":"patterns/away_summary","export":"AwaySummary","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","patterns/agent_tree"],"capabilities":{},"assets":[],"tokens":["away_summary.done","away_summary.failed","away_summary.needs","border","focus_ring","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],"environment":[]}
*/
// An inline recap of work that changed while someone was away.
// State: stable per-session ordering and disclosure/focus; the Host owns results and pending work.
// Example: AwaySummary(#{key:"away",session:"shift",revision:0,since_ms:0,now_ms:3600000,items:[]})
import "components/button" as cmd;
import "patterns/agent_tree" as ids;

define_component(#{
    metadata:#{id:"patterns/away_summary","export":"AwaySummary",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/button","patterns/agent_tree"],capabilities:#{},assets:[],tokens:["away_summary.done","away_summary.failed","away_summary.needs","border","focus_ring","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],environment:[]},
    schema:#{
        props:#{
            key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable card key."},
            session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"New interval ID resets the preserved arrival order."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted updates revision."},
            since_ms:#{schema:#{type:"integer",min:0,max:253402300799999},required:true,sensitive:false,doc:"When the person left, in Unix milliseconds."},
            now_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:253402300799999}},required:false,sensitive:false,doc:"Supply an accepted clock for deterministic snapshots. The view clock otherwise refreshes every 30 seconds."},
            time_style:#{schema:#{type:"string",allowed:["duration","clock"]},required:false,sensitive:false,doc:"Render elapsed time or the departure clock.","default":#{type:"string",value:"duration"}},
            time_zone:#{schema:#{type:"string"},required:false,sensitive:false,doc:"Zone used for clock display.","default":#{type:"string",value:"local"}},
            title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Visible summary heading."},
            items:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique update ID."},title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Task label."},detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"One short line of accepted results."},kind:#{schema:#{type:"string",allowed:["needs_you","failed","done"]},required:true,sensitive:false,doc:"Accepted update status."},action_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Action override. Defaults to Decide, Retry or Open."},hide_action:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Suppress this row's action."}}}},required:true,sensitive:true,doc:"Accepted updates; stable keys preserve their initial status group and arrival position."},
            limit:#{schema:#{type:"integer",min:1,max:24},required:false,sensitive:false,doc:"Initially visible rows.","default":#{type:"integer",value:5}},
            collapsed:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Controlled fold state when present."},
            default_collapsed:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Initial local fold state.","default":#{type:"bool",value:false}},
            pending:#{schema:#{type:"optional",value:#{type:"array",max_items:64,items:#{type:"string"}}},required:false,sensitive:false,doc:"IDs awaiting a Host response; their actions cannot be pressed again while pending."},
            on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a versioned row-action proposal."},
            on_collapsed_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a disclosure proposal or notification. The Host may treat collapse as acknowledgement."},
        },
        state:#{fields:#{order:#{schema:#{type:"ui_value"},"default":#{type:"null"}},view:#{schema:#{type:"ui_value"},"default":#{type:"null"}},clock:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        events:#{
            action:#{payload:#{type:"object",allow_unknown:false,fields:#{session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Away interval ID."},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision."},id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted update ID."},kind:#{schema:#{type:"string",allowed:["needs_you","failed","done"]},required:true,sensitive:false,doc:"Kind of that update at proposal time."}}},doc:"Ask the application to act; the UI never invents results."},
            collapsed_change:#{payload:#{type:"object",allow_unknown:false,fields:#{session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Away interval ID."},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision."},collapsed:#{schema:#{type:"bool"},required:true,sensitive:false,doc:"Requested fold state."}}},doc:"One versioned fold or acknowledgement operation."},
        },
        slots:#{},parts:["root","card","head","head_narrow","title","when","counts","count","dot","done","failed","needs_you","rows","row","mark","label","detail","action","action_needs","foot","more","toggle","toggle_folded","divider","rule","empty"],effects:["remember_order","restore_focus"],
    },
    render:Fn("render_away"),
});

fn AwaySummary(p){render_component("patterns/away_summary",p)}
fn get(m,k,d){if k in m&&m[k]!=(){m[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn u(n){theme_length("space.unit").times(n)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn model(ctx,session,initial){
    let v=ctx.get_state("view");if v!=()&&v.session==session{return v;}
    #{session:session,collapsed:initial,all:false,focused:"",return_to:"",restore:false}
}
fn write_view(ctx,v){if ctx.get_state("view")!=v{ctx.set_state("view",v);}}
fn epoch_tick(ctx,p){ctx.set_state("clock",ctx.now());}
fn away_time(ctx,p){
    if p.time_style=="clock"{
        let t=ctx.format_time(p.since_ms,"short",p.time_zone);
        return ctx.t_or("away_summary.since","since {time}",#{time:t});
    }
    let clock=get(p,"now_ms",());
    if clock==(){
        interval("away-clock",30000,false,Fn("epoch_tick"),());
        clock=ctx.get_state("clock");if clock==(){clock=ctx.now();}
    }
    let ms=max(0,clock-p.since_ms);
    if ms<60000{return ctx.t_or("away_summary.moment","a moment");}
    let step=if ms<86400000{60000}else{3600000};
    ctx.format_duration(ms/step*step)
}
fn order_cmp(a,b){if a.rank==b.rank{a.seq-b.seq}else{a.rank-b.rank}}
fn rank(kind){switch kind{"needs_you"=>0,"failed"=>1,_=>2}}
fn keep_order(ctx,o){if ctx.get_state("order")!=o{ctx.set_state("order",o);}}
fn noop(ctx,p){}
fn data(ctx,p){
    if !ids::valid(p.session){throw "AwaySummary requires a stable interval ID";}
    let old=ctx.get_state("order");
    if old==()||old.session!=p.session{old=#{session:p.session,next:0,ranks:#{}};}
    let plan=#{session:p.session,next:old.next,ranks:#{}};
    let seen=#{};let ranked=[];let counts=#{needs_you:0,done:0,failed:0};
    for item in p.items{
        if !ids::valid(item.id)||item.id in seen||item.title==""||utf8_byte_count(item.title)>512||utf8_byte_count(get(item,"detail",""))>2048{
            throw "AwaySummary needs unique IDs and bounded text";
        }
        seen[item.id]=true;counts[item.kind]+=1;
        let o=get(old.ranks,item.id,());
        if o==(){o=#{rank:rank(item.kind),seq:plan.next};plan.next+=1;}
        plan.ranks[item.id]=o;ranked.push(#{item:item,rank:o.rank,seq:o.seq});
    }
    ranked.sort(Fn("order_cmp"));
    effect("remember_order",plan,Fn("keep_order"),Fn("noop"));
    #{ranked:ranked,counts:counts}
}
fn count(ctx,kind,n){
    let word=ctx.format_number(n);
    switch kind{
        "needs_you"=>if n==1{ctx.t_or("away_summary.needs_one","{n} needs you",#{n:word})}
            else{ctx.t_or("away_summary.needs","{n} need you",#{n:word})},
        "failed"=>ctx.t_or("away_summary.failed","{n} failed",#{n:word}),
        _=>ctx.t_or("away_summary.done","{n} done",#{n:word}),
    }
}
fn get_state_word(ctx,k){
    switch k{
        "needs_you"=>ctx.t_or("away_summary.waiting_state","Needs you"),
        "failed"=>ctx.t_or("away_summary.failed_state","Failed"),
        _=>ctx.t_or("away_summary.done_state","Done"),
    }
}
fn paint(k){switch k{"needs_you"=>theme_color("away_summary.needs"),"failed"=>theme_color("away_summary.failed"),_=>theme_color("away_summary.done")}}
fn words(ctx,part,label,base){text(label).with_style(ctx.component_style(part,base.min_width(px(0))))}
fn marker(ctx,k){
    let paths=switch k{
        "needs_you"=>"<circle cx='7' cy='7' r='5.25' fill='none' stroke='currentColor' stroke-width='1.5'/><circle cx='7' cy='7' r='2.25' fill='currentColor'/>",
        "failed"=>"<circle cx='7' cy='7' r='6' fill='currentColor'/><path d='M7 4v3.4M7 9.8v.05' stroke='#fff' stroke-width='1.5' stroke-linecap='round'/>",
        _=>"<circle cx='7' cy='7' r='6' fill='currentColor'/><path d='M4.3 7.2 6.1 9 9.7 5.2' fill='none' stroke='#fff' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'/>",
    };
    svg("<svg viewBox='0 0 14 14'>"+paths+"</svg>").with_key("mark").accessibility_hidden(true)
      .with_style(ctx.component_style("mark",style().width(px(14)).height(px(14)).text_color(paint(k)).flex_shrink(false)))
}
fn dot(ctx,k){box([]).accessibility_hidden(true).with_style(ctx.component_style(["dot",k],style()
    .width(px(6)).height(px(6)).radius(theme_radius("pill")).background(paint(k)).flex_shrink(false)))}
fn disclosure_icon(kind){
    let d=if kind=="check"{"M4 8.4 6.7 11 12.2 5.1"}
         else if kind=="up"{"m4 10 4-4 4 4"}else{"m4 6 4 4 4-4"};
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.45' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
        .accessibility_hidden(true).with_style(style().width(px(14)).height(px(14)).flex_shrink(false))
}
fn action_text(ctx,k){switch k{
    "needs_you"=>ctx.t_or("away_summary.decide","Decide"),
    "failed"=>ctx.t_or("away_summary.retry","Retry"),
    _=>ctx.t_or("away_summary.open","Open"),
}}
fn row_proposal(t,ctx,e){ctx.emit("action",t);handled()}
fn remember_focus(c,id,ctx,in_scope){
    let v=model(ctx,c.session,c.initial);
    if in_scope{v.focused=id;}
    else if v.focused==id{v.focused="";}
    write_view(ctx,v)
}
fn one(ctx,p,c,item,index){
    let status=get_state_word(ctx,item.kind);
    let labels=[words(ctx,"label",item.title,style().typography("body").font_weight(500).text_color(theme_color("text_primary"))
       .whitespace_nowrap().text_ellipsis()).with_key("name")];
    let detail=get(item,"detail","");
    if detail!=""{
        labels.push(words(ctx,"detail",detail,style().typography("caption").text_color(theme_color("text_muted"))
          .whitespace_nowrap().text_ellipsis()).with_key("detail"));
    }
    let inner=[marker(ctx,item.kind),column(labels).with_key("body").with_style(style().min_width(px(0)).flex_grow())];
    if can_act(p,item){
        let label=get(item,"action_label",action_text(ctx,item.kind));
        let busy=item.id in get(p,"pending",[]);
        let req=#{session:p.session,revision:p.revision,id:item.id,kind:item.kind};
        let btn=cmd::Button(#{key:"request:"+item.id,text:label,size:"sm",variant:if item.kind=="needs_you"{"secondary"}else{"ghost"},
            disabled:busy,loading:busy,on_click:Fn("row_proposal").curry(req)})
            .with_key("request:"+item.id).with_ref(element_ref("row:"+index.to_string()))
            .tab_stop(true).accessibility_label(label+": "+item.title)
            .on_focus_within_change(Fn("remember_focus").curry(c,item.id))
            .with_style(ctx.component_style(if item.kind=="needs_you"{["action","action_needs"]}else{["action"]},
                style().max_width(relative(0.43)).min_width(px(0)).flex_shrink(false)));
        inner.push(btn);
    }
    row(inner).with_key("item:"+item.id).accessibility_role("listitem").accessibility_label(item.title)
        .accessibility_description(status+(if detail==""{""}else{" · "+detail}))
        .with_style(ctx.component_style("row",style().min_width(px(0)).min_height(u(12))
             .radius(theme_radius("sm")).padding_x(sp(2)).gap(sp(2.5)).items_center()))
        .paint_transition(160,"ease_out")
}
fn can_act(p,item){get(p,"on_action",())!=()&&!get(item,"hide_action",false)}
fn change_fold(c,want,preserve,ctx,e){
    if want==c.collapsed{return ();}
    let v=model(ctx,c.session,c.initial);
    if !c.controlled{v.collapsed=want;}
    if want{
        v.return_to=if preserve{v.focused}else{""};
        v.restore=false;
        ctx.focus("disclosure");
    }else{v.restore=true;}
    write_view(ctx,v);
    if c.emit{ctx.emit("collapsed_change",#{session:c.session,revision:c.revision,collapsed:want});}
    handled()
}
fn collapse_key(c,ctx,e){if !c.collapsed{change_fold(c,true,true,ctx,e)}else{()}}
fn more_rows(c,ctx,e){
    let v=model(ctx,c.session,c.initial);v.all=!v.all;write_view(ctx,v);handled()
}
fn finish_transfer(ctx,cmd){
    let v=model(ctx,cmd.session,cmd.initial);
    if v.restore{
        v.restore=false;v.return_to="";write_view(ctx,v);
        if cmd.ref!=""{ctx.focus(cmd.ref);}
    }
}
fn released(c,ctx,within){
    if !within&&c.collapsed{
        let v=model(ctx,c.session,c.initial);
        if v.restore{v.restore=false;v.return_to="";write_view(ctx,v);}
    }
}
fn heading(ctx,title,when,d,narrow){
    let stamps=[];let aggregate="";
    for k in ["needs_you","failed","done"]{
        if d.counts[k]>0{
            let say=count(ctx,k,d.counts[k]);aggregate+=if aggregate==""{say}else{" · "+say};
            stamps.push(row([dot(ctx,k),words(ctx,"count",say,style().typography("caption").text_color(theme_color("text_muted")))])
                .with_key(k).with_style(style().min_width(px(0)).items_center().gap(sp(1))));
        }
    }
    if aggregate==""{aggregate="0";}
    let names=row([words(ctx,"title",title,style().typography("body").font_weight(500).text_color(theme_color("text_primary"))
            .whitespace_nowrap().text_ellipsis()).accessibility_role("heading").accessibility_level(2),
        words(ctx,"when",when,style().typography("caption").text_color(theme_color("text_muted")))])
        .with_style(style().min_width(px(0)).gap(sp(1.5)).items_center().flex_grow());
    let counts=row(stamps).with_key("counts").accessibility_role("status").accessibility_label(aggregate)
        .with_style(ctx.component_style("counts",style().min_width(px(0)).flex_wrap().gap(sp(2)).items_center()));
    let node=if narrow{column([names,counts])}else{row([names,counts])};
    node.with_key("header").with_style(ctx.component_style(if narrow{["head","head_narrow"]}else{["head"]},style()
        .min_width(px(0)).padding_x(sp(2.5)).padding_y(sp(2)).gap(sp(1.5)).justify_between()))
}
fn footer(ctx,p,c,when,d,n){
    let inner=[];
    let rule=style().flex_grow().height(px(1)).background(theme_color("border"));
    if c.collapsed{inner.push(box([]).accessibility_hidden(true).with_style(ctx.component_style("rule",rule)));}
    if !c.collapsed&&d.ranked.len>p.limit{
        let name=if n==d.ranked.len{ctx.t_or("away_summary.less","Show less")}else{
            ctx.t_or("away_summary.more","Show {n} more",#{n:ctx.format_number(d.ranked.len-n)})
        };
        inner.push(cmd::Button(#{key:"more",text:name,size:"sm",variant:"ghost",
            suffix:disclosure_icon(if n==d.ranked.len{"up"}else{"down"}),on_click:Fn("more_rows").curry(c)})
            .with_style(ctx.component_style("more",style().max_width(relative(1.0)))));
    }
    let title=if c.collapsed{
        ctx.t_or("away_summary.folded","You were away {time} · {n} updates",#{time:when,n:ctx.format_number(d.ranked.len)})
    }else{ctx.t_or("away_summary.ack","Got it")};
    let control=#{key:"disclosure",text:title,size:"sm",variant:"ghost",
        on_click:Fn("change_fold").curry(c,!c.collapsed,false)};
    if c.collapsed{
        control.suffix=disclosure_icon("down");
        if d.counts.needs_you>0{control.prefix=dot(ctx,"needs_you");}
    }else{control.prefix=disclosure_icon("check");}
    let trigger=cmd::Button(control)
        .with_key("disclosure").with_ref(element_ref("disclosure"))
        .accessibility_expanded(!c.collapsed)
        .on_focus_within_change(Fn("released").curry(c))
        .with_style(ctx.component_style(if c.collapsed{["toggle","toggle_folded"]}else{["toggle"]},
            style().max_width(relative(if c.collapsed{0.94}else{1.0}))));
    inner.push(trigger);
    if c.collapsed{inner.push(box([]).accessibility_hidden(true).with_style(ctx.component_style("rule",rule)));}
    let base=style().min_width(px(0)).padding(sp(1)).gap(sp(2)).items_center().justify_between();
    if !c.collapsed&&d.ranked.len<=p.limit{base=base.justify_end();}
    row(inner).with_key("footer").with_style(ctx.component_style(if c.collapsed{["foot","divider"]}else{["foot"]},base))
}
fn render_away(ctx,p){
    let d=data(ctx,p);
    let v=model(ctx,p.session,p.default_collapsed);
    let controlled=get(p,"collapsed",())!=();
    let closed=if controlled{p.collapsed}else{v.collapsed};
    let c=#{session:p.session,revision:p.revision,initial:p.default_collapsed,controlled:controlled,collapsed:closed,
        emit:get(p,"on_collapsed_change",())!=()};
    let title=get(p,"title",ctx.t_or("away_summary.title","While you were away"));
    let when=away_time(ctx,p);
    let region=element_ref("summary");let narrow=ctx.element_width_class(region,[461])!=1;
    let total=d.ranked.len;let n=min(total,if v.all{total}else{p.limit});
    let payload=[];let rows=[];
    if !closed{
        payload.push(heading(ctx,title,when,d,narrow));
        let focus="";let first="";
        for index in 0..n{
            let item=d.ranked[index].item;rows.push(one(ctx,p,c,item,index));
            if can_act(p,item){
                let ref="row:"+index.to_string();if first==""{first=ref;}if v.return_to==item.id{focus=ref;}
            }
        }
        if v.restore{effect("restore_focus",#{session:p.session,initial:p.default_collapsed,ref:if focus!=""{focus}else{first}},
             Fn("finish_transfer"),Fn("noop"));}
        if n==0{rows.push(words(ctx,"empty",ctx.t_or("away_summary.empty","Nothing happened while you were away"),
             style().typography("caption").text_color(theme_color("text_muted")).padding(sp(3))).with_key("empty"));}
        payload.push(column(rows).with_key("items").accessibility_role("list")
            .with_style(ctx.component_style("rows",style().min_width(px(0)).width(relative(1.0)).padding(sp(1))
                .background(theme_color("surface_raised")).radius(theme_radius("lg")))));
    }
    let body=column(payload).with_key("body").with_style(style().min_width(px(0)).overflow_hidden())
        .layout_size_motion("visible:"+closed.to_string()+":"+n.to_string(),"height",400,ease());
    column([body,footer(ctx,p,c,when,d,n)]).with_key(p.key).with_ref(region).accessibility_role("group").accessibility_label(title)
        .on_bubble("key:plain+escape",Fn("collapse_key").curry(c))
        .with_style(ctx.component_style(if closed{["root"]}else{["root","card"]},style().min_width(px(0)).width(relative(1.0))))
}
