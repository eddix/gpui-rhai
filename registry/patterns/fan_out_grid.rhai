/* gpui-rhai
{"id":"patterns/fan_out_grid","export":"FanOutGrid","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"assets":[],"tokens":["border","fan_out_grid.done_bg","fan_out_grid.done_edge","fan_out_grid.done_ink","fan_out_grid.failed_bg","fan_out_grid.failed_edge","fan_out_grid.failed_ink","fan_out_grid.ink","fan_out_grid.mark","fan_out_grid.muted","fan_out_grid.pin","fan_out_grid.queued_bg","fan_out_grid.queued_edge","fan_out_grid.queued_ink","fan_out_grid.retrying_bg","fan_out_grid.retrying_edge","fan_out_grid.retrying_ink","fan_out_grid.running_bg","fan_out_grid.running_edge","fan_out_grid.running_ink","fan_out_grid.shine","fan_out_grid.tray","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.metadata"],"environment":[]}
*/
// Inspect a dense field of tasks and propose retrying the failed subset.
// Example: FanOutGrid(#{key:"workers",run:"batch-4",tasks:[#{id:"docs",label:"Index docs",status:"running"}]})
define_component(#{
    metadata:#{id:"patterns/fan_out_grid","export":"FanOutGrid",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:[],capabilities:#{},assets:[],tokens:["border","fan_out_grid.done_bg","fan_out_grid.done_edge","fan_out_grid.done_ink","fan_out_grid.failed_bg","fan_out_grid.failed_edge","fan_out_grid.failed_ink","fan_out_grid.ink","fan_out_grid.mark","fan_out_grid.muted","fan_out_grid.pin","fan_out_grid.queued_bg","fan_out_grid.queued_edge","fan_out_grid.queued_ink","fan_out_grid.retrying_bg","fan_out_grid.retrying_edge","fan_out_grid.retrying_ink","fan_out_grid.running_bg","fan_out_grid.running_edge","fan_out_grid.running_ink","fan_out_grid.shine","fan_out_grid.tray","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.metadata"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable instance key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Job identity. A new run clears local attention and selection."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted Host revision echoed in proposals."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Defaults to Fan-out."},
        job:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Description shared by the tasks."},
        icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative title icon."},
        tasks:#{schema:#{type:"array",max_items:1024,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable, unique task ID."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Task name."},
            status:#{schema:#{type:"string",allowed:["queued","running","retrying","done","failed"]},required:true,sensitive:false,doc:"Accepted task status."},
            worker:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Worker name or label."},
            start_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Relative to the job start."},
            end_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Accepted finish time, no earlier than start_ms."},
            cost_micros:#{schema:#{type:"optional",value:#{type:"integer",min:-1000000000000,max:1000000000000}},required:false,sensitive:true,doc:"Signed exact currency micro-units."},
            attempts:#{schema:#{type:"optional",value:#{type:"integer",min:1,max:1000000}},required:false,sensitive:false,doc:"Number of tries already made."},
            result:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Result detail when done."},
            error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Error detail when failed or retrying."}
        }}},required:true,sensitive:true,doc:"Up to 1,024 tasks, in visual order."},
        now_ms:#{schema:#{type:"integer",min:0,max:1000000000000},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Host-owned elapsed time; no script timer is started."},
        workers:#{schema:#{type:"optional",value:#{type:"integer",min:1,max:1024}},required:false,sensitive:false,doc:"Optional concurrency for the ETA. Defaults to at least one active worker."},
        pinned:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Controlled inspection ID; empty unpins. Omit for a locally controlled pin."},
        currency_prefix:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Visible currency prefix; defaults to $."},
        on_select:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Callback for pin and unpin proposals."},
        on_retry:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"If supplied, enables Retry failed when the snapshot contains errors."}
    },state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
    events:#{
        select:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            id:#{schema:#{type:"string"},required:true,sensitive:false}}},doc:"Select an exact business ID, or an empty string to clear it."},
        retry:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            ids:#{schema:#{type:"array",max_items:1024,items:#{type:"string"}},required:true,sensitive:false}}},doc:"Propose retrying exactly the failed task IDs in snapshot order."}
    },slots:#{},parts:["root","header","title","mark","job","summary","summary_compact","metric","terminal","body","body_compact","panel","panel_compact","field",
        "cell","cell_compact","cell_dim","cell_attention","cell_pinned","sheen","tip","tip_title","tip_status","tip_meta","tip_result","tip_error",
        "footer","legend","legend_compact","filter","filter_count","filter_on","filter_dim","filter_empty","swatch","retry","empty"],effects:["sync_run"]},
    render:Fn("render_fan_out")
});

