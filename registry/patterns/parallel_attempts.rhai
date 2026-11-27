/* gpui-rhai
{"id":"patterns/parallel_attempts","export":"ParallelAttempts","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"assets":[],"tokens":["accent","border","design.name","focus_ring","parallel_attempts.action_hover","parallel_attempts.best_bg","parallel_attempts.best_edge","parallel_attempts.done_bg","parallel_attempts.done_edge","parallel_attempts.done_ink","parallel_attempts.failed_bg","parallel_attempts.failed_edge","parallel_attempts.failed_ink","parallel_attempts.icon","parallel_attempts.ink","parallel_attempts.mark","parallel_attempts.muted","parallel_attempts.running_bg","parallel_attempts.running_edge","parallel_attempts.running_ink","parallel_attempts.shine","parallel_attempts.strong","parallel_attempts.text","parallel_attempts.track","parallel_attempts.tray","parallel_attempts.wash","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.initials","typography.metadata"],"environment":[]}
*/
// Compare independent attempts without letting a visual winner change Host state.
// Example: ParallelAttempts(#{key:"race",run:"trial-7",attempts:[#{id:"test",name:"Test",status:"done",score:92}]})
define_component(#{
    metadata:#{id:"patterns/parallel_attempts","export":"ParallelAttempts",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:[],capabilities:#{},assets:[],tokens:["accent","border","design.name","focus_ring","parallel_attempts.action_hover","parallel_attempts.best_bg","parallel_attempts.best_edge","parallel_attempts.done_bg","parallel_attempts.done_edge","parallel_attempts.done_ink","parallel_attempts.failed_bg","parallel_attempts.failed_edge","parallel_attempts.failed_ink","parallel_attempts.icon","parallel_attempts.ink","parallel_attempts.mark","parallel_attempts.muted","parallel_attempts.running_bg","parallel_attempts.running_edge","parallel_attempts.running_ink","parallel_attempts.shine","parallel_attempts.strong","parallel_attempts.text","parallel_attempts.track","parallel_attempts.tray","parallel_attempts.wash","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.initials","typography.metadata"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable card key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Attempt set identity. A new run clears any previous local pick."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Snapshot revision echoed by selection proposals."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Defaults to Best of N."},
        task:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Shared task description."},
        attempts:#{schema:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable ID unique within the run."},
            name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Short visible attempt name."},
            approach:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional method or strategy."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Name beside the attempt."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative node constrained to the model tile."},
            status:#{schema:#{type:"string",allowed:["running","done","failed"]},required:true,sensitive:false,doc:"Settled includes both done and failed."},
            progress:#{schema:#{type:"optional",value:#{type:"number",min:0,max:1}},required:false,sensitive:false,doc:"Accepted fraction; failed keeps its last supplied fraction."},
            step:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Current step while running."},
            note:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Outcome once settled."},
            score:#{schema:#{type:"optional",value:#{type:"number",min:0,max:100}},required:false,sensitive:false,doc:"Finished result score. The first done attempt wins a tie."},
            duration_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Elapsed milliseconds supplied by the Host."},
            cost_micros:#{schema:#{type:"optional",value:#{type:"integer",min:-1000000000000,max:1000000000000}},required:false,sensitive:true,doc:"Signed integer micro-units of currency; missing has no row cost label."}
        }}},required:true,sensitive:true,doc:"Attempts in visual and stable tie-break order."},
        picked:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Controlled accepted ID; empty string explicitly selects none. Omit for local selection and automatic best after settlement."},
        currency_prefix:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Visible prefix; defaults to $."},
        on_pick:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Selection notification or proposal. This never runs an attempt."}
    },state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
    events:#{pick:#{payload:#{type:"object",allow_unknown:false,fields:#{
        run:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
        id:#{schema:#{type:"string"},required:true,sensitive:false}}},doc:"Requests use of a settled successful attempt by its business ID."}},
    slots:#{},parts:["root","header","header_narrow","mark","title","task","summary","summary_narrow","stats","stats_value","panel","card","card_narrow",
        "picked","quiet","icon","who","name","model","approach","line","line_narrow","track","track_narrow","fill","sheen","glow","end",
        "end_narrow","score","score_running","score_failed","best","meta_slot","meta","pick","pick_on","empty"],effects:["reset_run"]},
    render:Fn("render_attempts")
});

