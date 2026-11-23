/* gpui-rhai
{"id":"patterns/agent_monitor","export":"AgentMonitor","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/popover","components/spinner","patterns/agent_tree","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","space.spacing_unit","space.unit","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted","text_primary","typography.body","typography.caption","typography.code"],"environment":[]}
*/
// A single accepted run, its activity clock, and recent steps with native inspection.
// Example: AgentMonitor(#{key:"monitor",run:"preview",revision:0,now_ms:0,state:"idle",steps:[]})
import "components/button" as button;
import "components/popover" as popover;
import "components/spinner" as spinner;
import "patterns/agent_tree" as team;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_monitor","export":"AgentMonitor",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/popover","components/spinner","patterns/agent_tree","patterns/approval_card"],capabilities:#{},
        tokens:["accent","border","focus_ring","space.spacing_unit","space.unit","surface_hover","surface_raised","text.danger","text.success",
            "text.warning","text_muted","text_primary","typography.body","typography.caption","typography.code"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable monitor instance key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business run ID; a retry may publish a new ID."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted business revision, independent of clock ticks."},
        now_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Authoritative Host clock, never a script timer."},
        state:#{schema:#{type:"string",allowed:["idle","running","waiting","paused","finished","failed","cancelled"]},
            required:true,sensitive:false,doc:"Accepted run state."},
        name:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Agent name in the folded bar and open monitor."},
        role:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Agent role or model caption."},
        mark:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative agent avatar or logo."},
        started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Start in the Host clock domain."},
        ended_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Fixed endpoint for a started terminal run."},
        paused_at_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Fixed endpoint while paused."},
        paused_ms:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,
            doc:"Completed pause intervals to subtract, excluding the current pause."},
        cost_micros:#{schema:#{type:"optional",value:#{type:"integer",min:-1000000000000,max:1000000000000}},required:false,sensitive:false,
            doc:"Run cost or credit in millionths of the currency unit."},
        currency:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Currency caption; defaults to USD."},
        steps:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable step ID. The history row and its duration segment share it."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Short action caption."},
            kind:#{schema:#{type:"optional",value:#{type:"string",allowed:["read","search","edit","run","think","web"]}},
                required:false,sensitive:false,doc:"Tool kind used for its decorative tile."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional override for the tool tile."},
            state:#{schema:#{type:"string",allowed:["queued","running","done","failed"]},required:true,sensitive:false,doc:"Accepted step state."},
            started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Host start of a running step."},
            elapsed_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Fixed accepted elapsed time of a terminal step."},
            paused_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Completed pause intervals affecting this step."},
            tool:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Tool or system caption."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Step description shown in the inspector."},
            output:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Literal text shown without evaluating or executing it."},
        }}},required:true,sensitive:true,doc:"Steps in chronological display order. At most one may be running."},
        current:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable presentation identity of the current caption."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Current work caption, independent from the history."},
            kind:#{schema:#{type:"optional",value:#{type:"string",allowed:["read","search","edit","run","think","web"]}},required:false,sensitive:false,doc:"Tile kind."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative tool override."},
        }}},required:false,sensitive:true,doc:"Override only the activity well; not a new running step."},
        ask:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable request ID required in Allow/Decline proposals."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Approval question."},
            command:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Exact command or input displayed as literal evidence."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Tool mark for the request."},
        }}},required:false,sensitive:true,doc:"Approved, versioned evidence from the Host, not an executable command."},
        outcome:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            added:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Added lines."},
            removed:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Removed lines."},
            files:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Touched files."},
            note:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Outcome caption."},
        }}},required:false,sensitive:true,doc:"Summary displayed when finished."},
        error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short failure evidence."},
        default_folded:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Begin in the compact bar."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disable run mutations while the Host is resolving a proposal."},
        actions:#{schema:#{type:"optional",value:#{type:"array",max_items:7,items:#{type:"string",allowed:["pause","resume","cancel","allow","decline","retry","open_log"]}}},
            required:false,sensitive:false,doc:"Enable a subset of actions. Omit for all valid actions when there is an on_action callback."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives proposals, not optimistic runtime state changes."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host feedback; a rejection leaves the accepted state unchanged."},
    },state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{
        action:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run being acted on."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Observed snapshot revision."},
            action:#{schema:#{type:"string",allowed:["pause","resume","cancel","allow","decline","retry","open_log"]},required:true,sensitive:false,doc:"Requested action."},
            step:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Current accepted step ID, or empty."},
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Exact request ID for allow or decline, otherwise empty."},
        }},doc:"Host validates run, revision, step and request before applying a transition."},
    },slots:#{},parts:["root","card","card_folded","bar","bar_folded","agent","mark","name","role","fold","tools","tool","tool_live","status","status_running","status_waiting",
        "status_paused","status_finished","status_failed","cost","clock","well","well_waiting","well_failed","well_body","well_label","command","outcome",
        "outcome_add","outcome_remove","strip","segment","segment_done","segment_failed","segment_running","segment_hot","segment_waiting",
        "history","history_content","history_row","history_hot","history_name","history_time","details","details_title","details_kind","details_output","footer","actions",
        "steer","counter","notice"],effects:["prune_history"]},render:Fn("render_AgentMonitor")
});