fn FanOutGrid(props){render_component("patterns/fan_out_grid",props)}
fn get(m,k,d){if k in m&&m[k]!=(){m[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn u(n){theme_length("space.unit").times(n)}
fn tx(ctx,k,v){ctx.t_or("fan_out_grid."+k,v)}
fn fill(k){let key="fan_out_grid."+k;theme_color(key)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn caption(){style().typography("caption").text_color(theme_color("text_muted")).min_width(px(0))}
fn label(ctx,part,words,base){text(words).with_style(ctx.component_style(part,base))}
fn states(){["done","running","retrying","failed"]}
fn words(ctx,st){switch st{
    "done"=>tx(ctx,"done","Done"),"running"=>tx(ctx,"running","Running"),"retrying"=>tx(ctx,"retrying","Retrying"),
    "failed"=>tx(ctx,"failed","Failed"),_=>tx(ctx,"queued","Queued")
}}
fn money(ctx,p,cost){
    let x=abs(cost);let prefix=get(p,"currency_prefix","$");let sign=if cost<0{"−"}else{""};
    if x==0{return prefix+"0";}
    if x<1000{return sign+"<"+prefix+"0.001";}
    let digits=if x<10000{3}else if x<100_000_000{2}else{0};
    sign+prefix+ctx.format_number(x.to_float()/1_000_000.0,#{min_fraction_digits:digits,max_fraction_digits:digits})
}
fn two(n){if n<10{"0"+n.to_string()}else{n.to_string()}}
fn took(ctx,ms){
    if ms<1000{return ctx.format_number(ms)+"ms";}
    if ms<10000{return ctx.format_number(ms.to_float()/1000.0,#{min_fraction_digits:1,max_fraction_digits:1})+"s";}
    let s=(ms+500)/1000;let m=s/60;let h=m/60;
    if s<60{return s.to_string()+"s";}
    if m<60{return m.to_string()+"m "+two(s%60)+"s";}
    if h<24{return h.to_string()+"h "+two(m%60)+"m";}
    (h/24).to_string()+"d "+(h%24).to_string()+"h"
}
fn local(ctx,run){
    let old=ctx.get_state("ui");
    if old!=()&&old.run==run{old}else{#{run:run,hover:"",focus:"",cursor:"",pin:"",filter:"",suppressed:false}}
}
fn sync_run(ctx,data){
    let old=ctx.get_state("ui");
    if old!=()&&old.run!=data.run{let s=local(ctx,data.run);ctx.set_state("ui",s);}
}
fn idle(ctx,data){}
fn letter(i){"tile:"+i.to_string()}
fn attend(run,which,id,ctx,inside){
    let s=local(ctx,run);
    if inside{
        if s[which]==id{return;}
        s[which]=id;s.suppressed=false;
        if which=="focus"{s.cursor=id;}
    }else if s[which]==id{s[which]="";}else{return;}
    ctx.set_state("ui",s);
}
fn set_filter(run,id,ctx,e){
    let s=local(ctx,run);s.filter=if s.filter==id{""}else{id};ctx.set_state("ui",s);handled()
}
fn request_pick(ticket,requested,ctx,e){
    let s=local(ctx,ticket.run);s.suppressed=false;
    let id=if requested==ticket.pin{""}else{requested};
    if !ticket.controlled{s.pin=id;}
    ctx.set_state("ui",s);
    ctx.emit("select",#{run:ticket.run,revision:ticket.revision,id:id});handled()
}
fn clear(ticket,ctx,e){
    let s=local(ctx,ticket.run);
    if s.hover==""&&s.focus==""&&ticket.pin==""{return ();}
    s.suppressed=true;s.hover="";
    if !ticket.controlled{s.pin="";}
    ctx.set_state("ui",s);
    if ticket.pin!=""{ctx.emit("select",#{run:ticket.run,revision:ticket.revision,id:""});}
    handled()
}
fn seek(key,ticket,ctx,e){
    let s=local(ctx,ticket.run);
    s.hover="";s.suppressed=false;
    if !ticket.controlled{s.pin="";}
    ctx.set_state("ui",s);
    if ticket.pin!=""{ctx.emit("select",#{run:ticket.run,revision:ticket.revision,id:""});}
    ctx.focus(key);
    let g=ctx.element_bounds(key);
    if g!=()&&g.clip!=(){
        let v=g.visual;let c=g.clip;
        if v.y<c.y||v.y+v.height>c.y+c.height{ctx.scroll_into_view(key);}
    }
    handled()
}
fn retry(ticket,ctx,e){ctx.emit("retry",ticket);handled()}
fn ttl(ctx,p,m){
    let title=get(p,"title",tx(ctx,"title","Fan-out"));
    let icon=get(p,"icon",());
    if icon==(){
        icon=svg("<svg viewBox='0 0 16 16' fill='currentColor'><circle cx='3' cy='8' r='1.9'/><rect x='10.2' y='1.6' width='3.6' height='3.6' rx='1'/><rect x='10.2' y='6.2' width='3.6' height='3.6' rx='1' opacity='.6'/><rect x='10.2' y='10.8' width='3.6' height='3.6' rx='1' opacity='.3'/><path d='M5 8h5M5 8h2q1 0 1-1l.5-2.5q0-1 2-1M5 8h2q1 0 1 1l.5 2.5q0 1 2 1' fill='none' stroke='currentColor'/></svg>");
    }
    let heading=[box([icon]).accessibility_hidden(true).with_style(ctx.component_style("mark",style().width(u(4)).height(u(4)).flex_shrink(false).text_color(fill("mark")))),
        label(ctx,"title",title,style().typography("body").font_weight(600).min_width(px(0)).whitespace_nowrap().text_ellipsis())];
    let job=get(p,"job","");
    if job!=""{heading.push(label(ctx,"job",job,caption().typography("metadata").whitespace_nowrap().text_ellipsis()));}
    let count=ctx.format_number(m.stats.counts.done);
    let total=ctx.format_number(p.tasks.len);
    let money=money(ctx,p,m.stats.total_cost);
    let clock=if m.stats.finished{ctx.t_or("fan_out_grid.finished","Finished in {time}",#{time:took(ctx,m.stats.elapsed_ms)})}
        else if m.stats.eta_ms!=(){ctx.t_or("fan_out_grid.eta","~{time} left",#{time:took(ctx,m.stats.eta_ms)})}
        else{ctx.t_or("fan_out_grid.elapsed","{time} so far",#{time:took(ctx,m.stats.elapsed_ms)})};
    let flags=caption().font_feature("tnum",1).whitespace_nowrap();
    let amount=flags.font_weight(500).text_color(fill("ink"));
    let fraction=row([label(ctx,"metric",count,amount),label(ctx,"metric"," / "+total,flags.text_color(fill("muted")))])
        .with_style(style().items_center().gap(px(0)));
    let summary=row([fraction,label(ctx,"metric",money,amount),
        label(ctx,if m.stats.finished{["metric","terminal"]}else{["metric"]},clock,
            flags.text_color(if m.stats.finished{fill("done_ink")}else{fill("muted")}))])
        .with_style(ctx.component_style(if m.narrow{["summary","summary_compact"]}else{["summary"]},
            style().margin_start(auto()).gap_x(sp(3.5)).gap_y(sp(0.5)).items_center().flex_wrap()));
    let progress=ctx.format_number(m.stats.counts.done)+" "+tx(ctx,"done","Done")+" · "+ctx.format_number(m.stats.counts.failed)+" "+
        tx(ctx,"failed","Failed")+" · "+money+" · "+clock;
    heading.push(summary.accessibility_role("status").accessibility_label(progress));
    row(heading).with_style(ctx.component_style("header",style().min_width(px(0)).gap_x(sp(2)).gap_y(sp(1)).items_center().flex_wrap()))
}
fn button(n,cb){
    n.tab_stop(true).accessibility_role("button").on_click(cb)
        .on_key_value("plain+space",cb,()).on_key_value("plain+enter",cb,())
}
fn controls(ctx,p,m){
    let children=[];
    for id in states(){
        let n=m.stats.counts[id];
        if n==0&&id!="done"&&!m.narrow&&m.ui.filter!=id{continue;}
        let picked=m.ui.filter==id;let dim=m.ui.filter!=""&&!picked;let part=["filter"];
        if picked{part.push("filter_on");}else if dim{part.push("filter_dim");}
        if n==0&&id!="done"{part.push("filter_empty");}
        let style=style().min_height(u(7)).items_center().gap_x(sp(1.5)).padding_x(sp(2.25)).radius(theme_radius("sm")).cursor_pointer()
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-1),theme_color("focus_ring"));
        let mark=box([]).accessibility_hidden(true).with_style(ctx.component_style("swatch",style().width(u(2.25)).height(u(2.25)).flex_shrink(false)
            .radius(theme_radius("sm")).background(fill(id+"_bg")).border(px(1)).border_color(fill(id+"_edge"))));
        let number=ctx.format_number(n);let word=words(ctx,id).to_lower();
        let name=number+" "+word;
        let mark_count=label(ctx,["metric","filter_count"],number,style().whitespace_nowrap().font_feature("tnum",1).text_color(fill("ink")));
        let mark_word=label(ctx,"metric",word,style().whitespace_nowrap());
        children.push(button(row([mark,mark_count,mark_word]).with_key(id)
            .accessibility_label(name).accessibility_pressed(picked).with_style(ctx.component_style(part,style))
            .paint_transition(160,"ease_out"),Fn("set_filter").curry(p.run,id)));
    }
    let legend=if m.narrow{box(children).with_style(style().grid_cols_max_content(2).gap_x(sp(0.5)))}
        else{row(children).with_style(style().flex_wrap().gap_x(sp(0.5)))};
    let out=[legend.with_key("legends").accessibility_role("group").accessibility_label(tx(ctx,"highlight","Highlight task status"))
        .with_style(ctx.component_style(if m.narrow{["legend","legend_compact"]}else{["legend"]},style().min_width(px(0))))];
    if get(p,"on_retry",())!=()&&m.stats.failed.len>0{
        let name=tx(ctx,"retry","Retry failed");
        let mark=svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.25' stroke-linecap='round' stroke-linejoin='round'><path d='M13.3 6A5.5 5.5 0 1 0 13 10M13.3 2.6V6H9.9'/></svg>")
            .accessibility_hidden(true).with_style(style().width(u(3.5)).height(u(3.5)));
        let retry_base=style().items_center().padding_x(sp(2.5)).gap_x(sp(1.5)).min_height(u(7.5)).radius(theme_radius("sm"))
            .typography("caption").text_color(fill("failed_ink")).font_weight(500).margin_start(auto()).cursor_pointer()
            .hover(style().background(fill("failed_bg"))).focus_ring(px(2),offset_px(-1),theme_color("focus_ring"));
        out.push(button(row([mark,text(name)]).with_key("retry").accessibility_label(name)
            .with_style(ctx.component_style("retry",retry_base)),Fn("retry").curry(#{run:p.run,revision:p.revision,ids:m.stats.failed})));
    }
    row(out).with_key("footer").with_style(ctx.component_style("footer",style().min_width(px(0)).gap_x(sp(3)).gap_y(sp(1)).items_center().flex_wrap()))
}
fn tile_style(ctx,st,compact){
    let s=style().absolute().cursor_pointer().radius(theme_radius("sm")).background(fill(st+"_bg"))
        .border(px(1)).border_color(fill(st+"_edge")).hover(style().border_color(fill(st+"_ink")))
        .focus_ring(px(1.5),px(1),theme_color("focus_ring"));
    ctx.component_style(if compact{["cell","cell_compact"]}else{["cell"]},s)
}
fn dot(ctx,p,t,m,styles,i,animations,shine){
    let g=m.grid;let x=(i%g.columns).to_float()*g.pitch;let y=(i/g.columns).to_float()*g.stride;
    let dim=m.ui.filter!=""&&m.ui.filter!=t.status;
    let open=m.pin==t.id;let active=m.shown==t.id||m.ui.hover==t.id;
    let base=styles[t.status].inset_start(px(x)).top(px(y)).width(px(g.cell)).height(px(g.cell));
    if dim{base=base.merge(styles.dim);}
    let body=[];
    if animations&&t.status=="running"&&g.cell>0.0{
        body.push(box([]).with_key("glint").accessibility_hidden(true).with_style(shine)
            .motion(motion_transition("translate_x_fraction",-1.0,1.0,
                #{duration_ms:1300,delay_ms:(i%14)*97,iterations:"infinite",intent:"decorative"})));
        base=base.clip();
    }
    if active||open{
        // At the authored 1.3x scale, even the narrow 10/13 tracks do
        // not overlap their neighbors. GPUI projects the hit box with it.
        base=base.border_color(fill(t.status+"_ink")).opacity(1.0).transform(#{scale_x:1.3,scale_y:1.3});
        base=base.merge(if open{styles.pin}else{styles.attention});
    }
    let hint=words(ctx,t.status)+if get(t,"worker","")==""{""}else{" · "+t.worker};
    let target=box(body).with_key("task:"+t.id).with_ref(element_ref(letter(i))).accessibility_role("gridcell")
        .accessibility_label(t.label).accessibility_description(hint).accessibility_selected(open).tab_stop(m.cursor==t.id)
        .on_hover_change(Fn("attend").curry(p.run,"hover",t.id))
        .on_focus_within_change(Fn("attend").curry(p.run,"focus",t.id))
        .on_click(Fn("request_pick").curry(m.ticket,t.id))
        .with_style(base);
    if animations{
        target=target.paint_transition(300,"ease_out");
        if t.status=="retrying"&&!open&&!active{
            target=target.motion(motion_transition("opacity",if dim{0.12}else{0.55},if dim{0.2}else{1.0},
                #{duration_ms:1100,iterations:"infinite",autoreverse:true,intent:"decorative"}));
        }else if !dim{
            target=target.enter_motion(motion_transition("opacity",0.0,1.0,
                #{duration_ms:420,delay_ms:min(i,240)*2,easing:ease(),intent:"decorative"}));
        }
    }
    target
}
fn tooltip(ctx,p,m,shell_ref,grid_ref){
    let frame=ctx.element_bounds(shell_ref);
    let field=ctx.element_bounds(grid_ref);
    if frame==()||field==(){return ();}
    let origin_x=if ctx.text_direction()=="rtl"{
        frame.layout.x+frame.layout.width-field.layout.x-field.layout.width
    }else{field.layout.x-frame.layout.x};
    let origin_y=field.layout.y-frame.layout.y;
    let i=m.stats.ids[m.shown];let t=p.tasks[i];let g=m.grid;let st=t.status;
    let anchor_x=(i%g.columns).to_float()*g.pitch+g.cell/2.0;
    let top=(i/g.columns).to_float()*g.stride;
    let nw=max(0.0,m.width-16.0);
    let width=min(290.0,nw);
    let root=element_ref("tip");
    let size=ctx.element_size(root);
    let height=if size==(){78.0}else{size.height};
    let x=max(8.0,min(anchor_x-width/2.0,m.width-width-8.0));
    let y=if top-height-10.0>=4.0{top-height-10.0}else{top+g.cell+10.0};
    let list=[];
    let start=get(t,"start_ms",());let end=get(t,"end_ms",());
    let worker=get(t,"worker","");if worker!=""{list.push(worker);}
    if start!=(){list.push(took(ctx,if end!=(){end-start}else{max(0,p.now_ms-start)}));}
    let cost=get(t,"cost_micros",());if cost!=(){list.push(money(ctx,p,cost));}
    let tries=get(t,"attempts",1);
    if tries>1{list.push(ctx.format_number(tries)+" "+tx(ctx,"attempts","attempts"));}
    let meta="";
    for item in list{if meta!=""{meta+=" · ";}meta+=item;}
    if meta==""{meta=tx(ctx,"worker","Waiting for a worker");}
    let pill=label(ctx,"tip_status",words(ctx,st),caption().flex_shrink(false).padding_x(sp(2)).font_weight(500)
        .radius(theme_radius("sm")).background(fill(st+"_bg")).text_color(fill(st+"_ink")));
    let caption_words=label(ctx,"tip_title",t.label,style().typography("body").font_weight(600).line_clamp(2).min_width(px(0)).flex_grow());
    let v=[row([caption_words,pill]).with_style(style().min_width(px(0)).items_start().gap_x(sp(2))),
        label(ctx,"tip_meta",meta,caption().font_feature("tnum",1).line_clamp(2))];
    if st=="done"&&get(t,"result","")!=""{v.push(label(ctx,"tip_result",t.result,caption().line_clamp(4)));}
    if st in ["failed","retrying"]&&get(t,"error","")!=""{v.push(label(ctx,"tip_error",t.error,caption().line_clamp(4).text_color(fill("failed_ink"))));}
    column(v).with_key("tip").with_ref(root).accessibility_role("tooltip").accessibility_label(t.label)
        .accessibility_description(words(ctx,st)+" · "+meta)
        .with_style(ctx.component_style("tip",style().absolute().inset_start(offset_px(origin_x+x)).top(offset_px(origin_y+y)).width(px(width))
            .min_width(px(0)).padding_x(sp(3.5)).padding_y(sp(2.5)).gap_y(sp(0.75)).radius(theme_radius("lg")).background(theme_color("surface_raised"))))
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:160,easing:ease(),intent:"feedback"}))
}
fn field(ctx,p,m,reference){
    if p.tasks.len==0{
        return column([label(ctx,"empty",tx(ctx,"empty","No tasks in this job"),caption().padding_y(sp(2)))])
            .with_ref(reference).with_key("field").with_style(style().width(relative(1.0)).min_width(px(0)));
    }
    let styles=#{dim:ctx.component_style("cell_dim",style().opacity(0.2)),attention:ctx.component_style("cell_attention",style()),
        pin:ctx.component_style("cell_pinned",style())};
    for st in ["queued","running","retrying","done","failed"]{styles[st]=tile_style(ctx,st,m.narrow);}
    let clear=rgba(0x00000000);let shine=ctx.component_style("sheen",style().absolute().top(px(0)).inset_start(px(0))
        .width(relative(1.0)).height(relative(1.0)).layered_gradient(layered_gradient([#{angle:110,
            stops:[#{offset:0.25,color:clear},#{offset:0.5,color:fill("shine")},#{offset:0.75,color:clear}]}])).opacity(0.8));
    let animate=ctx.motion_preference()=="normal";
    let tiles=[];for i in 0..p.tasks.len{tiles.push(dot(ctx,p,p.tasks[i],m,styles,i,animate,shine));}
    let idx=m.stats.ids[m.cursor];
    let n=p.tasks.len;let rtl=ctx.text_direction()=="rtl";
    let cols=m.grid.columns;
    let region=box(tiles).with_key("field").with_ref(reference).accessibility_role("grid").accessibility_label(tx(ctx,"grid","Tasks"))
        .accessibility_description(tx(ctx,"keys","Arrow keys move; Enter pins the task; Escape clears it."))
        .with_style(ctx.component_style("field",style().relative().min_width(px(0)).width(relative(1.0)).height(px(m.grid.height))))
        .on_key_value("plain+left",Fn("seek").curry(letter(max(0,min(n-1,idx+if rtl{1}else{-1}))),m.ticket),())
        .on_key_value("plain+right",Fn("seek").curry(letter(max(0,min(n-1,idx+if rtl{-1}else{1}))),m.ticket),())
        .on_key_value("plain+up",Fn("seek").curry(letter(max(0,idx-cols)),m.ticket),())
        .on_key_value("plain+down",Fn("seek").curry(letter(min(n-1,idx+cols)),m.ticket),())
        .on_key_value("plain+home",Fn("seek").curry(letter(0),m.ticket),())
        .on_key_value("plain+end",Fn("seek").curry(letter(n-1),m.ticket),())
        .on_key_value("plain+enter",Fn("request_pick").curry(m.ticket,m.cursor),())
        .on_key_value("plain+space",Fn("request_pick").curry(m.ticket,m.cursor),())
        .on_key_value("plain+escape",Fn("clear").curry(m.ticket),());
    region
}
fn render_fan_out(ctx,p){
    if p.run==""||utf8_byte_count(p.run)>96||utf8_byte_count(get(p,"currency_prefix","$"))>12{throw "FanOutGrid requires a bounded run identity and currency prefix";}
    for t in p.tasks{
        if t.label==""||utf8_byte_count(t.label)>512||utf8_byte_count(get(t,"worker",""))>512||
            utf8_byte_count(get(t,"result",""))>4096||utf8_byte_count(get(t,"error",""))>4096{throw "FanOutGrid labels and details must be bounded";}
    }
    let shell=element_ref("fan-out-grid");let reference=element_ref("field");
    let size=ctx.element_size(reference);
    let width=if size==(){560.0}else{min(32768.0,size.width)};
    let narrow=ctx.element_width_class(shell,[520])==0;
    let grid=fan_out_layout(width,if narrow{10}else{12},if narrow{3}else{4},p.tasks.len);
    let stats=fan_out_summary(p.tasks,p.now_ms,get(p,"workers",-1));
    let ui=local(ctx,p.run);
    let controlled=get(p,"pinned",())!=();
    let pinned=get(p,"pinned",ui.pin);if !(pinned in stats.ids){pinned="";}
    let focus=if ui.cursor in stats.ids{ui.cursor}else if pinned!=""{pinned}else if p.tasks.len>0{p.tasks[0].id}else{""};
    let hot=if ui.hover in stats.ids{ui.hover}else if ui.focus in stats.ids{ui.focus}else{""};
    let chosen=if pinned!=""{pinned}else if !ui.suppressed{hot}else{""};
    let m=#{grid:grid,stats:stats,ui:ui,narrow:narrow,pin:pinned,shown:chosen,width:width,cursor:focus,
        ticket:#{run:p.run,revision:p.revision,pin:pinned,controlled:controlled}};
    effect("sync_run",#{run:p.run},Fn("sync_run"),Fn("idle"));
    // Same retained branch in both designs. Modern paints the inner grid
    // panel; Clean paints the body containing the grid and its controls.
    let stretch=style().min_width(px(0)).width(relative(1.0));
    let panel=column([field(ctx,p,m,reference)]).with_key("panel:"+p.run)
        .with_style(ctx.component_style(if narrow{["panel","panel_compact"]}else{["panel"]},stretch));
    let body=column([panel,controls(ctx,p,m)]).with_key("body")
        .with_style(ctx.component_style(if narrow{["body","body_compact"]}else{["body"]},stretch));
    let sections=[ttl(ctx,p,m),body];
    if chosen!=""&&width>=64.0{
        let hint=tooltip(ctx,p,m,shell,reference);
        if hint!=(){sections.push(hint);}
    }
    column(sections).with_key("fan-out-grid").with_ref(shell)
        .with_style(ctx.component_style("root",style().relative().min_width(px(0)).width(relative(1.0)).padding(sp(1))
            .radius(theme_radius("lg")).gap_y(sp(1)).background(fill("tray"))))
}
