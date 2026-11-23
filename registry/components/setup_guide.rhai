/* gpui-rhai
{"id":"components/setup_guide","export":"SetupGuide","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/accordion","components/button","components/icon_button","components/progress_tracker"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","surface_raised","surface_hover","text_primary","text_muted","radius.sm","radius.lg","typography.control","typography.caption","radius.md","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// An onboarding panel with a bounded local disclosure model and Host-owned data.
// Example: SetupGuide(#{key:"setup",groups:[#{id:"first",title:"First steps",tasks:[#{id:"invite",label:"Invite teammates"}]}]})
import "components/accordion" as accordion;
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/progress_tracker" as checklist;

define_component(#{
    metadata:#{id:"components/setup_guide","export":"SetupGuide",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/accordion","components/button","components/icon_button","components/progress_tracker"],
        capabilities:#{},assets:[],
        tokens:["accent","border","focus_ring","surface_raised","surface_hover","text_primary","text_muted","radius.sm","radius.lg","typography.control","typography.caption","radius.md","space.spacing_unit","space.unit"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of this panel."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Panel title."},
        groups:#{schema:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique nonempty group ID."},
            title:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Group heading."},
            tasks:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Task ID, unique across the panel."},
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Task label and accessible name."},
                done:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Initial local state for a newly admitted task."},
                disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Prevents manual completion, for example while the app owns the task."},
            }}},required:true,sensitive:false,doc:"Sequential tasks. The panel supports up to 128 tasks across its groups."},
        }}},required:true,sensitive:false,doc:"Groups in display order."},
        value:#{schema:#{type:"optional",value:#{type:"array",max_items:128,items:#{type:"string"}}},required:false,sensitive:false,
            doc:"Controlled completed IDs; local completion is used when omitted."},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disables actions and task changes."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Complete list of proposed completed task IDs."},
        on_edit:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Shows an Edit action; the Host decides what it opens."},
        on_close:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Shows Close; the Host decides whether to remove the panel."},
    },state:#{fields:#{
        local:#{schema:#{type:"array",max_items:128,items:#{type:"string"}},"default":#{type:"array",value:[]}},
        seen:#{schema:#{type:"array",max_items:128,items:#{type:"string"}},"default":#{type:"array",value:[]}},
        model:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        open:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        minimized:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        focused_group:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        pending:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        advance:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        change:#{payload:#{type:"array",max_items:128,items:#{type:"string"}},doc:"Completed task proposal in definition order. A rejected proposal never advances disclosure."},
        edit:#{payload:#{type:"null"},doc:"Edit requested."},
        close:#{payload:#{type:"null"},doc:"Close requested."},
    },slots:#{},parts:["root","header","title","actions","edit","icon_button","icon","progress","count","digit",
        "track","fill","body","group","group_open","group_header","group_title","group_done","chevron","tasks","task","task_current",
        "task_locked","mark","mark_done","mark_locked","locked_icon","check","label","label_done","label_locked","strike"],effects:["accepted"]},
    render:Fn("render_SetupGuide")
});

fn SetupGuide(props){render_component("components/setup_guide",props)}
fn header_ref(index){`setup-group-${index}`}
fn group_index(model,id){for i in 0..model.groups.len{if model.groups[i].id==id{return i;}}-1}
fn unfinished(group,done){for id in group.ids{if !(id in done){return id;}}""}
fn first_group(groups,done){for group in groups{if unfinished(group,done)!=""{return group.id;}}""}
fn make_model(ctx,props){
    let groups=[];let items=[];let known=#{};
    for group in props.groups{
        if group.id==""||group.id in known{throw "SetupGuide group IDs must be unique and nonempty";}
        known[group.id]=true;
        let ids=[];for item in group.tasks{ids.push(item.id);items.push(item);}
        groups.push(#{id:group.id,ids:ids});
    }
    if items.len>128{throw "SetupGuide supports up to 128 tasks";}
    let model=checklist::project_steps(items);
    let value=checklist::opt(props,"value");
    model.done=checklist::done_ids(model,value,ctx.get_state("seen"),ctx.get_state("local"));
    model.controlled=value!=();
    model.editable=!props.disabled&&(!model.controlled||checklist::has_callback(props,"on_change"));
    model.groups=groups;model.disabled=props.disabled;model.minimized=ctx.get_state("minimized");
    let open=ctx.get_state("open");
    model.open=if open==()||(open!=""&&!(open in known)){first_group(groups,model.done)}else{open};
    model
}
fn accept(ctx,data){
    let model=data[0];let pending=data[1];let previous=ctx.get_state("model");
    ctx.set_state("model",model);ctx.set_state("seen",model.ids);ctx.set_state("local",model.done);
    ctx.set_state("open",model.open);
    let ticket=ctx.get_state("advance");
    if ticket!=()&&(ticket.from!=model.open||ticket.done!=model.done||model.minimized||model.disabled){
        ctx.set_state("advance",());
    }
    if pending!=(){
        let index=group_index(model,pending.group);
        if previous!=()&&pending.want==model.done&&pending.group==model.open&&index>=0
            &&unfinished(model.groups[index],model.done)==""&&!model.minimized&&!model.disabled{
            ctx.set_state("advance",#{from:pending.group,done:model.done});
        }
        ctx.set_state("pending",());
    }
}
fn cleanup(ctx,data){}
fn advance(ctx,ticket){
    let model=ctx.get_state("model");
    if model==()||model.disabled||model.minimized||ticket.from!=model.open||ticket.done!=model.done{return;}
    let current=ctx.get_state("advance");
    if current==()||current.from!=ticket.from||current.done!=ticket.done{return;}
    let next=first_group(model.groups,model.done);
    ctx.set_state("open",next);ctx.set_state("advance",());
    if ctx.get_state("focused_group")==ticket.from{
        let i=group_index(model,if next!=""{next}else{ticket.from});
        if i>=0{ctx.focus(header_ref(i));}
    }
}
fn toggle_group(ctx,id){
    let model=ctx.get_state("model");
    if model!=()&&!model.disabled&&!model.minimized&&group_index(model,id)>=0{
        ctx.set_state("advance",());ctx.set_state("open",if model.open==id{""}else{id});
    }
    handled()
}
fn group_focus(ctx,p){
    if p.focused{ctx.set_state("focused_group",p.value);}
    else if ctx.get_state("focused_group")==p.value{ctx.set_state("focused_group","");}
}
fn toggle_task(group,id,ctx,payload){
    let model=ctx.get_state("model");
    if model==()||!model.editable||model.minimized||model.open!=group||id in model.locked{return handled();}
    let index=group_index(model,group);
    if index<0||!(id in model.groups[index].ids){return handled();}
    if !(id in model.done)&&unfinished(model.groups[index],model.done)!=id{return handled();}
    let next=checklist::toggle_ids(model.ids,model.done,id);
    ctx.set_state("advance",());ctx.set_state("pending",#{group:group,want:next});
    if !model.controlled{ctx.set_state("local",next);}
    ctx.emit("change",next);
    handled()
}
fn minimize(ctx,payload){
    let model=ctx.get_state("model");
    if model!=()&&!model.disabled{
        ctx.set_state("minimized",!ctx.get_state("minimized"));ctx.set_state("advance",());
    }
    handled()
}
fn edit(ctx,payload){let model=ctx.get_state("model");if model!=()&&!model.disabled{ctx.emit("edit",());}handled()}
fn close(ctx,payload){let model=ctx.get_state("model");if model!=()&&!model.disabled{ctx.emit("close",());}handled()}
fn glyph(ctx,path){
    svg(`<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("icon",style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4))))
}
fn toolbar(ctx,props,model){
    let actions=[];
    if checklist::has_callback(props,"on_edit"){actions.push(button::Button(#{key:"edit",text:ctx.t_or("checklist.edit","Edit"),variant:"ghost",size:"sm",
        disabled:model.disabled,on_click:Fn("edit")}).with_style(ctx.component_style("edit",style())));}
    let minimize_label=if model.minimized{ctx.t_or("checklist.expand","Expand setup guide")}else{ctx.t_or("checklist.minimize","Minimize setup guide")};
    let icon_style=ctx.component_style("icon_button",style().width(theme_length("space.unit").times(6.5)).height(theme_length("space.unit").times(6.5)).min_height(theme_length("space.unit").times(6.5)));
    actions.push(icon_button::IconButton(#{key:"minimize",size:"sm",label:minimize_label,
        icon:glyph(ctx,if model.minimized{"M3 8h10M8 3v10"}else{"M3 8h10"}),on_click:Fn("minimize"),disabled:model.disabled})
        .with_style(icon_style).accessibility_expanded(!model.minimized));
    if checklist::has_callback(props,"on_close"){
        actions.push(icon_button::IconButton(#{key:"close",size:"sm",label:ctx.t_or("checklist.close","Close setup guide"),
            icon:glyph(ctx,"m4 4 8 8m0-8-8 8"),on_click:Fn("close"),disabled:model.disabled}).with_style(icon_style));
    }
    row(actions).with_key("actions").with_style(ctx.component_style("actions",style().gap(theme_length("space.spacing_unit")).items_center().flex_shrink(false)))
}
fn locked_mark(ctx,clean){
    let stroke=if clean{"stroke-width='1.1' stroke-dasharray='2 2'"}else{"stroke-width='1.5'"};
    let shape=if clean{"<circle cx='8' cy='8' r='6.5'/>"}else{"<circle cx='8' cy='8' r='6'/><path d='m4 12 8-8'/>"};
    svg(`<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' ${stroke}>${shape}</svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("locked_icon",style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4))
            .text_color(theme_color("text_muted"))))
}
fn task_row(ctx,model,group,item,first,shown,clean){
    let done=item.id in model.done;
    let waiting=!done&&item.id!=first;
    let disabled=model.disabled||!model.editable||waiting||item.id in model.locked||!shown;
    let part=if waiting{["task","task_locked"]}else if !done{["task","task_current"]}else{["task"]};
    let mark=if waiting{locked_mark(ctx,clean)}else{checklist::check_mark(ctx,done)};
    let mark_parts=if waiting{["mark","mark_locked"]}else if done{["mark","mark_done"]}else{["mark"]};
    let icon=row([mark]).with_key("mark").accessibility_hidden(true)
        .with_style(ctx.component_style(mark_parts,style().width(theme_length("space.unit").times(4.5)).height(theme_length("space.unit").times(4.5))
            .items_center().justify_center().flex_shrink(false).radius(theme_length("space.unit").times(249.75)).border(px(if waiting{0}else{1}))
            .border_color(theme_color("border"))));
    let label=checklist::struck_label(ctx,item.label,done);
    if waiting{label=label.with_style(ctx.component_style("label_locked",style().text_color(theme_color("text_muted"))));}
    let result=row([icon,label]).with_key(item.id)
        .with_style(ctx.component_style(part,style().width(relative(1.0)).min_width(px(0)).min_height(theme_length("space.unit").times(8.5))
            .gap(theme_length("space.spacing_unit").times(2)).items_center().padding_x(theme_length("space.spacing_unit").times(2.5)).radius(theme_radius("sm"))
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .accessibility_role("checkbox").accessibility_label(item.label).accessibility_checked(done)
        .disabled(disabled).tab_stop(!disabled);
    if !done&&!waiting{result=result.accessibility_current("step");}
    if !disabled{result=result.on_click_value(Fn("toggle_task").curry(group,item.id),());}
    result
}
fn group_view(ctx,model,group,index,clean){
    let ids=model.groups[index];
    let open=model.open==group.id;let shown=open&&!model.minimized;
    let first=unfinished(ids,model.done);
    let done=ids.ids.len>0&&first=="";
    let arrow=glyph(ctx,"m4.5 6.25 3.5 3.5 3.5-3.5").with_key("chevron")
        .with_style(ctx.component_style("chevron",style().flex_shrink(false).text_color(theme_color("text_muted"))))
        .motion(motion_transition("rotate",if open{180.0}else{0.0},if open{180.0}else{0.0},
            #{duration_ms:260,intent:"feedback"}));
    let title=checklist::struck_label(ctx,group.title,done).with_key("group-title")
        .with_style(ctx.component_style(if done{["group_title","group_done"]}else{["group_title"]},
            style().min_width(px(0)).typography("control").font_weight(600)));
    let header=row([title,arrow]).with_key("header").with_ref(element_ref(header_ref(index)))
        .with_style(ctx.component_style("group_header",style().gap(theme_length("space.spacing_unit").times(2)).justify_between().items_center()
            .width(relative(1.0)).min_width(px(0)).min_height(theme_length("space.unit").times(9)).padding_x(theme_length("space.spacing_unit").times(2.5))
            .radius(theme_radius("md")).hover(style().background(theme_color("surface_hover")))
            .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .accessibility_role("button").accessibility_label(group.title).accessibility_expanded(shown)
        .disabled(model.disabled||model.minimized).tab_stop(!model.disabled&&!model.minimized);
    if done{header=header.accessibility_description(ctx.t_or("checklist.completed","Completed"));}
    if !model.disabled&&!model.minimized{header=header.on_click_value(Fn("toggle_group"),group.id);}
    let rows=[];for item in group.tasks{rows.push(task_row(ctx,model,group.id,item,first,shown,clean));}
    let pane=column(rows).with_key("tasks").accessibility_hidden(!shown)
        .with_style(ctx.component_style("tasks",style().min_width(px(0)).padding_x(theme_length("space.spacing_unit").times(1.5)).padding_bottom(theme_length("space.spacing_unit").times(1.5))));
    column([header,accordion::disclosure_panel(ctx,`setup-tasks-${index}`,open,(),pane)])
        .with_key(group.id).tab_stop(false)
        .with_style(ctx.component_style(if open{["group","group_open"]}else{["group"]},
            style().min_width(px(0)).radius(theme_radius("md"))))
        // Nested task text follows the checkmark, not the disclosure title.
        .audit_allow(["text-edge-misaligned"])
        .on_focus_within_value(Fn("group_focus"),group.id)
}
fn render_SetupGuide(ctx,props){
    let model=make_model(ctx,props);
    effect("accepted",[model,ctx.get_state("pending")],Fn("accept"),Fn("cleanup"));
    let ticket=ctx.get_state("advance");
    if ticket!=()&&!model.disabled&&!model.minimized&&ticket.from==model.open&&ticket.done==model.done{
        timeout("advance",if ctx.motion_preference()=="normal"{420}else{1},false,Fn("advance"),ticket);
    }
    let title=checklist::opt(props,"title");if title==(){title=ctx.t_or("checklist.setup","Setup guide");}
    let heading=row([text(title).with_key("title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("title",style().min_width(px(0)).typography("control").font_weight(600))),toolbar(ctx,props,model)])
        .with_key("header").with_style(ctx.component_style("header",style().min_width(px(0)).items_center().justify_between().gap(theme_length("space.spacing_unit").times(2))));
    let meter=checklist::completion_meter(ctx,title,model.done.len,model.ids.len,700,false);
    let theme=ctx.theme_variant();let clean=theme!=()&&theme.family=="Clean";
    let meters=[meter.with_style(style().flex_grow().flex_basis(px(0)).min_width(px(0)))];
    if clean{meters.push(checklist::counter(ctx,model.done.len,model.ids.len));}
    let progress=row(meters).with_key("progress").with_style(ctx.component_style("progress",style().width(relative(1.0)).min_width(px(0))
        .items_center().gap(theme_length("space.spacing_unit").times(2))));
    let groups=[];for i in 0..props.groups.len{groups.push(group_view(ctx,model,props.groups[i],i,clean));}
    let body=column(groups).with_key("groups").accessibility_hidden(model.minimized)
        .with_style(ctx.component_style("body",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(0.5))));
    column([heading,progress,accordion::disclosure_panel(ctx,"setup-groups",!model.minimized,(),body)])
        .with_key(props.key).disabled(model.disabled).accessibility_role("region").accessibility_label(title)
        // The card heading and group controls have independent insets.
        .audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(320)).min_width(px(0))
            .radius(theme_radius("lg")).padding(theme_length("space.spacing_unit").times(2.5)).gap(theme_length("space.spacing_unit").times(2)).background(theme_color("surface_raised")).text_color(theme_color("text_primary"))))
}
