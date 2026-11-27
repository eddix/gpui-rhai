/* gpui-rhai
{"id":"patterns/delegate_board","export":"DelegateBoard","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/drag_source","components/drop_zone","components/scroll_area","components/switch","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","radius.lg","selection","surface_hover","surface_raised","text.success","text_muted","text_primary","typography.body","typography.caption","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// Move work between an unassigned pile and agent queues with one versioned assignment contract.
// State: keyboard pick, focus handoff, and one automatic attempt per accepted board revision.
// Example: DelegateBoard(#{key:"work",board:"work",revision:0,now_ms:0,agents:[],tasks:[]})
import "components/button" as button;
import "components/drag_source" as drag;
import "components/drop_zone" as drop;
import "components/scroll_area" as scroll;
import "components/switch" as toggle;
import "patterns/approval_card" as review;

define_component(#{
    metadata:#{id:"patterns/delegate_board","export":"DelegateBoard",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/drag_source","components/drop_zone","components/scroll_area","components/switch","patterns/approval_card"],
        capabilities:#{},tokens:["accent","border","focus_ring","radius.lg","selection","surface_hover","surface_raised","text.success","text_muted",
            "text_primary","typography.body","typography.caption","space.spacing_unit","space.unit"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable component instance key."},
        board:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable business identity; use a distinct drag namespace for each board in a window. At most 72 UTF-8 bytes."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted board revision. Advance it when tasks, assignments, agent availability or mode change."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Board heading."},
        mode:#{schema:#{type:"string",allowed:["manual","auto"]},"default":#{type:"string",value:"manual"},required:false,sensitive:false,
            doc:"The accepted policy; auto proposes a batch for a new board revision while there is available work."},
        agents:#{schema:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique nonempty stable agent ID."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Agent name."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Model or role label."},
            avatar:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative agent mark."},
            current_minutes:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:525600}},required:false,sensitive:false,
                doc:"Remaining minutes for the first queued task; defaults to that task's estimate."},
            rate_cents:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:100000}},required:false,sensitive:false,
                doc:"Optional price per minute in hundredths of the displayed currency."},
            accepting:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Defaults to true; false prevents new assignments without clearing the current queue."},
        }}},required:true,sensitive:true,doc:"Agent order also breaks ties in the least-loaded allocation."},
        tasks:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique nonempty stable task ID."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Task caption."},
            estimate:#{schema:#{type:"integer",min:0,max:525600},required:true,sensitive:false,doc:"Estimated minutes of work."},
            agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted agent ID; omit or pass null for unassigned."},
            tools:#{schema:#{type:"optional",value:#{type:"array",max_items:3,items:#{type:"object",allow_unknown:false,fields:#{
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Tool name for the accessibility description."},
                icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative tool icon; falls back to a text mark."},
            }}}},required:false,sensitive:false,doc:"Up to three tool marks, with no tool execution."},
        }}},required:true,sensitive:true,doc:"Tasks in accepted queue order. The first task assigned to each agent is its running task."},
        suggestion:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            task:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Task being suggested."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Proposed agent."},
        }}},required:false,sensitive:false,doc:"Optional caller suggestion; highlights the task and previews the agent's new workload, without assigning it."},
        now_ms:#{schema:#{type:"integer",min:0,max:253402300799999},required:true,sensitive:false,doc:"Host epoch milliseconds used for busy-until estimates."},
        time_zone:#{schema:#{type:"string"},"default":#{type:"string",value:"UTC"},required:false,sensitive:false,doc:"Zone used by the locale time formatter."},
        currency:#{schema:#{type:"string"},"default":#{type:"string",value:"$"},required:false,sensitive:false,doc:"Display currency prefix for optional queue costs."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Host is resolving a proposal; interaction stops and accepted tasks stay in place."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host acceptance, refusal, or other information."},
        on_assign:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives one assignment proposal, containing one move or a batch."},
        on_mode_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Requests a manual/auto mode change; pass the new mode only after acceptance."},
    },state:#{fields:#{
        pick:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        focus_plan:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        auto_attempt:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        assign:#{payload:#{type:"object",allow_unknown:false,fields:#{
            board:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Board identity."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot used to plan these moves."},
            kind:#{schema:#{type:"string",allowed:["manual","all","auto"]},required:true,sensitive:false,doc:"The source of the proposal."},
            moves:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
                task:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Moved task ID."},
                from:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Expected current agent, or empty for the unassigned pile."},
                agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Destination agent, or empty to put the task back."},
            }}},required:true,sensitive:false,doc:"Moves in placement order. Validate the whole batch against the same revision before applying it."},
        }},doc:"A proposal does not move accepted tasks. Only a Host update changes ownership."},
        mode_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            board:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Board identity."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted board revision."},
            mode:#{schema:#{type:"string",allowed:["manual","auto"]},required:true,sensitive:false,doc:"Proposed policy mode."},
        }},doc:"Requests a mode change; does not assign work on its own."},
    },slots:#{},parts:["root","head","title","hint","mode","lanes","pile","agents","section_title","count","task","task_picked","task_suggested",
        "task_title","tools","tool","time","empty","agent","agent_suggested","agent_target","identity","avatar","agent_name","model","load","free",
        "queue","queue_task","queued","running","estimate","cost","footer","notice","status","pick_help","symbol"],
        effects:["auto_assign","keyboard_focus"]},
    render:Fn("render_DelegateBoard")
});

fn DelegateBoard(props){render_component("patterns/delegate_board",props)}
fn board_token(props){#{board:props.board,revision:props.revision}}
fn present_agent(t){review::value_or(t,"agent","")}
fn accepting(a){review::value_or(a,"accepting",true)}
fn bounded_id(id){
    if id==""||utf8_byte_count(id)>128{return false;}
    for c in id.chars(){if c<' '||(c>='\u007f'&&c<='\u009f'){return false;}}
    true
}
fn model(props){
    let data=#{picks:#{}, agents:#{}, pile:[], queues:#{}, minutes:#{}, available:[], next:[]};
    for a in props.agents{
        if !bounded_id(a.id)||a.id in data.agents{throw "DelegateBoard agent IDs must be unique and bounded";}
        data.agents[a.id]=a;data.queues[a.id]=[];data.minutes[a.id]=0;
        if accepting(a){data.available.push(a.id);}
    }
    for i in 0..props.tasks.len{
        let t=props.tasks[i];let owner=present_agent(t);
        if !bounded_id(t.id)||t.id in data.picks{throw "DelegateBoard task IDs must be unique and bounded";}
        if owner!=""&&!(owner in data.agents){throw "DelegateBoard assigned task refers to an unknown agent";}
        let entry=t;entry.ordinal=i;
        data.picks[t.id]=entry;
        if owner==""{data.pile.push(entry);}
        else{
            let active=data.queues[owner].len==0;
            let minutes=if active{review::value_or(data.agents[owner],"current_minutes",t.estimate)}else{t.estimate};
            data.minutes[owner]+=minutes;data.queues[owner].push(entry);
        }
    }
    if data.available.len!=0{
        let load=data.minutes;
        for t in data.pile{
            let id=data.available[0];
            for candidate in data.available{if load[candidate]<load[id]{id=candidate;}}
            load[id]+=t.estimate;data.next.push(move(t.id,"",id));
        }
    }
    data
}
fn move(task,from,agent){#{task:task,from:from,agent:agent}}
fn proposal(t,kind,moves){#{board:t.board,revision:t.revision,kind:kind,moves:moves}}
fn emit_assign(payload,ctx,p){
    if payload.moves.len>0{ctx.emit("assign",payload);}
    handled()
}
fn emit_mode(t,ctx,checked){
    ctx.emit("mode_change",#{board:t.board,revision:t.revision,mode:if checked{"auto"}else{"manual"}});handled()
}
fn auto_assign(ctx,request){
    if request==(){return;}
    let token=#{board:request.board,revision:request.revision};
    if ctx.get_state("auto_attempt")==token{return;}
    ctx.set_state("auto_attempt",token);
    ctx.emit("assign",request);
}
fn no_effect(ctx,p){}
fn origin(cfg,task){#{scope:cfg.scope,board:cfg.token.board,revision:cfg.token.revision,task:task.id,from:present_agent(task)}}
fn packet(cfg,task){let p=origin(cfg,task);p.source=source_id(cfg,task.ordinal);p}
fn source_id(cfg,i){cfg.wire+":s"+i.to_string()}
fn target_id(cfg,i){cfg.wire+":d"+i.to_string()}
fn task_focus(i){"task-"+i.to_string()}
fn goal_focus(i){"goal-"+i.to_string()}
fn pick(cfg,t,first,ctx,p){
    let current=ctx.get_state("pick");
    if current!=()&&current==t{
        ctx.set_state("pick",());ctx.set_state("focus_plan",());return handled();
    }
    ctx.set_state("pick",t);ctx.set_state("focus_plan",#{scope:cfg.scope,revision:cfg.revision,key:first});handled()
}
fn focus_target(ctx,plan){
    if plan==()||ctx.get_state("focus_plan")!=plan{return;}
    ctx.set_state("focus_plan",());ctx.focus(plan.key);
}
fn cancel_pick(picked,idx,ctx,p){
    if ctx.get_state("pick")!=picked{return;}
    ctx.set_state("pick",());ctx.set_state("focus_plan",());ctx.focus(task_focus(idx));handled()
}
fn focus_next(key,ctx,p){ctx.focus(key);handled()}
fn finish_pick(cfg,picked,agent,ctx,p){
    if ctx.get_state("pick")!=picked||picked.scope!=cfg.scope||picked.revision!=cfg.token.revision||picked.from==agent||
        !(picked.task in cfg.leases)||cfg.leases[picked.task].from!=picked.from{return;}
    ctx.set_state("pick",());ctx.set_state("focus_plan",());
    emit_assign(proposal(cfg.token,"manual",[move(picked.task,picked.from,agent)]),ctx,p)
}
fn finish_drop(cfg,agent,id,ctx,p){
    if p.operation!="move"||p.target_id!=id||p.payload_type!=cfg.wire{return;}
    let x=p.payload;
    if type_of(x)!="map"||!("scope" in x)||x.scope!=cfg.scope||x.board!=cfg.token.board||x.revision!=cfg.token.revision||
        p.source_id!=x.source||x.from==agent||!(x.task in cfg.leases)||cfg.leases[x.task].from!=x.from||cfg.leases[x.task].source!=x.source{return;}
    ctx.set_state("pick",());ctx.set_state("focus_plan",());
    emit_assign(proposal(cfg.token,"manual",[move(x.task,x.from,agent)]),ctx,p)
}
fn time_label(ctx,n){
    if n<60{return ctx.t_or("delegate.minutes","{n} min",#{n:ctx.format_number(n)});}
    ctx.t_or("delegate.hours","{h} h {m} min",#{h:ctx.format_number(n/60),m:ctx.format_number(n%60)})
}
fn cost_label(ctx,symbol,cents){
    symbol+ctx.format_number(cents.to_float()/100.0,#{min_fraction_digits:2,max_fraction_digits:2})
}
fn heading(ctx,value,part){review::heading_note(ctx,value,part).with_style(style().min_width(px(0)))}
fn glyph(ctx,label,part){
    let first=if label==""{"?"}else{label.sub_string(0,1).to_upper()};
    row([text(first).with_key("mark")]).with_style(ctx.component_style(part,style().width(theme_length("space.unit").times(5.75)).height(theme_length("space.unit").times(5.75))
        .items_center().justify_center().flex_shrink(false).radius(px(5)).background(theme_color("surface_hover")).typography("caption")))
}
fn task_description(ctx,task){
    let result=time_label(ctx,task.estimate);
    for tool in review::value_or(task,"tools",[]){result+=" · "+tool.label;}
    result
}
fn tools(ctx,task){
    let nodes=[];let list=review::value_or(task,"tools",[]);
    for i in 0..list.len{
        let tool=list[i];let icon=review::opt(tool,"icon");
        let value=if icon==(){glyph(ctx,tool.label,"tool")}else{icon.with_style(ctx.component_style("tool",style().width(theme_length("space.unit").times(4.75)).height(theme_length("space.unit").times(4.75))))};
        nodes.push(value.with_key("tool-"+i.to_string()));
    }
    row(nodes).with_key("tools").accessibility_hidden(true).with_style(ctx.component_style("tools",style().items_center().gap(theme_length("space.spacing_unit").times(0.75)).flex_shrink(false)))
}
fn task_card(ctx,cfg,task,picked,queued,running){
    let same=picked!=()&&picked.task==task.id;
    let hint=cfg.suggestion!=()&&cfg.suggestion.task==task.id;
    let parts=if queued{["task","queue_task"]}else{["task"]};
    if same{parts.push("task_picked");}
    if hint{parts.push("task_suggested");}
    let main=[text(task.title).with_key("title").with_style(ctx.component_style("task_title",style().min_width(px(0)).flex_grow().typography("body")
        .font_weight(500).whitespace_nowrap().text_ellipsis()))];
    if !queued{main.push(tools(ctx,task));}
    if running{main.push(heading(ctx,ctx.t_or("delegate.running","Running"),"running").with_key("running"));}
    main.push(heading(ctx,time_label(ctx,task.estimate),"time").with_key("time").with_style(style().flex_shrink(false)));
    let body=row(main).audit_allow(["mixed-type-in-row"]).with_key("task-face")
        .with_style(ctx.component_style(parts,style().min_width(px(0)).min_height(px(if queued{27}else{39})).padding_x(theme_length("space.spacing_unit").times(2.25)).padding_y(theme_length("space.spacing_unit"))
            .items_center().gap(theme_length("space.spacing_unit").times(1.25)).radius(theme_radius("lg")).background(theme_color(if same{"selection"}else{"surface_raised"}))
            .hover(style().background(theme_color("surface_hover")))));
    let id=task_focus(task.ordinal);let source_ref=element_ref(id);
    let label=ctx.t_or("delegate.move","Move {task}",#{task:task.title});
    let node=drag::DragSource(#{key:"move-"+task.id,label:label,source_id:source_id(cfg,task.ordinal),payload_type:cfg.wire,
        payload:packet(cfg,task),content:body,disabled:!cfg.ready,source_ref:source_ref,description:task_description(ctx,task)})
        .with_key("move-"+task.id).shared_layout(cfg.scope,task.id)
        .layout_motion(ctx.motion_duration("normal"),ctx.motion_easing("standard"));
    if !cfg.ready{return node;}
    let allowed=destinations(cfg,origin(cfg,task));
    if allowed.len==0{return node;}
    let lease=#{scope:cfg.scope,revision:cfg.token.revision};
    node.on_capture("key:plain+enter",Fn("pick").curry(lease,origin(cfg,task),goal_focus(allowed[0].index)))
        .on_capture("key:plain+space",Fn("pick").curry(lease,origin(cfg,task),goal_focus(allowed[0].index)))
}
fn destinations(cfg,picked){
    let rows=[];
    for entry in cfg.dest{
        if entry.agent!=picked.from{rows.push(entry);}
    }
    rows
}
fn destination_control(ctx,cfg,picked,i,agent,label){
    let open=picked!=()&&agent!=picked.from&&cfg.ready;
    if !open{return ();}
    let node=button::Button(#{key:if agent==""{"return-to-pile"}else{"assign-to-"+agent},text:label,size:"xs",variant:"secondary",
        on_click:Fn("finish_pick").curry(cfg.identity,picked,agent)})
        .with_key("assign").with_ref(element_ref(goal_focus(i)))
        .with_style(ctx.component_style("agent_target",style().self_start()));
    let choices=destinations(cfg,picked);let current=0;
    for j in 0..choices.len{if choices[j].index==i{current=j;}}
    let prev=choices[(current+choices.len-1)%choices.len].index;
    let next=choices[(current+1)%choices.len].index;
    let rtl=ctx.text_direction()=="rtl";
    node.on_key_value("plain+up",Fn("focus_next").curry(goal_focus(prev)),())
        .on_key_value("plain+down",Fn("focus_next").curry(goal_focus(next)),())
        .on_key_value("plain+left",Fn("focus_next").curry(goal_focus(if rtl{next}else{prev})),())
        .on_key_value("plain+right",Fn("focus_next").curry(goal_focus(if rtl{prev}else{next})),())
}
fn zone(cfg,idx,agent,label,content,enabled){
    let id=target_id(cfg,idx);
    drop::DropZone(#{key:if agent==""{"pile"}else{"agent-"+agent},label:label,target_id:id,payload_types:[cfg.wire],
        operations:["move"],content:content,disabled:!cfg.ready||!enabled,
        on_drop:Fn("finish_drop").curry(cfg.identity,agent,id)})
}
fn section_label(ctx,title,count){
    row([text(title).with_key("heading").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("section_title",style().typography("body").font_weight(500))),
        heading(ctx,ctx.format_number(count),"count").with_key("count")])
        .with_key("section-label").audit_allow(["mixed-type-in-row"])
        .with_style(style().min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(1.25)))
}
fn pile(ctx,cfg,data,picked){
    let queue=[];for t in data.pile{queue.push(task_card(ctx,cfg,t,picked,false,false));}
    if queue.len==0{queue.push(heading(ctx,ctx.t_or("delegate.empty_pile","No unassigned tasks"),"empty").with_key("pile-empty"));}
    let caption=ctx.t_or("delegate.pile","Unassigned");
    let list=column(queue).with_key("unassigned-tasks").with_style(ctx.component_style("queue",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25))));
    let content=scroll::ScrollArea(#{key:"pile-scroll",label:caption,height:px(392),content:list});
    if queue.len<=9{content=content.with_style(style().height(auto()).overflow_hidden());}
    let body=[section_label(ctx,caption,data.pile.len)];
    let choose=destination_control(ctx,cfg,picked,0,"",ctx.t_or("delegate.return","Return to unassigned"));
    if choose!=(){body.push(choose);}body.push(content);
    zone(cfg,0,"",caption,column(body).with_key("pile-inner")
        .with_style(ctx.component_style("pile",style().min_width(px(0)).min_height(theme_length("space.unit").times(22)).gap(theme_length("space.spacing_unit").times(1.75)).padding(theme_length("space.spacing_unit").times(2))
            .radius(theme_radius("lg")).background(theme_color("surface_hover")))),true)
        .with_style(style().min_width(px(0)).width(if cfg.compact{relative(1.0)}else{relative(0.42)}).flex_shrink(false))
}
fn agent_status(ctx,a,minutes,cfg,load){
    let text=if load==0{
        if accepting(a){ctx.t_or("delegate.free","Free now")}else{ctx.t_or("delegate.paused","Not accepting work")}
    }else{
        let remaining=time_label(ctx,minutes);
        let at=cfg.now+minutes*60000;
        let wait_until=remaining;
        if minutes<=1440&&at<=253402300799999{
            let today=time_parts(cfg.now,cfg.zone).date;
            let local=time_parts(at,cfg.zone);
            let clock=local.time.sub_string(0,5);let date=local.date;
            // An embedded Host may have no locale manager. Time-zone
            // validation still happens above; the ISO fallback is usable.
            try{clock=ctx.format_time(at,"short",cfg.zone);}catch{}
            if local.date!=today{try{date=ctx.format_date(local.date,"short");}catch{}}
            wait_until=if local.date==today{clock}else{date+" "+clock};
        }
        ctx.t_or("delegate.busy","Busy until {time}",#{time:wait_until})
    };
    heading(ctx,text,if load==0&&accepting(a){"free"}else{"load"}).with_key("load")
        .accessibility_description(if load==0{""}else{time_label(ctx,minutes)})
}
fn agent_card(ctx,cfg,data,a,i,picked){
    let queue=data.queues[a.id];let minutes=data.minutes[a.id];
    let has=cfg.suggestion!=()&&cfg.suggestion.agent==a.id;
    let dest=ctx.t_or("delegate.assign_agent","Assign to {agent}",#{agent:a.name});
    let avatar=review::opt(a,"avatar");
    let icon=(if avatar==(){glyph(ctx,a.name,"avatar")}else{avatar.with_style(ctx.component_style("avatar",style().width(theme_length("space.unit").times(6.5)).height(theme_length("space.unit").times(6.5))))})
        .with_key("avatar").accessibility_hidden(true);
    let names=[text(a.name).with_key("name").with_style(ctx.component_style("agent_name",style().min_width(px(0)).typography("body").font_weight(600)))];
    if review::opt(a,"model")!=(){names.push(heading(ctx,a.model,"model").with_key("model"));}
    let head=row([icon,column(names).with_key("nameplate").with_style(style().min_width(px(0)).flex_grow().gap(theme_length("space.spacing_unit").times(0.5))),
        agent_status(ctx,a,minutes,cfg,queue.len)])
        .with_key("identity").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("identity",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).items_center().flex_wrap()));
    let bits=[head];
    if accepting(a){
        let choose=destination_control(ctx,cfg,picked,i,a.id,dest);
        if choose!=(){bits.push(choose);}
    }
    if has{
        let t=data.picks[cfg.suggestion.task];let rate=review::opt(a,"rate_cents");
        let text=ctx.t_or("delegate.suggest","Suggested: {task} · +{time}",#{task:t.title,time:time_label(ctx,t.estimate)});
        if rate!=(){text+=" · "+cost_label(ctx,cfg.currency,rate*t.estimate);}
        bits.push(heading(ctx,text,"estimate").with_key("estimate"));
    }
    let rows=[];for j in 0..queue.len{rows.push(task_card(ctx,cfg,queue[j],picked,true,j==0));}
    if queue.len>0{
        bits.push(column(rows).with_key("queue").with_style(ctx.component_style("queue",style().min_width(px(0)).gap(theme_length("space.spacing_unit")))));
    }
    let parts=if has{["agent","agent_suggested"]}else{["agent"]};
    zone(cfg,i,a.id,ctx.t_or("delegate.queue_for","{agent} queue",#{agent:a.name}),column(bits).with_key("agent-body")
        .with_style(ctx.component_style(parts,style().min_width(px(0)).min_height(theme_length("space.unit").times(13.5)).gap(theme_length("space.spacing_unit").times(1.25)).padding(theme_length("space.spacing_unit").times(2.25))
            .radius(theme_radius("lg")).background(theme_color("surface_raised")))),accepting(a))
        .with_key("agent-"+a.id)
}
fn agents(ctx,cfg,data,records,picked){
    let rows=[];for i in 0..records.len{rows.push(agent_card(ctx,cfg,data,records[i],i+1,picked));}
    if rows.len==0{rows.push(heading(ctx,ctx.t_or("delegate.no_agents","No agents available"),"empty").with_key("empty"));}
    column([section_label(ctx,ctx.t_or("delegate.agents","Agents"),records.len),column(rows).with_key("queues")
        .with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)))])
        .with_key("agents").with_style(ctx.component_style("agents",style().min_width(px(0)).flex_grow().gap(theme_length("space.spacing_unit").times(1.75)).padding(theme_length("space.spacing_unit").times(2))))
}
fn header(ctx,props,t){
    let enabled=!props.busy&&review::opt(props,"on_mode_change")!=();
    let title=review::value_or(props,"title",ctx.t_or("delegate.title","Delegate work"));
    let hint=if props.mode=="auto"{ctx.t_or("delegate.auto_hint","Work goes to the agent who will be free first")}
        else{ctx.t_or("delegate.manual_hint","Drag tasks or pick an agent with the keyboard")};
    row([glyph(ctx,"✦","symbol").with_key("symbol").accessibility_hidden(true),
        text(title).with_key("title").accessibility_role("heading").accessibility_level(2).with_style(ctx.component_style("title",style().min_width(px(0)).typography("body").font_weight(600))),
        heading(ctx,hint,"hint").with_key("hint").with_style(style().flex_grow()),
        toggle::Switch(#{key:"auto",label:ctx.t_or("delegate.auto","Auto"),checked:props.mode=="auto",disabled:!enabled,size:"sm",
            on_change:Fn("emit_mode").curry(t)}).with_style(ctx.component_style("mode",style().flex_shrink(false)))])
        .with_key("head").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("head",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).flex_wrap().items_center()))
}
fn footer(ctx,props,cfg,data){
    let queued=props.tasks.len-data.pile.len;
    let text=ctx.t_or("delegate.queued","{n} queued",#{n:ctx.format_number(queued)});
    let total=0;let known=false;
    for a in props.agents{
        let rate=review::opt(a,"rate_cents");
        if rate!=(){known=true;total+=rate*data.minutes[a.id];}
    }
    let items=[heading(ctx,text,"queued").with_key("queued")];
    if known{items.push(heading(ctx,ctx.t_or("delegate.cost","{cost} estimated",#{cost:cost_label(ctx,props.currency,total)}),"cost").with_key("cost"));}
    let button=button::Button(#{key:"assign-all",text:ctx.t_or("delegate.assign_all","Assign all"),size:"sm",variant:"primary",
        disabled:!cfg.ready||data.next.len==0,on_click:Fn("emit_assign").curry(proposal(cfg.token,"all",data.next))});
    row([row(items).with_key("summary").with_style(style().min_width(px(0)).flex_grow().items_center().gap(theme_length("space.spacing_unit").times(1.25)).flex_wrap()),button])
        .with_key("footer").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("footer",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.75)).items_center().flex_wrap()))
}
fn render_DelegateBoard(ctx,props){
    if !bounded_id(props.board)||utf8_byte_count(props.board)>72{throw "DelegateBoard needs a 1–72 byte board identifier without control characters";}
    let reference=element_ref("board");let t=board_token(props);
    let compact=ctx.element_width_class(reference,[640])==0;
    let wire="delegate:"+utf8_byte_count(props.board).to_string()+":"+props.board+":"+props.revision.to_string();
    let scope=reference.semantic_id;
    let ready=!props.busy&&review::opt(props,"on_assign")!=();
    let data=model(props);let suggest=review::opt(props,"suggestion");
    if suggest!=()&&(!(suggest.agent in data.available)||!(suggest.task in data.picks)||present_agent(data.picks[suggest.task])==suggest.agent){suggest=();}
    let dest=[#{index:0,agent:""}];
    for i in 0..props.agents.len{if accepting(props.agents[i]){dest.push(#{index:i+1,agent:props.agents[i].id});}}
    let leases=#{};
    for id in data.picks.keys(){
        let item=data.picks[id];leases[id]=#{from:present_agent(item),source:wire+":s"+item.ordinal.to_string()};
    }
    let identity=#{scope:scope,token:t,wire:wire,leases:leases};
    let cfg=#{identity:identity,scope:scope,token:t,wire:wire,ready:ready,compact:compact,suggestion:suggest,dest:dest,
        now:props.now_ms,zone:props.time_zone,currency:props.currency};
    let picked=ctx.get_state("pick");let plan=ctx.get_state("focus_plan");
    if picked!=()&&(!ready||picked.scope!=scope||picked.revision!=t.revision||!(picked.task in data.picks)||
        picked.from!=present_agent(data.picks[picked.task])||destinations(cfg,picked).len==0){picked=();}
    effect("auto_assign",if ready&&props.mode=="auto"&&data.next.len>0{proposal(t,"auto",data.next)}else{()},Fn("auto_assign"),Fn("no_effect"));
    effect("keyboard_focus",if ready&&plan!=()&&plan.scope==scope&&plan.revision==t.revision{plan}else{()},Fn("focus_target"),Fn("no_effect"));
    let panes=[pile(ctx,cfg,data,picked),agents(ctx,cfg,data,props.agents,picked)];
    let stage=(if compact{column(panes)}else{row(panes).with_style(style().items_start())})
        .with_key("lanes").with_style(ctx.component_style("lanes",style().gap(theme_length("space.spacing_unit").times(2)).min_width(px(0))));
    let controls=[header(ctx,props,t),stage,footer(ctx,props,cfg,data)];
    if picked!=(){
        controls.push(heading(ctx,ctx.t_or("delegate.pick","Moving {task} — choose a destination or press Escape",#{task:data.picks[picked.task].title}),"pick_help")
            .with_key("picked").accessibility_role("status"));
    }
    if props.notice!=(){controls.push(heading(ctx,props.notice,"notice").with_key("notice").accessibility_role("status"));}
    // The top bar, inset task tracks and bottom bar align their separate
    // surfaces; the decorative mark and queue labels are not one text column.
    let root=column(controls).with_key(props.key).with_ref(reference).accessibility_role("group")
        .audit_allow(["text-edge-misaligned"])
        .accessibility_label(review::value_or(props,"title",ctx.t_or("delegate.title","Delegate work")))
        .with_style(ctx.component_style("root",style().min_width(px(0)).max_width(px(940)).width(relative(1.0)).gap(theme_length("space.spacing_unit").times(2)).padding(theme_length("space.spacing_unit").times(2))));
    if picked!=(){root=root.on_bubble("key:plain+escape",Fn("cancel_pick").curry(picked,data.picks[picked.task].ordinal));}
    root
}
