/* gpui-rhai
{"id":"patterns/agent_inbox","export":"AgentInbox","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","selection","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted","text_primary","typography.body","typography.caption","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// One agent-task inbox in board and list form. Actions never mutate accepted jobs.
// State: local layout, focused task and disclosure. Timing is supplied by the Host.
// Example: AgentInbox(#{key:"activity",inbox:"release",revision:0,now_ms:0,tasks:[]})
import "components/button" as button;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_inbox","export":"AgentInbox",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","patterns/approval_card"],capabilities:#{},
        tokens:["accent","border","focus_ring","selection","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted","text_primary",
            "typography.body","typography.caption","space.spacing_unit","space.unit"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable instance key."},
        inbox:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business identity of the inbox."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision, advanced on task changes, not time ticks."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Heading above the lanes."},
        now_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"One Host clock for all live tasks."},
        tasks:#{schema:#{type:"array",max_items:96,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable task ID."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Task name; the title is not an identity."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Name of the assigned agent or model."},
            status:#{schema:#{type:"string",allowed:["waiting","running","done","failed"]},required:true,sensitive:false,doc:"Accepted task state."},
            description:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Summary shown on the card and expanded on inspection."},
            started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Start of the waiting or running phase on the Host clock."},
            elapsed_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Fixed completed or failed duration, never advanced by the live clock."},
            tools:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Number of tool calls to report."},
            sources:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Number of sources read."},
            steps:#{schema:#{type:"optional",value:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable business ID within the task."},
                label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Step summary."},
                time:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional display time supplied by the Host."},
            }}}},required:false,sensitive:true,doc:"Task evidence in chronological order; its four most recent steps are shown."},
            busy:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Disables this task's actions without changing its accepted state."},
        }}},required:true,sensitive:true,doc:"Accepted tasks in order within their lane. Waiting and failed tasks share Needs you."},
        view:#{schema:#{type:"optional",value:#{type:"string",allowed:["board","list"]}},required:false,sensitive:false,doc:"Controlled layout. Omit for local layout state."},
        default_view:#{schema:#{type:"string",allowed:["board","list"]},"default":#{type:"string",value:"board"},required:false,sensitive:false,
            doc:"Initial local layout."},
        lanes:#{schema:#{type:"optional",value:#{type:"array",max_items:3,items:#{type:"string",allowed:["needs","running","done"]}}},required:false,sensitive:false,
            doc:"Limit the visible lanes, in their canonical order; omit to show all."},
        highlight:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"An accepted task to emphasize. Other tasks stay usable."},
        new_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional caption for a Host new-task proposal."},
        allow_open:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Show an Open proposal in running and done details."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disables all task proposals; inspection and layout stay usable."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Accepted Host feedback."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a revisioned task proposal."},
        on_view_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a layout choice. A controlled caller publishes the accepted layout."},
    },state:#{fields:#{
        view:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        open:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        cursor:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        action:#{payload:#{type:"object",allow_unknown:false,fields:#{
            inbox:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Exact business inbox."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot used for this proposal."},
            task:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Task identity, or empty for a new task."},
            kind:#{schema:#{type:"string",allowed:["approve","reject","retry","open","new"]},required:true,sensitive:false,doc:"Requested action."},
        }},doc:"Validate against the current task, status, and revision before applying."},
        view_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            inbox:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Which inbox is being changed."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted version for this notification."},
            view:#{schema:#{type:"string",allowed:["board","list"]},required:true,sensitive:false,doc:"Requested layout."},
        }},doc:"Only changes the layout, never the task data."},
    },slots:#{},parts:["root","head","crumb","title","controls","switcher","option","option_active","lanes","lane","lane_head","lane_mark",
        "lane_title","count","cards","empty","card","card_open","card_highlight","main","main_list","card_mark","task_title","description",
        "foot","state","state_waiting","state_running","state_done","state_failed","metrics","metric","body","agent","steps","step","step_time",
        "actions","notice","summary"]},
    render:Fn("render_AgentInbox")
});

fn AgentInbox(props){render_component("patterns/agent_inbox",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,d){common::value_or(m,k,d)}
fn valid(id){id!=""&&utf8_byte_count(id)<=128}
fn lane_of(status){if status=="waiting"||status=="failed"{"needs"}else{status}}
fn project(props){
    if !valid(props.inbox){throw "Inbox needs a bounded business identity";}
    let requested=value(props,"lanes",["needs","running","done"]);let show=#{};
    for id in requested{if id in show{throw "Inbox lane IDs must not repeat";}show[id]=true;}
    let groups=#{needs:[],running:[],done:[]};let seen=#{};
    for i in 0..props.tasks.len{
        let t=props.tasks[i];if !valid(t.id)||t.id in seen{throw "Inbox task IDs must be unique and bounded";}
        let steps=#{};
        for step in value(t,"steps",[]){
            if !valid(step.id)||step.id in steps{throw "Inbox step IDs must be unique within their task";}
            steps[step.id]=true;
        }
        seen[t.id]=true;t.ordinal=i;groups[lane_of(t.status)].push(t);
    }
    let lanes=[];let walk=[];let lookup=#{};
    for id in ["needs","running","done"]{
        if !(id in show){continue;}
        lanes.push(id);
        for t in groups[id]{lookup[t.id]=walk.len;walk.push(t);}
    }
    #{lanes:lanes,groups:groups,walk:walk,index:lookup}
}
fn caption(ctx,v,part){common::heading_note(ctx,v,part).with_style(style().min_width(px(0)))}
fn state_text(ctx,status){
    if status=="waiting"{ctx.t_or("agent_inbox.waiting","Waiting for you")}
    else if status=="running"{ctx.t_or("agent_queue.running","Running")}
    else if status=="done"{ctx.t_or("agent_queue.done","Done")}
    else{ctx.t_or("agent_queue.failed","Failed")}
}
fn clock(ctx,n){
    let s=n/1000;
    if s<60{return ctx.t_or("agent_queue.seconds","{n} s",#{n:ctx.format_number(s)});}
    let m=s/60;let r=s%60;
    if m<60{return ctx.t_or("agent_inbox.minutes_seconds","{m} min {s} s",#{m:ctx.format_number(m),s:ctx.format_number(r)});}
    let h=m/60;let rest=m%60;
    ctx.t_or("agent_inbox.hours_minutes","{h} h {m} min",#{h:ctx.format_number(h),m:ctx.format_number(rest)})
}
fn timed(ctx,job,now){
    let from=opt(job,"started_ms");let duration=opt(job,"elapsed_ms");
    if job.status=="waiting"||job.status=="running"{duration=if from==(){()}else{max(0,now-from)};}
    let state=state_text(ctx,job.status);
    if duration==(){state}else{state+" · "+clock(ctx,duration)}
}
fn color(status){
    theme_color(if status=="waiting"{"text.warning"}else if status=="running"{"accent"}else if status=="failed"{"text.danger"}else{"text.success"})
}
fn read_selection(ctx,key,container,default_value){
    let state=ctx.get_state(key);if state!=()&&state.inbox==container{state.value}else{default_value}
}
fn store(ctx,key,container,v){ctx.set_state(key,#{inbox:container,value:v});}
fn ref_task(task){"task-"+task.ordinal.to_string()}
fn pick(token,layout,ctx,event){
    if !token.controlled{store(ctx,"view",token.inbox,layout);}
    if token.notify{ctx.emit("view_change",#{inbox:token.inbox,revision:token.revision,view:layout});}
    handled()
}
fn toggle(inbox,id,ctx,event){
    let old=read_selection(ctx,"open",inbox,"");store(ctx,"open",inbox,if old==id{""}else{id});
    store(ctx,"cursor",inbox,id);handled()
}
fn close(inbox,key,ctx,event){store(ctx,"open",inbox,"");ctx.focus(key);handled()}
fn focus_card(inbox,id,key,ctx,event){store(ctx,"cursor",inbox,id);ctx.focus(key);handled()}
fn action(token,id,kind,ctx,event){
    ctx.emit("action",#{inbox:token.inbox,revision:token.revision,task:id,kind:kind});handled()
}
fn glyph(ctx,id,symbol,ink,part){
    row([text(symbol)]).with_key(id).accessibility_hidden(true).with_style(ctx.component_style(part,style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4.25))
        .items_center().justify_center().flex_shrink(false).typography("caption").text_color(ink)))
}
fn task_actions(ctx,t,cfg){
    if !cfg.actions{return [];}
    let locked=cfg.busy||value(t,"busy",false);
    let nodes=[];
    if t.status=="waiting"{
        nodes.push(button::Button(#{key:"reject-"+t.id,text:ctx.t_or("gates.reject","Reject"),size:"xs",variant:"secondary",disabled:locked,
            on_click:Fn("action").curry(cfg.token,t.id,"reject")}));
        nodes.push(button::Button(#{key:"approve-"+t.id,text:ctx.t_or("review.approve","Approve"),size:"xs",disabled:locked,
            on_click:Fn("action").curry(cfg.token,t.id,"approve")}));
    }else if t.status=="failed"{
        nodes.push(button::Button(#{key:"retry-"+t.id,text:ctx.t_or("agent_queue.retry","Retry job"),size:"xs",disabled:locked,
            on_click:Fn("action").curry(cfg.token,t.id,"retry")}));
    }else if cfg.allow_open{
        nodes.push(button::Button(#{key:"open-"+t.id,text:ctx.t_or("agent_lanes.open","Open task"),size:"xs",variant:"secondary",disabled:locked,
            on_click:Fn("action").curry(cfg.token,t.id,"open")}));
    }
    nodes
}
fn disclosure(ctx,t,cfg){
    let details=[caption(ctx,t.agent,"agent").with_key("agent")];
    let trail=value(t,"steps",[]);let rows=[];
    for j in max(0,trail.len-4)..trail.len{
        let step=trail[j];
        let cells=[caption(ctx,step.label,"step").with_key("label").with_style(style().flex_grow())];
        if opt(step,"time")!=(){cells.push(caption(ctx,step.time,"step_time").with_key("time"));}
        rows.push(row(cells).with_key("step-"+step.id).accessibility_role("listitem").accessibility_label(step.label)
            .accessibility_description(value(step,"time","")).audit_allow(["mixed-type-in-row"])
            .with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).items_center().flex_wrap()));
    }
    if rows.len>0{details.push(column(rows).with_key("steps").accessibility_role("list")
        .accessibility_label(ctx.t_or("agent_inbox.steps","Recent steps"))
        .with_style(ctx.component_style("steps",style().min_width(px(0)).gap(theme_length("space.spacing_unit")))));}
    let actions=task_actions(ctx,t,cfg);
    if actions.len>0{details.push(row(actions).with_key("actions").with_style(ctx.component_style("actions",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).flex_wrap())));}
    column(details).with_key("details").accessibility_role("region")
        .accessibility_label(ctx.t_or("agent_inbox.details","Details for {task}",#{task:t.title}))
        .with_style(ctx.component_style("body",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).padding(theme_length("space.spacing_unit").times(2)).background(theme_color("surface_hover")).radius(px(7))))
        .motion(motion_transition("opacity",0.45,1.0,#{duration_ms:ctx.motion_duration("fast"),intent:"feedback"}))
}
fn metrics(ctx,t){
    let display=[];let description="";
    if opt(t,"sources")!=(){
        let text=ctx.t_or("agent_inbox.sources","Sources: {n}",#{n:ctx.format_number(t.sources)});
        display.push(caption(ctx,text,"metric").with_key("sources"));description+=" · "+text;
    }
    if opt(t,"tools")!=(){
        let text=ctx.t_or("agent_inbox.tools","Tools: {n}",#{n:ctx.format_number(t.tools)});
        display.push(caption(ctx,text,"metric").with_key("tools"));description+=" · "+text;
    }
    #{nodes:display,description:description}
}
fn card(ctx,t,cfg,data){
    let open=t.id==cfg.open;let tone=color(t.status);
    let reading=timed(ctx,t,cfg.now);let meta=metrics(ctx,t);
    let title=row([glyph(ctx,"spark","✦",tone,"card_mark"),
        text(t.title).with_key("title").with_style(ctx.component_style("task_title",style().min_width(px(0)).flex_grow()
            .typography("body").font_weight(600).whitespace_nowrap().text_ellipsis()))]).with_key("title")
        .audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).items_center());
    let status=caption(ctx,reading,["state","state_"+t.status]).with_key("state").with_style(style().text_color(tone));
    let foot=[status];
    if meta.nodes.len>0{
        foot.push(row(meta.nodes).with_key("metrics").with_style(ctx.component_style("metrics",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap())));
    }
    let rows=[title];
    if opt(t,"description")!=(){
        let blurb=caption(ctx,t.description,"description").with_key("description");
        rows.push(if open{blurb}else{blurb.with_style(style().line_clamp(2))});
    }
    rows.push(row(foot).with_key("foot").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("foot",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).flex_wrap().items_center().justify_between())));
    let name=t.agent+" · "+reading+meta.description;
    let main=column(rows).with_key("main").with_ref(element_ref(ref_task(t))).accessibility_role("button")
        .accessibility_label(t.title).accessibility_description(name).accessibility_expanded(open).tab_stop(t.id==cfg.cursor)
        .on_click(Fn("toggle").curry(cfg.token.inbox,t.id))
        .with_style(ctx.component_style(if cfg.wide{["main"]}else{["main","main_list"]},
            style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).padding(theme_length("space.spacing_unit").times(2)).radius(theme_length("space.unit").times(2))
                .focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))));
    let i=data.index[t.id];let prev=data.walk[max(0,i-1)];let next=data.walk[min(data.walk.len-1,i+1)];
    main=main.on_key_value("plain+up",Fn("focus_card").curry(cfg.token.inbox,prev.id,ref_task(prev)),())
        .on_key_value("plain+down",Fn("focus_card").curry(cfg.token.inbox,next.id,ref_task(next)),());
    if cfg.can_layout{
        if cfg.layout!="board"{main=main.on_key_value("plain+b",Fn("pick").curry(cfg.view_token,"board"),());}
        if cfg.layout!="list"{main=main.on_key_value("plain+l",Fn("pick").curry(cfg.view_token,"list"),());}
    }
    let bits=[main];if open{bits.push(disclosure(ctx,t,cfg));}
    let parts=["card"];if open{parts.push("card_open");}
    if t.id==cfg.highlight{parts.push("card_highlight");}
    column(bits).with_key("card-"+t.id).with_style(ctx.component_style(parts,style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(0.5)).padding(theme_length("space.spacing_unit").times(0.75)).radius(theme_length("space.unit").times(2.25))
        .background(theme_color("surface_raised")).hover(style().background(theme_color("surface_hover")))))
        .shared_layout(cfg.scope,t.id).layout_motion(ctx.motion_duration("normal"),ctx.motion_easing("standard"))
}
fn lane_title(ctx,id){
    if id=="needs"{ctx.t_or("agent_inbox.needs","Needs you")}else{state_text(ctx,id)}
}
fn lane(ctx,id,items,cfg,data){
    let label=lane_title(ctx,id);
    let tint=color(if id=="needs"{"waiting"}else{id});
    let title=row([glyph(ctx,"mark","✦",tint,"lane_mark"),
        text(label).with_key("label").accessibility_role("heading").accessibility_level(3)
            .with_style(ctx.component_style("lane_title",style().min_width(px(0)).typography("body").font_weight(500))),
        caption(ctx,ctx.format_number(items.len),"count").with_key("count")])
        .with_key("header").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("lane_head",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).padding_x(theme_length("space.spacing_unit").times(1.25)).items_center()));
    let cards=[];for t in items{cards.push(card(ctx,t,cfg,data));}
    if cards.len==0{cards.push(caption(ctx,ctx.t_or("agent_inbox.empty","Nothing here"),"empty").with_key("empty")
        .with_style(ctx.component_style("empty",style().padding(theme_length("space.spacing_unit").times(2.5)).min_width(px(0)))));}
    let body=column([title,column(cards).with_key("cards").with_style(ctx.component_style("cards",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25))))])
        .with_key("lane-"+id).accessibility_role("group").accessibility_label(label).accessibility_description(ctx.format_number(items.len))
        .with_style(ctx.component_style("lane",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5))));
    body.with_style(if cfg.wide{style().flex_basis(px(0)).flex_grow()}else{style().width(relative(1.0))})
}
fn controls(ctx,props,cfg){
    let bits=[];
    if cfg.actions&&opt(props,"new_label")!=(){
        bits.push(button::Button(#{key:"new-task",text:props.new_label,size:"xs",variant:"ghost",disabled:props.busy,
            on_click:Fn("action").curry(cfg.token,"","new")}));
    }
    if cfg.can_layout{
        let options=[];
        for v in ["board","list"]{
            let chosen=cfg.layout==v;
            let label=if v=="board"{ctx.t_or("agent_inbox.board","Board")}else{ctx.t_or("agent_inbox.list","List")};
            let node=row([text(label)]).with_key(v).accessibility_role("button").accessibility_label(label).accessibility_pressed(chosen).tab_stop(true)
                .on_click(Fn("pick").curry(cfg.view_token,v))
                .with_style(ctx.component_style(if chosen{["option","option_active"]}else{["option"]},
                    style().min_width(px(0)).min_height(theme_length("space.unit").times(6.25)).padding_x(theme_length("space.spacing_unit").times(1.75)).items_center().justify_center().radius(px(5))
                        .background(if chosen{theme_color("surface_raised")}else{rgba(0x00000000)})
                        .typography("caption").focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))));
            options.push(node);
        }
        bits.push(row(options).with_key("views").accessibility_role("group").accessibility_label(ctx.t_or("agent_inbox.layout","Layout"))
            .with_style(ctx.component_style("switcher",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(0.5)).gap(theme_length("space.spacing_unit").times(0.5)).radius(theme_length("space.unit").times(2))
                .background(theme_color("surface_hover")).items_center())));
    }
    row(bits).with_key("controls").with_style(ctx.component_style("controls",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap()))
}
fn render_AgentInbox(ctx,props){
    let shell=element_ref("inbox");let data=project(props);let id=props.inbox;
    let selected=read_selection(ctx,"open",id,"");let active=if selected in data.index{selected}else{""};
    let saved=read_selection(ctx,"cursor",id,"");let cursor=if saved in data.index{saved}else if data.walk.len>0{data.walk[0].id}else{""};
    let external_view=opt(props,"view");let layout=if external_view!=(){external_view}else{read_selection(ctx,"view",id,props.default_view)};
    let token=#{inbox:id,revision:props.revision};
    let view_token=#{inbox:id,revision:props.revision,controlled:external_view!=(),notify:opt(props,"on_view_change")!=()};
    let cfg=#{token:token,view_token:view_token,layout:layout,scope:shell.semantic_id,
        wide:layout=="board"&&ctx.element_width_class(shell,[650])!=0,open:active,cursor:cursor,now:props.now_ms,allow_open:props.allow_open,
        busy:props.busy,highlight:value(props,"highlight",""),actions:opt(props,"on_action")!=(),can_layout:!view_token.controlled||view_token.notify};
    let title=value(props,"title",ctx.t_or("agent_inbox.title","Agent inbox"));
    let breadcrumb=row([caption(ctx,ctx.t_or("delegate.agents","Agents"),"crumb").with_key("crumb"),
        text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
            .with_style(ctx.component_style("title",style().min_width(px(0)).typography("body").font_weight(600)))])
        .with_key("name").audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).flex_grow().gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap());
    let head=row([breadcrumb,controls(ctx,props,cfg)]).with_key("head").with_style(ctx.component_style("head",
        style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap()));
    let lanes=[];for tag in data.lanes{lanes.push(lane(ctx,tag,data.groups[tag],cfg,data));}
    // Board/list are style changes on the same keyed tree, not two copies.
    let board=if cfg.wide{row(lanes).with_style(style().items_start())}else{column(lanes)};
    let nodes=[head,board.with_key("workspace").with_style(ctx.component_style("lanes",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5))))];
    let reading=ctx.t_or("agent_inbox.summary","{needs} need attention · {running} running",
        #{needs:ctx.format_number(data.groups.needs.len),running:ctx.format_number(data.groups.running.len)});
    nodes.push(caption(ctx,reading,"summary").with_key("summary").accessibility_role("status"));
    if opt(props,"notice")!=(){nodes.push(caption(ctx,props.notice,"notice").with_key("notice").accessibility_role("status"));}
    let root=column(nodes).with_key(props.key).with_ref(shell).accessibility_role("group").accessibility_label(title)
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(1000)).padding(theme_length("space.spacing_unit").times(1.5)).gap(theme_length("space.spacing_unit").times(1.75))));
    if active!=""{root=root.on_bubble("key:plain+escape",Fn("close").curry(id,ref_task(data.walk[data.index[active]])));}
    root
}
