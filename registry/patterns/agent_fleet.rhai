/* gpui-rhai
{"id":"patterns/agent_fleet","export":"AgentFleet","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/popover","patterns/agent_tree","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","agent_tree.link","border","focus_ring","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],"environment":[]}
*/
// A run overview with one expanded lead, a child roster and a task inspector.
// Example: AgentFleet(#{key:"team",run:"preview",revision:0,now_ms:0,agents:[]})
import "components/popover" as popover;
import "patterns/agent_tree" as team;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_fleet","export":"AgentFleet",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/popover","patterns/agent_tree","patterns/approval_card"],capabilities:#{},
        tokens:["accent","agent_tree.link","border","focus_ring","space.spacing_unit","space.unit","surface_hover","surface_raised",
            "text_muted","text_primary","typography.body","typography.caption"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable fleet instance key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business identity of the run."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision, independent of clock ticks."},
        now_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Host clock used by every active agent."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Fleet heading."},
        currency:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Currency label, defaults to USD."},
        agents:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable unique business identity."},
            parent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Parent agent ID; omit for a lead."},
            name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Agent display name."},
            task:#{schema:#{type:"string"},required:true,sensitive:true,doc:"What the agent was assigned."},
            state:#{schema:#{type:"string",allowed:["queued","running","waiting","done","failed","stopped"]},required:true,sensitive:false,doc:"Accepted execution state."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional model caption."},
            avatar:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative avatar, defaults to initials."},
            tools:#{schema:#{type:"optional",value:#{type:"array",max_items:6,items:#{type:"object",allow_unknown:false,fields:#{
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Tool name."},
                icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative tool mark."},
            }}}},required:false,sensitive:false,doc:"Tools present in the task's summary."},
            cost_micros:#{schema:#{type:"optional",value:#{type:"integer",min:-1000000000000,max:1000000000000}},required:false,sensitive:false,doc:"Own cost (or credit if negative) in millionths of the currency unit."},
            started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Start time in the Host clock domain."},
            ended_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"End time in the Host clock domain."},
            ask:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Approval evidence for a waiting agent."},
            error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Failure evidence."},
            steps:#{schema:#{type:"optional",value:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique step ID within this agent."},
                title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Task detail."},
                state:#{schema:#{type:"optional",value:#{type:"string",allowed:["queued","running","waiting","done","failed","stopped"]}},
                    required:false,sensitive:false,doc:"Defaults to done."},
                elapsed_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Fixed task duration."},
                tool:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional tool caption."},
            }}}},required:false,sensitive:true,doc:"Ordered task history available in the inspector."},
        }}},required:true,sensitive:true,doc:"A flat forest, including all leads and their sub-agents. Parents may follow their children in input order."},
        controlled_open:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Use open as the only source of disclosure. When false, the component owns disclosure."},
        open:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted open lead ID in controlled mode; null means none."},
        default_open:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initial local open lead, defaults to first. An empty string starts closed."},
        highlight:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Emphasize this agent. A new highlight reveals its lead in local mode; it never overrides controlled disclosure."},
        on_open_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a versioned disclosure proposal."},
        on_select:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives selection of a sub-agent (for example, to open a log)."},
        on_resolve:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives approval or failure-resolution intent."},
        busy_agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Agent whose resolution is still being processed."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host feedback, also visible inside an open inspector."},
    },state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{
        open_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run identity."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Observed revision."},
            lead:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Requested lead; empty string requests closing."},
        }},doc:"The Host must accept the requested lead when controlled_open is enabled."},
        select:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run identity."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Observed revision."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Selected sub-agent."},
        }},doc:"Does not modify the accepted agent state."},
        resolve:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run identity."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Observed revision."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Agent to resolve."},
            action:#{schema:#{type:"string",allowed:["approve","reject","retry","dismiss"]},required:true,sensitive:false,doc:"Requested action."},
        }},doc:"Validate run, revision, agent and state in the Host. A refusal leaves the inspector available."},
    },slots:#{},parts:["root","bar","title","stats","count","total","body","grid","lead","lead_open","lead_quiet","sub","highlight","head","avatar","name",
        "model","status","status_queued","status_running","status_waiting","status_done","status_failed","status_stopped","tool","tools","task","cost",
        "time","summary","summary_value","branch","branch_empty","links","inspect","inspect_title","evidence","steps","step","step_tool","step_state","step_time",
        "review_actions","empty","notice"],effects:["settle_branch"]},
    render:Fn("render_AgentFleet")
});

