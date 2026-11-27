/* gpui-rhai
{"id":"patterns/project_card","export":"ProjectCard","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["patterns/agent_tree","components/avatar","components/button"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","project_card.accent","project_card.amber","project_card.blue","project_card.gray","project_card.green","project_card.red","project_card.ring","project_card.ring_hot","project_card.card","project_card.tray","project_card.muted","project_card.violet","project_card.amber_active","project_card.amber_wash","project_card.blue_active","project_card.blue_wash","project_card.gray_active","project_card.gray_wash","project_card.green_active","project_card.green_wash","project_card.red_active","project_card.red_wash","project_card.violet_active","project_card.violet_wash","project_card.option_fill","project_card.option_hover","project_card.option_ink","project_card.option_text","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],"environment":[]}
*/
// An expandable project summary with a segmented progress ring.
// State: disclosure, hover/focus, roster filter and picker; task and status snapshots belong to the Host.
// Example: ProjectCard(#{key:"release",project:"r1",revision:0,title:"Ship the release",tasks:[#{id:"test",label:"Test",done:true}]})
import "patterns/agent_tree" as ids;
import "components/avatar" as avatar;
import "components/button" as btn;
define_component(#{
    metadata:#{"id":"patterns/project_card","export":"ProjectCard","version":"0.1.0","runtime_api":#{"min_inclusive":3,"max_exclusive":4},"dependencies":["patterns/agent_tree","components/avatar","components/button"],"capabilities":#{},"assets":[],tokens:["border","focus_ring","project_card.accent","project_card.amber","project_card.blue","project_card.gray","project_card.green","project_card.red","project_card.ring","project_card.ring_hot","project_card.card","project_card.tray","project_card.muted","project_card.violet","project_card.amber_active","project_card.amber_wash","project_card.blue_active","project_card.blue_wash","project_card.gray_active","project_card.gray_wash","project_card.green_active","project_card.green_wash","project_card.red_active","project_card.red_wash","project_card.violet_active","project_card.violet_wash","project_card.option_fill","project_card.option_hover","project_card.option_ink","project_card.option_text","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],environment:[]},
    schema:#{
        "props":#{
            "key":#{"schema":#{"type":"string"},"required":true,"sensitive":false,"doc":"Stable retained card key."},
            "project":#{"schema":#{"type":"string"},"required":true,"sensitive":false,"doc":"Stable project identity."},
            "revision":#{"schema":#{"type":"integer","min":0},"required":true,"sensitive":false,"doc":"Accepted snapshot revision."},
            "title":#{"schema":#{"type":"string"},"required":true,"sensitive":true,"doc":"Visible project title."},
            "tasks":#{"schema":#{"type":"array","items":#{"type":"object","allow_unknown":false,"fields":#{"id":#{"schema":#{"type":"string"},"required":true,"sensitive":false,"doc":"Unique task ID."},"label":#{"schema":#{"type":"string"},"required":true,"sensitive":true,"doc":"Task label."},"done":#{"schema":#{"type":"optional","value":#{"type":"bool"}},"required":false,"sensitive":false,"doc":"Accepted completion."},"assignee":#{"schema":#{"type":"optional","value":#{"type":"string"}},"required":false,"sensitive":false,"doc":"Optional member ID, not their display name."}}},"max_items":48},"required":true,"sensitive":true,"doc":"Subtasks, in ring and list order."},
            "members":#{"schema":#{"type":"optional","value":#{"type":"array","items":#{"type":"object","allow_unknown":false,"fields":#{"id":#{"schema":#{"type":"string"},"required":true,"sensitive":false,"doc":"Unique member ID."},"name":#{"schema":#{"type":"string"},"required":true,"sensitive":true,"doc":"Display name."},"handle":#{"schema":#{"type":"optional","value":#{"type":"handle","kind":"image"}},"required":false,"sensitive":false,"doc":"Optional prepared image; Avatar handles fallback."}}},"max_items":16}},"required":false,"sensitive":false,"doc":"Optional project roster."},
            "statuses":#{"schema":#{"type":"optional","value":#{"type":"array","items":#{"type":"object","allow_unknown":false,"fields":#{"id":#{"schema":#{"type":"string"},"required":true,"sensitive":false,"doc":"Unique status ID."},"label":#{"schema":#{"type":"string"},"required":true,"sensitive":false,"doc":"Visible status label."},"tone":#{"schema":#{"type":"optional","value":#{"type":"string","allowed":["gray","violet","amber","blue","green","red"]}},"required":false,"sensitive":false,"doc":"Status tone; gray by default."},"icon":#{"schema":#{"type":"optional","value":#{"type":"node"}},"required":false,"sensitive":false,"doc":"Optional decorative replacement icon."}}},"max_items":8}},"required":false,"sensitive":false,"doc":"Status choices; defaults to To do, In progress, In review and Done."},
            "status":#{"schema":#{"type":"optional","value":#{"type":"string"}},"required":false,"sensitive":false,"doc":"Accepted status ID, or the first available option."},
            "due":#{"schema":#{"type":"optional","value":#{"type":"string"}},"required":false,"sensitive":false,"doc":"Ready-to-display due date."},
            "open":#{"schema":#{"type":"optional","value":#{"type":"bool"}},"required":false,"sensitive":false,"doc":"Controlled disclosure when present."},
            "default_open":#{"schema":#{"type":"bool"},"required":false,"sensitive":false,"doc":"Initial uncontrolled disclosure.","default":#{"type":"bool","value":false}},
            "trigger":#{"schema":#{"type":"string","allowed":["hover","click"]},"required":false,"sensitive":false,"doc":"Open with pointer hover or explicit activation; keyboard always works.","default":#{"type":"string","value":"hover"}},
            "busy":#{"schema":#{"type":"bool"},"required":false,"sensitive":false,"doc":"Ignore task/status/action proposals while the Host commits.","default":#{"type":"bool","value":false}},
            "action_label":#{"schema":#{"type":"optional","value":#{"type":"string"}},"required":false,"sensitive":false,"doc":"Footer action label; hidden unless on_action is supplied."},
            "hint":#{"schema":#{"type":"optional","value":#{"type":"node"}},"required":false,"sensitive":false,"doc":"Optional footer content when nothing is highlighted."},
            "on_task_change":#{"schema":#{"type":"optional","value":#{"type":"callback"}},"required":false,"sensitive":false,"doc":"Receives a versioned task completion proposal."},
            "on_status_change":#{"schema":#{"type":"optional","value":#{"type":"callback"}},"required":false,"sensitive":false,"doc":"Receives a versioned status proposal."},
            "on_open_change":#{"schema":#{"type":"optional","value":#{"type":"callback"}},"required":false,"sensitive":false,"doc":"Receives a disclosure change or proposal."},
            "on_action":#{"schema":#{"type":"optional","value":#{"type":"callback"}},"required":false,"sensitive":false,"doc":"Receives a versioned footer action request."},
        },
        "state":#{"fields":#{"ui":#{"schema":#{"type":"ui_value"},"default":#{"type":"null"}}}},
        "events":#{
            "task_change":#{"payload":#{"type":"object","allow_unknown":false,"fields":#{"project":#{"schema":#{"type":"string"},"required":true,"sensitive":false,"doc":"Stable project identity."},"revision":#{"schema":#{"type":"integer","min":0},"required":true,"sensitive":false,"doc":"Accepted snapshot revision."},"id":#{"schema":#{"type":"string"},"required":true,"sensitive":false,"doc":"Task ID."},"done":#{"schema":#{"type":"bool"},"required":true,"sensitive":false,"doc":"Proposed completion."}}},"doc":"Propose toggling one accepted task."},
            "status_change":#{"payload":#{"type":"object","allow_unknown":false,"fields":#{"project":#{"schema":#{"type":"string"},"required":true,"sensitive":false,"doc":"Stable project identity."},"revision":#{"schema":#{"type":"integer","min":0},"required":true,"sensitive":false,"doc":"Accepted snapshot revision."},"status":#{"schema":#{"type":"string"},"required":true,"sensitive":false,"doc":"Proposed status ID."}}},"doc":"Propose replacing the accepted status."},
            "open_change":#{"payload":#{"type":"object","allow_unknown":false,"fields":#{"project":#{"schema":#{"type":"string"},"required":true,"sensitive":false,"doc":"Stable project identity."},"revision":#{"schema":#{"type":"integer","min":0},"required":true,"sensitive":false,"doc":"Accepted snapshot revision."},"open":#{"schema":#{"type":"bool"},"required":true,"sensitive":false,"doc":"Requested disclosure."}}},"doc":"Notify or propose disclosure."},
            "action":#{"payload":#{"type":"object","allow_unknown":false,"fields":#{"project":#{"schema":#{"type":"string"},"required":true,"sensitive":false,"doc":"Stable project identity."},"revision":#{"schema":#{"type":"integer","min":0},"required":true,"sensitive":false,"doc":"Accepted snapshot revision."}}},"doc":"Invoke the caller's project action."},
        },
        "slots":#{},
        parts:["root","paper","card","head","ring","number","titles","title","sub","status","status_live","status_open","status_text","status_icon","people","face","face_active","more","picker","option","option_selected","body","tasks","task","task_hot","task_done","label","assignee","check","check_done","foot","caption","due","action","empty"],
    },
    render:Fn("render_project"),
});
fn ProjectCard(p){render_component("patterns/project_card",p)}
fn get(m,k,d){if k in m&&m[k]!=(){m[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn u(n){theme_length("space.unit").times(n)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn project_ticket(p){#{project:p.project,revision:p.revision}}
fn local(ctx,owner,initial){
    let s=ctx.get_state("ui");
    if s!=()&&s.project==owner{return s;}
    #{project:owner,open:initial,hover:(),row:"",ring:"",focus:"",member:"",pinned:"",picker:false,press:()}
}
fn commit(ctx,s){if ctx.get_state("ui")!=s{ctx.set_state("ui",s);}}
fn opened(ctx,p){if get(p,"open",())==(){local(ctx,p.project,p.default_open).open}else{p.open}}
fn update_local_disclosure(cfg,want,ctx){
    let s=local(ctx,cfg.ticket.project,cfg.initial);s.hover=();
    let old=if cfg.controlled{cfg.open}else{s.open};
    if want!=old{
        if !cfg.controlled{s.open=want;}
        if !want{s.picker=false;s.row="";s.ring="";s.focus="";s.member="";s.press=();}
    }
    commit(ctx,s);want!=old
}
fn disclose(cfg,want,ctx,e){
    if update_local_disclosure(cfg,want,ctx)&&cfg.emit{
        ctx.emit("open_change",#{project:cfg.ticket.project,revision:cfg.ticket.revision,open:want});
    }
    handled()
}
fn schedule(cfg,ctx,inside){
    let s=local(ctx,cfg.ticket.project,cfg.initial);s.hover=inside;commit(ctx,s);
}
fn due_hover(ctx,d){disclose(d.cfg,d.open,ctx,());}
fn escape(cfg,ctx,e){
    if !cfg.open{return ();}
    let s=local(ctx,cfg.ticket.project,cfg.initial);
    if s.picker{s.picker=false;commit(ctx,s);ctx.focus("status");handled()}
    else {ctx.focus("disclosure");disclose(cfg,false,ctx,e)}
}
fn cfg_for(ctx,p){#{ticket:project_ticket(p),initial:p.default_open,controlled:get(p,"open",())!=(),open:opened(ctx,p),emit:get(p,"on_open_change",())!=()}}
fn look(cfg,field,id,ctx,inside){
    let s=local(ctx,cfg.ticket.project,cfg.initial);
    let next=if inside{id}else if s[field]==id{""}else{s[field]};
    if s[field]!=next{s[field]=next;commit(ctx,s);}
}
fn select_member(cfg,id,ctx,e){
    let s=local(ctx,cfg.ticket.project,cfg.initial);s.pinned=if s.pinned==id{""}else{id};commit(ctx,s);handled()
}
fn picker(cfg,ctx,e){
    let s=local(ctx,cfg.ticket.project,cfg.initial);s.picker=!s.picker;commit(ctx,s);handled()
}
fn emit_task(t,ctx,e){ctx.emit("task_change",t);handled()}
fn emit_status(cfg,id,ctx,e){
    let s=local(ctx,cfg.ticket.project,cfg.initial);s.picker=false;commit(ctx,s);
    let t=cfg.ticket;t.status=id;ctx.emit("status_change",t);ctx.focus("status");handled()
}
fn fire(t,ctx,e){ctx.emit("action",t);handled()}
fn task_tick(){
    svg("<svg viewBox='0 0 12 12' fill='none' stroke='currentColor' stroke-width='1.4' stroke-linecap='round' stroke-linejoin='round'><path d='M2.6 6.1 4.8 8.3 9.3 3.7'/></svg>")
        .accessibility_hidden(true).with_style(style().width(px(12)).height(px(12)).flex_shrink(false))
}
fn tone_icon(tone,clean){
    let circle="<circle cx='6' cy='6' r='4.25'/>";
    let mark=switch tone{
        "amber"=>circle+"<path d='M6 3.4a2.6 2.6 0 0 1 0 5.2V3.4Z' fill='currentColor' stroke='none'/>",
        "blue"=>"<path d='M1.2 6S3 2.7 6 2.7 10.8 6 10.8 6 9 9.3 6 9.3 1.2 6 1.2 6Z'/><circle cx='6' cy='6' r='1.4' fill='currentColor' stroke='none'/>",
        "green"=>circle+"<path d='m4.1 6.1 1.3 1.3 2.5-2.7'/>",
        "red"=>circle+"<path d='M6 3.8v2.6M6 8.2v.01'/>",
        _=>"<circle cx='6' cy='6' r='4.25' stroke-dasharray='2.2 2.25'/>",
    };
    let width=if clean{"1.2"}else{"1.5"};
    svg("<svg viewBox='0 0 12 12' fill='none' stroke='currentColor' stroke-width='"+width+
        "' stroke-linecap='round' stroke-linejoin='round'>"+mark+"</svg>")
        .accessibility_hidden(true).with_style(style().width(px(12)).height(px(12)).flex_shrink(false))
}
fn line(ctx,word,part,base){
    text(word).with_style(ctx.component_style(part,base.min_width(px(0)).whitespace_nowrap().text_ellipsis()))
}
fn defaults(ctx){
    [#{id:"todo",label:ctx.t_or("project_card.todo","To do"),tone:"violet"},
        #{id:"progress",label:ctx.t_or("project_card.progress","In progress"),tone:"amber"},
        #{id:"review",label:ctx.t_or("project_card.review","In review"),tone:"blue"},
        #{id:"done",label:ctx.t_or("project_card.done","Done"),tone:"green"}]
}
fn data(p,ctx){
    if !ids::valid(p.project){throw "ProjectCard requires a stable project ID";}
    let members=#{};
    let roster=get(p,"members",[]);
    for member in roster{
        if !ids::valid(member.id)||member.id in members||member.name==""{throw "Project member IDs must be unique and names nonempty";}
        members[member.id]=member;
    }
    let tasks=#{};
    let n=0;
    for t in p.tasks{
        if !ids::valid(t.id)||t.id in tasks||t.label==""||utf8_byte_count(t.label)>600{throw "Project tasks need unique IDs and bounded labels";}
        if get(t,"assignee","")!=""&&!(t.assignee in members){throw "A project task assignee must belong to the roster";}
        tasks[t.id]=t;if get(t,"done",false){n+=1;}
    }
    let statuses=get(p,"statuses",defaults(ctx));let by_id=#{};
    for a in statuses{
        if !ids::valid(a.id)||a.id in by_id||a.label==""{throw "Project status options must have distinct IDs and labels";}
        by_id[a.id]=a;
    }
    let selected=get(p,"status",if statuses.len>0{statuses[0].id}else{""});
    if selected!=""&&!(selected in by_id){throw "The accepted project status must be in the choices";}
    #{tasks:tasks,members:members,roster:roster,done:n,statuses:statuses,status:get(by_id,selected,())}
}
fn person(s,d){
    if s.member in d.members{return s.member;}
    if s.pinned in d.members{return s.pinned;}
    ""
}
fn hot(s,d){
    for k in [s.ring,s.row,s.focus]{if k in d.tasks{return k;}}
    ""
}
fn mark_ring(cfg,ctx,e){
    let s=local(ctx,cfg.ticket.project,cfg.initial);let hit=get(e,"canvas_key","");
    if s.ring!=hit{s.ring=hit;commit(ctx,s);}
}
fn clear_ring(cfg,ctx,inner){
    if !inner{let s=local(ctx,cfg.ticket.project,cfg.initial);s.ring="";s.press=();commit(ctx,s);}
}
fn down_ring(cfg,ctx,e){
    if get(e,"button","")!="left"{return ();}
    let s=local(ctx,cfg.ticket.project,cfg.initial);let id=get(e,"canvas_key","");
    s.press=if id==""{()}else{#{id:id,revision:cfg.ticket.revision}};commit(ctx,s);handled()
}
fn up_ring(cfg,known,ctx,e){
    if get(e,"button","")!="left"{return ();}
    let s=local(ctx,cfg.ticket.project,cfg.initial);let press=s.press;s.press=();commit(ctx,s);
    let id=get(e,"canvas_key","");
    if press!=()&&press.id==id&&press.revision==cfg.ticket.revision&&id in known{
        let t=cfg.ticket;t.id=id;t.done=!known[id];ctx.emit("task_change",t);
    }
    handled()
}
fn ring(p,ctx,d,cfg,who,active){
    let all=[];let states=#{};let total=p.tasks.len;
    let tau=6.283185307179586;let n=max(1,total).to_float();
    let slice=1.0/n;let gap=if total>1{min(0.055,slice*0.35)}else{0.0};
    for i in 0..total{
        let t=p.tasks[i];let a=tau*(i.to_float()*slice+gap/2.0)-tau/4.0;let z=a+tau*(slice-gap);
        let steps=max(4,24/max(1,total));let points=[];
        for j in 0..steps+1{
            let angle=a+(z-a)*j.to_float()/steps.to_float();
            let x=16.0+12.5*cos(angle);let y=16.0+12.5*sin(angle);
            points.push(if j==0{path_move(x,y)}else{path_line(x,y)});
        }
        let done=get(t,"done",false);states[t.id]=done;
        let c=if done{theme_color("project_card.accent")}else if active==t.id{theme_color("project_card.ring_hot")}else{theme_color("project_card.ring")};
        if who!=""&&get(t,"assignee","")!=who{c=alpha(c,0.30);}
        all.push(canvas_stroke_path(t.id,points,if active==t.id{4.25}else{3.0},c).stroke_cap("round"));
    }
    let chart=canvas(canvas_scene(all).view_box(0,0,32,32)).with_key("segments").with_ref(element_ref("ring"))
        .accessibility_role("presentation").tab_stop(false)
        .with_style(style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)));
    if cfg.open{
        chart=chart.on("pointer_move",Fn("mark_ring").curry(cfg)).on_hover_change(Fn("clear_ring").curry(cfg));
        if !p.busy&&get(p,"on_task_change",())!=(){chart=chart.on("pointer_down",Fn("down_ring").curry(cfg)).on("pointer_up",Fn("up_ring").curry(cfg,states));}
    }else{
        chart=chart.on_click(Fn("disclose").curry(cfg,true)).tab_stop(false);
    }
    let count=line(ctx,ctx.format_number(d.done),"number",style().text_center().typography("caption").font_feature("tnum",1))
        .with_key("done").accessibility_hidden(true);
    let overlay=row([count]).accessibility_hidden(true).with_style(style().absolute().left(px(0)).top(px(0))
        .width(relative(1.0)).height(relative(1.0)).items_center().justify_center());
    let label=ctx.t_or("project_card.progress_label","Task progress");
    let summary=ctx.t_or("project_card.task_count","{done} of {total} done",#{done:ctx.format_number(d.done),total:ctx.format_number(total)});
    stack([overlay,chart]).with_key("progress").accessibility_role("image").accessibility_label(label).accessibility_description(summary)
        .with_style(ctx.component_style("ring",style().width(u(8)).height(u(8))
        .flex_shrink(false)))
}
fn badge(ctx,a,key,live,picker_open,selected){
    let candidate=get(a,"tone","gray");
    let tone=if candidate in ["red","amber","green","blue","violet"]{candidate}else{"gray"};
    let role="project_card."+tone;
    let tint=theme_color(role);
    let wash=theme_color(role+"_wash");
    let active=theme_color(role+"_active");
    let clean=ctx.theme_token("design.name")=="clean";
    let status=key=="status";
    let attrs=if status{if picker_open{["status","status_live","status_open"]}else if live{["status","status_live"]}else{["status"]}}
        else if selected{["option","option_selected"]}else{["option"]};
    let base=style().min_width(px(0)).items_center().gap(sp(1.25)).padding_x(sp(1.5))
        .min_height(u(if status{5}else{6})).radius(theme_radius("sm")).text_color(tint)
        .focus_ring(px(2),offset_px(-1),theme_color("focus_ring"));
    if status{
        if clean{
            base=base.background(wash);
        }else{
            base=base.background(if picker_open{active}else{wash});
            if live&&!picker_open{base=base.hover(style().background(active));}
        }
    }else if clean{
        let strong=theme_color("project_card.option_ink");
        base=base.text_color(if selected{strong}else{theme_color("project_card.option_text")});
        if selected{base=base.background(theme_color("project_card.option_fill"));}
        else{base=base.hover(style().background(theme_color("project_card.option_hover")).text_color(strong));}
    }else{
        base=base.background(if selected{active}else{wash}).hover(style().background(active));
    }
    let glyph=get(a,"icon",());
    if glyph==(){
        glyph=if !status&&clean{box([]).with_style(style().width(px(6)).height(px(6)).radius(px(1.5))
            .background(tint).flex_shrink(false))}else{tone_icon(tone,clean)};
    }else if !status&&clean{glyph=glyph.with_style(style().text_color(tint));}
    let caption=style().typography("caption").font_weight(500);
    let words=if status{line(ctx,a.label,"status_text",caption)}
        else{text(a.label).with_style(caption.min_width(px(0)).whitespace_nowrap().text_ellipsis())};
    let mark=glyph.with_key("mark").accessibility_hidden(true).with_style(ctx.component_style("status_icon",style()));
    row([mark,words.with_key("label")]).with_key(key)
        .with_style(ctx.component_style(attrs,base))
        .paint_transition(if status&&!clean{200}else{160},cubic_bezier(0.25,0.1,0.25,1.0))
}
fn portrait(ctx,m,key,part,dim){
    let props=#{key:key,name:m.name,size:"xs",diameter:u(if part=="assignee"{4.5}else{5.5})};
    if get(m,"handle",())!=(){props.handle=m.handle;}
    avatar::Avatar(props).accessibility_hidden(true)
        .with_style(ctx.component_style(part,style().opacity(if dim{if part=="assignee"{0.0}else{0.5}}else{1.0})))
}
fn top(ctx,p,d,s,cfg,who,active){
    let can=get(p,"on_status_change",())!=()&&!p.busy;
    let title=line(ctx,p.title,"title",style().text_color(theme_color("text_primary")).typography("body").font_weight(600))
        .with_key("toggle").with_ref(element_ref("disclosure")).accessibility_role("button")
        .accessibility_label(p.title).accessibility_expanded(cfg.open).tab_stop(true)
        .with_style(style().focus_ring(px(2),offset_px(0),theme_color("focus_ring")))
        .on_click(Fn("disclose").curry(cfg,!cfg.open))
        .on_key_value("plain+enter",Fn("disclose").curry(cfg,!cfg.open),())
        .on_key_value("plain+space",Fn("disclose").curry(cfg,!cfg.open),());
    let sub=[];
    if d.status!=(){
        let state=badge(ctx,d.status,"status",cfg.open&&can,s.picker,false).with_ref(element_ref("status"));
        if cfg.open&&can{
            state=state.accessibility_role("button").accessibility_label(d.status.label).accessibility_expanded(s.picker).tab_stop(true)
                .on_click(Fn("picker").curry(cfg)).on_key_value("plain+enter",Fn("picker").curry(cfg),()).on_key_value("plain+space",Fn("picker").curry(cfg),());
        }
        sub.push(state);
    }
    if get(p,"due","")!=""{sub.push(line(ctx,p.due,["caption","due"],style().typography("caption").text_color(theme_color("text_muted"))).with_key("due"));}
    let people=[];
    for i in 0..min(4,d.roster.len){
        let m=d.roster[i];let same=who==m.id;
        let node=row([portrait(ctx,m,"head:"+m.id,"face",who!=""&&!same)]).with_key("member:"+m.id)
            .with_style(ctx.component_style(if same{["people","face_active"]}else{["people"]},style().flex_shrink(false)));
        if cfg.open{
            node=node.accessibility_role("button").accessibility_label(m.name).accessibility_pressed(s.pinned==m.id).tab_stop(true)
                .on_hover_change(Fn("look").curry(cfg,"member",m.id))
                .on_click(Fn("select_member").curry(cfg,m.id)).on_key_value("plain+enter",Fn("select_member").curry(cfg,m.id),())
                .on_key_value("plain+space",Fn("select_member").curry(cfg,m.id),());
        }
        people.push(node);
    }
    if d.roster.len>4{people.push(line(ctx,"+"+ctx.format_number(d.roster.len-4),"more",style().typography("caption")));}
    row([ring(p,ctx,d,cfg,who,active),
        column([title,row(sub).with_style(ctx.component_style("sub",style().min_width(px(0)).items_center().gap(sp(1.5))))])
            .with_style(ctx.component_style("titles",style().min_width(px(0)).flex_grow())),
        row(people).with_key("roster").with_style(style().flex_shrink(false).items_center()),
    ]).with_key("head").with_style(ctx.component_style("head",style().min_width(px(0)).items_center().gap(sp(2.5)).padding(sp(3))))
}
fn content(ctx,p,d,s,cfg,who,active){
    let nodes=[];
    if s.picker&&get(p,"on_status_change",())!=()&&!p.busy{
        let opts=[];
        for a in d.statuses{
            let n=badge(ctx,a,"option:"+a.id,false,false,d.status!=()&&a.id==d.status.id)
                .accessibility_role("radio").accessibility_label(a.label).accessibility_checked(d.status!=()&&a.id==d.status.id).tab_stop(true)
                .on_click(Fn("emit_status").curry(cfg,a.id)).on_key_value("plain+enter",Fn("emit_status").curry(cfg,a.id),())
                .on_key_value("plain+space",Fn("emit_status").curry(cfg,a.id),());
            opts.push(n);
        }
        nodes.push(row(opts).with_key("options").accessibility_role("radiogroup").accessibility_label(ctx.t_or("project_card.picker","Project status"))
            .with_style(ctx.component_style("picker",style().min_width(px(0)).flex_wrap().gap(sp(1.5)))));
    }
    let tasks=[];
    for t in p.tasks{
        let done=get(t,"done",false);let member=get(d.members,get(t,"assignee",""),());
        let tick=if done{[task_tick()]}else{[]};
        let marks=if done{["check","check_done"]}else{["check"]};
        let parts=["task"];if active==t.id{parts.push("task_hot");}if done{parts.push("task_done");}
        let children=[row(tick).with_key("check").accessibility_hidden(true)
            .with_style(ctx.component_style(marks,style().items_center().justify_center().width(u(4)).height(u(4))
                .radius(theme_radius("pill")).border(px(1)).border_color(theme_color("project_card.ring")).flex_shrink(false))),
            line(ctx,t.label,"label",style().typography("body").flex_grow())];
        if member!=(){children.push(portrait(ctx,member,"task:"+t.id,"assignee",active!=t.id));}
        let task=row(children).with_key("task:"+t.id).accessibility_role("checkbox").accessibility_label(t.label)
            .accessibility_description(if member==(){""}else{member.name}).accessibility_checked(done)
            .tab_stop(true).on_hover_change(Fn("look").curry(cfg,"row",t.id))
            .on_focus_within_change(Fn("look").curry(cfg,"focus",t.id))
            .with_style(ctx.component_style(parts,style().min_width(px(0)).min_height(u(8)).gap(sp(2.5)).padding_x(sp(1.5))
                .items_center().radius(theme_radius("sm")).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))
                .opacity(if who==""||get(t,"assignee","")==who{1.0}else{0.35}))).paint_transition(160,"ease_out");
        if !p.busy&&get(p,"on_task_change",())!=(){
            let req=cfg.ticket;req.id=t.id;req.done=!done;
            task=task.on_click(Fn("emit_task").curry(req)).on_key_value("plain+space",Fn("emit_task").curry(req),())
                .on_key_value("plain+enter",Fn("emit_task").curry(req),());
        }
        tasks.push(task);
    }
    if tasks.len==0{tasks.push(line(ctx,ctx.t_or("project_card.empty","No tasks"),"empty",style().typography("caption")));}
    nodes.push(column(tasks).with_key("tasks").accessibility_role("list")
        .with_style(ctx.component_style("tasks",style().min_width(px(0)).max_height(u(88))
            .overflow_y_scroll().relative().border_top(px(1)).border_color(theme_color("border")))).scrollbars("hidden","auto"));
    nodes
}
fn foot(ctx,p,d,who,active){
    let custom=get(p,"hint",());let message="";
    if active!=""{
        let t=d.tasks[active];let m=get(d.members,get(t,"assignee",""),());
        if m==(){message=if get(t,"done",false){ctx.t_or("project_card.done","Done")}else{ctx.t_or("project_card.unassigned","Nobody assigned")};}
        else{
            message=if get(t,"done",false){ctx.t_or("project_card.done_by","Done by {name}",#{name:m.name})}
                else{ctx.t_or("project_card.assigned","Assigned to {name}",#{name:m.name})};
        }
    }else if who!=""{
        let n=0;let total=0;
        for t in p.tasks{if get(t,"assignee","")==who{total+=1;if get(t,"done",false){n+=1;}}}
        message=ctx.t_or("project_card.member_count","{member} · {done} of {total} done",#{member:d.members[who].name,done:ctx.format_number(n),total:ctx.format_number(total)});
    }else if custom==(){
        message=if d.done==p.tasks.len{ctx.t_or("project_card.all","All done")}
            else{ctx.t_or("project_card.left","{n} left",#{n:ctx.format_number(p.tasks.len-d.done)})};
    }
    let children=[];
    if message!=""{children.push(line(ctx,message,"caption",style().flex_grow().typography("caption").text_color(theme_color("text_muted")))
        .with_key("hint").accessibility_role("status"));}
    else if custom!=(){children.push(custom);}
    if get(p,"action_label","")!=""&&get(p,"on_action",())!=(){
        children.push(btn::Button(#{key:"action",text:p.action_label,size:"sm",variant:"primary",disabled:p.busy,on_click:Fn("fire").curry(project_ticket(p))})
            .with_style(ctx.component_style("action",style().max_width(relative(0.60)))));
    }
    row(children).with_key("footer").with_style(ctx.component_style("foot",style().min_width(px(0)).min_height(u(10))
        .padding_x(sp(2.5)).gap(sp(2)).items_center().justify_between()))
}
fn render_project(ctx,p){
    let d=data(p,ctx);let s=local(ctx,p.project,p.default_open);let cfg=cfg_for(ctx,p);
    if s.hover!=()&&s.hover!=cfg.open{timeout("hover-disclosure",if s.hover{90}else{220},false,Fn("due_hover"),#{cfg:cfg,open:s.hover});}
    let who=person(s,d);let active=hot(s,d);
    let body=column(if cfg.open{content(ctx,p,d,s,cfg,who,active)}else{[]}).with_key("body")
        .layout_size_motion(if cfg.open{if s.picker{"choose"}else{"open"}}else{"closed"},"height",480,ease())
        .with_style(ctx.component_style("body",style().overflow_hidden().min_width(px(0))));
    let card=column([top(ctx,p,d,s,cfg,who,active),body]).with_key("card").with_style(ctx.component_style("card",style()
        .min_width(px(0)).width(relative(1.0)).background(theme_color("project_card.card")).radius(theme_radius("lg"))));
    let nodes=[card];if cfg.open{nodes.push(foot(ctx,p,d,who,active));}
    let paper=column(nodes).with_key("paper").with_style(ctx.component_style("paper",style().min_width(px(0))));
    let root=column([paper]).with_key(p.key).accessibility_role("group").accessibility_label(p.title)
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(320))))
        .on_bubble("key:plain+escape",Fn("escape").curry(cfg));
    if p.trigger=="hover"{root=root.on_hover_change(Fn("schedule").curry(cfg));}
    root
}
