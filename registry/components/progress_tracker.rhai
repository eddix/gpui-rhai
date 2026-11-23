/* gpui-rhai
{"id":"components/progress_tracker","export":"ProgressTracker","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","surface_raised","surface_hover","text_primary","text_muted","on_accent","radius.md","radius.lg","typography.control","typography.caption","space.spacing_unit","space.unit","tracker.sheen"],"environment":["density"]}
*/
// A keyed checklist with progress. SetupGuide uses the same projection and meter.
// Example: ProgressTracker(#{key:"launch",steps:[#{id:"tests",label:"Run tests"}]})

define_component(#{
    metadata:#{id:"components/progress_tracker","export":"ProgressTracker",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{},assets:[],
        tokens:["accent","border","focus_ring","surface_raised","surface_hover","text_primary","text_muted","on_accent","radius.md","radius.lg","typography.control","typography.caption","space.spacing_unit","space.unit","tracker.sheen"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of the checklist."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Visible heading and meter name."},
        steps:#{schema:#{type:"array",max_items:128,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique nonempty ID; completion follows it across reordering."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible text and accessible checkbox name."},
            done:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Initial local state for a newly admitted ID."},
            disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Counts toward the total but cannot be changed here."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative trailing node."},
        }}},required:true,sensitive:false,doc:"The ordered, bounded set of steps."},
        value:#{schema:#{type:"optional",value:#{type:"array",max_items:128,items:#{type:"string"}}},required:false,sensitive:false,
            doc:"Controlled completed IDs. Local state is used only when omitted."},
        autoplay:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,
            doc:"Opt-in active-view demo playback; user focus or pointer input stops it."},
        interval_ms:#{schema:#{type:"integer",min:100,max:60000},"default":#{type:"integer",value:2200},
            required:false,sensitive:false,doc:"Playback interval; the reset holds for twice this duration."},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Blocks changes and autoplay."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Proposes the completed IDs in display order."},
    },state:#{fields:#{
        local:#{schema:#{type:"array",max_items:128,items:#{type:"string"}},"default":#{type:"array",value:[]}},
        seen:#{schema:#{type:"array",max_items:128,items:#{type:"string"}},"default":#{type:"array",value:[]}},
        model:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        stopped:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        awaiting:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{change:#{payload:#{type:"array",max_items:128,items:#{type:"string"}},doc:"A full completion proposal; no optimistic change in controlled mode."}},
        slots:#{},parts:["root","header","title","count","digit","track","fill","shine","list","row","label","label_done",
            "strike","mark","mark_done","check","icon","icon_done"],effects:["accepted"]},
    render:Fn("render_ProgressTracker")
});

