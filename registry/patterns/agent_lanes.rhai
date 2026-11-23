/* gpui-rhai
{"id":"patterns/agent_lanes","export":"AgentLanes","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","selection","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted","text_primary","typography.body","typography.caption","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// A clipped, interactive time window with non-overlapping agent tasks and presented hand-offs.
// State: locally selected and focused task. Only the Host changes tasks or accepts a decision.
// Example: AgentLanes(#{key:"activity",timeline:"build",revision:0,now_ms:0,agents:[],tasks:[]})
import "components/button" as button;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_lanes","export":"AgentLanes",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","patterns/approval_card"],capabilities:#{},
        tokens:["accent","border","focus_ring","selection","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted","text_primary",
            "typography.body","typography.caption","space.spacing_unit","space.unit"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable instance key."},
        timeline:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business identity of the activity stream."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision; change it on business updates, not clock ticks."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Heading above the lanes."},
        agents:#{schema:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable lane ID."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Agent name."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional model or role."},
            avatar:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative mark; defaults to a text mark."},
        }}},required:true,sensitive:false,doc:"Lanes in supplied order."},
        tasks:#{schema:#{type:"array",max_items:128,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable task ID."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"One declared lane ID."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Task name on the timeline and in inspection."},
            status:#{schema:#{type:"string",allowed:["running","waiting","done","failed"]},required:true,sensitive:false,doc:"Accepted activity state."},
            start_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Start on the Host clock."},
            end_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"An explicit end after or equal to start. Required for done or failed; omitted ongoing tasks reach now."},
            handoff_to:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Successor ID. Drawn only when both ends are in the displayed window."},
            ask:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Decision presented for a waiting task."},
            summary:#{schema:#{type:"optional",value:#{type:"array",max_items:8,items:#{type:"string"}}},required:false,sensitive:true,doc:"Read-only observations shown in the task inspector."},
            tools:#{schema:#{type:"optional",value:#{type:"array",max_items:3,items:#{type:"object",allow_unknown:false,fields:#{
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Tool name included in the accessible description."},
                icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative tool mark; no tool is invoked."},
            }}}},required:false,sensitive:false,doc:"Up to three tool marks."},
            busy:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Disables waiting-task proposals until the Host decides."},
        }}},required:true,sensitive:true,doc:"Tasks on the shared timeline. Overlapping tasks occupy different tracks without changing their time."},
        now_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Current time supplied by the Host."},
        span_ms:#{schema:#{type:"integer",min:1000,max:604800000},"default":#{type:"integer",value:1800000},required:false,sensitive:false,doc:"Width of the visible history window up to now."},
        highlight:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional highlighted agent. Other lane tasks remain available."},
        allow_open:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Show a Host navigation proposal in the inspector."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives approve, reject, or open proposals for the accepted revision."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"A Host response displayed without changing local task state."},
    },state:#{fields:#{
        open:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        cursor:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
    }},events:#{
        action:#{payload:#{type:"object",allow_unknown:false,fields:#{
            timeline:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Exact business stream."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision displayed when the proposal was made."},
            task:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Task being acted on."},
            kind:#{schema:#{type:"string",allowed:["approve","reject","open"]},required:true,sensitive:false,doc:"Intent; opening has no approval side effect."},
        }},doc:"Validate the task and revision in the Host before accepting a proposal."},
    },slots:#{},parts:["root","head","title","summary","pressure","card","stage","axis","tick","lane","agent","agent_name","model","avatar","track","task",
        "task_running","task_waiting","task_done","task_failed","task_open","task_title","duration","marker","tool","inspector","body","inspector_title","question","empty","notice","paint"]},
    render:Fn("render_AgentLanes")
});

fn AgentLanes(props){render_component("patterns/agent_lanes",props)}
fn opt(m,key){common::opt(m,key)}
fn value(m,key,default_value){common::value_or(m,key,default_value)}
fn valid_id(id){id!=""&&utf8_byte_count(id)<=128}
fn clamp(n){min(1.0,max(0.0,n))}
fn fraction(time,from,span){clamp((time-from).to_float()/span.to_float())}
fn order(a,b){
    if a.start_ms==b.start_ms{return a.ordinal-b.ordinal;}
    if a.start_ms<b.start_ms{-1}else{1}
}
fn occupied(data,width,from,span,now){
    let finish=value(data,"end_ms",now);
    if data.start_ms>now||finish<from||(finish==from&&data.start_ms<from){return ();}
    let start=fraction(data.start_ms,from,span);let end=fraction(finish,from,span);
    let min_hit=min(1.0,22.0/max(22.0,width));
    let left=min(start,1.0-min_hit);
    let right=max(end,left+min_hit);
    #{job:data,left:left,right:right,x1:start,x2:end}
}
fn place(tasks,width,from,span,now){
    tasks.sort(Fn("order"));
    let placed=[];let cursors=[];
    // Visual targets, including the minimum hit width, must never overlap on a track.
    let gap=min(0.1,3.0/max(22.0,width));
    for t in tasks{
        let p=occupied(t,width,from,span,now);
        if p==(){continue;}
        let slot=cursors.len;
        for i in 0..cursors.len{if cursors[i]+gap<=p.left{slot=i;break;}}
        if slot==cursors.len{cursors.push(p.right);}else{cursors[slot]=p.right;}
        p.track=slot;placed.push(p);
    }
    #{tracks:max(1,cursors.len),items:placed}
}
fn prepare(props,width){
    if !valid_id(props.timeline){throw "Timeline identity must be nonempty and bounded";}
    let lists=#{};for agent in props.agents{
        if !valid_id(agent.id)||agent.id in lists{throw "Timeline agent IDs must be unique and bounded";}
        lists[agent.id]=[];
    }
    let seen=#{};let from=props.now_ms-props.span_ms;let running=0;let waiting=0;
    for i in 0..props.tasks.len{
        let t=props.tasks[i];t.ordinal=i;
        if !valid_id(t.id)||t.id in seen||!(t.agent in lists){throw "Timeline tasks need a unique ID and a declared agent";}
        let finish=opt(t,"end_ms");
        if (finish!=()&&finish<t.start_ms)||(t.status in ["done","failed"]&&(finish==()||finish>props.now_ms)){
            throw "Timeline task end does not match the accepted state or clock";
        }
        seen[t.id]=true;lists[t.agent].push(t);
        if t.start_ms<=props.now_ms&&(finish==()||finish>props.now_ms){
            if t.status=="running"{running+=1;}else if t.status=="waiting"{waiting+=1;}
        }
    }
    let lanes=#{};let lookup=#{};let walk=[];
    for a in props.agents{
        let packed=place(lists[a.id],width,from,props.span_ms,props.now_ms);
        lanes[a.id]=packed;for p in packed.items{lookup[p.job.id]=p;walk.push(p.job);}
    }
    #{lanes:lanes,lookup:lookup,walk:walk,running:running,waiting:waiting,from:from}
}
fn duration(ctx,ms){
    if ms<60000{return ctx.t_or("agent_lanes.seconds","{n} s",#{n:ctx.format_number(ms/1000)});}
    if ms<3600000{return ctx.t_or("agent_lanes.minutes","{n} min",#{n:ctx.format_number(ms/60000)});}
    ctx.t_or("agent_lanes.hours","{n} h",#{n:ctx.format_number(ms/3600000)})
}
fn axis_duration(ctx,ms){
    // Tick locations are fractions of the exact span. Do not floor the
    // label to a whole minute and imply a different point on the axis.
    let units=if ms<60000{1000.0}else if ms<3600000{60000.0}else{3600000.0};
    let n=ctx.format_number(ms.to_float()/units,#{min_fraction_digits:0,max_fraction_digits:2});
    if ms<60000{ctx.t_or("agent_lanes.seconds","{n} s",#{n:n})}
    else if ms<3600000{ctx.t_or("agent_lanes.minutes","{n} min",#{n:n})}
    else{ctx.t_or("agent_lanes.hours","{n} h",#{n:n})}
}
fn status(ctx,s){
    if s=="running"{ctx.t_or("agent_lanes.running","Running")}else if s=="waiting"{ctx.t_or("agent_lanes.waiting","Needs review")}
    else if s=="failed"{ctx.t_or("agent_lanes.failed","Failed")}else{ctx.t_or("agent_lanes.done","Done")}
}
fn tone(s){theme_color(if s=="running"{"accent"}else if s=="waiting"{"text.warning"}else if s=="failed"{"text.danger"}else{"text.success"})}
fn caption(ctx,v,part){common::heading_note(ctx,v,part).with_style(style().min_width(px(0)))}
fn key_ref(t){"task-"+t.ordinal.to_string()}
fn select_lane_task(timeline,id,ctx,event){
    let old=ctx.get_state("open");ctx.set_state("open",if old!=()&&old.timeline==timeline&&old.id==id{()}else{#{timeline:timeline,id:id}});
    ctx.set_state("cursor",id);handled()
}
fn focus_task(id,key,ctx,event){ctx.set_state("cursor",id);ctx.focus(key);handled()}
fn close(key,ctx,event){ctx.set_state("open",());ctx.focus(key);handled()}
fn propose(t,id,kind,ctx,event){ctx.emit("action",#{timeline:t.timeline,revision:t.revision,task:id,kind:kind});handled()}
fn tool_mark(ctx,t,i){
    let icon=opt(t,"icon");
    let mark=if icon==(){text(if t.label==""{"•"}else{t.label.sub_string(0,1).to_upper()})}else{icon};
    row([mark.with_key("mark")]).with_key("tool-"+i.to_string()).accessibility_hidden(true)
        .with_style(ctx.component_style("tool",style().width(px(13)).height(px(13)).typography("caption").items_center().justify_center().flex_shrink(false)))
}
fn track_offset(index){4+index*34}
fn track_length(points){theme_length("space.unit").times(points.to_float()/4.0)}
fn task(ctx,p,cfg,ref_item,walk,index){
    let t=p.job;let chosen=t.id==cfg.open;let styles=["task","task_"+t.status];
    if chosen{styles.push("task_open");}
    let ink=tone(t.status);let width=p.right-p.left;
    let left=if cfg.rtl{1.0-p.right}else{p.left};
    let stamp=clamp((p.x1-p.left)/width);let true_width=clamp((p.x2-p.x1)/width);
    let offset=if cfg.rtl{1.0-stamp-true_width}else{stamp};
    let painted=max(true_width,min(1.0,1.0/(cfg.width*width)));
    let line=row([]).with_key("exact-time").accessibility_hidden(true).with_style(ctx.component_style("marker",style().absolute().bottom(px(0))
        .left(relative(min(1.0-painted,clamp(offset)))).width(relative(painted)).height(px(1)).background(ink)));
    let contents=[text(t.title).with_key("title").with_style(ctx.component_style("task_title",style().min_width(px(0)).flex_grow()
        .typography("caption").font_weight(500).text_ellipsis().whitespace_nowrap()))];
    let marks=value(t,"tools",[]);
    if width*cfg.width>=100.0{for i in 0..marks.len{contents.push(tool_mark(ctx,marks[i],i));}}
    contents.push(line);
    let desc=status(ctx,t.status)+" · "+duration(ctx,min(cfg.now,value(t,"end_ms",cfg.now))-t.start_ms);
    for m in marks{desc+=" · "+m.label;}
    let next=walk[min(walk.len-1,index+1)];let prev=walk[max(0,index-1)];
    let item=row(contents).with_key("task-"+t.id).with_ref(ref_item).accessibility_role("button").accessibility_label(t.title)
        .accessibility_description(desc).accessibility_pressed(chosen).tab_stop(t.id==cfg.cursor)
        .with_style(ctx.component_style(styles,style().gap(theme_length("space.spacing_unit")).padding_x(theme_length("space.spacing_unit").times(1.25)).items_center().min_width(px(0))
            .radius(px(6)).text_color(ink).background(theme_color(if chosen{"selection"}else{"surface_hover"}))
            .hover(style().background(theme_color("selection"))).focus_ring(px(1),offset_px(-1),theme_color("focus_ring")).clip()))
        // Position and hit box come from the same rectangle. Theme variants
        // restyle the tile but cannot replace its interval or track geometry.
        .with_style(style().absolute().top(track_length(track_offset(p.track))).left(relative(left)).width(relative(width)).height(track_length(29)))
        .shared_layout(cfg.scope,t.id).layout_motion(ctx.motion_duration("normal"),ctx.motion_easing("standard"))
        .on_click(Fn("select_lane_task").curry(cfg.token.timeline,t.id))
        .on_key_value("plain+up",Fn("focus_task").curry(prev.id,key_ref(prev)),())
        .on_key_value("plain+down",Fn("focus_task").curry(next.id,key_ref(next)),())
        .on_key_value("plain+left",Fn("focus_task").curry(if cfg.rtl{next.id}else{prev.id},if cfg.rtl{key_ref(next)}else{key_ref(prev)}),())
        .on_key_value("plain+right",Fn("focus_task").curry(if cfg.rtl{prev.id}else{next.id},if cfg.rtl{key_ref(prev)}else{key_ref(next)}),());
    item.on_key_value("plain+home",Fn("focus_task").curry(walk[0].id,key_ref(walk[0])),())
        .on_key_value("plain+end",Fn("focus_task").curry(walk[walk.len-1].id,key_ref(walk[walk.len-1])),())
}
fn agent_mark(ctx,a){
    let icon=opt(a,"avatar");let base=if icon==(){text(if a.name==""{"•"}else{a.name.sub_string(0,1).to_upper()})}else{icon};
    row([base]).with_key("avatar").accessibility_hidden(true).with_style(ctx.component_style("avatar",style().width(theme_length("space.unit").times(6)).height(theme_length("space.unit").times(6))
        .flex_shrink(false).items_center().justify_center().typography("caption").radius(theme_length("space.unit").times(5)).background(theme_color("surface_hover"))))
}
fn lane(ctx,a,packed,cfg,refs,data,indices){
    let labels=[text(a.name).with_key("name").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("agent_name",style().min_width(px(0)).typography("body").font_weight(500).text_ellipsis().whitespace_nowrap()))];
    if opt(a,"model")!=(){labels.push(caption(ctx,a.model,"model").with_key("model").with_style(style().text_ellipsis().whitespace_nowrap()));}
    let lead=row([agent_mark(ctx,a),column(labels).with_key("who").with_style(style().min_width(px(0)).flex_grow())]).with_key("agent")
        .audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("agent",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).items_center().padding_x(theme_length("space.spacing_unit").times(1.25))));
    if !cfg.compact{lead=lead.with_style(style().width(theme_length("space.unit").times(32)).flex_shrink(false));}
    let children=[];
    for p in packed.items{children.push(task(ctx,p,cfg,refs[p.job.id],data.walk,indices[p.job.id]));}
    // Track offsets and its containing lane use the same themed coordinate scale.
    let lane_style=style().min_width(px(0)).height(track_length(track_offset(packed.tracks)+5)).relative();
    let rail=box(children).with_key("track").with_style(ctx.component_style("track",lane_style))
        .with_style(if cfg.compact{style().width(relative(1.0))}else{style().flex_basis(px(0)).flex_grow()});
    let combined=if cfg.compact{column([lead,rail]).with_style(style().gap(theme_length("space.spacing_unit")))}else{row([lead,rail]).with_style(style().items_center())};
    let subdued=cfg.highlight!=""&&cfg.highlight!=a.id;
    combined.with_key("agent-"+a.id).with_style(ctx.component_style("lane",style().min_width(px(0))
        .border_bottom(px(1)).border_color(theme_color("border")).opacity(if subdued{0.62}else{1.0})))
}
fn axis(ctx,props,cfg,axis_ref){
    let labels=[];let markers=if cfg.compact{3}else{5};
    for i in 0..markers{
        let remain=(markers-1-i)*props.span_ms/(markers-1);
        let title=if i==markers-1{ctx.t_or("agent_lanes.now","Now")}else{ctx.t_or("agent_lanes.ago","{time} ago",#{time:axis_duration(ctx,remain)})};
        let width=if cfg.compact{0.30}else{0.20};
        let offset=clamp(i.to_float()/(markers-1).to_float()-width/2.0);
        let x=min(offset,1.0-width);
        let align=if i==0{style().justify_start()}else if i==markers-1{style().justify_end()}else{style().justify_center()};
        labels.push(row([caption(ctx,title,"tick").with_key("label").with_style(style().whitespace_nowrap().text_ellipsis())])
            .with_key("tick-"+i.to_string()).with_style(align.min_width(px(0)).items_center().absolute()
            .left(relative(if cfg.rtl{1.0-x-width}else{x})).width(relative(width)).height(relative(1.0))));
    }
    let bar=box(labels).with_key("timeline-ticks").with_ref(axis_ref).accessibility_role("group")
        .accessibility_label(ctx.t_or("agent_lanes.axis","Time axis"))
        .with_style(ctx.component_style("axis",style().min_width(px(0)).height(theme_length("space.unit").times(6)).relative()));
    if cfg.compact{return bar;}
    row([box([]).with_key("label-gutter").with_style(style().width(theme_length("space.unit").times(32)).flex_shrink(false)),bar.with_style(style().flex_basis(px(0)).flex_grow())]).with_key("axis-row")
}
fn anchor(b,relative_time,rtl){
    #{x:b.x+b.width*(if rtl{1.0-relative_time}else{relative_time}),y:b.y+b.height*0.5}
}
fn links(ctx,stage_ref,axis_ref,refs,data,cfg){
    let stage=ctx.element_bounds(stage_ref);let rail=ctx.element_bounds(axis_ref);
    if stage==()||rail==(){return ();}
    let box=stage.visual;let width=box.width;let height=box.height;
    if width<10.0||height<10.0{return ();}
    let r=rail.visual;let commands=[];let ticks=if cfg.compact{3}else{5};
    for i in 0..ticks{
        let x=r.x-box.x+(r.width-1.0)*i.to_float()/(ticks-1).to_float();
        commands.push(canvas_line("grid-"+i.to_string(),x,r.y+r.height-box.y,x,height-1.0,0.5,theme_color("border")));
    }
    for t in data.walk{
        let id=opt(t,"handoff_to");if id==()||id==t.id||!(id in refs){continue;}
        let source=ctx.element_bounds(refs[t.id]);let dest=ctx.element_bounds(refs[id]);
        if source==()||dest==(){continue;}
        let a=data.lookup[t.id];let b=data.lookup[id];
        let from=anchor(source.visual,clamp((a.x2-a.left)/(a.right-a.left)),cfg.rtl);
        let to=anchor(dest.visual,clamp((b.x1-b.left)/(b.right-b.left)),cfg.rtl);
        let x=from.x-box.x;let y=from.y-box.y;let tx=to.x-box.x;let ty=to.y-box.y;
        let bend=(if cfg.rtl{-1.0}else{1.0})*max(9.0,(tx-x).abs()*0.42);
        let line=if b.job.status=="running"{theme_color("accent")}else{theme_color("text_muted")};
        commands.push(canvas_stroke_path("handoff-"+t.ordinal.to_string(),[path_move(x,y),path_cubic(tx,ty,x+bend,y,tx-bend,ty)],
            1,line).stroke_cap("round"));
    }
    canvas(canvas_scene(commands).view_box(0,0,width,height)).with_key("handoffs").accessibility_hidden(true)
        .with_style(ctx.component_style("paint",style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0))))
}
fn inspector(ctx,props,cfg,t,data){
    let lines=[text(t.title).with_key("title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("inspector_title",style().typography("body").font_weight(600))),
        caption(ctx,status(ctx,t.status)+" · "+duration(ctx,min(cfg.now,value(t,"end_ms",cfg.now))-t.start_ms),"body").with_key("state")];
    let question=opt(t,"ask");if question!=(){lines.push(caption(ctx,question,"question").with_key("question"));}
    let content=value(t,"summary",[]);for i in 0..content.len{lines.push(caption(ctx,content[i],"body").with_key("note-"+i.to_string()));}
    let id=opt(t,"handoff_to");
    if id!=()&&id in data.lookup{lines.push(caption(ctx,ctx.t_or("agent_lanes.handoff","Hands off to {task}",#{task:data.lookup[id].job.title}),"body").with_key("handoff"));}
    let controls=[];let enabled=opt(props,"on_action")!=();
    if enabled&&t.status=="waiting"{
        let busy=value(t,"busy",false);
        controls.push(button::Button(#{key:"approve",text:ctx.t_or("review.approve","Approve"),size:"sm",disabled:busy,on_click:Fn("propose").curry(cfg.token,t.id,"approve")}));
        controls.push(button::Button(#{key:"reject",text:ctx.t_or("gates.reject","Reject"),size:"sm",variant:"secondary",disabled:busy,on_click:Fn("propose").curry(cfg.token,t.id,"reject")}));
    }
    if enabled&&props.allow_open{
        controls.push(button::Button(#{key:"open",text:ctx.t_or("agent_lanes.open","Open task"),size:"sm",variant:"ghost",on_click:Fn("propose").curry(cfg.token,t.id,"open")}));
    }
    controls.push(button::Button(#{key:"close",text:ctx.t_or("agent_queue.close","Close details"),size:"sm",variant:"ghost",on_click:Fn("close").curry(key_ref(t))}));
    lines.push(row(controls).with_key("controls").with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap()));
    column(lines).with_key("inspector").accessibility_role("region").accessibility_label(ctx.t_or("agent_queue.details","Job details"))
        .with_style(ctx.component_style("inspector",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).padding(theme_length("space.spacing_unit").times(2)).background(theme_color("surface_hover")).radius(theme_length("space.unit").times(2))))
}
fn render_AgentLanes(ctx,props){
    let shell=element_ref("lanes");let stage_ref=element_ref("stage");let axis_ref=element_ref("axis");
    let measure=ctx.element_size(axis_ref);let width=if measure==()||measure.width<1.0{440.0}else{measure.width};
    let data=prepare(props,width);let selected=ctx.get_state("open");
    let active=if selected!=()&&selected.timeline==props.timeline&&selected.id in data.lookup{selected.id}else{""};
    let cursor=ctx.get_state("cursor");if !(cursor in data.lookup)&&data.walk.len>0{cursor=data.walk[0].id;}
    let cfg=#{token:#{timeline:props.timeline,revision:props.revision},now:props.now_ms,scope:shell.semantic_id,open:active,cursor:cursor,width:width,
        compact:ctx.element_width_class(shell,[530])==0,rtl:ctx.text_direction()=="rtl",highlight:value(props,"highlight","")};
    let refs=#{};let indices=#{};
    for i in 0..data.walk.len{let t=data.walk[i];refs[t.id]=element_ref(key_ref(t));indices[t.id]=i;}
    let rows=[axis(ctx,props,cfg,axis_ref)];
    for agent in props.agents{rows.push(lane(ctx,agent,data.lanes[agent.id],cfg,refs,data,indices));}
    if props.agents.len==0{rows.push(caption(ctx,ctx.t_or("agent_lanes.empty","No agents to show"),"empty").with_key("empty"));}
    let stage=column(rows).with_key("lanes").with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit")));
    let decor=links(ctx,stage_ref,axis_ref,refs,data,cfg);
    let content=if decor==(){[stage]}else{[decor,stage]};
    let title=value(props,"title",ctx.t_or("agent_lanes.title","Agent activity"));
    let label=ctx.t_or("agent_lanes.summary","{run} running · {wait} need review",#{run:ctx.format_number(data.running),wait:ctx.format_number(data.waiting)});
    let header=row([text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
        .with_style(ctx.component_style("title",style().min_width(px(0)).flex_grow().typography("body").font_weight(600))),
        caption(ctx,label,"summary").with_key("summary").accessibility_role("status")])
        .with_key("head").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("head",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.75)).items_center().flex_wrap()));
    let body=[stack(content).with_key("stage").with_ref(stage_ref).accessibility_role("group")
        .accessibility_label(ctx.t_or("agent_lanes.stage","Activity timeline"))
        .with_style(ctx.component_style("stage",style().min_width(px(0)).relative()))];
    if active!=""{body.push(inspector(ctx,props,cfg,data.lookup[active].job,data));}
    let nodes=[header,column(body).with_key("card").with_style(ctx.component_style("card",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(1.5)).gap(theme_length("space.spacing_unit").times(1.75))
        .radius(theme_length("space.unit").times(2.5)).background(theme_color("surface_raised"))))];
    if opt(props,"notice")!=(){nodes.push(caption(ctx,props.notice,"notice").with_key("notice").accessibility_role("status"));}
    let root=column(nodes).with_key(props.key).with_ref(shell).accessibility_role("group").accessibility_label(title)
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(960)).gap(theme_length("space.spacing_unit").times(1.5)).padding(theme_length("space.spacing_unit").times(1.5))));
    if active!=""{root=root.on_bubble("key:plain+escape",Fn("close").curry(key_ref(data.lookup[active].job)));}
    root
}
