/* gpui-rhai
{"id":"patterns/agent_handoff","export":"AgentHandoff","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/drag_source","components/drop_zone","components/scroll_area","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","selection","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted","text_primary","typography.body","typography.caption","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// Inspect relay evidence, restore omitted material, or propose assigning a task.
// State: inspection and the keyboard carry. The Host owns all transfers and assignments.
// Example: AgentHandoff(#{key:"relay",relay:"release",revision:0,agents:[],tasks:[],handoffs:[]})
import "components/button" as button;
import "components/drag_source" as drag;
import "components/drop_zone" as drop;
import "components/scroll_area" as scroll;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_handoff","export":"AgentHandoff",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/drag_source","components/drop_zone","components/scroll_area","patterns/approval_card"],capabilities:#{},
        tokens:["accent","border","focus_ring","selection","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted",
            "text_primary","typography.body","typography.caption","space.spacing_unit","space.unit"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable instance key."},
        relay:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business relay identity, at most 72 UTF-8 bytes. Use a distinct drag namespace in a window."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Version of the accepted relay snapshot."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Relay heading."},
        agents:#{schema:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable agent ID."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Display name."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional model or role caption."},
            description:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short role description."},
            avatar:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative avatar."},
            cost_cents:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9000000000000000}},required:false,sensitive:false,doc:"Accepted usage cost in hundredths of a dollar."},
            accepting:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Defaults to true; false prevents assignment to this destination."},
        }}},required:true,sensitive:true,doc:"Which agents appear in the relay, in order."},
        tasks:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique task ID; display titles are not identities."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Task name."},
            status:#{schema:#{type:"string",allowed:["todo","working","done","warning"]},required:true,sensitive:false,doc:"Accepted task state."},
            agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted current or last owner."},
            note:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short progress or outcome."},
            description:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Read-only task evidence."},
            tools:#{schema:#{type:"optional",value:#{type:"array",max_items:8,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable ID of a displayed tool."},
                name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Display name or purpose."},
                detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional trace detail."},
                logo:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative icon."},
            }}}},required:false,sensitive:true,doc:"Tool evidence, never invoked by the component."},
        }}},required:true,sensitive:true,doc:"Tasks keyed independently of their title. A historical transfer may outlive its current task record."},
        handoffs:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique transfer ID. A newer ID on an edge replays its delivery motion."},
            from:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Sender ID."},
            to:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Recipient ID."},
            task:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable task ID, not its title."},
            title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Historical caption to use if a task has been pruned."},
            items:#{schema:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Item identity within this transfer."},
                label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Display caption."},
                kind:#{schema:#{type:"optional",value:#{type:"string",allowed:["file","source","fact","decision","note"]}},required:false,sensitive:false,doc:"Kind of evidence."},
                lost:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"True if it did not reach the recipient."},
                why:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Reason shown when inspecting omitted material."},
            }}},required:true,sensitive:true,doc:"Display what arrived and what was omitted, not file contents."},
        }}},required:true,sensitive:true,doc:"Accepted records, oldest first. The latest is shown first on each edge; history stays inspectable."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Suspend assignment and restoration while the Host resolves a proposal."},
        on_assign:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Versioned task-assignment proposal. The Host updates the accepted task."},
        on_restore:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Versioned proposal to restore one omitted item in one transfer."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Accepted Host notice or refusal."},
    },state:#{fields:#{
        inspect:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        carry:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        assign:#{payload:#{type:"object",allow_unknown:false,fields:#{
            relay:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Exact relay namespace."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot version."},
            task:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Task to assign."},
            from:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Expected owner, or empty if unassigned."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Destination agent ID."},
        }},doc:"Requests an assignment, without changing the relay automatically."},
        restore:#{payload:#{type:"object",allow_unknown:false,fields:#{
            relay:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Exact relay namespace."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot version."},
            handoff:#{schema:#{type:"string"},required:true,sensitive:false,doc:"The accepted transfer that omitted material."},
            task:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Task of the transfer."},
            item:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Omitted item within the transfer."},
        }},doc:"Requests restoration of the exact material; does not mutate the receipt or open any file."},
    },slots:#{},parts:["root","head","title","summary","board","agent","agent_target","avatar","name","model","status","agent_desc","current",
        "pair","pair_head","pair_name","pair_count","latest","packet","packet_title","packet_meta","lost","task_list","task","task_title","handle","inspector",
        "inspector_title","line","items","item","item_lost","item_kind","reason","actions","empty","notice","wires","pulse"]},
    render:Fn("render_AgentHandoff")
});

fn AgentHandoff(props){render_component("patterns/agent_handoff",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,d){common::value_or(m,k,d)}
fn safe_id(id,max_bytes){
    if id==""||utf8_byte_count(id)>max_bytes{return false;}
    for c in id.chars(){if c<' '||(c>='\u007f'&&c<='\u009f'){return false;}}
    true
}
fn index(items,kind){
    let found=#{};
    for i in 0..items.len{
        let row=items[i];if !safe_id(row.id,128)||row.id in found{throw "Duplicate or invalid "+kind+" identity";}
        row.ordinal=i;found[row.id]=row;
    }
    found
}
fn owner(task){value(task,"agent","")}
fn can_move(task){task.status=="todo"||task.status=="working"}
fn receiving(a){value(a,"accepting",true)}
fn project(props){
    if !safe_id(props.relay,72){throw "Relay drag namespace must be 1–72 bytes without controls";}
    let agents=index(props.agents,"agent");let tasks=index(props.tasks,"task");let handoffs=index(props.handoffs,"handoff");
    let work=#{};
    for t in props.tasks{
        if owner(t)!=""&&!(owner(t) in agents){throw "Task refers to an unknown agent";}
        if owner(t)!=""&&(t.status=="working"||t.status=="todo")&&
            (!(owner(t) in work)||t.status=="working"&&work[owner(t)].status=="todo"){work[owner(t)]=t;}
        index(value(t,"tools",[]),"tool");
    }
    let pairs=[];let visible=#{};
    for i in 1..props.agents.len{
        let a=props.agents[i-1];let b=props.agents[i];
        pairs.push(#{from:a.id,to:b.id,history:[],latest:()});
    }
    for h in props.handoffs{
        index(h.items,"material");
        if !safe_id(h.task,128)||!safe_id(h.from,128)||!safe_id(h.to,128)||h.from==h.to{throw "Handoff needs distinct agents and a task ID";}
        if !(h.to in agents){continue;}
        let i=agents[h.to].ordinal-1;
        if i<0||pairs[i].from!=h.from{continue;}
        let p=pairs[i];p.history.push(h.id);p.latest=h;pairs[i]=p;visible[h.id]=true;
    }
    #{agents:agents,tasks:tasks,handoffs:handoffs,work:work,pairs:pairs,visible:visible}
}
fn caption(ctx,v,part){common::heading_note(ctx,v,part).with_style(style().min_width(px(0)))}
fn data_title(h,data){if h.task in data.tasks{data.tasks[h.task].title}else{value(h,"title",h.task)}}
fn key_agent(i){"agent-"+i.to_string()}
fn key_packet(i){"packet-"+i.to_string()}
fn key_task(i){"task-"+i.to_string()}
fn key_pick(){"choose-agent"}
fn pair_label(ctx,a,b){ctx.t_or("agent_handoff.pair","{from} → {to}",#{from:a,to:b})}
fn open(relay,kind,id,key,ctx,event){
    let old=ctx.get_state("inspect");let next=#{relay:relay,kind:kind,id:id,key:key};
    ctx.set_state("inspect",if old!=()&&old.relay==relay&&old.kind==kind&&old.id==id{()}else{next});
    ctx.set_state("carry",());handled()
}
fn close(key,ctx,event){ctx.set_state("inspect",());ctx.set_state("carry",());ctx.focus(key);handled()}
fn focus(key,ctx,event){ctx.focus(key);handled()}
fn start_pick(spec,key,ctx,event){ctx.set_state("carry",spec);ctx.focus(key);handled()}
fn cancel_pick(ctx,event){ctx.set_state("carry",());ctx.focus(key_pick());handled()}
fn assign_accepted(cfg,spec,agent,ctx,event){
    if spec.relay!=cfg.token.relay||spec.revision!=cfg.token.revision||spec.scope!=cfg.scope||spec.from==agent||
        !(spec.task in cfg.owners)||cfg.owners[spec.task].from!=spec.from||cfg.owners[spec.task].source!=spec.source{return;}
    ctx.set_state("carry",());ctx.emit("assign",#{relay:spec.relay,revision:spec.revision,task:spec.task,from:spec.from,agent:agent});handled()
}
fn assign_key(cfg,spec,agent,ctx,event){
    if ctx.get_state("carry")!=spec{return;}
    assign_accepted(cfg,spec,agent,ctx,event)
}
fn receive(cfg,agent,target,ctx,p){
    if p.operation!="move"||p.target_id!=target||p.payload_type!=cfg.wire{return;}
    let spec=p.payload;
    if type_of(spec)!="map"||!("source" in spec)||spec.source!=p.source_id{return;}
    assign_accepted(cfg,spec,agent,ctx,p)
}
fn restore(token,hand,task,item,ctx,event){
    ctx.emit("restore",#{relay:token.relay,revision:token.revision,handoff:hand,task:task,item:item});handled()
}
fn spec_for(cfg,t){#{relay:cfg.token.relay,revision:cfg.token.revision,scope:cfg.scope,task:t.id,from:owner(t),source:cfg.wire+":s"+t.ordinal.to_string()}}
fn text_status(ctx,state){
    if state=="working"{ctx.t_or("agent_handoff.working","Working")}else if state=="todo"{ctx.t_or("agent_handoff.todo","Ready")}
    else if state=="warning"{ctx.t_or("agent_handoff.warning","Needs attention")}else{ctx.t_or("agent_queue.done","Done")}
}
fn ink(state){theme_color(if state=="working"{"accent"}else if state=="warning"{"text.warning"}else if state=="done"{"text.success"}else{"text_muted"})}
fn face(ctx,a){
    let custom=opt(a,"avatar");let mark=if custom==(){text(if a.name==""{"•"}else{a.name.sub_string(0,1).to_upper()})}else{custom};
    row([mark]).with_key("avatar").accessibility_hidden(true).with_style(ctx.component_style("avatar",style().width(theme_length("space.unit").times(6.25)).height(theme_length("space.unit").times(6.25))
        .items_center().justify_center().flex_shrink(false).radius(px(6)).typography("caption").background(theme_color("surface_hover"))))
}
fn agent_tile(ctx,a,ref_item,cfg,data){
    let active=opt(data.work,a.id);let has=active!=()&&active.status=="working";
    let identity=[text(a.name).with_key("name").with_style(ctx.component_style("name",style().min_width(px(0)).typography("body").font_weight(600)
        .whitespace_nowrap().text_ellipsis()))];
    if opt(a,"model")!=(){identity.push(caption(ctx,a.model,"model").with_key("model"));}
    let state=if has{text_status(ctx,"working")}else{ctx.t_or("agent_queue.idle","Idle")};
    let head=[face(ctx,a),column(identity).with_key("identity").with_style(style().min_width(px(0)).flex_grow())];
    let cents=opt(a,"cost_cents");
    if cents!=(){head.push(caption(ctx,"$"+ctx.format_number(cents.to_float()/100.0,#{min_fraction_digits:2,max_fraction_digits:2}),"status").with_key("cost"));}
    head.push(caption(ctx,state,"status").with_key("status").with_style(style().text_color(ink(if has{"working"}else{"todo"}))));
    let contents=[row(head).with_key("identity").audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).items_center().flex_wrap())];
    if opt(a,"description")!=(){contents.push(caption(ctx,a.description,"agent_desc").with_key("about"));}
    let current=if active==(){ctx.t_or("agent_handoff.no_work","No current task")}else{active.title};
    contents.push(caption(ctx,current,"current").with_key("current").with_style(style().text_ellipsis().whitespace_nowrap()));
    let assigning=cfg.carry!=()&&cfg.carry.from!=a.id&&receiving(a);
    let caption=if assigning{ctx.t_or("delegate.assign_agent","Assign to {agent}",#{agent:a.name})}
        else{ctx.t_or("agent_handoff.inspect_agent","Inspect {agent}",#{agent:a.name})};
    let parts=if assigning{["agent","agent_target"]}else{["agent"]};
    let intent=if assigning{Fn("assign_key").curry(cfg.identity,cfg.carry,a.id)}
        else{Fn("open").curry(cfg.token.relay,"agent",a.id,key_agent(a.ordinal))};
    let view=column(contents).with_key("tile").with_ref(ref_item).accessibility_role("button")
        .accessibility_label(caption).accessibility_description(state+" · "+current).tab_stop(true)
        .on_click(intent).with_style(ctx.component_style(parts,style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).padding(theme_length("space.spacing_unit").times(2.25)).radius(theme_length("space.unit").times(2.5))
            .background(theme_color("surface_raised")).hover(style().background(theme_color("surface_hover")))
            .focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))));
    if assigning{
        let keys=[];let i=0;
        for candidate in cfg.targets{
            if candidate.id==cfg.carry.from{continue;}
            if candidate.id==a.id{i=keys.len;}
            keys.push(candidate.key);
        }
        let prev=keys[(i+keys.len-1)%keys.len];let next=keys[(i+1)%keys.len];
        view=view.on_key_value("plain+down",Fn("focus").curry(next),()).on_key_value("plain+up",Fn("focus").curry(prev),())
            .on_key_value("plain+left",Fn("focus").curry(if cfg.rtl{next}else{prev}),())
            .on_key_value("plain+right",Fn("focus").curry(if cfg.rtl{prev}else{next}),());
    }
    let target=cfg.wire+":a"+a.ordinal.to_string();
    drop::DropZone(#{key:"zone-"+a.id,label:ctx.t_or("agent_handoff.drop_to","Drop task on {agent}",#{agent:a.name}),target_id:target,payload_types:[cfg.wire],
        operations:["move"],content:view,disabled:!cfg.ready_assign||!receiving(a),on_drop:Fn("receive").curry(cfg.identity,a.id,target)})
        .with_key("agent-"+a.id)
}
fn packet(ctx,h,cfg,data,i,latest){
    let title=data_title(h,data);let missing=0;for item in h.items{if value(item,"lost",false){missing+=1;}}
    let summary=ctx.t_or("agent_handoff.materials","{delivered} delivered · {omitted} omitted",
        #{delivered:ctx.format_number(h.items.len-missing),omitted:ctx.format_number(missing)});
    let label=ctx.t_or("agent_handoff.inspect_packet","Inspect handoff: {task}",#{task:title});
    let p=if latest{["packet","latest"]}else{["packet"]};
    let current=cfg.inspect!=()&&cfg.inspect.kind=="handoff"&&cfg.inspect.id==h.id;
    column([text(title).with_key("title").with_style(ctx.component_style("packet_title",style().min_width(px(0)).typography("body").font_weight(500))),
        caption(ctx,summary,"packet_meta").with_key("materials")]).with_key("handoff-"+h.id).with_ref(element_ref(key_packet(i)))
        .accessibility_role("button").accessibility_label(label).accessibility_description(summary).accessibility_pressed(current).tab_stop(true)
        .on_click(Fn("open").curry(cfg.token.relay,"handoff",h.id,key_packet(i)))
        .with_style(ctx.component_style(p,style().min_width(px(0)).gap(theme_length("space.spacing_unit")).padding(theme_length("space.spacing_unit").times(1.75)).radius(px(7))
            .background(theme_color(if current{"selection"}else{"surface_raised"})).hover(style().background(theme_color("selection")))
            .focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))))
        .shared_layout(cfg.scope,"packet-"+h.id).layout_motion(ctx.motion_duration("normal"),ctx.motion_easing("standard"))
}
fn pair_tile(ctx,edge,i,anchor,cfg,data){
    let title=pair_label(ctx,data.agents[edge.from].name,data.agents[edge.to].name);
    let head=row([caption(ctx,title,"pair_name").with_key("heading").with_style(style().flex_grow()),
        caption(ctx,ctx.format_number(edge.history.len),"pair_count").with_key("count")])
        .with_key("header").with_style(ctx.component_style("pair_head",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).items_center()));
    let rows=[];
    for j in 0..edge.history.len{
        let k=edge.history.len-1-j;let h=data.handoffs[edge.history[k]];
        rows.push(packet(ctx,h,cfg,data,h.ordinal,j==0));
    }
    if rows.len==0{rows.push(caption(ctx,ctx.t_or("agent_handoff.no_transfer","No transfer yet"),"empty").with_key("empty"));}
    let list=scroll::ScrollArea(#{key:"edge-"+i.to_string(),label:title,height:px(170),
        content:column(rows).with_key("receipts").with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit")))});
    if rows.len<=3{list=list.with_style(style().height(auto()).overflow_hidden());}
    column([head,list]).with_key("pair-"+i.to_string()).with_ref(anchor).accessibility_role("group").accessibility_label(title)
        .with_style(ctx.component_style("pair",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).padding(theme_length("space.spacing_unit").times(1.5)).radius(theme_length("space.unit").times(2.25)).background(theme_color("surface_hover"))))
}
fn task_row(ctx,t,cfg){
    let dragger=();
    if cfg.ready_assign&&can_move(t){
        let spec=spec_for(cfg,t);
        let icon=row([text("⋮").with_key("dots")]).with_style(ctx.component_style("handle",style().width(theme_length("space.unit").times(6.25)).height(theme_length("space.unit").times(7)).items_center().justify_center()
            .text_color(theme_color("text_muted"))));
        dragger=drag::DragSource(#{key:"move-"+t.id,label:ctx.t_or("delegate.move","Move {task}",#{task:t.title}),source_id:spec.source,
            payload_type:cfg.wire,payload:spec,content:icon});
    }
    let info=[text(t.title).with_key("name").with_style(ctx.component_style("task_title",style().min_width(px(0)).flex_grow().typography("body")
            .text_ellipsis().whitespace_nowrap())),
        caption(ctx,text_status(ctx,t.status),"status").with_key("status").with_style(style().text_color(ink(t.status)))];
    let inspect=row(info).with_key("inspect").with_ref(element_ref(key_task(t.ordinal))).accessibility_role("button")
        .accessibility_label(t.title).accessibility_description(text_status(ctx,t.status)).tab_stop(true)
        .on_click(Fn("open").curry(cfg.token.relay,"task",t.id,key_task(t.ordinal))).audit_allow(["mixed-type-in-row"])
        .with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).padding_x(theme_length("space.spacing_unit").times(1.5)).items_center().flex_grow().min_height(theme_length("space.unit").times(7.25))
            .focus_ring(px(1),offset_px(-1),theme_color("focus_ring")));
    let kids=if dragger==(){[inspect]}else{[inspect,dragger]};
    row(kids).with_key("task-"+t.id).with_style(ctx.component_style("task",style().min_width(px(0)).gap(theme_length("space.spacing_unit")).items_center().background(theme_color("surface_raised"))
        .radius(px(7))));
}
fn task_list(ctx,props,cfg,data){
    if props.tasks.len==0{return ();}
    let nodes=[];for t in props.tasks{nodes.push(task_row(ctx,data.tasks[t.id],cfg));}
    let title=ctx.t_or("agent_handoff.tasks","Tasks");
    let list=scroll::ScrollArea(#{key:"task-list",label:title,height:px(195),
        content:column(nodes).with_key("tasks").with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit")))});
    if nodes.len<=5{list=list.with_style(style().height(auto()).overflow_hidden());}
    column([text(title).with_key("heading").accessibility_role("heading").accessibility_level(3)
        .with_style(style().typography("body").font_weight(500)),list])
        .with_key("work").with_style(ctx.component_style("task_list",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).padding(theme_length("space.spacing_unit").times(1.75)).radius(theme_length("space.unit").times(2)).background(theme_color("surface_hover"))))
}
fn evidence_kind(ctx,kind){
    if kind=="file"{ctx.t_or("agent_handoff.file","File")}else if kind=="source"{ctx.t_or("agent_handoff.source","Source")}
    else if kind=="fact"{ctx.t_or("agent_handoff.fact","Fact")}else if kind=="decision"{ctx.t_or("agent_handoff.decision","Decision")}
    else{ctx.t_or("agent_handoff.note","Note")}
}
fn evidence(ctx,h,cfg,props){
    let sections=[];for item in h.items{
        let missing=value(item,"lost",false);let part=if missing{["item","item_lost"]}else{["item"]};
        let line=[caption(ctx,evidence_kind(ctx,value(item,"kind","note")),"item_kind").with_key("kind"),
            text(item.label).with_key("label").with_style(style().typography("body").min_width(px(0)).flex_grow()),
            caption(ctx,if missing{ctx.t_or("agent_handoff.omitted","Omitted")}else{ctx.t_or("agent_handoff.delivered","Delivered")},"status")
                .with_key("state").with_style(style().text_color(theme_color(if missing{"text.danger"}else{"text.success"})))];
        let box=[row(line).with_key("header").audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap())];
        if missing{
            if opt(item,"why")!=(){box.push(caption(ctx,item.why,"reason").with_key("reason"));}
            if opt(props,"on_restore")!=(){
                box.push(button::Button(#{key:"restore-"+item.id,text:ctx.t_or("agent_handoff.restore","Restore {item}",#{item:item.label}),size:"xs",variant:"secondary",
                    disabled:props.busy,on_click:Fn("restore").curry(cfg.token,h.id,h.task,item.id)}).with_style(style().self_start()));
            }
        }
        sections.push(column(box).with_key("material-"+item.id).with_style(ctx.component_style(part,style().min_width(px(0)).gap(theme_length("space.spacing_unit")).padding(theme_length("space.spacing_unit").times(1.5)).radius(px(6))
            .background(theme_color("surface_hover")))));
    }
    if sections.len==0{sections.push(caption(ctx,ctx.t_or("agent_handoff.no_items","No materials recorded"),"empty").with_key("empty"));}
    column(sections).with_key("materials").with_style(ctx.component_style("items",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25))))
}
fn earliest_target(props,t){
    for i in 0..props.agents.len{let a=props.agents[i];if a.id!=owner(t)&&receiving(a){return key_agent(i);}}
    ""
}
fn inspector(ctx,props,cfg,data){
    let picked=cfg.inspect;if picked==(){return ();}
    let title="";let body=[];let actions=[];let job=();
    if picked.kind=="handoff"{
        let h=data.handoffs[picked.id];title=data_title(h,data);
        if h.task in data.tasks{job=data.tasks[h.task];}
        if h.from in data.agents&&h.to in data.agents{body.push(caption(ctx,pair_label(ctx,data.agents[h.from].name,data.agents[h.to].name),"line").with_key("transfer"));}
        body.push(evidence(ctx,h,cfg,props));
    }else if picked.kind=="task"{job=data.tasks[picked.id];title=job.title;}
    else{
        let a=data.agents[picked.id];title=a.name;
        if a.id in data.work{job=data.tasks[data.work[a.id].id];}
        if opt(a,"description")!=(){body.push(caption(ctx,a.description,"line").with_key("description"));}
        if job==(){body.push(caption(ctx,ctx.t_or("agent_handoff.no_work","No current task"),"empty").with_key("empty"));}
    }
    if job!=(){
        body.push(caption(ctx,job.title+" · "+text_status(ctx,job.status),"line").with_key("task"));
        if opt(job,"description")!=(){body.push(caption(ctx,job.description,"line").with_key("details"));}
        if opt(job,"note")!=(){body.push(caption(ctx,job.note,"line").with_key("note"));}
        for tool in value(job,"tools",[]){
            let nodes=[];if opt(tool,"logo")!=(){nodes.push(tool.logo.with_key("logo").accessibility_hidden(true).with_style(style().width(px(14)).height(px(14))));}
            nodes.push(caption(ctx,tool.name,"line").with_key("name"));if opt(tool,"detail")!=(){nodes.push(caption(ctx,tool.detail,"line").with_key("meta"));}
            body.push(row(nodes).with_key("tool-"+tool.id).with_style(style().gap(theme_length("space.spacing_unit").times(1.25)).flex_wrap().items_center()));
        }
        let target=earliest_target(props,job);
        if opt(props,"on_assign")!=()&&can_move(job)&&target!=""{
            actions.push(button::Button(#{key:"assign",text:ctx.t_or("agent_handoff.choose","Choose destination"),size:"sm",variant:"secondary",disabled:props.busy,
                on_click:Fn("start_pick").curry(spec_for(cfg,job),target)}).with_key("assign").with_ref(element_ref(key_pick())));
        }
    }
    actions.push(button::Button(#{key:"close",text:ctx.t_or("agent_queue.close","Close details"),size:"sm",variant:"ghost",on_click:Fn("close").curry(picked.key)}));
    let face=[text(title).with_key("title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("inspector_title",style().typography("body").font_weight(600)))];
    for info in body{face.push(info);}
    face.push(row(actions).with_key("actions").with_style(ctx.component_style("actions",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap())));
    column(face).with_key("inspector").accessibility_role("region").accessibility_label(ctx.t_or("agent_handoff.inspector","Relay details"))
        .with_style(ctx.component_style("inspector",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).padding(theme_length("space.spacing_unit").times(2)).radius(theme_length("space.unit").times(2)).background(theme_color("surface_raised"))))
}
fn paint_style(ctx,part){
    ctx.component_style(part,style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)))
}
fn curve(start_x,start_y,end_x,end_y){
    let d=(end_y-start_y)*0.49;
    [path_move(start_x,start_y),path_cubic(end_x,end_y,start_x,start_y+d,end_x,end_y-d)]
}
fn wiring(ctx,root,refs,pins,data,cfg){
    let stage=ctx.element_bounds(root);if stage==(){return [];}
    let box=stage.visual;if box.width<=4.0||box.height<=4.0{return [];}
    let rails=[];let pulses=[];
    for i in 0..data.pairs.len{
        let left=ctx.element_bounds(refs[i]);let right=ctx.element_bounds(refs[i+1]);let receipt=ctx.element_bounds(pins[i]);
        if left==()||right==()||receipt==(){continue;}
        let a=left.visual;let b=right.visual;let p=receipt.visual;
        let x1=a.x+a.width*0.5-box.x;let y1=a.y+a.height-box.y;
        let x2=b.x+b.width*0.5-box.x;let y2=if cfg.compact{b.y-box.y}else{b.y+b.height-box.y};
        let middle=p.x+p.width*0.5-box.x;let top=p.y-box.y;
        let enter=if cfg.compact{middle}else{middle+(if cfg.rtl{9.0}else{-9.0})};
        let leave=if cfg.compact{middle}else{middle+(if cfg.rtl{-9.0}else{9.0})};
        let outgoing=if cfg.compact{top+p.height}else{top};
        let to_packet=curve(x1,y1,enter,top);let to_agent=curve(leave,outgoing,x2,y2);
        let id=i.to_string();let accepted=data.pairs[i].latest;
        let color=if accepted==(){theme_color("border")}else{theme_color("accent")};
        rails.push(canvas_stroke_path("into-"+id,to_packet,1.25,color).stroke_cap("round"));
        rails.push(canvas_stroke_path("out-"+id,to_agent,1.25,color).stroke_cap("round"));
        if accepted!=(){
            let scene=canvas_scene([canvas_stroke_path("arrival",to_packet,2.0,theme_color("accent")),canvas_stroke_path("onward",to_agent,2.0,theme_color("accent"))])
                .view_box(0,0,box.width,box.height);
            pulses.push(canvas(scene).with_key("pulse-"+id).accessibility_hidden(true).with_style(paint_style(ctx,"pulse"))
                .motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:ctx.motion_duration("normal"),intent:"decorative"}))
                .motion(motion_transition("opacity",0.8,0.0,#{duration_ms:ctx.motion_duration("normal"),intent:"decorative"}))
                .motion_on_change(accepted.id));
        }
    }
    if rails.len==0{return [];}
    let board=canvas(canvas_scene(rails).view_box(0,0,box.width,box.height)).with_key("wires").accessibility_hidden(true).with_style(paint_style(ctx,"wires"));
    let result=[board];for item in pulses{result.push(item);}result
}
fn render_AgentHandoff(ctx,props){
    let data=project(props);let viewport=element_ref("relay");let stage=element_ref("stage");
    let token=#{relay:props.relay,revision:props.revision};let scope=viewport.semantic_id;
    let ready_assign=!props.busy&&opt(props,"on_assign")!=();
    let inspect=ctx.get_state("inspect");let carry=ctx.get_state("carry");
    if inspect!=()&&(inspect.relay!=props.relay||
        (inspect.kind=="task"&&!(inspect.id in data.tasks))||(inspect.kind=="agent"&&!(inspect.id in data.agents))||
        (inspect.kind=="handoff"&&!(inspect.id in data.visible))){inspect=();}
    if inspect!=(){
        inspect.key=if inspect.kind=="task"{key_task(data.tasks[inspect.id].ordinal)}
            else if inspect.kind=="agent"{key_agent(data.agents[inspect.id].ordinal)}else{key_packet(data.handoffs[inspect.id].ordinal)};
    }
    if carry!=()&&(!ready_assign||inspect==()||carry.relay!=props.relay||carry.revision!=props.revision||
        !(carry.task in data.tasks)||owner(data.tasks[carry.task])!=carry.from||!can_move(data.tasks[carry.task])){carry=();}
    let wire="relay:"+utf8_byte_count(props.relay).to_string()+":"+props.relay+":"+props.revision.to_string();
    let owners=#{};for t in props.tasks{
        if can_move(t){owners[t.id]=#{from:owner(t),source:wire+":s"+data.tasks[t.id].ordinal.to_string()};}
    }
    let identity=#{token:token,scope:scope,wire:wire,owners:owners};
    let targets=[];
    for i in 0..props.agents.len{
        let a=props.agents[i];if receiving(a){targets.push(#{id:a.id,key:key_agent(i)});}
    }
    let compact=props.agents.len>4||ctx.element_width_class(viewport,[690])==0;
    let cfg=#{token:token,scope:scope,wire:wire,identity:identity,inspect:inspect,carry:carry,compact:compact,ready_assign:ready_assign,targets:targets,
        rtl:ctx.text_direction()=="rtl"};
    let refs=[];let pins=[];let cells=[];
    for i in 0..props.agents.len{
        let a=data.agents[props.agents[i].id];let anchor=element_ref(key_agent(i));refs.push(anchor);
        cells.push(agent_tile(ctx,a,anchor,cfg,data).with_style(style().grid_cell(if compact{1}else{i+1},if compact{i*2+1}else{1}).min_width(px(0))));
        if i<data.pairs.len{
            let p=element_ref("edge-"+i.to_string());pins.push(p);
            cells.push(pair_tile(ctx,data.pairs[i],i,p,cfg,data).with_style(style().grid_cell(if compact{1}else{i+2},if compact{i*2+2}else{2})));
        }
    }
    let content=[];
    if cells.len>0{
        let plane=wiring(ctx,stage,refs,pins,data,cfg);
        plane.push(box(cells).with_key("grid").with_style(style().grid_cols(if compact{1}else{props.agents.len}).gap_x(px(9)).gap_y(px(if compact{17}else{27})).min_width(px(0))));
        content.push(stack(plane).with_key("stage").with_ref(stage).accessibility_role("group")
            .accessibility_label(ctx.t_or("agent_handoff.map","Relay map")).with_style(ctx.component_style("board",style().min_width(px(0)).relative())));
    }else{content.push(caption(ctx,ctx.t_or("agent_lanes.empty","No agents to show"),"empty").with_key("empty"));}
    let title=value(props,"title",ctx.t_or("agent_handoff.title","Agent handoff"));
    let summary=ctx.t_or("agent_handoff.summary","{agents} agents · {handoffs} transfers",#{agents:ctx.format_number(props.agents.len),handoffs:ctx.format_number(props.handoffs.len)});
    let head=row([text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
        .with_style(ctx.component_style("title",style().min_width(px(0)).flex_grow().typography("body").font_weight(600))),
        caption(ctx,summary,"summary").with_key("summary")]).with_key("head").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("head",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).items_center().flex_wrap()));
    let nodes=[head];
    for block in content{nodes.push(block);}
    if carry!=(){nodes.push(caption(ctx,ctx.t_or("agent_handoff.picked","Choose an agent or press Escape"),"notice").with_key("carry").accessibility_role("status"));}
    let detail=inspector(ctx,props,cfg,data);if detail!=(){nodes.push(detail);}
    let listed=task_list(ctx,props,cfg,data);if listed!=(){nodes.push(listed);}
    if opt(props,"notice")!=(){nodes.push(caption(ctx,props.notice,"notice").with_key("notice").accessibility_role("status"));}
    let root=column(nodes).with_key(props.key).with_ref(viewport).accessibility_role("group").accessibility_label(title)
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(1000)).padding(theme_length("space.spacing_unit").times(1.5)).gap(theme_length("space.spacing_unit").times(2))));
    if carry!=(){root=root.on_bubble("key:plain+escape",Fn("cancel_pick"));}
    else if inspect!=(){root=root.on_bubble("key:plain+escape",Fn("close").curry(inspect.key));}
    root
}