fn ParallelAttempts(props){render_component("patterns/parallel_attempts",props)}
fn get(m,k,d){if k in m&&m[k]!=(){m[k]}else{d}}
fn u(n){theme_length("space.unit").times(n)}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn tx(ctx,key,words){ctx.t_or("parallel_attempts."+key,words)}
fn paint(k){let path="parallel_attempts."+k;theme_color(path)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn line(){style().typography("body").whitespace_nowrap().text_ellipsis().min_width(px(0)).text_color(paint("ink"))}
fn small(){line().typography("caption").text_color(paint("muted"))}
fn detail(){small().typography("metadata")}
fn label(ctx,k,t,s){text(t).with_style(ctx.component_style(k,s))}
fn valid(id){
    if id==""||utf8_byte_count(id)>96{return false;}
    for c in id.chars(){if c<' '||c==''{return false;}}
    true
}
fn as_float(n){if type_of(n)=="i64"{n.to_float()}else{n}}
fn money(ctx,p,n){
    let x=abs(n);let cur=get(p,"currency_prefix","$");let sign=if n<0{"−"}else{""};
    if x==0{return cur+"0";}
    if x<1000{return sign+"<"+cur+"0.001";}
    let digits=if x<10_000{3}else if x<100_000_000{2}else{0};
    sign+cur+ctx.format_number(x.to_float()/1_000_000.0,#{min_fraction_digits:digits,max_fraction_digits:digits})
}
fn two(n){if n<10{"0"+n.to_string()}else{n.to_string()}}
fn separated(values){
    let out="";
    for value in values{if value!=""{if out!=""{out+=" · ";}out+=value;}}
    out
}
fn elapsed(ctx,n){
    if n<1000{return ctx.format_number(n)+"ms";}
    if n<10000{return ctx.format_number(n.to_float()/1000.0,#{min_fraction_digits:1,max_fraction_digits:1})+"s";}
    let secs=(n+500)/1000;let mins=secs/60;let hrs=mins/60;
    if secs<60{return secs.to_string()+"s";}
    if mins<60{return mins.to_string()+"m "+two(secs%60)+"s";}
    if hrs<24{return hrs.to_string()+"h "+two(mins%60)+"m";}
    (hrs/24).to_string()+"d "+(hrs%24).to_string()+"h"
}
fn meta(ctx,p,a){
    let time=get(a,"duration_ms",());let cost=get(a,"cost_micros",());let parts=[];
    if time!=(){parts.push(elapsed(ctx,time));}if cost!=(){parts.push(money(ctx,p,cost));}
    separated(parts)
}
fn local(ctx,run){
    let old=ctx.get_state("ui");
    if old!=()&&old.run==run{old}else{#{run:run,pick:"",focus:"",cursor:""}}
}
fn reset_run(ctx,data){
    let old=ctx.get_state("ui");
    if old!=()&&old.run!=data.run{ctx.set_state("ui",local(ctx,data.run));}
}
fn ignore(ctx,data){}
fn pick(token,ctx,event){
    let s=local(ctx,token.run);
    if !token.controlled{s.pick=token.id;ctx.set_state("ui",s);}
    ctx.emit("pick",#{run:token.run,revision:token.revision,id:token.id});
    handled()
}
fn focused(run,id,ctx,within){
    let s=local(ctx,run);
    if within{s.focus=id;s.cursor=id;}else if s.focus==id{s.focus="";}else{return;}
    ctx.set_state("ui",s);
}
fn ref_key(i){"use:"+i.to_string()}
fn jump(key,ctx,event){ctx.focus(key);ctx.scroll_into_view(key);handled()}
fn calculate(ctx,p,frame){
    if !valid(p.run)||utf8_byte_count(get(p,"currency_prefix","$"))>12{throw "ParallelAttempts requires a bounded run ID and currency prefix";}
    let seen=#{};let winners=[];let best="";let maxscore=-1.0;let finished=0;let total=0;
    for i in 0..p.attempts.len{
        let a=p.attempts[i];
        if !valid(a.id)||a.id in seen||a.name==""||utf8_byte_count(a.name)>512{throw "ParallelAttempts requires distinct IDs and nonempty names";}
        for key in ["approach","model","step","note"]{if utf8_byte_count(get(a,key,""))>2048{throw "ParallelAttempts captions must be bounded";}}
        seen[a.id]=i;total+=get(a,"cost_micros",0);
        if a.status!="running"{finished+=1;}
        if a.status=="done"{
            winners.push(a.id);
            let v=as_float(get(a,"score",0.0));
            if v>maxscore{maxscore=v;best=a.id;}
        }
    }
    let all=p.attempts.len>0&&finished==p.attempts.len;
    let external=get(p,"picked",());let s=local(ctx,p.run);
    let requested=if external!=(){external}else if s.pick in winners{s.pick}else{best};
    let chosen=if all&&requested in winners{requested}else{""};
    let cursor=if s.cursor in winners{s.cursor}else if chosen!=""{chosen}else if winners.len>0{winners[0]}else{""};
    #{narrow:ctx.element_width_class(frame,[560])==0,best:if all{best}else{""},chosen:chosen,controlled:external!=(),
        all:all,done:finished,total:total,cursor:cursor,focus:s.focus,winners:winners,by_id:seen}
}
fn title_icon(ctx){
    svg("<svg viewBox='0 0 16 16' fill='currentColor'><rect x='1.5' y='2.5' width='9' height='2.4' rx='1.2'/><rect x='1.5' y='6.8' width='12' height='2.4' rx='1.2' opacity='.55'/><rect x='1.5' y='11.1' width='6' height='2.4' rx='1.2' opacity='.3'/></svg>")
        .accessibility_hidden(true).with_style(ctx.component_style("mark",style().width(u(4)).height(u(4)).flex_shrink(false).text_color(paint("mark"))))
}
fn header(ctx,p,m,title){
    let a=[title_icon(ctx),label(ctx,"title",title,line().text_color(paint("strong")).font_weight(600))];
    let task=get(p,"task","");
    if task!=""{a.push(label(ctx,"task",task,detail()));}
    let completed=ctx.format_number(m.done);
    let rest=" "+tx(ctx,"of","of")+" "+ctx.format_number(p.attempts.len)+" "+tx(ctx,"done","done");
    let cost=money(ctx,p,m.total);
    let figures=detail().font_feature("tnum",1).flex_shrink(false);
    let value=figures.text_color(paint("ink")).font_weight(500);
    let count=row([label(ctx,["stats","stats_value"],completed,value),label(ctx,"stats",rest,figures)])
        .with_style(style().items_center());
    let stats=row([count,label(ctx,["stats","stats_value"],cost,value)])
        .accessibility_role("status").accessibility_label(completed+rest+" · "+cost)
        .with_style(ctx.component_style(if m.narrow{["summary","summary_narrow"]}else{["summary"]},
            style().items_center().gap(sp(3.5)).flex_shrink(false).margin_start(auto())));
    a.push(stats);
    row(a).with_style(ctx.component_style(if m.narrow{["header","header_narrow"]}else{["header"]},
        style().min_width(px(0)).items_center().gap_x(sp(2)).gap_y(sp(1)).flex_wrap()))
}
fn status_text(ctx,a){
    switch a.status{"running"=>tx(ctx,"working","Working…"),"failed"=>tx(ctx,"failed","Failed"),_=>tx(ctx,"complete","Done")}
}
fn who(ctx,a,m,description){
    let first=[label(ctx,"name",a.name,line().font_weight(600))];
    let model=get(a,"model","");let method=get(a,"approach","");
    // Keep the attempt name and model readable before shortening the approach.
    if model!=""{first.push(label(ctx,"model",model,detail().flex_shrink_weight(10)));}
    if !m.narrow&&method!=""{first.push(label(ctx,"approach","·  "+method,detail().flex_shrink_weight(100)));}
    let ink=if a.status=="done"{paint("text")}else{paint(a.status+"_ink")};
    let words=if m.narrow{small().whitespace_normal().line_clamp(2).min_height(u(9))}else{small()};
    let caption=label(ctx,if m.narrow{["line","line_narrow"]}else{["line"]},description,words.text_color(ink))
        .with_key(a.status).motion(motion_transition("opacity",0.0,1.0,#{duration_ms:320,intent:"decorative"}));
    column([row(first).with_style(style().min_width(px(0)).gap_x(sp(1.5)).items_baseline()),caption])
        .with_key("who").with_style(ctx.component_style("who",style().min_width(px(0)).flex_grow().flex_basis(px(0))))
}
fn lane(ctx,a,m,pct){
    let color=if m.best==a.id{"best"}else{a.status};
    let elements=[];
    if a.status=="running"{
        let clear=rgba(0x00000000);let shine=paint("shine");
        elements.push(box([]).with_key("sweep").with_style(ctx.component_style("sheen",style().absolute().top(px(0)).inset_start(px(0))
            .width(u(15)).height(relative(1.0)).layered_gradient(layered_gradient([#{angle:90,
                stops:[#{offset:0.0,color:clear},#{offset:0.5,color:shine},#{offset:1.0,color:clear}]}]))))
            .motion(motion_transition("translate_x_fraction",-1.0,9.5,#{duration_ms:1500,iterations:"infinite",intent:"decorative"})));
        elements.push(box([]).with_key("glow").with_style(ctx.component_style("glow",style().absolute().top(px(0)).inset_end(px(0)).width(u(6))
            .height(relative(1.0)).layered_gradient(layered_gradient([#{angle:90,
                stops:[#{offset:0.0,color:clear},#{offset:1.0,color:paint("running_edge")}]}]))))
            .motion(motion_transition("opacity",0.45,1.0,#{duration_ms:1100,iterations:"infinite",autoreverse:true,intent:"decorative"})));
    }
    let bar=box(elements).with_key("fill").with_style(ctx.component_style("fill",style().relative().width(relative(pct)).height(relative(1.0)).clip()
        .radius(theme_radius("sm")).background(paint(color+"_bg")).border_color(paint(color+"_edge")).border(px(1))))
        .motion(motion_transition("width_fraction",pct,pct,#{duration_ms:600,easing:ease(),intent:"decorative"}))
        .paint_transition(400,"ease_out");
    let base=style().height(u(2.5)).radius(theme_radius("sm")).background(paint("track")).min_width(px(0));
    row([bar]).with_key("lane").accessibility_hidden(true).with_style(ctx.component_style(if m.narrow{["track","track_narrow"]}else{["track"]},
        if m.narrow{base.width(relative(1.0))}else{base.width(relative(0.36)).max_width(relative(0.40)).flex_shrink(false)}))
}
fn best_badge(ctx){
    let clean=ctx.theme_variant()!=()&&ctx.theme_token("design.name")=="clean";
    let curve=if clean{cubic_bezier(0.32,0.72,0.24,1.0)}else{ease()};
    let frames=[#{offset:0.0,value:0.8},#{offset:0.6,value:1.06,easing:curve},#{offset:1.0,value:1.0,easing:curve}];
    let fade=[#{offset:0.0,value:0.0},#{offset:0.6,value:1.0,easing:curve},#{offset:1.0,value:1.0}];
    let timing=#{duration_ms:420,intent:"decorative"};
    label(ctx,"best",tx(ctx,"best","Best"),detail().font_weight(500).text_color(paint("done_ink"))
        .padding_x(sp(2)).radius(theme_radius("sm")).background(paint("done_bg"))).with_key("best")
        .enter_motion(motion_keyframes("scale_x",frames,timing))
        .enter_motion(motion_keyframes("scale_y",frames,timing))
        .enter_motion(motion_keyframes("opacity",fade,timing))
}
fn results(ctx,p,m,a,metrics){
    let score=get(a,"score",());
    let shown=if a.status=="running"{ctx.format_number((as_float(get(a,"progress",0.0))*100.0).round().to_int())+"%"}
        else if a.status=="done"&&score!=(){ctx.format_number(as_float(score).round().to_int())}else{status_text(ctx,a)};
    let cls=if a.status=="failed"{paint("failed_ink")}else if a.status=="running"{paint("muted")}else{paint("ink")};
    let score_parts=if a.status=="done"{["score"]}else{["score","score_"+a.status]};
    let heading=[];
    if m.best==a.id{heading.push(best_badge(ctx));}
    heading.push(label(ctx,score_parts,shown,line().font_weight(600).font_feature("tnum",1).text_color(cls)));
    let current=m.chosen==a.id;let selectable=m.all&&a.status=="done";let focused_on=m.focus==a.id;
    let time=detail().font_feature("tnum",1);
    if selectable{time=time.group_hover(style().opacity(0.0));}
    if current||focused_on&&selectable{time=time.opacity(0.0);}
    let tail=[label(ctx,"meta",metrics,time).accessibility_hidden(true)];
    if selectable{
        let ref=element_ref(ref_key(m.by_id[a.id]));
        let action=Fn("pick").curry(#{run:p.run,revision:p.revision,id:a.id,controlled:m.controlled});
        let body=if current{
            [svg("<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-linecap='round' stroke-linejoin='round' stroke-width='2.4'><path d='M5 14l3.5 3.5L19 6.5'/></svg>")
                .with_style(style().width(u(3)).height(u(3)).flex_shrink(false)).accessibility_hidden(true),text(tx(ctx,"in_use","In use"))]
        }else{[text(tx(ctx,"use_this","Use this one"))]};
        let btn=style().absolute().inset_end(px(0)).top(offset_px(-3)).min_height(u(6)).gap(sp(1)).items_center().padding_x(sp(1.5))
            .radius(theme_radius("sm")).focus_ring(px(2),offset_px(-1),theme_color("focus_ring")).typography("caption")
            .text_color(if current{paint("done_ink")}else{paint("text")}).font_weight(500)
            .opacity(if current||focused_on{1.0}else{0.0}).group_hover(style().opacity(1.0));
        if !current{btn=btn.cursor_pointer().hover(style().background(paint("action_hover")).text_color(paint("ink")));}
        let control=row(body).with_key("pick").with_ref(ref).accessibility_role("radio")
            .accessibility_label((if current{tx(ctx,"using","Using")}else{tx(ctx,"use","Use")})+" "+a.name)
            .accessibility_description(shown+if metrics==""{""}else{" · "+metrics}).accessibility_checked(current).tab_stop(m.cursor==a.id)
            .on_focus_within_change(Fn("focused").curry(p.run,a.id))
            .with_style(ctx.component_style(if current{["pick","pick_on"]}else{["pick"]},btn)).paint_transition(160,"ease_out");
        if !current{
            control=control.on_click(action).on_key_value("plain+enter",action,()).on_key_value("plain+space",action,());
        }
        let index=m.winners.index_of(a.id);let n=m.winners.len;
        control=control.on_key_value("plain+up",Fn("jump").curry(ref_key(m.by_id[m.winners[(index+n-1)%n]])),())
            .on_key_value("plain+down",Fn("jump").curry(ref_key(m.by_id[m.winners[(index+1)%n]])),())
            .on_key_value("plain+home",Fn("jump").curry(ref_key(m.by_id[m.winners[0]])),())
            .on_key_value("plain+end",Fn("jump").curry(ref_key(m.by_id[m.winners[n-1]])),());
        tail.push(control);
    }
    column([row(heading).with_key(a.status).with_style(style().items_center().justify_end().gap_x(sp(1.5)))
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:320,intent:"decorative"})),
        row(tail).with_style(ctx.component_style("meta_slot",style().relative().height(u(4.75)).justify_end().width(relative(1.0))))])
        .with_key("end").with_style(ctx.component_style(if m.narrow{["end","end_narrow"]}else{["end"]},
            style().width(u(if m.narrow{22}else{25})).flex_shrink(false).items_end()))
}
fn card(ctx,p,m,a,i){
    let supplied=get(a,"icon",());
    let logo=if supplied==(){text(a.name.sub_string(0,1).to_upper())}else{supplied};
    let icon=box([logo]).with_key("icon").accessibility_hidden(true).with_style(ctx.component_style("icon",
        style().width(u(8)).height(u(8)).flex_shrink(false).clip().flex_col().items_center().justify_center()
            .typography(if supplied==(){"initials"}else{"caption"}).font_weight(600).radius(theme_radius("sm"))
            .background(paint("icon")).text_color(paint("text"))));
    let note=get(a,if a.status=="running"{"step"}else{"note"},status_text(ctx,a));
    let metrics=meta(ctx,p,a);
    let raw=as_float(get(a,"progress",0.0));
    let pct=if a.status=="done"{1.0}else if a.status=="running"{max(0.03,raw)}else{raw};
    let bar=lane(ctx,a,m,pct);let labels=who(ctx,a,m,note);let end=results(ctx,p,m,a,metrics);
    let current=m.chosen==a.id;let dim=m.chosen!=""&&!current&&m.focus!=a.id;
    let parts=["card"];if m.narrow{parts.push("card_narrow");}if current{parts.push("picked");}if dim{parts.push("quiet");}
    let base=style().relative().width(relative(1.0)).min_width(px(0)).items_center().gap(sp(3)).padding(sp(3)).radius(theme_radius("lg"))
        .background(theme_color("surface_raised")).hover(style().opacity(1.0)).isolated_opacity(true).opacity(if dim{0.55}else{1.0});
    if current{base=base.background(paint("wash"));}
    // Keyed siblings stay under the same parent when the bar wraps.
    let children=if m.narrow{[icon,labels,end,bar]}else{[icon,labels,bar,end]};
    if m.narrow{base=base.flex_wrap();}
    let row_node=row(children);
    let hint=separated([get(a,"model",""),get(a,"approach",""),note,metrics]);
    row_node.with_key("attempt:"+a.id).accessibility_role("group").accessibility_label(a.name).accessibility_description(hint)
        .with_style(ctx.component_style(parts,base)).paint_transition(320,"ease_out")
        .enter_motion(motion_transition("translate_y",4.0,0.0,#{duration_ms:420,delay_ms:min(i,12)*70,easing:ease(),intent:"decorative"}))
}
fn render_attempts(ctx,p){
    let root=element_ref("parallel-attempts");
    let m=calculate(ctx,p,root);
    effect("reset_run",#{run:p.run},Fn("reset_run"),Fn("ignore"));
    let override=get(p,"title",());let title=if override==(){tx(ctx,"best_of","Best of")+" "+ctx.format_number(p.attempts.len)}else{override};
    let lines=[];for i in 0..p.attempts.len{lines.push(card(ctx,p,m,p.attempts[i],i));}
    if lines.len==0{lines.push(label(ctx,"empty",tx(ctx,"empty","No attempts yet"),small().padding(sp(3))));}
    column([header(ctx,p,m,title),column(lines).with_key("attempts:"+p.run).accessibility_role("radiogroup").accessibility_label(title)
        .with_style(ctx.component_style("panel",style().min_width(px(0)).gap_y(sp(1))))])
        .with_key("parallel-attempts").with_ref(root).with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).padding(sp(1))
            .radius(theme_radius("lg")).background(paint("tray"))))
}