fn AgentMonitor(props){render_component("patterns/agent_monitor",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,d){common::value_or(m,k,d)}
fn monitor_unit(n){theme_length("space.unit").times(n)}
fn monitor_gap(n){theme_length("space.spacing_unit").times(n)}
fn note(ctx,s,part){common::heading_note(ctx,s,part)}
fn accepted_clock(props){
    let since=opt(props,"started_ms");let at=props.now_ms;
    let end=opt(props,"ended_ms");let halt=opt(props,"paused_at_ms");
    if props.state=="paused"{
        if halt==(){throw "Paused monitor snapshots need paused_at_ms";}
        at=halt;
    }else if props.state in ["finished","failed","cancelled"]&&since!=(){
        if end==(){throw "Started terminal monitor snapshots need ended_ms";}
        at=end;
    }
    if at>props.now_ms||(since!=()&&at<since){throw "Monitor endpoints must be inside the accepted run clock";}
    #{instant:at,elapsed:if since==(){0}else{max(0,at-since-props.paused_ms)}}
}
fn spent(s,clock){
    if s.state!="running"{return value(s,"elapsed_ms",0);}
    let start=opt(s,"started_ms");if start==(){return value(s,"elapsed_ms",0);}
    max(0,max(0,clock.instant-start)-value(s,"paused_ms",0))
}
fn project(props,clock){
    let all=#{};let history=[];let active=();let started=0;let total=0.0;
    for i in 0..props.steps.len{
        let s=props.steps[i];
        if !team::valid(s.id)||s.id in all{throw "Monitor steps need stable unique IDs";}
        let t=spent(s,clock);
        let entry=#{data:s,ordinal:i,elapsed:t};all[s.id]=entry;
        if s.state!="queued"{history.push(entry);started+=1;total+=max(1,t).to_float();}
        if s.state=="running"{if active!=(){throw "A monitor has only one running step";}active=entry;}
    }
    #{all:all,history:history,active:active,started:started,total:max(1.0,total)}
}
fn local(ctx,run){
    let s=ctx.get_state("ui");if s!=()&&s.run==run{return s;}
    #{run:run,folded:(),inspect:"",hot:""}
}
fn prune_history(ctx,p){
    let ui=local(ctx,p.run);let update=false;
    if ui.inspect!=""&&!(ui.inspect in p.keys){ui.inspect="";update=true;}
    if ui.hot!=""&&!(ui.hot in p.keys){ui.hot="";update=true;}
    if update{ctx.set_state("ui",ui);}
}
fn stop_history(ctx,p){}
fn toggle(token,folded,ctx,e){
    let ui=local(ctx,token.run);ui.folded=!folded;ui.inspect="";ui.hot="";
    ctx.set_state("ui",ui);ctx.focus("monitor-fold");handled()
}
fn hover_step(run,id,ctx,inside){
    let ui=local(ctx,run);let next=if inside{id}else if ui.hot==id{""}else{ui.hot};
    if next!=ui.hot{ui.hot=next;ctx.set_state("ui",ui);}
}
fn inspect_step(run,id,ctx,open){
    let ui=local(ctx,run);
    if open{ui.inspect=id;}
    else if ui.inspect==id{ui.inspect="";}
    ctx.set_state("ui",ui);
}
fn inspect_in_history(run,id,key,ctx,open){
    inspect_step(run,id,ctx,open);
    if open{common::reveal_in(ctx,key,"monitor-history-viewport","monitor-history-content");}
}
fn show_from_strip(run,id,key,ctx,e){
    inspect_in_history(run,id,key,ctx,true);handled()
}
fn close_step(run,id,key,ctx,e){
    inspect_step(run,id,ctx,false);ctx.focus(key);handled()
}
fn focus_request(ctx,e){ctx.focus("monitor-request");handled()}
fn dispatch(cfg,kind,ctx,e){
    ctx.emit("action",#{run:cfg.run,revision:cfg.revision,action:kind,step:cfg.step,
        request:if kind in ["allow","decline"]{cfg.request}else{""}});
    if kind in ["allow","decline","retry","cancel"]{ctx.focus("monitor-fold");}
    handled()
}
fn allowed(props,cfg,kind){
    cfg.connected&&(kind=="open_log"||!props.busy)&&listed_action(cfg,kind)
}
fn listed_action(cfg,kind){cfg.actions==()||kind in cfg.actions}
fn run_phrase(ctx,state){
    switch state{
        "running"=>ctx.t_or("agent_queue.running","Running"),
        "waiting"=>ctx.t_or("agent_monitor.waiting","Waiting on you"),
        "paused"=>ctx.t_or("agent_monitor.paused","Paused"),
        "finished"=>ctx.t_or("agent_monitor.finished","Finished"),
        "failed"=>ctx.t_or("agent_queue.failed","Failed"),
        "cancelled"=>ctx.t_or("agent_queue.cancelled","Cancelled"),
        _=>ctx.t_or("agent_graph.idle","Idle"),
    }
}
fn elapsed_phrase(ctx,ms){
    let s=ms/1000;
    if s<60{return ctx.t_or("agent_lanes.seconds","{n} s",#{n:ctx.format_number(s)});}
    ctx.t_or("agent_inbox.minutes_seconds","{m} min {s} s",#{m:ctx.format_number(s/60),s:ctx.format_number(s%60)})
}
fn clock_digits(ctx,ms){
    let s=ms/1000;let r=s%60;
    ctx.format_number(s/60)+":"+(if r<10{ctx.format_number(0)}else{""})+ctx.format_number(r)
}
fn step_key(entry){"monitor-step-"+entry.ordinal.to_string()}
fn tile(ctx,a,part){
    let icon=opt(a,"icon");let kind=value(a,"kind","think");
    let p=switch kind{
        "read"=>"M6 3h7l3 3v11H6zM9 8h4m-4 4h4",
        "search"=>"M12 9a4 4 0 1 0-8 0 4 4 0 1 0 8 0m-1 3 5 5",
        "edit"=>"M5 15 14 6l3 3-9 9H5z",
        "run"=>"M3 4h14v12H3zM6 7l3 3-3 3m5 0h3",
        "web"=>"M17 10a7 7 0 1 0-14 0 7 7 0 1 0 14 0M3 10h14M10 3v14",
        _=>"M10 3v14M3 10h14M5 5l10 10m0-10L5 15",
    };
    let child=if icon!=(){icon}else{svg(`<svg viewBox='0 0 20 20' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><path d='${p}'/></svg>`)
        .with_style(style().width(px(14)).height(px(14)))};
    row([child]).with_key("tile").accessibility_hidden(true).with_style(ctx.component_style(part,style()
        .width(monitor_unit(5)).height(monitor_unit(5)).items_center().justify_center().flex_shrink(false).radius(px(5))
        .background(theme_color("surface_raised"))))
}
fn used_tools(ctx,props,data){
    let recent=[];let seen=#{};let omitted=0;
    for i in 0..data.history.len{
        let s=data.history[data.history.len-i-1].data;
        let kind=value(s,"tool",value(s,"kind","think"));
        if kind in seen{continue;}
        seen[kind]=true;
        if recent.len<4{recent.insert(0,s);}else{omitted+=1;}
    }
    let items=[];let current=if data.active==(){""}else{data.active.data.id};
    if omitted>0{items.push(note(ctx,"+"+ctx.format_number(omitted),"history_time").with_key("more"));}
    for s in recent{items.push(tile(ctx,s,if s.id==current&&props.state=="running"{"tool_live"}else{"tool"}).with_key("tool:"+s.id));}
    row(items).with_key("tools").with_style(ctx.component_style("tools",style().gap(monitor_gap(0.5)).flex_wrap().items_center()))
}
fn header(ctx,props,cfg,data,clock,ref_fold){
    let name=value(props,"name",ctx.t_or("agent_monitor.agent","Agent"));let role=opt(props,"role");let mark=opt(props,"mark");
    let label=text(name).with_key("name").accessibility_role("heading").accessibility_level(2)
        .with_style(ctx.component_style("name",style().min_width(px(0)).typography("body").font_weight(600)));
    let notes=[label];if role!=()&&!cfg.folded{notes.push(note(ctx,role,"role").with_key("role"));}
    let identity=row([row([if mark!=(){mark}else{text(if name==""{"•"}else{name.sub_string(0,1).to_upper()})}]).with_key("mark").accessibility_hidden(true)
        .with_style(ctx.component_style("mark",style().width(monitor_unit(6)).height(monitor_unit(6)).items_center().justify_center().flex_shrink(false)
            .radius(px(6)).background(theme_color("surface_raised")).typography("caption").font_weight(600))),
        column(notes).with_key("name").with_style(style().min_width(px(0)))])
        // The avatar and stacked name/model intentionally use different type sizes.
        .with_key("agent").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("agent",style().gap(monitor_gap(1.25)).items_center().min_width(px(0))));
    let bits=[if cfg.folded{identity}else{identity.with_style(style().flex_grow())}];
    if cfg.folded{bits.push(used_tools(ctx,props,data).with_style(style().flex_grow()));}
    let cost=opt(props,"cost_micros");
    if cost!=(){bits.push(note(ctx,team::amount(ctx,cost,cfg.currency),"cost").with_key("cost"));}
    let phase=props.state;let active=phase in ["running","waiting","paused","finished","failed"];
    let label=run_phrase(ctx,phase);
    bits.push(text(label).with_key("state").accessibility_role("status").with_style(ctx.component_style(
        if active{["status","status_"+phase]}else{["status"]},style().typography("caption")
        .padding_x(monitor_gap(1.25)).padding_y(monitor_gap(0.25)).radius(px(5)).background(theme_color("surface_hover")))));
    if cfg.folded&&opt(props,"started_ms")!=(){bits.push(clock_node(ctx,clock));}
    let fold=button::Button(#{key:"monitor-fold",text:if cfg.folded{"⌄"}else{"⌃"},variant:"ghost",size:"xs",
        on_click:Fn("toggle").curry(cfg.token,cfg.folded)}).with_key("fold").with_ref(ref_fold).accessibility_expanded(!cfg.folded)
        .accessibility_label(if cfg.folded{ctx.t_or("agent_monitor.unfold","Open monitor")}else{ctx.t_or("agent_monitor.fold","Fold monitor")})
        .with_style(ctx.component_style("fold",style().flex_shrink(false)));
    bits.push(fold);
    row(bits).with_key("header").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style(if cfg.folded{["bar","bar_folded"]}else{["bar"]},style().min_width(px(0))
            .gap(monitor_gap(1.5)).items_center().justify_between().flex_wrap()))
}
fn clock_node(ctx,clock){
    text(clock_digits(ctx,clock.elapsed)).with_key("clock").accessibility_role("status")
        .accessibility_label(ctx.t_or("agent_monitor.elapsed","Elapsed: {time}",#{time:elapsed_phrase(ctx,clock.elapsed)}))
        .with_style(ctx.component_style("clock",style().typography("caption").font_feature("tnum",1).flex_shrink(false)))
}
fn activity(ctx,props,cfg,data,clock){
    let phase=props.state;let ask=opt(props,"ask");
    let active=if data.active==(){()}else{data.active.data};
    let item=value(props,"current",active);
    let change=phase;let title="";let children=[];
    if phase=="running"&&item!=(){
        change+=":"+item.id;title=item.label;
        children.push(tile(ctx,item,"tool_live"));
    }else if phase=="waiting"{
        if ask!=(){change+=":"+ask.id;title=ask.title;children.push(tile(ctx,ask,"tool"));}
        else{title=ctx.t_or("agent_monitor.waiting","Waiting on you");}
    }else if phase=="paused"{
        title=ctx.t_or("agent_monitor.paused","Paused");if item!=(){title+=" · "+item.label;}
    }else if phase=="finished"{
        title=ctx.t_or("agent_monitor.finished_in","Finished in {time}",#{time:elapsed_phrase(ctx,clock.elapsed)});
    }else if phase=="failed"{title=value(props,"error",ctx.t_or("agent_monitor.error","Something went wrong"));}
    else if phase=="cancelled"{title=ctx.t_or("agent_monitor.cancelled_after","Cancelled after {time}",#{time:elapsed_phrase(ctx,clock.elapsed)});}
    else if phase=="running"{title=ctx.t_or("agent_queue.running","Running");}
    else{title=ctx.t_or("agent_monitor.empty","No task yet");}
    children.push(text(title).with_key("label").with_style(ctx.component_style("well_label",style().min_width(px(0)).typography("body").font_weight(500))));
    let body=[row(children).with_key("line").audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).gap(monitor_gap(1.25))
        .items_center().flex_wrap())];
    if phase=="waiting"&&ask!=()&&opt(ask,"command")!=(){
        body.push(text(ask.command).with_key("command").with_style(ctx.component_style("command",style().min_width(px(0)).typography("code"))));
    }
    let resolved=opt(props,"outcome");
    if phase=="finished"&&resolved!=(){
        let stats=[];let a=opt(resolved,"added");let r=opt(resolved,"removed");let f=opt(resolved,"files");let n=opt(resolved,"note");
        if a!=(){stats.push(note(ctx,"+"+ctx.format_number(a),"outcome_add").with_key("add").with_style(style().text_color(theme_color("text.success"))));}
        if r!=(){stats.push(note(ctx,"−"+ctx.format_number(r),"outcome_remove").with_key("remove").with_style(style().text_color(theme_color("text.danger"))));}
        if f!=(){stats.push(note(ctx,ctx.t_or("agent_monitor.files","Files: {n}",#{n:ctx.format_number(f)}),"outcome").with_key("files"));}
        if n!=(){stats.push(note(ctx,n,"outcome").with_key("note"));}
        body.push(row(stats).with_key("outcome").with_style(style().min_width(px(0)).gap(monitor_gap(1.25)).flex_wrap()));
    }
    let line=column(body).with_key("well-content").with_style(ctx.component_style("well_body",style().min_width(px(0)).flex_grow().gap(monitor_gap(1))))
        .motion(motion_transition("translate_y_fraction",0.22,0.0,#{duration_ms:ctx.motion_duration("normal"),intent:"decorative"}))
        .motion(motion_transition("opacity",0.3,1.0,#{duration_ms:ctx.motion_duration("normal"),intent:"decorative"}))
        .motion_on_change(cfg.run+":"+change);
    let nodes=[line];
    if phase=="running"{nodes.push(spinner::Spinner(#{key:"work-indicator",label:ctx.t_or("agent_queue.running","Running"),size:"xs"}).accessibility_hidden(true));}
    if opt(props,"started_ms")!=(){nodes.push(clock_node(ctx,clock));}
    let parts=if phase in ["waiting","failed"]{["well","well_"+phase]}else{["well"]};
    let n=row(nodes).with_key("well").accessibility_role("region").accessibility_label(ctx.t_or("agent_monitor.activity","Current activity"))
        .audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style(parts,style().min_width(px(0)).padding(monitor_gap(2))
            .gap(monitor_gap(1.5)).items_center().flex_wrap().clip().radius(monitor_unit(2.5)).background(theme_color("surface_hover"))));
    // Approval shortcuts belong to this explicitly focused request, never to the view.
    if phase=="waiting"&&ask!=(){
        let yes=allowed(props,cfg,"allow");let no=allowed(props,cfg,"decline");
        if yes||no{n=n.with_ref(element_ref("monitor-request")).tab_stop(true).on_click(Fn("focus_request"))
            .accessibility_key_shortcuts(if yes&&no{"Enter Escape"}else if yes{"Enter"}else{"Escape"});}
        if yes{n=n.on_key_value("plain+enter",Fn("dispatch").curry(cfg.token,"allow"),());}
        if no{n=n.on_key_value("plain+escape",Fn("dispatch").curry(cfg.token,"decline"),());}
    }
    n
}
fn segment(ctx,cfg,data,e,interactive,hot){
    let s=e.data;let weight=max(1,e.elapsed).to_float()/data.total;
    let inks=theme_color(if s.state=="failed"{"text.danger"}else if s.state=="running"{"accent"}else{"border"});
    let parts=["segment","segment_"+s.state];if hot==s.id{parts.push("segment_hot");}
    let foot=box([]).with_key("mark").accessibility_hidden(true).with_style(ctx.component_style(parts,
        style().height(px(3)).width(relative(1.0)).radius(px(2)).background(inks)));
    let box=row([foot]).with_key("seg:"+s.id).with_style(style().flex_basis(px(0)).min_width(px(3)).flex_grow(weight)
        .height(if interactive{monitor_unit(4)}else{px(7)}).items_center());
    if !interactive{return box.accessibility_hidden(true);}
    box.accessibility_role("button").accessibility_label(ctx.t_or("agent_monitor.segment","Duration: {step}",#{step:s.label}))
        .accessibility_description(elapsed_phrase(ctx,e.elapsed)).tab_stop(false)
        .on_hover_change(Fn("hover_step").curry(cfg.run,s.id)).on_click(Fn("show_from_strip").curry(cfg.run,s.id,step_key(e)))
}
fn strip(ctx,props,cfg,data,interactive,hot){
    let nodes=[];for e in data.history{nodes.push(segment(ctx,cfg,data,e,interactive,hot));}
    if props.state=="waiting"{nodes.push(box([]).with_key("pending").accessibility_hidden(true)
        .with_style(ctx.component_style("segment_waiting",style().width(px(8)).height(px(3)).radius(px(2))
            .background(theme_color("text.warning")))));}
    let base=style().min_width(px(0)).gap(monitor_gap(0.5)).items_center();
    let n=row(nodes).with_key("strip");
    if interactive{n=n.scrollbars("hidden","hidden");base=base.overflow_x_scroll();}
    else{base=base.clip();}
    n.with_style(ctx.component_style("strip",base))
}
fn inspect_panel(ctx,cfg,e){
    let s=e.data;let status=team::status(ctx,s.state);
    let close=button::Button(#{key:"close:"+s.id,text:"×",size:"xs",variant:"ghost",on_click:Fn("close_step").curry(cfg.run,s.id,step_key(e))})
        .with_key("close").accessibility_label(ctx.t_or("agent_monitor.detail_close","Close step details"));
    let rows=[row([tile(ctx,s,"tool"),text(s.label).with_key("name").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("details_title",style().min_width(px(0)).typography("body").flex_grow().font_weight(600))),close])
        .with_key("head").audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).gap(monitor_gap(1.25)).items_center().flex_wrap()),
        note(ctx,value(s,"tool",value(s,"kind","think"))+" · "+status+" · "+elapsed_phrase(ctx,e.elapsed),"details_kind").with_key("kind")];
    let detail=opt(s,"detail");if detail!=(){rows.push(note(ctx,detail,"details_kind").with_key("detail"));}
    let out=opt(s,"output");
    if out!=(){rows.push(text(out).with_key("output").with_style(ctx.component_style("details_output",style().min_width(px(0))
        .padding(monitor_gap(1.25)).background(theme_color("surface_hover")).radius(px(5)).typography("code"))));}
    column(rows).with_key("inspect").audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("details",style().min_width(px(0)).gap(monitor_gap(1.25))))
}
fn history_row(ctx,cfg,e,ui,ref_item,viewport){
    let s=e.data;let showing=false;
    if ui.inspect==s.id{
        let target=ctx.element_bounds(ref_item);let vp=ctx.element_bounds(viewport);
        if target!=()&&vp!=(){
            let r=target.visual;let b=vp.visual;
            // Do not place a connected overlay from an old, clipped anchor.
            let visible=min(r.y+r.height,b.y+b.height)-max(r.y,b.y);
            showing=visible>=max(1.0,min(r.height,b.height)-1.0);
        }
    }
    let caption=text(s.label).with_key("caption").with_style(ctx.component_style("history_name",style().min_width(px(0)).flex_grow()
        .typography("caption").text_ellipsis().whitespace_nowrap()));
    let time=if s.state=="failed"{ctx.t_or("agent_queue.failed","Failed")}else{elapsed_phrase(ctx,e.elapsed)};
    let parts=if ui.hot==s.id{["history_row","history_hot"]}else{["history_row"]};
    let item=row([tile(ctx,s,"tool"),caption,note(ctx,time,"history_time").with_key("time")]).with_key("trigger").with_ref(ref_item)
        .accessibility_role("button").accessibility_label(ctx.t_or("agent_monitor.details","Details for {step}",#{step:s.label}))
        .accessibility_description(team::status(ctx,s.state)+" · "+time).accessibility_expanded(showing).tab_stop(true)
        .audit_allow(["mixed-type-in-row"]).on_hover_change(Fn("hover_step").curry(cfg.run,s.id))
        .with_style(ctx.component_style(parts,style().min_width(px(0)).min_height(monitor_unit(7)).gap(monitor_gap(1))
            .padding_x(monitor_gap(1)).items_center().radius(px(5)).focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))));
    popover::Popover(#{key:"info:"+s.id,trigger:item,label:s.label,open:showing,placement:"bottom",align:"start",initial_focus:"first",
        content:if showing{inspect_panel(ctx,cfg,e)}else{column([])},on_open_change:Fn("inspect_in_history").curry(cfg.run,s.id,step_key(e))})
        .with_key("item:"+s.id).accessibility_role("listitem").accessibility_label(s.label)
}
fn history(ctx,cfg,data,ui){
    let rows=[];let vp=element_ref("monitor-history-viewport");
    for i in 0..data.history.len{
        let e=data.history[data.history.len-i-1];
        rows.push(history_row(ctx,cfg,e,ui,element_ref(step_key(e)),vp));
    }
    let items=column(rows).with_key("rows").with_ref(element_ref("monitor-history-content"))
        .with_style(ctx.component_style("history_content",style().min_width(px(0)).gap(monitor_unit(0.5)).flex_shrink(false)));
    column([items]).with_key("history").with_ref(vp).accessibility_role("list").accessibility_label(ctx.t_or("agent_monitor.history","Recent steps"))
        .scrollbars("hidden","auto").with_style(ctx.component_style("history",style().min_width(px(0))
            .max_height(monitor_unit(22)).overflow_y_scroll()))
}
fn action_mark(action){
    let p=switch action{
        "pause"=>"M7 5v10m6-10v10",
        "resume"=>"M6 4l10 6-10 6z",
        "cancel"=>"M5 5h10v10H5z",
        "retry"=>"M4 8a6 6 0 1 1 0 5M4 4v4h4",
        "open_log"=>"M5 3h7l4 4v10H5zM8 10h5m-5 3h5",
        _=>"",
    };
    if p==""{return ();}
    svg(`<svg viewBox='0 0 20 20' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><path d='${p}'/></svg>`)
        .accessibility_hidden(true).with_style(style().width(px(13)).height(px(13)))
}
fn control(ctx,cfg,key,action,label,variant,disabled){
    button::Button(#{key:key,text:label,size:"xs",variant:variant,disabled:disabled,
        prefix:action_mark(action),on_click:Fn("dispatch").curry(cfg.token,action)}).with_key(key)
}
fn footer(ctx,props,cfg,data){
    let buttons=[];let phase=props.state;
    let expose=cfg.connected;
    if expose&&phase in ["running","paused"]{
        let p=phase=="paused";let a=if p{"resume"}else{"pause"};
        if listed_action(cfg,a){
            buttons.push(control(ctx,cfg,"steer",a,if p{ctx.t_or("agent_monitor.resume","Resume")}else{ctx.t_or("agent_monitor.pause","Pause")},
                "secondary",props.busy).with_style(ctx.component_style("steer",style())));
        }
        if listed_action(cfg,"cancel"){buttons.push(control(ctx,cfg,"cancel","cancel",ctx.t_or("agent_monitor.cancel","Cancel"),"secondary",props.busy));}
    }else if expose&&phase=="waiting"&&cfg.token.request!=""{
        if listed_action(cfg,"decline"){buttons.push(control(ctx,cfg,"decline","decline",ctx.t_or("agent_monitor.decline","Decline"),"secondary",props.busy));}
        if listed_action(cfg,"allow"){buttons.push(control(ctx,cfg,"allow","allow",ctx.t_or("agent_monitor.allow","Allow"),"primary",props.busy));}
    }else if expose&&phase=="failed"&&listed_action(cfg,"retry"){
        buttons.push(control(ctx,cfg,"retry","retry",ctx.t_or("agent_monitor.retry","Try again"),"primary",props.busy));
    }
    let output=[];
    if buttons.len>0{output.push(row(buttons).with_key("actions").with_style(ctx.component_style("actions",
        style().min_width(px(0)).gap(monitor_gap(1)).flex_wrap())));}
    if phase!="idle"&&props.steps.len>0{
        output.push(note(ctx,ctx.format_number(data.started)+"/"+ctx.format_number(props.steps.len),"counter").with_key("count")
            .with_style(style().text_end().flex_grow()));
    }
    if expose&&listed_action(cfg,"open_log"){output.push(control(ctx,cfg,"log","open_log",ctx.t_or("agent_monitor.log","Open log"),"ghost",false));}
    if output.len==0{return ();}
    row(output).with_key("footer").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("footer",style()
        .min_width(px(0)).gap(monitor_gap(1.5)).items_center().flex_wrap().justify_between()))
}
fn render_AgentMonitor(ctx,props){
    if !team::valid(props.run){throw "AgentMonitor needs a stable run ID";}
    let ask=opt(props,"ask");if ask!=()&&!team::valid(ask.id){throw "Monitor approval needs a stable request ID";}
    let current=opt(props,"current");if current!=()&&!team::valid(current.id){throw "Monitor current work needs a stable ID";}
    let clock=accepted_clock(props);let data=project(props,clock);let ui=local(ctx,props.run);
    let keys=[];for step in data.history{keys.push(step.data.id);}
    effect("prune_history",#{run:props.run,keys:keys},Fn("prune_history"),Fn("stop_history"));
    let cfg=#{run:props.run,folded:if ui.folded==(){props.default_folded}else{ui.folded},
        connected:opt(props,"on_action")!=(),actions:opt(props,"actions"),currency:value(props,"currency","USD"),
        token:#{run:props.run,revision:props.revision,step:if data.active==(){""}else{data.active.data.id},
            request:if ask==()||props.state!="waiting"{""}else{ask.id}}};
    let fold=element_ref("monitor-fold");let card=[header(ctx,props,cfg,data,clock,fold)];
    if cfg.folded{
        if data.history.len>0{card.push(strip(ctx,props,cfg,data,false,""));}
    }else{
        card.push(activity(ctx,props,cfg,data,clock));
        if data.history.len>0{
            card.push(strip(ctx,props,cfg,data,true,ui.hot));card.push(history(ctx,cfg,data,ui));
        }
    }
    let output=[column(card).with_key("card").audit_allow(["text-edge-misaligned"]).with_style(ctx.component_style(
        if cfg.folded{["card","card_folded"]}else{["card"]},style().min_width(px(0)).padding(monitor_gap(1.5))
            .gap(monitor_gap(1.5)).radius(monitor_unit(3)).background(theme_color("surface_raised"))))];
    if !cfg.folded{
        let tools=footer(ctx,props,cfg,data);if tools!=(){output.push(tools);}
    }
    let feedback=opt(props,"notice");
    if feedback!=(){output.push(note(ctx,feedback,"notice").with_key("notice").accessibility_role("status"));}
    column(output).with_key(props.key).accessibility_role("group").accessibility_label(ctx.t_or("agent_monitor.title","Run monitor"))
        .audit_allow(["text-edge-misaligned"]).with_style(ctx.component_style("root",style().min_width(px(0))
            .padding(monitor_gap(1.5)).radius(monitor_unit(4)).gap(monitor_gap(1.5)).background(theme_color("surface_raised"))))
}
