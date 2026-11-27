/* gpui-rhai
{"id":"patterns/scheduled_runs","export":"ScheduledRuns","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/drag_source","components/drop_zone","components/radio_group","patterns/agent_tree"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","radius.lg","radius.pill","radius.sm","scheduled_runs.ink","scheduled_runs.card","scheduled_runs.capacity","scheduled_runs.current","scheduled_runs.drop","scheduled_runs.hatch","scheduled_runs.lane","scheduled_runs.track","scheduled_runs.well","scheduled_runs.muted","scheduled_runs.text","scheduled_runs.faint","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.label","typography.caption","typography.metadata"],"environment":[]}
*/
// A capacity-checked schedule board. The caller owns all accepted assignments.
// Example: ScheduledRuns(#{key:"week",board:"jobs",revision:0,mode:"manual",days:[],periods:[],tasks:[],assignments:[]})
import "components/button" as btn;
import "components/drag_source" as drag;
import "components/drop_zone" as drop;
import "components/radio_group" as radio;
import "patterns/agent_tree" as ids;

define_component(#{
    metadata:#{id:"patterns/scheduled_runs","export":"ScheduledRuns",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/drag_source","components/drop_zone","components/radio_group","patterns/agent_tree"],capabilities:#{},assets:[],tokens:["accent","border","focus_ring","radius.lg","radius.pill","radius.sm","scheduled_runs.ink","scheduled_runs.card","scheduled_runs.capacity","scheduled_runs.current","scheduled_runs.drop","scheduled_runs.hatch","scheduled_runs.lane","scheduled_runs.track","scheduled_runs.well","scheduled_runs.muted","scheduled_runs.text","scheduled_runs.faint","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.label","typography.caption","typography.metadata"],environment:[]},
    schema:#{
        props:#{
            key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable component instance key."},
            board:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business board identity."},
            revision:#{schema:#{type:"integer",min:0,max:2147483647},required:true,sensitive:false,doc:"Accepted schedule snapshot revision."},
            days:#{schema:#{type:"array",max_items:7,items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable day ID."},label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Day name."},date:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional date label."}}}},required:true,sensitive:false,doc:"Day columns in chronological order."},
            periods:#{schema:#{type:"array",max_items:8,items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable session ID within a day."},label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Session start label."},limit:#{schema:#{type:"integer",min:1,max:1000000},required:true,sensitive:false,doc:"Capacity in whole usage units."}}}},required:true,sensitive:false,doc:"Sessions in chronological order for every day."},
            tasks:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable task ID."},title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Task title."},units:#{schema:#{type:"integer",min:1,max:1000000},required:true,sensitive:false,doc:"Total usage to allocate."},agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional agent ID."},detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Task detail."},stats:#{schema:#{type:"optional",value:#{type:"array",max_items:2,items:#{type:"object",allow_unknown:false,fields:#{label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Name read in details."},value:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Displayed quantity."}}}}},required:false,sensitive:false,doc:"Up to two compact metrics, with full labels in details."}}}},required:true,sensitive:true,doc:"Tasks, in the desired backlog order."},
            agents:#{schema:#{type:"array",max_items:12,items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable agent ID."},label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Name in the filter."},mark:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional short avatar initials."}}}},required:false,sensitive:false,doc:"Optional agent filter choices.","default":#{type:"array",value:[]}},
            assignments:#{schema:#{type:"array",max_items:224,items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable booked piece ID."},task:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Task ID."},day:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Day ID."},period:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Session ID."},units:#{schema:#{type:"integer",min:1,max:1000000},required:true,sensitive:false,doc:"Booked usage."},ordinal:#{schema:#{type:"integer",min:1,max:56},required:true,sensitive:false,doc:"One-based part number within its task."},origin:#{schema:#{type:"string",allowed:["booked","manual","auto"]},required:true,sensitive:false,doc:"Accepted assignment source."}}}},required:true,sensitive:false,doc:"Accepted placements. Each booked task must be represented completely."},
            current:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{day:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Current day ID."},period:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Current session ID."}}}},required:false,sensitive:false,doc:"Current session. Earlier bookings remain inspectable but cannot be picked up or receive placements."},
            minimum:#{schema:#{type:"integer",min:1,max:1000000},required:false,sensitive:false,doc:"Minimum capacity of a split part (one-part tasks may be smaller).","default":#{type:"integer",value:15}},
            mode:#{schema:#{type:"string",allowed:["auto","manual"]},required:true,sensitive:false,doc:"Accepted mode. Changing to manual removes only automatic placements when the Host accepts the proposal."},
            title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Heading."},
            note:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Secondary heading, such as the session duration."},
            pending:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Suppress further proposals while the Host is deciding.","default":#{type:"bool",value:false}},
            on_propose:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a complete proposed next assignment set and revision."},
        },
        state:#{fields:#{
            carry:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
            pointer:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
            inspect:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
            agent:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
            notice:#{schema:#{type:"ui_value"},"default":#{type:"null"}}
        }},
        events:#{propose:#{payload:#{type:"object",allow_unknown:false,fields:#{board:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Board identity."},revision:#{schema:#{type:"integer",min:0,max:2147483647},required:true,sensitive:false,doc:"Accepted revision used."},kind:#{schema:#{type:"string",allowed:["auto","manual","place","move"]},required:true,sensitive:false,doc:"Request type."},mode:#{schema:#{type:"string",allowed:["auto","manual"]},required:true,sensitive:false,doc:"Requested mode."},assignments:#{schema:#{type:"array",max_items:224,items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable booked piece ID."},task:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Task ID."},day:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Day ID."},period:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Session ID."},units:#{schema:#{type:"integer",min:1,max:1000000},required:true,sensitive:false,doc:"Booked usage."},ordinal:#{schema:#{type:"integer",min:1,max:56},required:true,sensitive:false,doc:"One-based part number within its task."},origin:#{schema:#{type:"string",allowed:["booked","manual","auto"]},required:true,sensitive:false,doc:"Accepted assignment source."}}}},required:true,sensitive:false,doc:"Complete candidate assignment set."},remaining:#{schema:#{type:"array",max_items:64,items:#{type:"string"}},required:true,sensitive:false,doc:"Task IDs still in the backlog."}}},doc:"The candidate assignment set; publish with a new revision only after acceptance."}},
        slots:#{},parts:["root","heading","title","note","tools","agent","agent_active","agent_mark","agent_name","counter","panel","stack","stack_head","stack_count","stack_header",
            "columns","day","day_current","day_caption","day_date","time","cell","past","target","blocked","current","usage","cards","subdued","bar","bar_fill","bar_fill_past","task","task_dim",
            "task_title","task_meta","task_metrics","task_stat","usage_badge","usage_label","task_part","task_mark","task_mark_box","grip","detail","detail_name","detail_line","footer","empty","mode_track","mode","mode_active"],
        effects:["placement_lifetime"],
    },render:Fn("render_schedule")
});
fn ScheduledRuns(p){render_component("patterns/scheduled_runs",p)}
fn get(p,k,d){if k in p&&p[k]!=(){p[k]}else{d}}
fn text_at(ctx,key,word){ctx.t_or("scheduled_runs."+key,word)}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn unit(n){theme_length("space.unit").times(n)}
fn curve(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn ink(ctx,part,value,base){text(value).with_style(ctx.component_style(part,base.min_width(px(0))))}
fn quiet(){style().typography("caption").text_color(theme_color("scheduled_runs.muted")).whitespace_nowrap().text_ellipsis()}
fn row_ref(kind,id){kind+":"+id}
fn cell_ref(id){row_ref("cell",id)}
fn cell_id(day,period){day.len.to_string()+":"+day+period}
fn valid(x){ids::valid(x)&&utf8_byte_count(x)<=48}
fn check(x){x!=""&&utf8_byte_count(x)<=512}
fn entities(list,short){
    let out=#{};
    for x in list {
        if !ids::valid(x.id)||(short&&!valid(x.id))||x.id in out{throw "ScheduledRuns requires unique, bounded IDs";}
        out[x.id]=x;
    }
    out
}
fn scoped(p,m){#{board:p.board,revision:p.revision,scope:m.scope}}
fn same(base,other){type_of(other)=="map"&&base.board==get(other,"board","")&&base.revision==get(other,"revision",-1)&&base.scope==get(other,"scope","")}
fn task_label(ctx,t){ctx.t_or("scheduled_runs.task","Inspect {task}",#{task:t.title})}
fn project(ctx,p,reference){
    if !ids::valid(p.board){throw "ScheduledRuns requires a stable board ID";}
    let days=entities(p.days,true);let periods=entities(p.periods,true);let tasks=entities(p.tasks,false);
    let agents=entities(p.agents,false);let people=p.agents;
    for a in p.agents{if !check(a.label)||utf8_byte_count(get(a,"mark",""))>16{throw "Invalid ScheduledRuns agent label";}}
    for t in p.tasks{
        let a=get(t,"agent","");
        if !check(t.title)||utf8_byte_count(get(t,"detail",""))>2048||(a!=""&&!ids::valid(a)){throw "Invalid ScheduledRuns task title, detail or agent";}
        if a!=""&&!(a in agents){
            if people.len>=12{throw "ScheduledRuns supports up to twelve agents";}
            let person=#{id:a,label:a};agents[a]=person;people.push(person);
        }
    }
    let now=get(p,"current",());
    if now!=()&&(!(now.day in days)||!(now.period in periods)){throw "ScheduledRuns current session must be in the accepted calendar";}
    let cells=#{};let order=[];let past=now!=();
    for day in p.days{
        if !check(day.label){throw "Invalid ScheduledRuns day label";}
        for period in p.periods{
            if !check(period.label){throw "Invalid ScheduledRuns session label";}
            let id=cell_id(day.id,period.id);
            let current=now!=()&&now.day==day.id&&now.period==period.id;
            if current{past=false;}
            cells[id]=#{id:id,day:day.id,period:period.id,label:day.label+" "+period.label,limit:period.limit,
                booked:0,cards:[],past:past,current:current,index:order.len};
            order.push(id);
        }
    }
    let seen=entities(p.assignments,false);let sums=#{};let ordinals=#{};let by_task=#{};let parts=#{};
    for part in p.assignments{
        let id=cell_id(part.day,part.period);
        if !(id in cells)||!(part.task in tasks){throw "ScheduledRuns placement references an unknown task or session";}
        let ranks=get(ordinals,part.task,[]);
        if part.ordinal in ranks{throw "Duplicate ScheduledRuns part ordinal";}
        ranks.push(part.ordinal);ordinals[part.task]=ranks;
        let scheduled_cells=get(by_task,part.task,[]);
        if id in scheduled_cells{throw "ScheduledRuns accepts at most one piece of a task per session";}
        scheduled_cells.push(id);by_task[part.task]=scheduled_cells;
        sums[part.task]=get(sums,part.task,0)+part.units;
        let c=cells[id];c.cards.push(part.id);c.booked+=part.units;cells[id]=c;
        parts[part.id]=#{part:part,cell:id,rank:parts.len};
    }
    for task in sums.keys(){
        if sums[task]!=tasks[task].units{throw "ScheduledRuns accepted placements must add up to the task's usage";}
        let ranks=ordinals[task];for i in 1..ranks.len+1{if !(i in ranks){throw "ScheduledRuns part ordinals must be contiguous";}}
    }
    let backlog=[];let demand=[];let ranks=#{};
    for i in 0..p.tasks.len{
        let t=p.tasks[i];ranks[t.id]=i;
        if !(t.id in sums){backlog.push(t.id);demand.push(#{id:t.id,units:t.units});}
    }
    let windows=[];
    for id in order{let c=cells[id];windows.push(#{id:id,limit:c.limit,booked:c.booked,cards:c.cards.len,card_limit:2,past:c.past});}
    #{cells:cells,order:order,windows:windows,tasks:tasks,agents:agents,people:people,backlog:backlog,demand:demand,
      parts:parts,part_ids:seen,counts:ordinals,ranks:ranks,by_task:by_task,scope:reference.semantic_id}
}
fn tx(cfg){#{board:cfg.board,revision:cfg.revision,scope:cfg.scope}}
fn payload(cfg,kind,id){let p=tx(cfg);p.kind=kind;p.id=id;p}
fn wire_id(cfg,kind,id){
    if kind=="task"{cfg.wire+":t"+cfg.ranks[id].to_string()}else{cfg.wire+":p"+cfg.parts[id].rank.to_string()}
}
fn known_source(cfg,source){
    if !same(cfg,source){return false;}
    let id=get(source,"id","");
    switch get(source,"kind",""){
        "task"=>id in cfg.backlog,
        "part"=>id in cfg.parts,
        _=>false,
    }
}
fn movable_source(cfg,source){
    cfg.ready&&known_source(cfg,source)&&(source.kind=="task"||!cfg.cells[cfg.parts[source.id].cell].past)
}
fn eligible(cfg,source,id){
    if !movable_source(cfg,source)||!(id in cfg.cells){return false;}
    let c=cfg.cells[id];
    if c.past||c.cards.len>=2||c.booked>=c.limit{return false;}
    if source.kind=="task"{return true;}
    let p=cfg.parts[source.id];
    id!=p.cell&&!(id in cfg.by_task[p.part.task])&&p.part.units<=c.limit-c.booked
}
fn say(cfg,ctx,word){ctx.set_state("notice",#{board:cfg.board,revision:cfg.revision,scope:cfg.scope,message:word});}
fn requested(cfg,kind,mode,assignments,ctx){
    if !cfg.ready{return;}
    if assignments.len>224{say(cfg,ctx,text_at(ctx,"too_many","Too many placements for this board."));return;}
    let included=#{};
    for part in assignments{included[part.task]=true;}
    let remaining=[];
    for t in cfg.task_order{if !(t in included){remaining.push(t);}}
    let carry=ctx.get_state("carry");
    if same(cfg,carry){ctx.focus(row_ref(carry.kind,carry.id));}
    ctx.set_state("carry",());ctx.set_state("pointer",());ctx.set_state("notice",());
    ctx.emit("propose",#{board:cfg.board,revision:cfg.revision,kind:kind,mode:mode,assignments:assignments,remaining:remaining});
}
fn allocation(cfg,demand,target){
    let request=#{windows:cfg.windows,tasks:demand,minimum:cfg.minimum};
    if target!=""{request.target=target;}
    schedule_capacity(request)
}
fn apply_plan(cfg,answer,origin){
    let output=cfg.assignments;let names=cfg.part_ids;
    for item in answer.placements{
        for i in 0..item.pieces.len{
            let bit=item.pieces[i];let c=cfg.cells[bit.window];
            let prefix="s:"+cfg.revision.to_string()+":"+cfg.ranks[item.task].to_string()+":"+(i+1).to_string();
            let part_id=prefix;let attempt=0;
            while part_id in names{attempt+=1;part_id=prefix+":"+attempt.to_string();}
            names[part_id]=true;
            output.push(#{id:part_id,task:item.task,day:c.day,period:c.period,units:bit.units,ordinal:i+1,origin:origin});
        }
    }
    output
}
fn mode_request(cfg,ctx,mode){
    if !cfg.ready{return handled();}
    if mode=="manual"{
        let pinned=#{};for part in cfg.assignments{if part.origin!="auto"{pinned[part.task]=true;}}
        let keep=[];for part in cfg.assignments{if part.task in pinned{keep.push(part);}}
        if mode!=cfg.mode||keep.len!=cfg.assignments.len{requested(cfg,"manual",mode,keep,ctx);}
    }else if cfg.windows.len>0{
        let result=allocation(cfg,cfg.demand,"");
        if result.placements.len==0&&cfg.mode==mode{say(cfg,ctx,text_at(ctx,"no_room","No room for the remaining tasks"));}
        else{requested(cfg,"auto",mode,apply_plan(cfg,result,"auto"),ctx);}
    }
    handled()
}
fn submit(cfg,source,id,ctx){
    if !eligible(cfg,source,id){
        say(cfg,ctx,text_at(ctx,"no_room_here","This session cannot accept the task."));
        return handled();
    }
    if source.kind=="part"{
        let after=[];let c=cfg.cells[id];
        for part in cfg.assignments{
            if part.id==source.id{part.day=c.day;part.period=c.period;part.origin="manual";}
            after.push(part);
        }
        requested(cfg,"move",cfg.mode,after,ctx);
    }else{
        let units=cfg.units[source.id];
        let result=allocation(cfg,[#{id:source.id,units:units}],id);
        if result==(){say(cfg,ctx,text_at(ctx,"no_room_here","This session cannot accept the task."));}
        else{requested(cfg,"place",cfg.mode,apply_plan(cfg,result,"manual"),ctx);}
    }
    handled()
}
fn dropped(cfg,id,target,ctx,event){
    if !cfg.ready||event.operation!="move"||event.target_id!=target||event.payload_type!=cfg.wire{return;}
    let source=event.payload;
    if type_of(source)!="map"||!("scope" in source)||!same(cfg,source)||!("kind" in source)||!("id" in source){return;}
    if source.kind=="task"&&!(source.id in cfg.backlog)||source.kind=="part"&&!(source.id in cfg.parts){return;}
    if !(source.kind in ["task","part"])||wire_id(cfg,source.kind,source.id)!=event.source_id{return;}
    submit(cfg,source,id,ctx);
}
fn select_agent(cfg,agent,ctx,event){
    let before=ctx.get_state("agent");
    ctx.set_state("agent",if before!=()&&before.board==cfg.board&&before.agent==agent{()}else{#{board:cfg.board,agent:agent}});
    handled()
}
fn placement_targets(cfg,source){
    let targets=[];
    for target in cfg.order{
        if eligible(cfg,source,target)&&(source.kind=="part"||allocation(cfg,[#{id:source.id,units:cfg.units[source.id]}],target)!=()){
            targets.push(target);
        }
    }
    targets
}
fn pointer_start(cfg,source,ctx,event){
    if !movable_source(cfg,source){return;}
    source.targets=placement_targets(cfg,source);
    // Even a drag with no valid target presents the blocked states.
    ctx.set_state("pointer",source);ctx.set_state("carry",());ctx.set_state("inspect",());
}
fn pointer_end(cfg,source,ctx,event){
    let pointer=ctx.get_state("pointer");
    if same(cfg,pointer)&&pointer.kind==source.kind&&pointer.id==source.id{ctx.set_state("pointer",());}
}
fn clear_expired_placement(ctx,lifetime){
    if lifetime.pointer!=()&&ctx.get_state("pointer")==lifetime.pointer{ctx.set_state("pointer",());}
    if lifetime.carry!=()&&ctx.get_state("carry")==lifetime.carry{
        ctx.set_state("carry",());
        if lifetime.focus!=""{ctx.focus(lifetime.focus);}
    }
}
fn placement_cleanup(ctx,lifetime){()}
fn pick_up(cfg,source,ctx,event){
    if !movable_source(cfg,source)||movable_source(cfg,ctx.get_state("pointer")){return handled();}
    let targets=placement_targets(cfg,source);
    if targets.len>0{
        source.targets=targets;
        ctx.set_state("carry",source);ctx.set_state("inspect",());ctx.set_state("notice",());ctx.focus(cell_ref(targets[0]));
    }else{say(cfg,ctx,text_at(ctx,"no_room","No room for the remaining tasks"));}
    handled()
}
fn enter_zone(cfg,id,ctx,event){let source=ctx.get_state("carry");if movable_source(cfg,source){submit(cfg,source,id,ctx)}else{handled()}}
fn leave_pick(source,ctx,event){
    ctx.set_state("carry",());ctx.set_state("notice",());ctx.focus(row_ref(source.kind,source.id));handled()
}
fn focus_cell(key,ctx,event){ctx.focus(cell_ref(key));handled()}
fn usage_label(ctx,units,limit){
    ctx.format_number((units.to_float()*100.0/limit.to_float()).round().to_int())+"%"
}
fn detail_open(source,ctx,value){
    if value{ctx.set_state("inspect",source);ctx.set_state("carry",());}
    else if ctx.get_state("inspect")==source{ctx.set_state("inspect",());}
}
fn mark(ctx,t,model){
    let person=get(t,"agent","");
    let letter=if person in model.agents{let a=model.agents[person];get(a,"mark",a.label.sub_string(0,1).to_upper())}
        else{t.title.sub_string(0,1).to_upper()};
    row([ink(ctx,"task_mark",letter,style().typography("caption").font_weight(500)
        .text_color(theme_color("scheduled_runs.ink")))]).accessibility_hidden(true)
        .with_style(ctx.component_style("task_mark_box",style().width(unit(4.5)).height(unit(4.5)).flex_shrink(false)
            .items_center().justify_center().radius(unit(1)).background(theme_color("scheduled_runs.card"))))
}
fn drag_grip(ctx){
    svg("<svg viewBox='0 0 12 12' fill='currentColor'><circle cx='6' cy='3' r='1'/><circle cx='6' cy='6' r='1'/><circle cx='6' cy='9' r='1'/></svg>")
        .accessibility_hidden(true).with_style(ctx.component_style("grip",style().width(unit(3)).height(unit(3))
            .text_color(theme_color("scheduled_runs.muted"))))
}
fn details(ctx,cfg,source,t,units,limit,period){
    let lines=[ink(ctx,"detail_name",t.title,style().typography("body").font_weight(600)),
        ink(ctx,"detail_line",get(t,"detail",text_at(ctx,"no_detail","No additional details")),style().typography("caption").text_color(theme_color("text_muted"))),
        ink(ctx,"detail_line",usage_label(ctx,units,limit)+if period!=""{" · "+period}else{""},quiet())];
    for v in get(t,"stats",[]){lines.push(ink(ctx,"detail_line",v.label+": "+v.value,quiet()));}
    if movable_source(cfg,source){
        lines.push(btn::Button(#{key:"pick:"+row_ref(source.kind,source.id),size:"xs",text:text_at(ctx,"move_keys","Move with keyboard"),variant:"ghost",
          on_click:Fn("pick_up").curry(cfg,source)}));
    }
    column(lines).with_key("detail").accessibility_role("dialog").accessibility_label(t.title)
      .with_style(ctx.component_style("detail",style().width(px(264)).min_width(px(0)).max_width(relative(1.0))
        .gap(sp(2)).padding(sp(3)).background(theme_color("surface_raised")).radius(theme_radius("lg"))
        .border(px(1)).border_color(theme_color("border"))))
}
fn card(ctx,cfg,model,selected,t,record,cell){
    let kind=if record==(){"task"}else{"part"};let id=if record==(){t.id}else{record.part.id};
    let amount=if record==(){t.units}else{record.part.units};
    let limit=if cell==(){cfg.default_limit}else{cell.limit};
    let source=payload(cfg,kind,id);
    let shown=ctx.get_state("inspect")==source;
    let can_move=movable_source(cfg,source);
    let label=task_label(ctx,t);
    let percentage=usage_label(ctx,amount,limit);
    let rank=if record==(){""}else{
        let n=model.counts[t.id].len;
        if n>1{ctx.format_number(record.part.ordinal)+"/"+ctx.format_number(n)}else{""}
    };
    let content=[mark(ctx,t,model),ink(ctx,"task_title",t.title,style().typography("caption").font_weight(600)
        .whitespace_nowrap().text_ellipsis().flex_grow()).with_key("title")];
    if rank!=""{content.push(ink(ctx,"task_part",rank,quiet().flex_shrink(false)));}
    let agent=get(t,"agent","");
    let tagline=if agent in model.agents{model.agents[agent].label}else{""};
    let description=get(t,"detail","");
    if description!=""{tagline+=if tagline!=""{" · "+description}else{description};}
    let metadata=ink(ctx,"task_meta",tagline,quiet().line_height(unit(3.5)).min_height(unit(3.5))
        .margin_start(unit(6))).with_key("summary");
    let metrics=[];
    for stat in get(t,"stats",[]){
        metrics.push(ink(ctx,"task_stat",stat.value,quiet().font_feature("tnum",1))
            .accessibility_label(stat.label+": "+stat.value));
    }
    let badge=row([ink(ctx,"usage_label",percentage,quiet().font_feature("tnum",1)).with_key("percentage")])
        .with_key("usage-pill").with_style(ctx.component_style("usage_badge",style().items_center()
            .height(unit(3.75)).padding_x(unit(1.25)).background(theme_color("scheduled_runs.track"))
            .radius(theme_radius("sm")).flex_shrink(false)));
    let stats=row([row(metrics).with_style(style().min_width(px(0)).flex_grow().items_center().gap(unit(2.5))),badge])
        .with_key("stats").with_style(ctx.component_style("task_metrics",style()
            .min_width(px(0)).margin_start(unit(6)).height(unit(4)).items_center().gap(unit(2.5))));
    // One compact card serves the backlog and every booked state. Only the
    // decorative rows change here; the primary inspection/drag keys do not.
    let button=column([row(content).with_key("title-line").with_style(style().items_center().min_width(px(0)).gap(unit(1.5))),
        metadata,stats])
        .with_key("inspection").with_ref(element_ref(row_ref(kind,id))).tab_stop(true).accessibility_role("button")
        .accessibility_label(label).accessibility_description(percentage+(if rank!=""{" · "+rank}else{""})+(if tagline!=""{" · "+tagline}else{""}))
        .accessibility_expanded(shown)
        .with_style(style().width(relative(1.0)).min_width(px(0)).justify_center()
          .gap(unit(0.75)).padding_start(unit(1.5)).padding_end(unit(1))
          .radius(theme_radius("sm")).focus_ring(px(2),offset_px(-1),theme_color("focus_ring")));
    if can_move{button=button.on_key_value("plain+space",Fn("pick_up").curry(cfg,source),());}
    let inspect=overlay(button,if shown{details(ctx,cfg,source,t,amount,limit,if cell==(){""}else{cell.label})}else{column([])},
        #{id:"detail-"+kind+":"+id,kind:"popover",placement:"bottom",align:"start",gap:6.0,open:shown,modal:false,initial_focus:"panel",
        dismiss_on_outside:true,dismiss_on_escape:true})
        .with_key("detail").on_open_change(Fn("detail_open").curry(source))
        .with_style(style().flex_grow().flex_basis(px(0)).min_width(px(0)));
    let inside=[inspect];
    if can_move{
        let grip_label=ctx.t_or("scheduled_runs.grab","Drag {task}",#{task:t.title});
        inside.push(drag::DragSource(#{key:"drag:"+row_ref(kind,id),label:grip_label,source_id:wire_id(cfg,kind,id),payload_type:cfg.wire,payload:source,
             on_drag_start:Fn("pointer_start").curry(cfg,source),on_drag_end:Fn("pointer_end").curry(cfg,source),
             content:row([drag_grip(ctx)]).with_style(style().width(unit(5)).height(unit(8)).items_center().justify_center())})
            .with_style(style().width(unit(5)).flex_shrink(false)));
    }
    let dim=selected!=""&&agent!=selected;
    let layout_key=if record==()||record.part.ordinal==1{"task:"+t.id}else{"part:"+id};
    row(inside).with_key(row_ref(kind,id))
        .with_style(ctx.component_style(if dim{["task","task_dim"]}else{["task"]},
             style().min_width(px(0)).height(unit(15)).items_center().justify_between().padding_end(sp(1))
                .radius(theme_radius("sm")).background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border"))
                .isolated_opacity(dim).opacity(if dim{0.35}else{1.0})))
        .shared_layout(cfg.scope,layout_key).layout_motion(540,curve()).paint_transition(160,"ease_out")
}
// The faded layer is local; never apply it to a gridcell or its drop reason.
fn session_part(ctx,part,base,dim){
    if dim{ctx.component_style([part,"subdued"],base.isolated_opacity(true).opacity(0.45))}
    else{ctx.component_style(part,base)}
}
fn blocked_background(){
    let well=theme_color("scheduled_runs.well");
    let ink=theme_color("scheduled_runs.hatch");
    let edge=6.0/7.0;
    layered_gradient([#{angle:135,repeat_period:7,stops:[
        #{offset:0,color:well},#{offset:edge,color:well},
        #{offset:edge,color:ink},#{offset:1,color:ink}
    ]}])
}
fn load_bar(ctx,c,blocked){
    let proportion=min(1.0,max(0.0,c.booked.to_float()/c.limit.to_float()));
    let fill_style=style().height(relative(1.0)).width(relative(proportion))
        .background(theme_color("scheduled_runs.capacity")).radius(theme_radius("pill"));
    let fill=box([]).with_key("fill")
        .with_style(if c.past{ctx.component_style(["bar_fill","bar_fill_past"],fill_style.opacity(0.5))}
          else{ctx.component_style("bar_fill",fill_style)})
        .layout_motion(420,curve());
    row([fill]).with_key("bar").accessibility_role("progressbar").accessibility_label(c.label)
        .accessibility_description(ctx.format_number(c.booked)+"/"+ctx.format_number(c.limit))
        .accessibility_value(min(c.limit,c.booked)).accessibility_value_min(0).accessibility_value_max(c.limit)
        .with_style(session_part(ctx,"bar",style().height(unit(1)).width(relative(1.0)).radius(theme_radius("pill"))
          .background(theme_color("scheduled_runs.track")),blocked))
}
fn zone_button(ctx,c,cfg,source,active){
    let target=ctx.t_or("scheduled_runs.place_here","Place in {session}",#{session:c.label});
    let reference=element_ref(cell_ref(c.id));
    let base=style().min_width(px(0)).items_center().justify_center().min_height(unit(6)).radius(theme_radius("sm"))
        .focus_ring(px(2),offset_px(-1),theme_color("focus_ring"));
    let available=active&&c.id in source.targets;
    let words=if active{if available{text_at(ctx,"drop","Place here")}else{text_at(ctx,"unavailable","Unavailable")}}else{""};
    let node=row([ink(ctx,"usage_label",words,quiet())]).with_key("keyboard").with_ref(reference).tab_stop(active).with_style(base);
    if !active{return node.accessibility_hidden(true).with_style(style().min_height(px(0)).height(px(0)).overflow_hidden());}
    let count=cfg.order.len;let index=c.index;let rows=cfg.period_count;
    node.accessibility_role("button").accessibility_label(target)
        .accessibility_description(if available{text_at(ctx,"drop","Place here")}else{text_at(ctx,"unavailable","Unavailable")})
        .on_click(Fn("enter_zone").curry(cfg,c.id)).on_key_value("plain+enter",Fn("enter_zone").curry(cfg,c.id),())
        .on_key_value("plain+space",Fn("enter_zone").curry(cfg,c.id),())
        .on_key_value("plain+escape",Fn("leave_pick").curry(source),())
        .on_key_value("plain+up",Fn("focus_cell").curry(cfg.order[if index%rows>0{index-1}else{index}]),())
        .on_key_value("plain+down",Fn("focus_cell").curry(cfg.order[if index%rows<rows-1{index+1}else{index}]),())
        .on_key_value("plain+left",Fn("focus_cell").curry(cfg.order[if index>=rows{index-rows}else{index}]),())
        .on_key_value("plain+right",Fn("focus_cell").curry(cfg.order[if index+rows<count{index+rows}else{index}]),())
}
fn cell_tile(ctx,c,cfg,m,selected,source,keyboard){
    let items=[];
    for id in c.cards{
        let placed=m.parts[id];
        items.push(card(ctx,cfg,m,selected,m.tasks[placed.part.task],placed,c));
    }
    let carrying=source!=();
    let from_here=carrying&&source.kind=="part"&&source.id in cfg.parts&&cfg.parts[source.id].cell==c.id;
    let blocked=carrying&&!from_here&&!(c.id in source.targets);
    let stage=[];
    if c.current{stage.push(box([]).with_key("line").accessibility_hidden(true)
        .with_style(session_part(ctx,"current",style().absolute().top(px(0)).inset_start(px(0))
             .width(relative(1.0)).height(unit(0.375)).background(theme_color("scheduled_runs.current")),blocked)));}
    stage.push(column(items).with_key("cards").with_style(session_part(ctx,"cards",
        style().min_height(px(0)).min_width(px(0)).flex_grow().gap(sp(1)).overflow_y_scroll(),
        (c.past||blocked)&&items.len>0)));
    stage.push(zone_button(ctx,c,cfg,source,carrying&&keyboard));
    stage.push(load_bar(ctx,c,blocked));
    let label=c.label;
    let shell=ctx.component_style("cell",style().relative().height(unit(40)).min_width(px(0)).gap(sp(1)).padding(sp(1))
        .radius(theme_radius("sm")).background(theme_color("scheduled_runs.well")));
    if c.past{shell=shell.merge(ctx.component_style("past",style().background(rgba(0x00000000))));}
    // Apply the blocked background after the past background as separate
    // declarations. A preset's transparent past fill must not win here.
    if blocked{shell=shell.merge(ctx.component_style("blocked",style().layered_gradient(blocked_background())));}
    else if keyboard&&carrying&&!from_here{shell=shell.merge(ctx.component_style("target",style()));}
    let body=column(stage).with_key("window").accessibility_role("gridcell").accessibility_label(label)
        .accessibility_row_index(c.index%cfg.period_count+2).accessibility_column_index(c.index/cfg.period_count+2)
        .accessibility_description(ctx.format_number(c.booked)+"/"+ctx.format_number(c.limit)+(if c.past{" · "+text_at(ctx,"past","Past")}else{""}))
        .with_style(shell);
    let target=cfg.wire+":w"+c.index.to_string();
    drop::DropZone(#{key:"drop:"+c.id,label:ctx.t_or("scheduled_runs.drop_here","Drop on {session}",#{session:label}),
        target_id:target,payload_types:[cfg.wire],operations:["move"],content:body,
        disabled:!cfg.ready||c.past||c.cards.len>=2||c.booked>=c.limit,on_drop:Fn("dropped").curry(cfg,c.id,target)})
      .with_key("slot:"+c.id)
}
fn grid_track(){style().min_width(unit(32)).flex_basis(px(0)).flex_grow()}
fn time_track(){style().width(unit(13)).flex_shrink(false)}
fn grid(ctx,p,cfg,m,selected,source,keyboard){
    let base=style().height(unit(7)).min_width(px(0)).items_center().padding_x(sp(2)).gap(sp(1))
         .background(theme_color("scheduled_runs.well")).radius(theme_radius("sm"));
    let headings=[box([]).with_key("spacer").accessibility_hidden(true).with_style(time_track())];
    for i in 0..p.days.len{
        let d=p.days[i];
        let today=get(p,"current",())!=()&&p.current.day==d.id;
        let bits=[ink(ctx,"day_caption",d.label.to_upper(),quiet().typography("label").font_weight(500)),
            ink(ctx,"day_date",get(d,"date",""),quiet().font_feature("tnum",1)
                .text_color(theme_color("scheduled_runs.text")))];
        headings.push(row(bits).with_key(d.id).accessibility_column_header(d.label,i+2)
           .with_style(grid_track()).with_style(ctx.component_style(if today{["day","day_current"]}else{["day"]},base)));
    }
    let lanes=[row(headings).with_key("days").accessibility_role("row").accessibility_row_index(1)
        .with_style(style().min_width(px(0)).gap(sp(1)))];
    for j in 0..p.periods.len{
        let part=p.periods[j];
        let tiles=[row([ink(ctx,"time",part.label,quiet().font_feature("tnum",1)
            .text_color(theme_color("scheduled_runs.faint")))]).with_key("period").accessibility_role("rowheader")
            .accessibility_label(part.label).accessibility_column_index(1)
            .with_style(time_track().items_start().padding_top(unit(1.5)))];
        for d in p.days{tiles.push(cell_tile(ctx,m.cells[cell_id(d.id,part.id)],cfg,m,selected,source,keyboard).with_style(grid_track()));}
        lanes.push(row(tiles).with_key(part.id).accessibility_role("row").accessibility_row_index(j+2)
            .with_style(style().min_width(px(0)).gap(sp(1))));
    }
    column(lanes).with_key("columns").accessibility_role("grid").accessibility_label(text_at(ctx,"week","Week schedule"))
        .accessibility_row_count(p.periods.len+1).accessibility_column_count(p.days.len+1)
        .with_style(style().width(relative(1.0)).min_width(unit(13+33*p.days.len)).gap(sp(1)))
}
fn mode_choices(ctx,cfg){
    let spark=svg("<svg viewBox='0 0 16 16' fill='currentColor'><path d='M8 1.75c.3 2.9 1.35 3.95 4.25 4.25C9.35 6.3 8.3 7.35 8 10.25 7.7 7.35 6.65 6.3 3.75 6 6.65 5.7 7.7 4.65 8 1.75zM12.25 9.5c.15 1.3.6 1.75 1.9 1.9-1.3.15-1.75.6-1.9 1.9-.15-1.3-.6-1.75-1.9-1.9 1.3-.15 1.75-.6 1.9-1.9z'/></svg>")
        .with_style(style().width(unit(3)).height(unit(3)).flex_shrink(false));
    radio::RadioGroup(#{key:"mode",label:text_at(ctx,"mode_label","Scheduling"),orientation:"horizontal",appearance:"chip",
        value:cfg.mode,disabled:!cfg.ready,options:[
            #{value:"auto",label:text_at(ctx,"auto","Auto"),leading:spark},
            #{value:"manual",label:text_at(ctx,"manual","Manual")}
        ],on_change:Fn("mode_request").curry(cfg),part_styles:#{
            root:ctx.component_style("mode_track",style().padding(sp(0.5)).gap(px(0)).radius(theme_radius("sm")).background(theme_color("surface_hover"))),
            option:ctx.component_style("mode",style().gap(sp(1.25))),
            option_selected:ctx.component_style("mode_active",style())
        }}).with_key("mode-track").with_ref(element_ref("mode-track"))
}
fn task_count(ctx,n){
    let vars=#{n:ctx.format_number(n)};
    if n!=1{return ctx.t_or("scheduled_runs.total","{n} tasks",vars);}
    ctx.t_or("scheduled_runs.total_one",ctx.t_or("scheduled_runs.total","{n} task"),vars)
}
fn header(ctx,p,cfg,m,agent){
    let buttons=[];
    for a in m.people{
        let active=a.id==agent;let initials=get(a,"mark",a.label.sub_string(0,1).to_upper());
        let inactive=agent!=""&&!active;
        let avatar=row([ink(ctx,"agent_mark",initials,quiet().font_weight(500).text_color(theme_color("scheduled_runs.ink")))])
            .with_key("avatar").accessibility_hidden(true)
            .with_style(style().width(unit(4)).height(unit(4)).items_center().justify_center().flex_shrink(false));
        let content=[avatar];
        if active{
            content.push(ink(ctx,"agent_name",a.label,quiet().font_weight(500).text_color(theme_color("scheduled_runs.ink"))
                .max_width(unit(30)).margin_start(sp(1.5)).margin_end(sp(0.75)))
                .with_key("name").accessibility_hidden(true));
        }
        let border=style().min_width(unit(6.5)).height(unit(6.5)).items_center().justify_center().padding_x(sp(1.25)).radius(theme_radius("sm"))
             .background(theme_color("scheduled_runs.card")).focus_ring(px(2),offset_px(-1),theme_color("focus_ring"))
             .isolated_opacity(true).opacity(if inactive{0.6}else{1.0})
             .border_color(theme_color("border")).border(px(1));
        let cb=Fn("select_agent").curry(cfg,a.id);
        buttons.push(row(content).with_key("agent:"+a.id)
            .tab_stop(true).accessibility_role("button").accessibility_label(a.label).accessibility_pressed(active)
            .on_click(cb).on_key_value("plain+enter",cb,()).on_key_value("plain+space",cb,())
            .with_style(ctx.component_style(if active{["agent","agent_active"]}else{["agent"]},border))
            .paint_transition(200,"ease_out").layout_size_motion(if active{a.label}else{""},"width",380,curve()));
    }
    let tools=[row(buttons).with_style(style().min_width(px(0)).gap(sp(0.5))),
        ink(ctx,"counter",task_count(ctx,p.tasks.len),quiet()),
        mode_choices(ctx,cfg)];
    let headline=style().typography("body").font_size(unit(3.75)).line_height(unit(6)).font_weight(600);
    row([row([ink(ctx,"title",get(p,"title",text_at(ctx,"title","This week")),headline),
         ink(ctx,"note",get(p,"note",""),quiet().typography("metadata"))])
        .with_key("heading-text").with_style(headline.items_baseline().gap(unit(2)).min_width(px(0))),
        row(tools).with_style(style().min_width(px(0)).items_center().flex_wrap().gap(sp(2)))])
        .with_key("head").with_style(ctx.component_style("heading",style().min_width(px(0)).items_center().justify_between()
             .flex_wrap().gap(sp(2)).padding(sp(2))))
}
fn render_schedule(ctx,p){
    let reference=element_ref("week");let m=project(ctx,p,reference);
    let cfg=#{board:p.board,revision:p.revision,scope:m.scope,wire:"schedule:"+p.board,ready:!p.pending&&get(p,"on_propose",())!=(),
        cells:m.cells,order:m.order,windows:m.windows,assignments:p.assignments,parts:m.parts,part_ids:m.part_ids,by_task:m.by_task,
        ranks:m.ranks,demand:m.demand,backlog:m.backlog,mode:p.mode,minimum:p.minimum,period_count:p.periods.len,
        default_limit:if p.periods.len>0{p.periods[0].limit}else{100},task_order:[],units:#{}};
    for task in p.tasks{cfg.task_order.push(task.id);cfg.units[task.id]=task.units;}
    let pointer=ctx.get_state("pointer");
    let carry=ctx.get_state("carry");
    let carrying_pointer=movable_source(cfg,pointer);
    let carrying_keyboard=movable_source(cfg,carry);
    let expired=if !carrying_keyboard{carry}else{()};
    effect("placement_lifetime",#{pointer:if !carrying_pointer{pointer}else{()},carry:expired,
        focus:if known_source(cfg,expired){row_ref(expired.kind,expired.id)}else{""}},
        Fn("clear_expired_placement"),Fn("placement_cleanup"));
    let source=if carrying_pointer{pointer}else if carrying_keyboard{carry}else{()};
    let picked=ctx.get_state("agent");let selected=if picked!=()&&picked.board==p.board&&picked.agent in m.agents{picked.agent}else{""};
    let count=row([text(ctx.format_number(m.backlog.len))]).with_key("backlog-count")
        .with_style(ctx.component_style("stack_count",quiet().typography("metadata").min_height(unit(4.5)).min_width(unit(5)).padding_x(sp(1.25))
            .radius(theme_radius("sm")).items_center().justify_center().font_feature("tnum",1)));
    let standby=[row([ink(ctx,"stack_header",text_at(ctx,"backlog","To schedule"),quiet().font_weight(500)),count])
        .with_key("backlog-head").with_style(ctx.component_style("stack_head",
            style().justify_between().items_center().flex_shrink(false).min_height(unit(7.5)).padding_start(sp(2)).padding_end(sp(1.5))))];
    let cards=[];
    for id in m.backlog{cards.push(card(ctx,cfg,m,selected,m.tasks[id],(),()));}
    if cards.len==0{cards.push(ink(ctx,"empty",text_at(ctx,"empty","Everything is scheduled"),quiet().padding(sp(1))));}
    standby.push(column(cards).with_key("items").with_style(style().min_width(px(0)).gap(sp(1)).max_height(unit(80)).overflow_y_scroll()));
    let compact=ctx.element_width_class(reference,[760])==0;
    let base=style().min_width(px(0)).padding(sp(1)).gap(sp(1)).radius(theme_radius("lg")).background(theme_color("scheduled_runs.lane"));
    if compact{base=base.width(relative(1.0));}else{base=base.width(unit(55)).flex_shrink(false).min_height(unit(60));}
    let pile=column(standby).with_key("stack").accessibility_role("group").accessibility_label(text_at(ctx,"backlog","To schedule"))
         .with_style(ctx.component_style("stack",base));
    let days=column([grid(ctx,p,cfg,m,selected,source,!carrying_pointer)]).with_key("calendar")
        .with_style(style().min_width(px(0)).flex_grow().overflow_x_scroll());
    let panel=if compact{column([pile,days])}else{row([pile,days])};
    let note=ctx.get_state("notice");
    let children=[header(ctx,p,cfg,m,selected),panel.with_key("panel").with_style(ctx.component_style("panel",
        style().min_width(px(0)).gap(sp(2)).padding(sp(1)).radius(theme_radius("lg")).background(theme_color("surface_raised"))))];
    if same(cfg,note){children.push(ink(ctx,"footer",note.message,quiet()).with_key("notice").accessibility_role("status").accessibility_label(note.message));}
    column(children).with_key(p.key).with_ref(reference).accessibility_role("group").accessibility_label(get(p,"title",text_at(ctx,"title","This week")))
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(1280)).min_width(px(0)).radius(theme_radius("lg"))))
}