fn ProgressTracker(props){render_component("components/progress_tracker",props)}
fn opt(map,key){if key in map{map[key]}else{()}}
fn has_callback(props,name){opt(props,name)!=()}
fn project_steps(items){
    let ids=[];let defaults=[];let locked=[];let seen=#{};
    for item in items{
        if item.id==""||item.id in seen{throw "Checklist item IDs must be unique and nonempty";}
        seen[item.id]=true;ids.push(item.id);
        if opt(item,"done")==true{defaults.push(item.id);}
        if opt(item,"disabled")==true{locked.push(item.id);}
    }
    #{ids:ids,defaults:defaults,locked:locked}
}
fn done_ids(model,controlled,seen,local){
    let done=[];
    for id in model.ids{
        let selected=if controlled!=(){id in controlled}else if id in seen{id in local}else{id in model.defaults};
        if selected{done.push(id);}
    }
    done
}
fn toggle_ids(ids,done,id){
    let next=[];
    for entry in ids{if (entry==id)!=(entry in done){next.push(entry);}}
    next
}
fn count_text(ctx,count,total){ctx.t_or("checklist.of","{count} of {total} done",#{count:ctx.format_number(count),total:ctx.format_number(total)})}
fn counter(ctx,count,total){
    let digit=text(ctx.format_number(count)).with_key("number")
        .with_style(ctx.component_style("digit",style().font_feature("tnum",1)))
        .motion(motion_transition("translate_y",7.0,0.0,#{duration_ms:560,easing:cubic_bezier(0.22,1.0,0.36,1.0),intent:"feedback"}))
        .motion(motion_transition("opacity",0.35,1.0,#{duration_ms:560,intent:"feedback"}))
        .motion_on_change(`${count}`);
    row([digit,text("/"+ctx.format_number(total))]).with_key("count").accessibility_hidden(true)
        .with_style(ctx.component_style("count",style().items_baseline().clip().flex_shrink(false)
            .typography("caption").text_color(theme_color("text_muted"))))
}
fn completion_meter(ctx,title,count,total,duration,shine){
    let ratio=if total==0{0.0}else{count.to_float()/total.to_float()};
    let meter_ref=element_ref("completion-meter");
    let previous=ctx.element_size(meter_ref);
    let rtl=ctx.text_direction()=="rtl";
    let highlights=[];
    if shine&&ratio>0.0{
        highlights.push(row([]).with_key("shine").accessibility_hidden(true)
            .with_style(ctx.component_style("shine",style().width(relative(0.26)).height(relative(1.0))
                .background(theme_color("tracker.sheen"))))
            .motion(motion_transition("translate_x",if rtl{70.0}else{-70.0},if rtl{-360.0}else{360.0},
                #{duration_ms:1400,intent:"feedback"}))
            .motion(motion_transition("opacity",0.75,0.0,#{duration_ms:1400,intent:"feedback"}))
            .motion_on_change(`${count}/${total}`));
    }
    let fill=row(highlights).with_key("fill").accessibility_hidden(true)
        .with_style(ctx.component_style("fill",style().width(relative(ratio)).max_width(relative(1.0)).height(relative(1.0))
            .radius(theme_length("space.unit").times(249.75)).background(theme_color("accent")).clip()));
    if previous!=()&&ctx.motion_preference()=="normal"{
        let target=previous.width*ratio;
        fill=fill.motion(motion_transition("width",target,target,#{duration_ms:duration,
            easing:cubic_bezier(0.22,1.0,0.36,1.0),intent:"feedback"}));
    }
    row([fill]).with_key("meter").with_ref(meter_ref)
        .with_style(ctx.component_style("track",style().width(relative(1.0)).min_width(px(0)).height(px(6))
            .items_center().radius(theme_length("space.unit").times(249.75)).background(theme_color("surface_hover")).clip()))
        .accessibility_role("progressbar").accessibility_label(title)
        .accessibility_value(count).accessibility_value_min(0).accessibility_value_max(max(total,1))
        .accessibility_description(count_text(ctx,count,total))
}
fn check_mark(ctx,done){
    let path=[path_move(6.5,12.5),path_line(10,16),path_line(17.5,8)];
    canvas(canvas_scene([canvas_stroke_path("check",path,2.35,current_color())
        .stroke_cap("round").stroke_join("round")]).view_box(0,0,24,24))
        .with_key("check").accessibility_hidden(true)
        .with_style(ctx.component_style("check",style().width(px(14)).height(px(14))
            .text_color(theme_color("on_accent")).opacity(if done{1.0}else{0.0})))
        .motion(motion_transition("path_progress",0.0,if done{1.0}else{0.0},
            #{duration_ms:if done{420}else{1},delay_ms:if done{80}else{0},intent:"feedback"}))
        .motion_on_change(if done{"done"}else{"open"})
}
fn struck_label(ctx,label,done){
    let caption=text(label).with_key("caption").with_style(style().whitespace_nowrap().text_ellipsis().min_width(px(0)));
    let children=[caption];
    if done{
        children.push(row([]).with_key("stroke").accessibility_hidden(true)
            .with_style(ctx.component_style("strike",style().absolute().inset_start(px(0)).inset_end(px(0))
                .top(relative(0.56)).height(px(1)).background(current_color())))
            .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:380,intent:"feedback"})));
    }
    row(children).with_key("label").with_style(ctx.component_style(
        if done{["label","label_done"]}else{["label"]},
        style().min_width(px(0)).relative().typography("control")
            .text_color(theme_color(if done{"text_muted"}else{"text_primary"}))))
}
fn accept(ctx,data){
    let model=data[0];let awaiting=data[1];let before=ctx.get_state("model");
    if before!=()&&!before.autoplay&&model.autoplay{ctx.set_state("stopped",false);}
    ctx.set_state("model",model);ctx.set_state("seen",model.ids);ctx.set_state("local",model.done);
    if awaiting!=(){
        if model.done!=awaiting{ctx.set_state("stopped",true);}
        ctx.set_state("awaiting",());
    }
}
fn cleanup(ctx,data){}
fn propose(ctx,model,ids,automatic){
    if !model.editable||ids==model.done{return;}
    if automatic{ctx.set_state("awaiting",ids);}
    if !model.controlled{ctx.set_state("local",ids);}
    ctx.emit("change",ids);
}
fn stop(ctx,payload){if !ctx.get_state("stopped"){ctx.set_state("stopped",true);}}
fn focused(ctx,within){if within{stop(ctx,());}}
fn pressed(ctx,id){
    stop(ctx,());
    let model=ctx.get_state("model");
    if model!=()&&model.editable&&id in model.ids&&!(id in model.locked){
        propose(ctx,model,toggle_ids(model.ids,model.done,id),false);
    }
    handled()
}
fn tick(ctx,event){
    let model=ctx.get_state("model");
    if model==()||!model.editable||!model.autoplay||ctx.get_state("stopped")||event.before!=model.done{return;}
    if event.target==""{
        let reset=[];
        for id in model.ids{
            if (id in model.locked && id in model.done)||(!(id in model.locked)&&id in model.defaults){reset.push(id);}
        }
        propose(ctx,model,reset,true);
    }else if event.target in model.ids&&!(event.target in model.locked)&&!(event.target in model.done){
        propose(ctx,model,toggle_ids(model.ids,model.done,event.target),true);
    }
}
fn tracker_row(ctx,item,model,hide_icon){
    let done=item.id in model.done;let disabled=!model.editable||item.id in model.locked;
    let mark=row([check_mark(ctx,done)]).with_key("mark").accessibility_hidden(true)
        .with_style(ctx.component_style(if done{["mark","mark_done"]}else{["mark"]},
            style().width(theme_length("space.unit").times(5)).height(theme_length("space.unit").times(5)).flex_shrink(false).items_center().justify_center()
                .radius(theme_length("space.unit").times(249.75)).border(px(1)).border_color(theme_color("border")).background(theme_color(if done{"accent"}else{"surface_raised"}))));
    let content=[mark,struck_label(ctx,item.label,done),row([]).with_style(style().flex_grow()).accessibility_hidden(true)];
    let icon=opt(item,"icon");
    if icon!=(){content.push(row([icon]).with_key("icon").accessibility_hidden(true).retained_page(!hide_icon)
        .with_style(ctx.component_style(if done{["icon","icon_done"]}else{["icon"]},
            style().width(theme_length("space.unit").times(5)).height(theme_length("space.unit").times(5)).flex_shrink(false).items_center().justify_center())));}
    let result=row(content).with_key(item.id)
        .with_style(ctx.component_style("row",style().width(relative(1.0)).min_width(px(0)).items_center()
            .min_height(theme_length("space.unit").times(8.5)).gap(theme_length("space.spacing_unit").times(2)).padding_x(theme_length("space.spacing_unit").times(1.5)).radius(theme_radius("md"))
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .accessibility_role("checkbox").accessibility_label(item.label).accessibility_checked(done).disabled(disabled)
        .paint_transition(160,cubic_bezier(0.22,1.0,0.36,1.0));
    if !disabled{result=result.on_click_value(Fn("pressed"),item.id);}
    result
}
fn render_ProgressTracker(ctx,props){
    let model=project_steps(props.steps);
    let value=opt(props,"value");
    model.done=done_ids(model,value,ctx.get_state("seen"),ctx.get_state("local"));
    model.controlled=value!=();model.editable=!props.disabled&&(!model.controlled||has_callback(props,"on_change"));
    model.autoplay=props.autoplay;
    effect("accepted",[model,ctx.get_state("awaiting")],Fn("accept"),Fn("cleanup"));
    if model.autoplay&&model.editable&&!ctx.get_state("stopped"){
        let next="";let can_reset=false;
        for id in model.ids{
            if !(id in model.locked)&&id in model.done&&!(id in model.defaults){can_reset=true;}
            if next==""&&!(id in model.locked)&&!(id in model.done){next=id;}
        }
        if next!=""||can_reset{timeout("advance",props.interval_ms*(if next==""{2}else{1}),false,
            Fn("tick"),#{before:model.done,target:next});}
    }
    let title=opt(props,"title");if title==(){title=ctx.t_or("checklist.tracker","Getting started");}
    let reference=element_ref("tracker-root");
    let theme=ctx.theme_variant();
    let width=ctx.element_width_class(reference,[300]);
    let hide_icon=theme!=()&&theme.family=="Clean"&&width==0;
    let rows=[];for item in props.steps{rows.push(tracker_row(ctx,item,model,hide_icon));}
    let heading=row([text(title).with_key("title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("title",style().min_width(px(0)).typography("control").font_weight(500))),
        counter(ctx,model.done.len,model.ids.len)]).with_key("header").with_style(ctx.component_style("header",
        style().items_baseline().justify_between().gap(theme_length("space.spacing_unit").times(2.5)).min_width(px(0))));
    column([heading,completion_meter(ctx,title,model.done.len,model.ids.len,1100,true),
        column(rows).with_key("list").accessibility_role("list").accessibility_label(title)
            .with_style(ctx.component_style("list",style().gap(theme_length("space.spacing_unit").times(0.5)).min_width(px(0))))])
        .with_key(props.key).with_ref(reference)
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(360)).min_width(px(0))
            .padding(theme_length("space.spacing_unit").times(3)).gap(theme_length("space.spacing_unit").times(2)).radius(theme_radius("lg")).background(theme_color("surface_raised"))
            .text_color(theme_color("text_primary"))))
        .accessibility_role("group").accessibility_label(title).disabled(props.disabled).tab_stop(false)
        .on("pointer_down",Fn("stop")).on_focus_within_change(Fn("focused"))
        // The inset title and the full-width rail deliberately use different edges.
        .audit_allow(["text-edge-misaligned"]);
}
