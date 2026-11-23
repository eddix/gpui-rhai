/* gpui-rhai
{"id":"components/thinking_state","export":"ThinkingState","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/accordion","components/thinking_loader","components/tooltip"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","surface_hover","surface_raised","text_primary","text_muted","text.success","text.danger","radius.sm","typography.caption","typography.control","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// ThinkingState presents application-visible workflow steps and their evidence.
// State: steps and status are fully controlled; expansion may be local.
// Example: ThinkingState(#{key:"steps",steps:[#{id:"verify",label:"Inspect build log"}]})
import "components/accordion" as accordion;
import "components/thinking_loader" as loader;
import "components/tooltip" as tooltip;

define_component(#{
    metadata:#{id:"components/thinking_state","export":"ThinkingState",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/accordion","components/thinking_loader","components/tooltip"],capabilities:#{},assets:[],
        tokens:["border","focus_ring","surface_hover","surface_raised","text_primary","text_muted","text.success","text.danger","radius.sm","typography.caption","typography.control","space.spacing_unit","space.unit"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of the work summary."},
        label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Heading while work is active."},
        steps:#{schema:#{type:"array",max_items:48,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable nonempty step ID. Must be unique."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Public description of the work, not internal model reasoning."},
            active_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional caption while this step is active."},
            kind:#{schema:#{type:"string",allowed:["step","search"]},required:false,sensitive:false,doc:"An ordinary step or a search with a distinct marker."},
            state:#{schema:#{type:"string",allowed:["running","done","failed"]},required:false,sensitive:false,doc:"Accepted result of this step; defaults to done."},
            sources:#{schema:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Nonempty source ID, unique within the step."},
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Source chip text."},
                href:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Opaque destination; the component never opens it."},
                hint:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional pointer and keyboard tooltip."},
                icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative source icon."},
                extra:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Hide behind the +N button until expanded."},
            }},},required:false,sensitive:false,doc:"Application supplied sources for this step."},
        }}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Public steps in display order. The widget never synthesizes workflow events."},
        status:#{schema:#{type:"string",allowed:["working","done","stopped","failed"]},"default":#{type:"string",value:"working"},
            required:false,sensitive:false,doc:"Accepted aggregate status. Terminal states stop decorative activity."},
        elapsed_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9000000000000}},required:false,sensitive:false,
            doc:"Host-supplied elapsed duration; displayed without maintaining a second clock."},
        done_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Text of the terminal success step."},
        open:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Accepted disclosure state; omit to keep disclosure local."},
        default_open:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Initial local disclosure state."},
        on_open_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Proposed disclosure boolean."},
        on_open_source:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Explicit source activation. Receives #{step,id,href}."},
    },state:#{fields:#{
        open:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        extras:#{schema:#{type:"array",max_items:48,items:#{type:"string"}},"default":#{type:"array",value:[]}},
    }},events:#{
        open_change:#{payload:#{type:"bool"},doc:"Proposed disclosure open state."},
        open_source:#{payload:#{type:"object",allow_unknown:false,fields:#{
            step:#{schema:#{type:"string"},required:true,sensitive:false},
            id:#{schema:#{type:"string"},required:true,sensitive:false},
            href:#{schema:#{type:"string"},required:true,sensitive:false},
        }},doc:"Only emitted for explicit activation of a source. Host decides what to do."},
    },slots:#{},parts:["root","header","head_label","head_mark","elapsed","chevron","body","row","marker","rail","active_dot",
        "done_mark","failed_mark","copy","title","active","sources","source","source_icon","source_title","more","terminal","sheen"],effects:["maintain"]},
    render:Fn("render_ThinkingState")
});

fn ThinkingState(props){render_component("components/thinking_state",props)}
fn opt(props,name){if name in props{props[name]}else{()}}
fn maintain(ctx,payload){
    if ctx.get_state("open")==(){ctx.set_state("open",payload[0]);}
    let extras=[];
    for id in ctx.get_state("extras"){if id in payload[1]{extras.push(id);}}
    ctx.set_state("extras",extras);
}
fn cleanup(ctx,payload){}
fn disclosure(controlled,next,ctx,event){
    if !controlled{ctx.set_state("open",next);}
    ctx.emit("open_change",next);
    handled()
}
fn toggle_extra(ctx,id){
    let extras=ctx.get_state("extras");
    if id in extras{extras.remove(extras.index_of(id));}else{extras.push(id);}
    ctx.set_state("extras",extras);handled()
}
fn source_requested(ctx,value){ctx.emit("open_source",value);handled()}
fn svg_mark(mark){
    let shape=switch mark {
        "chevron"=>"m6 9 6 6 6-6",
        "search"=>"M21 21l-4.5-4.5m1.5-5.5a7 7 0 1 1-14 0 7 7 0 0 1 14 0zM4 11h14M11 4c3 4 3 10 0 14m0-14c-3 4-3 10 0 14",
        "done"=>"M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0zM7.5 12l3 3 6-6",
        _=>"M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0zM12 7v6m0 3v.1",
    };
    svg(`<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><path d='${shape}'/></svg>`)
        .accessibility_hidden(true)
}
fn marker(ctx,kind,active){
    if kind=="step"{
        let dot=row([]).with_key("dot").with_style(ctx.component_style(if active{["active_dot"]}else{["active_dot","marker"]},
            style().width(px(7)).height(px(7)).radius(theme_length("space.unit").times(24.75)).background(theme_color(if active{"text_primary"}else{"text_muted"}))));
        if !active{return dot;}
        return dot.motion(motion_transition("opacity",1.0,0.56,#{duration_ms:620,iterations:"infinite",autoreverse:true,intent:"decorative"}));
    }
    let part=if kind=="done"{"done_mark"}else if kind=="failed"{"failed_mark"}else{"marker"};
    svg_mark(kind).with_style(ctx.component_style(part,style().width(theme_length("space.unit").times(4.25)).height(theme_length("space.unit").times(4.25))
        .text_color(theme_color(if kind=="done"{"text.success"}else if kind=="failed"{"text.danger"}else{"text_muted"}))))
}
fn source_node(ctx,step,source,enabled,delay){
    let href=opt(source,"href");let hint=opt(source,"hint");
    let parts=[];
    if opt(source,"icon")!=(){parts.push(source.icon.with_style(ctx.component_style("source_icon",
        style().width(px(14)).height(px(14)).flex_shrink(false))).accessibility_hidden(true));}
    parts.push(text(source.label).with_style(ctx.component_style("source_title",style().min_width(px(0))
        .text_ellipsis().whitespace_nowrap())));
    let base=style().min_width(px(0)).items_center().max_width(relative(1.0)).min_height(theme_length("space.unit").times(6)).gap(theme_length("space.spacing_unit"))
        .padding_x(theme_length("space.spacing_unit").times(1.75)).radius(theme_radius("sm")).typography("caption")
        .background(theme_color("surface_hover")).text_color(theme_color("text_primary"))
        .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"));
    let node=row(parts).with_key(source.id).with_style(ctx.component_style("source",base))
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:280,delay_ms:delay,intent:"decorative"}));
    if href==()||href==""||!enabled{return node;}
    node=node.accessibility_role("button").accessibility_label(source.label)
        .accessibility_description(if hint==(){href}else{hint})
        .on_click_value(Fn("source_requested"),#{step:step,id:source.id,href:href});
    if hint!=()&&hint!=""{
        return tooltip::Tooltip(#{key:source.id,label:hint,trigger:node,content:text(hint)});
    }
    node
}
fn step_node(ctx,item,followed,visible,expand,active,can_open,index,total){
    let kind=opt(item,"kind");if kind==(){kind="step";}
    let copy=item.label;let busy=opt(item,"active_label");
    if active&&busy!=()&&busy!=""{copy=busy;}
    let style=ctx.component_style(if active{["title","active"]}else{["title"]},style().typography("control").min_width(px(0)));
    let label=text(copy).with_key("label").with_style(style);
    if active{label=loader::glyph_sheen(ctx,label,style,1600);}
    let content=[label];
    let sources=opt(item,"sources");if sources==(){sources=[];}
    let chips=[];let ids=[];let extra=0;
    for source in sources{
        if source.id==""||source.id in ids{throw "ThinkingState source IDs must be unique within a step";}
        ids.push(source.id);
        if opt(source,"extra")==true{extra+=1;if !expand{continue;}}
        chips.push(source_node(ctx,item.id,source,visible&&can_open,120+min(chips.len,8)*65));
    }
    if extra>0{
        let caption=if expand{ctx.t_or("thinking.less","Show less")}
            else{ctx.t_or("thinking.more","+{count} more",#{count:ctx.format_number(extra)})};
        let button=row([text(caption)]).with_key("more").accessibility_role("button").accessibility_label(caption)
            .accessibility_expanded(expand).tab_stop(visible).disabled(!visible)
            .with_style(ctx.component_style(["source","more"],style().min_height(theme_length("space.unit").times(6)).padding_x(theme_length("space.spacing_unit").times(1.75)).items_center()
                .radius(theme_radius("sm")).typography("caption").background(theme_color("surface_hover"))
                .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))));
        if visible{button=button.on_click_value(Fn("toggle_extra"),item.id);}
        chips.push(button);
    }
    if chips.len>0{content.push(row(chips).with_key("sources").with_style(ctx.component_style("sources",style().min_width(px(0))
        .flex_wrap().gap(theme_length("space.spacing_unit")))));}
    let cells=[];
    if followed{
        cells.push(row([]).with_key("rail").accessibility_hidden(true).with_style(ctx.component_style("rail",style()
            .absolute().top(px(21)).bottom(px(0)).inset_start(px(9.5)).width(px(1)).background(theme_color("border")))));
    }
    cells.push(row([marker(ctx,kind,active)]).with_key("mark").accessibility_hidden(true)
        .with_style(style().width(theme_length("space.unit").times(5)).height(theme_length("space.unit").times(5.25)).justify_center().items_center().flex_shrink(false)));
    cells.push(column(content).with_key("copy").with_style(ctx.component_style("copy",style().min_width(px(0)).flex_grow().gap(theme_length("space.spacing_unit")))));
    row(cells)
        .with_key(item.id).accessibility_role("listitem").accessibility_label(copy)
        .accessibility_position_in_set(index+1).accessibility_size_of_set(total)
        .with_style(ctx.component_style("row",style().relative().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2.5)).padding_bottom(theme_length("space.spacing_unit").times(2.5))))
        .motion(motion_transition("opacity",0.35,1.0,#{duration_ms:440,intent:"feedback"}))
        .motion(motion_transition("translate_y",4.0,0.0,#{duration_ms:410,intent:"feedback"}))
}
fn render_ThinkingState(ctx,props){
    let controlled=opt(props,"open");let stored=ctx.get_state("open");
    let open=if controlled!=(){controlled}else if stored!=(){stored}else{props.default_open};
    let eligible=controlled==()||opt(props,"on_open_change")!=();
    let duration=opt(props,"elapsed_ms");let terminal=props.status!="working";
    let title=if props.status=="done"{loader::finish_label(ctx,(),duration)}
        else if props.status=="stopped"{ctx.t_or("thinking.stopped","Stopped")}
        else if props.status=="failed"{ctx.t_or("thinking.failed","Work incomplete")}
        else if opt(props,"label")!=(){props.label}else{ctx.t_or("thinking.label","Thinking")};
    let mark=svg_mark("chevron").with_key("chevron").with_style(ctx.component_style("chevron",style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4)).flex_shrink(false)))
        .motion(motion_transition("rotate",if open{0.0}else if ctx.text_direction()=="rtl"{90.0}else{-90.0},
            if open{0.0}else if ctx.text_direction()=="rtl"{90.0}else{-90.0},#{duration_ms:210,intent:"feedback"}));
    let header_content=[mark,loader::text_sheen(ctx,title,"head_label",!terminal).with_key("label")];
    if !terminal&&duration!=(){header_content.push(text(ctx.format_duration(duration)).with_key("elapsed")
        .with_style(ctx.component_style("elapsed",style().typography("caption").font_feature("tnum",1).flex_shrink(false))));}
    let head=row(header_content).with_key("header").accessibility_role("button").accessibility_label(title)
        .accessibility_expanded(open).disabled(!eligible).tab_stop(eligible)
        .with_style(ctx.component_style("header",style().min_width(px(0)).min_height(theme_length("space.unit").times(7)).self_start().max_width(relative(1.0))
            .items_center().gap(theme_length("space.spacing_unit").times(1.5)).padding_x(theme_length("space.spacing_unit")).radius(theme_radius("sm")).text_color(theme_color("text_muted"))
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .paint_transition(140,cubic_bezier(0.22,1.0,0.36,1.0));
    if eligible{head=head.on_click(Fn("disclosure").curry(controlled!=(),!open));}
    let ids=[];let rows=[];let kept_extras=ctx.get_state("extras");
    let total=props.steps.len+(if terminal{1}else{0});
    for i in 0..props.steps.len{
        let item=props.steps[i];
        if item.id==""||item.id in ids{throw "ThinkingState step IDs must be unique and nonempty";}
        ids.push(item.id);
        rows.push(step_node(ctx,item,i+1<total,open,item.id in kept_extras,
            open&&!terminal&&opt(item,"state")=="running",opt(props,"on_open_source")!=(),i,total));
    }
    effect("maintain",[open,ids],Fn("maintain"),Fn("cleanup"));
    if terminal{
        let label=if props.status=="done"{if opt(props,"done_label")!=(){props.done_label}else{ctx.t_or("thinking.complete","Finished thinking")}}else{title};
        rows.push(row([row([marker(ctx,if props.status=="done"{"done"}else{"failed"},false)])
            .with_style(style().width(theme_length("space.unit").times(5)).height(theme_length("space.unit").times(5.25)).items_center().justify_center().flex_shrink(false)),
            text(label).with_style(ctx.component_style("terminal",style().typography("control").min_width(px(0))))])
            .with_key("terminal-status").with_style(ctx.component_style("row",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2.5)).padding_bottom(theme_length("space.spacing_unit"))))
            .accessibility_role("listitem").accessibility_label(label).accessibility_position_in_set(total).accessibility_size_of_set(total));
    }
    let body=column(rows).with_key("body").accessibility_role("list").accessibility_label(ctx.t_or("thinking.steps","Work steps"))
        .accessibility_hidden(!open).with_style(ctx.component_style("body",style().min_width(px(0)).padding_top(theme_length("space.spacing_unit").times(2))));
    column([head,accordion::disclosure_panel(ctx,"steps",open,(),body)]).with_key(props.key)
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(440)).min_width(px(0)).text_color(theme_color("text_primary"))));
}