fn AgentFleet(props){render_component("patterns/agent_fleet",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,d){common::value_or(m,k,d)}
fn note(ctx,s,part){common::heading_note(ctx,s,part)}
fn fleet_unit(n){theme_length("space.unit").times(n)}
fn fleet_gap(n){theme_length("space.spacing_unit").times(n)}
fn local(ctx,run){
    let s=ctx.get_state("ui");if s!=()&&s.run==run{return s;}
    #{run:run,open:(),cursor:"",inspect:"",inspect_lead:"",received_highlight:"",branch:""}
}
fn key_ref(data,id){"agent-"+data.rows[id].i.to_string()}
fn active_lead(props,ui,data,highlight){
    let first=if data.roots.len>0{data.roots[0]}else{""};
    let id=if props.controlled_open{value(props,"open","")}
        else if highlight!=""&&ui.received_highlight!=highlight{data.family[highlight]}
        else if ui.open!=(){ui.open}
        else{value(props,"default_open",first)};
    if id in data.roots{id}else if !props.controlled_open&&id!=""{first}else{""}
}
fn propose(token,id,ctx,e){
    let ui=local(ctx,token.run);
    if !token.controlled{ui.open=id;ui.received_highlight=token.highlight;}
    ctx.set_state("ui",ui);
    ctx.emit("open_change",#{run:token.run,revision:token.revision,lead:id});handled()
}
fn focus_lead(run,id,reference,ctx,e){
    let ui=local(ctx,run);ui.cursor=id;ctx.set_state("ui",ui);
    ctx.focus(reference);ctx.scroll_into_view(reference);handled()
}
fn sync_lead_focus(run,id,ctx,inside){
    if inside{let ui=local(ctx,run);if ui.cursor!=id{ui.cursor=id;ctx.set_state("ui",ui);}}
}
fn inspect(token,id,lead,ctx,open){
    let ui=local(ctx,token.run);
    if open{ui.inspect=id;ui.inspect_lead=lead;}
    else if ui.inspect==id{ui.inspect="";}
    ctx.set_state("ui",ui);
    if open{ctx.emit("select",#{run:token.run,revision:token.revision,agent:id});}
}
fn select_only(token,id,ctx,e){ctx.emit("select",#{run:token.run,revision:token.revision,agent:id});handled()}
fn settle(ctx,p){
    let ui=local(ctx,p.run);
    if ui.branch==p.lead&&ui.inspect==""{return;}
    if ui.branch!=p.lead{
        let underneath=ui.inspect!=""&&ui.inspect==p.old_inspect&&ui.inspect_lead!=p.lead;
        ui.branch=p.lead;
        if underneath{ui.inspect="";ui.inspect_lead="";}
        ctx.set_state("ui",ui);
    }
}
fn stop(ctx,p){}
fn summarize(data){
    let result=#{};
    for root in data.roots{result[root]=#{count:0,done:0,running:0,waiting:0,depth:0};}
    for id in data.order{
        let top=data.family[id];if id==top{continue;}
        let r=result[top];let state=data.rows[id].a.state;
        r.count+=1;r.depth=max(r.depth,data.depth[id]);
        if state=="done"{r.done+=1;}else if state=="running"{r.running+=1;}else if state=="waiting"{r.waiting+=1;}
        result[top]=r;
    }
    result
}
fn tool_marks(ctx,tools){
    let nodes=[];
    for i in 0..min(4,tools.len){
        let tool=tools[i];let icon=opt(tool,"icon");
        let v=if icon!=(){icon}else{text(if tool.label==""{"?"}else{tool.label.sub_string(0,1).to_upper()})};
        nodes.push(row([v]).with_key("tool-"+i.to_string()).accessibility_hidden(true).with_style(ctx.component_style("tool",
            style().width(fleet_unit(4)).height(fleet_unit(4)).radius(px(4)).items_center().justify_center().flex_shrink(false)
            .typography("caption").background(theme_color("surface_hover")))));
    }
    row(nodes).with_key("tools").with_style(ctx.component_style("tools",style().gap(fleet_gap(0.5)).flex_wrap()))
}
fn progress_mark(done,total){
    let ink=theme_color("accent");
    let commands=[canvas_circle("edge",6,6,5,theme_color("border")),canvas_circle("well",6,6,4,theme_color("surface_raised"))];
    if done>0{
        let angle=6.283185307179586*done.to_float()/max(1,total).to_float();
        let slice=[path_move(6,6)];
        for i in 0..17{
            let theta=angle*i.to_float()/16.0-1.5707963267948966;
            slice.push(path_line(6.0+theta.cos()*3.6,6.0+theta.sin()*3.6));
        }
        slice.push(path_close());commands.push(canvas_fill_path("done",slice,ink));
    }
    canvas(canvas_scene(commands).view_box(0,0,12,12)).with_key("progress-mark").accessibility_hidden(true)
        .with_style(style().width(px(12)).height(px(12)).flex_shrink(false))
}
fn header(ctx,props,cfg,data,title){
    let total=team::amount(ctx,data.total,cfg.currency);
    let pieces=[note(ctx,ctx.t_or("agent_tree.agents","Agents: {n}",#{n:ctx.format_number(props.agents.len)}),"count").with_key("count")];
    if data.running>0{pieces.push(note(ctx,ctx.t_or("agent_tree.running","Running: {n}",#{n:ctx.format_number(data.running)}),"count").with_key("running"));}
    if data.waiting.len>0{pieces.push(note(ctx,ctx.t_or("agent_tree.needs","Needs you: {n}",#{n:ctx.format_number(data.waiting.len)}),"count").with_key("waiting"));}
    if data.failed.len>0{pieces.push(note(ctx,ctx.t_or("agent_tree.failures","Failed: {n}",#{n:ctx.format_number(data.failed.len)}),"count").with_key("failed"));}
    pieces.push(note(ctx,total,"total").with_key("total").accessibility_role("status")
        .accessibility_label(ctx.t_or("agent_tree.total","Run total: {amount}",#{amount:total})));
    row([text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
            .with_style(ctx.component_style("title",style().min_width(px(0)).typography("body").font_weight(600))),
        row(pieces).with_key("stats").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("stats",style().min_width(px(0))
            .flex_grow().justify_end().items_center().gap(fleet_gap(1.5)).flex_wrap()))])
        .with_key("bar").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("bar",style().min_width(px(0)).gap(fleet_gap(2)).items_center().flex_wrap()))
}
fn overview(ctx,cfg,data,id,lead){
    let a=data.rows[id].a;let model=opt(a,"model");let tools=value(a,"tools",[]);
    let who=[text(a.name).with_key("name").with_style(ctx.component_style("name",style().min_width(px(0)).typography("body").font_weight(600)))];
    if model!=(){who.push(note(ctx,model,"model").with_key("model"));}
    let caption=column(who).with_key("who").with_style(style().min_width(px(0)).flex_grow());
    let lines=[row([team::avatar(ctx,a),caption,team::badge(ctx,a)]).with_key("head").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("head",style().min_width(px(0)).items_center().flex_wrap().gap(fleet_gap(1.25))))];
    let assignment=[note(ctx,a.task,"task").with_key("task").with_style(style().flex_grow())];
    if tools.len>0{assignment.insert(0,tool_marks(ctx,tools));}
    lines.push(row(assignment).with_key("assignment").audit_allow(["mixed-type-in-row"])
        .with_style(style().min_width(px(0)).items_center().gap(fleet_gap(1.25)).flex_wrap()));
    let cost=team::amount(ctx,data.cost[id],cfg.currency);let elapsed=team::duration(ctx,a,cfg.now);
    let numbers=[note(ctx,cost,"cost").with_key("cost")];
    if elapsed!=""{numbers.push(note(ctx,elapsed,"time").with_key("time"));}
    lines.push(row(numbers).with_key("numbers").with_style(style().min_width(px(0)).gap(fleet_gap(1)).justify_between().flex_wrap()));
    let desc=a.task+" · "+team::status(ctx,a.state)+" · "+cost;
    if elapsed!=""{desc+=" · "+elapsed;}
    for tool in tools{desc+=" · "+tool.label;}
    if lead{
        let stats=cfg.stats[id];
        let n=stats.count;let done=stats.done;let running=stats.running;let waiting=stats.waiting;
        let work=ctx.t_or("agent_fleet.progress","{done}/{n} done",#{done:ctx.format_number(done),n:ctx.format_number(n)});
        if running>0{work+=" · "+ctx.t_or("agent_tree.running","Running: {n}",#{n:ctx.format_number(running)});}
        if waiting>0{work+=" · "+ctx.t_or("agent_tree.needs","Needs you: {n}",#{n:ctx.format_number(waiting)});}
        if n==0{work=ctx.t_or("agent_fleet.empty_branch","No sub-agents yet");}
        let affordance=if cfg.open==id{"⌃"}else{"⌄"};
        lines.push(row([progress_mark(done,n),note(ctx,work,"summary_value").with_key("progress").with_style(style().flex_grow()),
            text(affordance).with_key("mark").accessibility_hidden(true)]).with_key("progress")
            .with_style(ctx.component_style("summary",style().min_width(px(0)).gap(fleet_gap(1)).items_center())));
        desc+=" · "+work;
    }
    #{lines:lines,desc:desc}
}
fn nav_lead(n,combo,cfg,data,id){
    n.on_key_value(combo,Fn("focus_lead").curry(cfg.token.run,id,key_ref(data,id)),())
}
fn lead_card(ctx,cfg,data,id,index,reference){
    let packed=overview(ctx,cfg,data,id,true);let open=cfg.open==id;
    let target=if open{""}else{id};
    let live=!cfg.token.controlled||cfg.can_open;
    let parts=["lead"];
    if open{parts.push("lead_open");}else if cfg.open!=""{parts.push("lead_quiet");}
    if cfg.highlight==id{parts.push("highlight");}
    let a=data.rows[id].a;let n=column(packed.lines).with_key("lead:"+id).with_ref(reference)
        .accessibility_role("button").accessibility_label(a.name).accessibility_description(packed.desc).accessibility_expanded(open)
        .tab_stop(id==cfg.focus).audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style(parts,style().min_width(px(0)).gap(fleet_gap(1.25)).padding(fleet_gap(2))
            .radius(fleet_unit(2.5)).background(theme_color("surface_raised")).border(px(1)).border_color(theme_color(if open{"accent"}else{"border"}))
            .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .on_focus_within_change(Fn("sync_lead_focus").curry(cfg.token.run,id));
    if live{n=n.on_click(Fn("propose").curry(cfg.token,target))
        .on_key_value("plain+enter",Fn("propose").curry(cfg.token,target),());}
    let leads=data.roots;
    n=nav_lead(n,"plain+up",cfg,data,leads[max(0,index-1)]);
    n=nav_lead(n,"plain+down",cfg,data,leads[min(leads.len-1,index+1)]);
    n=nav_lead(n,"plain+home",cfg,data,leads[0]);
    n=nav_lead(n,"plain+end",cfg,data,leads[leads.len-1]);
    let into=if cfg.rtl{"plain+left"}else{"plain+right"};let away=if cfg.rtl{"plain+right"}else{"plain+left"};
    if open&&data.kids[id].len>0{
        let first=data.kids[id][0];
        n=n.on_key_value(into,Fn("focus_lead").curry(cfg.token.run,id,key_ref(data,first)),());
    }else if !open&&live{n=n.on_key_value(into,Fn("propose").curry(cfg.token,id),());}
    if open&&live{n=n.on_key_value(away,Fn("propose").curry(cfg.token,""),());}
    n
}
fn inspector(ctx,props,cfg,a,anchor){
    let output=[text(a.name).with_key("title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("inspect_title",style().min_width(px(0)).typography("body").font_weight(600))),
        note(ctx,a.task,"evidence").with_key("assignment")];
    let evidence=opt(a,if a.state=="waiting"{"ask"}else{"error"});
    if evidence!=()&&a.state in ["waiting","failed"]{output.push(note(ctx,evidence,"evidence").with_key("evidence"));}
    if cfg.resolves&&a.state in ["waiting","failed"]{
        let options=cfg;options.return_focus=anchor;
        output.push(team::resolution_actions(ctx,options,a,a.state));
    }
    let rows=[];
    for step in value(a,"steps",[]){
        let phase=value(step,"state","done");let status=team::status(ctx,phase);
        let items=[text(step.title).with_key("title").with_style(style().min_width(px(0)).typography("caption").flex_grow())];
        let tool=opt(step,"tool");if tool!=(){items.push(note(ctx,tool,"step_tool").with_key("tool"));}
        items.push(note(ctx,status,"step_state").with_key("status"));
        let elapsed=opt(step,"elapsed_ms");
        if elapsed!=(){items.push(note(ctx,ctx.t_or("agent_lanes.seconds","{n} s",#{n:ctx.format_number(elapsed.to_float()/1000.0,#{max_fraction_digits:1})}),"step_time").with_key("time"));}
        rows.push(row(items).with_key("step:"+step.id).accessibility_role("listitem").accessibility_label(step.title)
            .accessibility_description(status).audit_allow(["mixed-type-in-row"])
            .with_style(ctx.component_style("step",style().min_width(px(0)).gap(fleet_gap(1)).items_center().flex_wrap())));
    }
    if rows.len>0{
        output.push(column(rows).with_key("steps").accessibility_role("list").accessibility_label(ctx.t_or("agent_fleet.tasks","Tasks"))
            .scrollbars("hidden","auto").with_style(ctx.component_style("steps",
                style().min_width(px(0)).max_height(px(230)).overflow_y_scroll().gap(fleet_gap(0.5)).padding(fleet_gap(0.75))
                .background(theme_color("surface_hover")).radius(px(5)))));
    }
    let notice=opt(props,"notice");
    if notice!=(){output.push(note(ctx,notice,"notice").with_key("feedback").accessibility_role("status"));}
    column(output).with_key("panel").audit_allow(["text-edge-misaligned"]).with_style(ctx.component_style("inspect",
        style().min_width(px(0)).gap(fleet_gap(1.25))))
}
fn sub_card(ctx,props,cfg,data,id,reference,active){
    let a=data.rows[id].a;let packed=overview(ctx,cfg,data,id,false);
    let can=value(a,"steps",[]).len>0||(a.state=="waiting"&&(opt(a,"ask")!=()||cfg.resolves))
        ||(a.state=="failed"&&(opt(a,"error")!=()||cfg.resolves));
    let open=can&&active==id;let parts=["sub"];if id==cfg.highlight{parts.push("highlight");}
    let target=column(packed.lines).with_key("sub-trigger").with_ref(reference)
        .accessibility_role("button").accessibility_label(a.name).accessibility_description(packed.desc).accessibility_expanded(open)
        .tab_stop(true).audit_allow(["text-edge-misaligned"]).with_style(ctx.component_style(parts,style().min_width(px(0)).gap(fleet_gap(1.25))
            .padding(fleet_gap(2)).radius(fleet_unit(2.25)).background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border"))
            .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))));
    let child=target;
    if can{
        child=popover::Popover(#{key:"inspect:"+id,trigger:target,label:ctx.t_or("agent_fleet.inspect","Tasks for {name}",#{name:a.name}),
            open:open,align:"start",initial_focus:if cfg.resolves&&a.state in ["waiting","failed"]{"first"}else{"panel"},
            content:if open{inspector(ctx,props,cfg,a,key_ref(data,id))}else{column([])},
            on_open_change:Fn("inspect").curry(cfg.token,id,cfg.open)}).with_key("details");
    }else if cfg.can_select{child=target.on_click(Fn("select_only").curry(cfg.token,id));}
    let indent=max(0,data.depth[id]-1).to_float()*cfg.indent_step;
    column([child]).with_key("sub:"+id).accessibility_role("treeitem").accessibility_label(a.name).accessibility_level(data.depth[id])
        .with_style(style().min_width(px(0)).padding_start(fleet_unit(indent)))
        .enter_motion(motion_transition("opacity",0.6,1.0,#{duration_ms:ctx.motion_duration("fast"),intent:"decorative"}))
}
fn edge(a,rtl){if rtl{a.x+a.width}else{a.x}}
fn wires(ctx,data,cfg,visible,refs,stage){
    let frame=ctx.element_bounds(stage);if frame==(){return ();}
    let b=frame.visual;if b.width<=0.0||b.height<=0.0{return ();}
    let commands=[];let ink=theme_color("agent_tree.link");
    for id in visible{
        let parent=data.rows[id].parent;
        let left=ctx.element_bounds(refs[parent]);let right=ctx.element_bounds(refs[id]);
        if left==()||right==(){continue;}
        let a=left.visual;let z=right.visual;let y=a.y+a.height*0.5-b.y;let ty=z.y+z.height*0.5-b.y;
        let key="edge-"+data.rows[id].i.to_string();
        if cfg.wide&&parent==cfg.open{
            let ax=if cfg.rtl{a.x}else{a.x+a.width};let x=ax-b.x;let tx=edge(z,cfg.rtl)-b.x;
            if (tx-x).abs()<1.0{continue;}
            let mid=(x+tx)*0.5;
            commands.push(canvas_stroke_path(key,[path_move(x,y),path_cubic(tx,ty,mid,y,mid,ty)],1.5,ink).stroke_cap("round"));
        }else{
            let x=edge(a,cfg.rtl)-b.x;let tx=edge(z,cfg.rtl)-b.x;
            let mid=x+(if cfg.rtl{6.0}else{-6.0});
            if ty<=y{continue;}
            commands.push(canvas_stroke_path(key,[path_move(x,y),path_line(mid,y),path_line(mid,ty),path_line(tx,ty)],1.5,ink)
                .stroke_join("round").stroke_cap("round"));
        }
    }
    if commands.len==0{return ();}
    canvas(canvas_scene(commands).view_box(0,0,b.width,b.height)).with_key("fleet-wires").accessibility_hidden(true)
        .with_style(ctx.component_style("links",style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0))))
}
fn verify_steps(props){
    let count=0;
    for a in props.agents{
        let known=#{};
        for s in value(a,"steps",[]){
            if !team::valid(s.id)||s.id in known{throw "Fleet step IDs must be unique within their agent";}
            known[s.id]=true;count+=1;
        }
    }
    if count>384{throw "AgentFleet supports at most 384 retained tasks";}
}
fn render_AgentFleet(ctx,props){
    if !team::valid(props.run){throw "AgentFleet needs a stable run ID";}
    verify_steps(props);let data=team::roster(props.agents);
    let ui=local(ctx,props.run);let n=value(props,"highlight","");let highlight=if n in data.rows{n}else{""};
    let open=active_lead(props,ui,data,highlight);let shell=element_ref("fleet");let stage=element_ref("body");
    let focus=if ui.cursor in data.roots{ui.cursor}else if open!=""{open}else if data.roots.len>0{data.roots[0]}else{""};
    let wide=ctx.element_width_class(shell,[656])==1;
    let stats=summarize(data);
    // A long chain must not spend the whole narrow viewport on indentation.
    let levels=if open!=""{max(1,stats[open].depth-1)}else{1};
    let cfg=#{open:open,highlight:highlight,focus:focus,wide:wide,rtl:ctx.text_direction()=="rtl",now:props.now_ms,currency:value(props,"currency","USD"),
        stats:stats,indent_step:min(5.0,15.0/levels.to_float()),
        token:#{run:props.run,revision:props.revision,controlled:props.controlled_open,highlight:highlight},
        can_open:opt(props,"on_open_change")!=(),can_select:opt(props,"on_select")!=(),resolves:opt(props,"on_resolve")!=(),busy:opt(props,"busy_agent")};
    let title=value(props,"title",ctx.t_or("agent_fleet.title","Agents"));
    let outline=[header(ctx,props,cfg,data,title)];
    if data.roots.len==0{outline.push(note(ctx,ctx.t_or("agent_fleet.empty","No agents at work"),"empty").with_key("empty"));}
    else{
        let cards=[];let refs=#{};let open_row=0;
        for i in 0..data.roots.len{
            let id=data.roots[i];refs[id]=element_ref(key_ref(data,id));
            if id==open{open_row=i+1;}
        }
        let visible=[];
        if open!=""{
            for id in data.order{
                if id!=open&&data.family[id]==open{visible.push(id);refs[id]=element_ref(key_ref(data,id));}
            }
        }
        for i in 0..data.roots.len{
            let id=data.roots[i];let row=i+1+(if !wide&&open_row>0&&i+1>open_row{1}else{0});
            let cell=style().grid_cell(1,row).min_width(px(0)).self_start().width(relative(1.0));
            if wide{cell=cell.max_width(px(320));}
            cards.push(column([lead_card(ctx,cfg,data,id,i,refs[id])]).with_key("slot:"+id)
                .with_style(cell));
        }
        if open!=""{
            let subs=[];
            let active=if ui.inspect_lead==open{ui.inspect}else{""};
            for id in visible{subs.push(sub_card(ctx,props,cfg,data,id,refs[id],active));}
            if subs.len==0{subs.push(note(ctx,ctx.t_or("agent_fleet.empty_branch","No sub-agents yet"),"branch_empty").with_key("empty"));}
            let branch=column(subs).with_key("branch").accessibility_role("tree")
                .accessibility_label(ctx.t_or("agent_fleet.branch","Sub-agents of {name}",#{name:data.rows[open].a.name}))
                .with_style(ctx.component_style("branch",style().gap(fleet_gap(2.5)).min_width(px(0)).width(relative(1.0)).self_start()
                    .padding_start(if wide{fleet_unit(1.5)}else{fleet_unit(5)})));
            branch=branch.with_style(if wide{style().grid_cell(2,1).row_span(max(1,data.roots.len)).max_width(px(340))}
                else{style().grid_cell(1,open_row+1)});
            cards.push(branch);
        }
        let grid=box(cards).with_key("cards").with_style(ctx.component_style("grid",style().grid().grid_cols(if wide{2}else{1})
            .min_width(px(0)).gap_y(fleet_gap(2.5)).gap_x(fleet_gap(9))));
        let deck=[];if visible.len>0{
            let link=wires(ctx,data,cfg,visible,refs,stage);if link!=(){deck.push(link);}
        }
        deck.push(grid);
        outline.push(stack(deck).with_key("body").with_ref(stage).accessibility_role("group")
            .accessibility_label(ctx.t_or("agent_fleet.roster","Fleet roster"))
            .with_style(ctx.component_style("body",style().min_width(px(0)).relative().width(relative(1.0)))));
    }
    let feedback=opt(props,"notice");
    if feedback!=(){outline.push(note(ctx,feedback,"notice").with_key("notice").accessibility_role("status"));}
    effect("settle_branch",#{run:props.run,lead:open,old_inspect:ui.inspect},Fn("settle"),Fn("stop"));
    column(outline).with_key(props.key).with_ref(shell).accessibility_role("group").accessibility_label(title).audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).padding(fleet_gap(1.5)).gap(fleet_gap(2))
            .radius(fleet_unit(4)).background(theme_color("surface_hover"))))
}
