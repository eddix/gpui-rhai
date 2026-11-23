/* gpui-rhai
{"id":"patterns/model_picker","export":"ModelPicker","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/accordion","components/popover","components/switch"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","selection","surface_hover","surface_raised","text_muted","text_primary","radius.md","typography.caption","typography.control","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// ModelPicker presents the application-provided model list and request options.
// State: settings and open state have independent controlled or local ownership.
// Example: ModelPicker(#{key:"assistant",models:[#{id:"auto",name:"Auto"}]})
import "components/accordion" as accordion;
import "components/popover" as popover;
import "components/switch" as toggle;

define_component(#{
    metadata:#{id:"patterns/model_picker","export":"ModelPicker",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/accordion","components/popover","components/switch"],capabilities:#{},assets:[],
        tokens:["accent","border","focus_ring","selection","surface_hover","surface_raised","text_muted","text_primary","radius.md","typography.caption","typography.control","space.spacing_unit","space.unit"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable picker and overlay identity."},
        models:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique nonempty model ID."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Display name."},
            description:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Brief subtitle and accessible description."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative icon. The default tile shows an initial."},
            speed:#{schema:#{type:"optional",value:#{type:"integer",min:1,max:5}},required:false,sensitive:false,doc:"Relative speed score; five is fastest."},
            quality:#{schema:#{type:"optional",value:#{type:"integer",min:1,max:5}},required:false,sensitive:false,doc:"Relative quality score; five is highest."},
            cost:#{schema:#{type:"optional",value:#{type:"integer",min:1,max:5}},required:false,sensitive:false,doc:"Relative cost score; five is most expensive."},
            latency_ms:#{schema:#{type:"optional",value:#{type:"number",min:0.0,max:86400000.0}},required:false,sensitive:false,doc:"Application estimate for first-token latency in milliseconds."},
            price_per_million:#{schema:#{type:"optional",value:#{type:"number",min:0.0,max:1000000.0}},required:false,sensitive:false,doc:"Application estimate in US dollars per million tokens."},
            disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Keep the model visible but unavailable for selection."},
        }}},required:true,sensitive:false,doc:"Display data only; stats are application estimates."},
        value:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            model:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted model ID; empty means none."},
            thinking:#{schema:#{type:"bool"},required:true,sensitive:false,doc:"Whether extended thinking is enabled."},
            search:#{schema:#{type:"bool"},required:true,sensitive:false,doc:"Whether web search is enabled."},
            effort:#{schema:#{type:"string",allowed:["low","medium","high"]},required:true,sensitive:false,doc:"Requested reasoning effort, displayed while thinking is enabled."},
        }}},required:false,sensitive:false,doc:"Accepted configuration; omit for local settings."},
        open:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Accepted overlay state; omit for local disclosure."},
        placement:#{schema:#{type:"string",allowed:["top","bottom"]},"default":#{type:"string",value:"bottom"},
            required:false,sensitive:false,doc:"Prefer top when used near the bottom of a Composer."},
        align:#{schema:#{type:"string",allowed:["start","end"]},"default":#{type:"string",value:"start"},required:false,sensitive:false,doc:"Logical alignment of the panel with the trigger."},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Prevent interaction and remove the open panel."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Full proposed configuration."},
        on_open_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Proposed overlay open state."},
    },state:#{fields:#{
        local:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        local_open:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        cursor:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
    }},events:#{
        change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            model:#{schema:#{type:"string"},required:true,sensitive:false},
            thinking:#{schema:#{type:"bool"},required:true,sensitive:false},
            search:#{schema:#{type:"bool"},required:true,sensitive:false},
            effort:#{schema:#{type:"string",allowed:["low","medium","high"]},required:true,sensitive:false},
        }},doc:"Application-owned configuration proposal."},
        open_change:#{payload:#{type:"bool"},doc:"Application-owned overlay proposal."},
    },slots:#{},parts:["root","trigger","name","glyph","model_icon","model_icon_fallback","panel","heading","head_label","list","item","selected","active",
        "item_name","description","scores","score","score_on","settings","switch","effort","effort_label","segments","segment",
        "segment_selected","footer","stat","empty"]},
    render:Fn("render_ModelPicker")
});

fn ModelPicker(props){render_component("patterns/model_picker",props)}
fn optional(props,name){if name in props{props[name]}else{()}}
fn join_details(parts){
    let result="";for part in parts{if result!=""{result+=" · ";}result+=part;}result
}
fn glyph(ctx,kind){
    let path=switch kind{
        "chevron"=>"m6 9 6 6 6-6",
        "check"=>"m5 12 4 4L19 6",
        "thinking"=>"M9 18h6m-5 3h4m-4-6c0-1.7-3-2.9-3-6a5 5 0 0 1 10 0c0 3.1-3 4.3-3 6",
        _=>"M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zM2 12h20M12 2c-2.5 2.5-4 6-4 10s1.5 7.5 4 10m0-20c2.5 2.5 4 6 4 10s-1.5 7.5-4 10"
    };
    svg(`<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("glyph",style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4)).flex_shrink(false)))
}
fn open_change(controlled,allowed,ctx,open){
    if !allowed{return;}
    if !controlled{ctx.set_state("local_open",open);}
    ctx.set_state("cursor","");ctx.emit("open_change",open);
}
fn trigger_key(controlled,allowed,ctx,event){
    if !allowed{return propagate();}
    open_change(controlled,allowed,ctx,true);handled()
}
fn propose(model,controlled,field,ctx,value){
    if model[field]==value{return false;}
    model[field]=value;
    if !controlled{ctx.set_state("local",model);}
    ctx.emit("change",model);
    true
}
fn pick(config,ctx,id){
    // Close via the same overlay policy as pointer dismissal. Rejected
    // configuration proposals never paint or announce an optimistic model.
    propose(config.value,config.controlled,"model",ctx,id);
    open_change(config.open_controlled,config.can_close,ctx,false);
    handled()
}
fn set_field(model,controlled,field,value,ctx,payload){
    propose(model,controlled,field,ctx,value);handled()
}
fn by_switch(model,controlled,field,ctx,value){propose(model,controlled,field,ctx,value);}
fn move(items,refs,id,delta,ctx,event){
    let at=items.index_of(id);if items.len==0{return propagate();}
    let next=if delta==64{items.len-1}else if delta== -64{0}else{(at+items.len+delta)%items.len};
    let target=refs[items[next]];
    ctx.focus(target);ctx.scroll_into_view(target);
    ctx.set_state("cursor",items[next]);handled()
}
fn hovered(ctx,ev){if ev.hovered{ctx.set_state("cursor",ev.value);}else if ctx.get_state("cursor")==ev.value{ctx.set_state("cursor","");}}
fn focused(ctx,ev){if ev.focused{ctx.set_state("cursor",ev.value);}else if ctx.get_state("cursor")==ev.value{ctx.set_state("cursor","");}}
fn model_icon(ctx,model){
    let icon=optional(model,"icon");
    let visual=if icon==(){text(model.name.sub_string(0,1)).with_style(ctx.component_style("model_icon_fallback",
        style().typography("caption").font_weight(600).text_color(theme_color("text_primary"))))}
        else{icon.with_style(style().width(theme_length("space.unit").times(4.5)).height(theme_length("space.unit").times(4.5)))};
    row([visual]).with_key("model-icon").accessibility_hidden(true)
        .with_style(ctx.component_style("model_icon",style().width(theme_length("space.unit").times(6)).height(theme_length("space.unit").times(6)).flex_shrink(false).items_center()
            .justify_center().radius(theme_radius("md")).background(theme_color("surface_hover"))))
}
fn score_text(ctx,model,field){
    let score=optional(model,field);if score==(){""}else{ctx.format_number(score)+"/5"}
}
fn score_cells(ctx,model){
    if optional(model,"speed")==()&&optional(model,"quality")==()&&optional(model,"cost")==(){
        return row([text(ctx.t_or("model.varies","Varies")).with_style(ctx.component_style("description",style()
            .typography("caption").text_color(theme_color("text_muted"))))]).with_key("varies")
            .accessibility_hidden(true).with_style(style().width(theme_length("space.unit").times(32.5)).justify_end().flex_shrink(false));
    }
    let cells=[];
    for field in ["speed","quality","cost"]{
        let value=optional(model,field);let bars=[];
        for n in 1..=5{
            bars.push(row([]).with_key(n.to_string()).with_style(ctx.component_style(
                if value!=()&&value>=n{["score","score_on"]}else{["score"]},
                style().width(px(3)).height(px(4+n*2)).radius(px(1))
                    .background(theme_color(if value!=()&&value>=n{"accent"}else{"border"})))));
        }
        cells.push(row(bars).with_key(field).with_style(style().width(theme_length("space.unit").times(10.5)).items_end().justify_center().gap(theme_length("space.spacing_unit").times(0.5))));
    }
    row(cells).with_key("scores").accessibility_hidden(true)
        .with_style(ctx.component_style("scores",style().gap(theme_length("space.spacing_unit").times(0.5)).flex_shrink(false)))
}
fn model_row(ctx,config,ids,model,active,compact){
    let selected=config.value.model==model.id;let disabled=optional(model,"disabled")==true||!config.editable;
    let description=optional(model,"description");
    let label=[text(model.name).with_key("name").with_style(ctx.component_style("item_name",style().font_weight(500)
        .text_ellipsis().whitespace_nowrap().min_width(px(0))))];
    if description!=(){label.push(text(description).with_key("description").with_style(ctx.component_style("description",style()
        .typography("caption").text_color(theme_color("text_muted")).text_ellipsis().whitespace_nowrap().min_width(px(0)))));}
    let content=[model_icon(ctx,model),column(label).with_style(style().min_width(px(0)).flex_grow())];
    if !compact{content.push(score_cells(ctx,model));}
    content.push(row([glyph(ctx,"check").with_style(style().opacity(if selected{1.0}else{0.0}))])
        .with_key("check").accessibility_hidden(true).with_style(style().flex_shrink(false)));
    let parts=["item"];if selected{parts.push("selected");}if active{parts.push("active");}
    let details=[];
    if description!=(){details.push(description);}
    for field in ["speed","quality","cost"]{
        let count=score_text(ctx,model,field);if count!=""{details.push(metric_name(ctx,field)+" "+count);}
    }
    let base=style().width(relative(1.0)).min_width(px(0)).min_height(theme_length("space.unit").times(11.5)).padding_x(theme_length("space.spacing_unit").times(1.5)).gap(theme_length("space.spacing_unit").times(1.5)).items_center()
        .radius(theme_radius("md")).typography("control").focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))
        .hover(style().background(theme_color("surface_hover")));
    if selected{base=base.background(theme_color("selection"));}
    let node=row(content).with_key(model.id).with_ref(element_ref(config.refs[model.id]))
        .accessibility_role("option").accessibility_label(model.name).accessibility_description(join_details(details))
        .accessibility_selected(selected).disabled(disabled).tab_stop(!disabled)
        .with_style(ctx.component_style(parts,base)).paint_transition(145,cubic_bezier(0.22,1.0,0.36,1.0));
    if disabled{return node;}
    node.on_click_value(Fn("pick").curry(config),model.id)
        .on_hover_value(Fn("hovered"),model.id).on_focus_within_value(Fn("focused"),model.id)
        .on_key_value("down",Fn("move").curry(ids,config.refs,model.id,1),()).on_key_value("up",Fn("move").curry(ids,config.refs,model.id,-1),())
        .on_key_value("home",Fn("move").curry(ids,config.refs,model.id,-64),()).on_key_value("end",Fn("move").curry(ids,config.refs,model.id,64),())
}
fn metric_name(ctx,field){
    if field=="speed"{ctx.t_or("model.speed","Speed")}
    else if field=="quality"{ctx.t_or("model.quality","Quality")}
    else{ctx.t_or("model.cost","Cost")}
}
fn effort_name(ctx,effort){
    if effort=="low"{ctx.t_or("model.low","Low")}
    else if effort=="medium"{ctx.t_or("model.medium","Medium")}
    else{ctx.t_or("model.high","High")}
}
fn settings(ctx,config){
    let value=config.value;
    let nodes=[toggle::Switch(#{checked:value.thinking,label:ctx.t_or("model.thinking","Extended thinking"),
        disabled:!config.editable,placement:"end",size:"sm",on_change:Fn("by_switch").curry(value,config.controlled,"thinking")})
        .with_key("thinking").with_style(ctx.component_style("switch",style()))];
    if value.thinking{
        let segments=[];
        for id in ["low","medium","high"]{
            let selected=value.effort==id;let label=effort_name(ctx,id);
            let parts=if selected{["segment","segment_selected"]}else{["segment"]};
            let base=style().min_height(theme_length("space.unit").times(6)).padding_x(theme_length("space.spacing_unit").times(2)).items_center().justify_center().radius(theme_radius("md"))
                .typography("caption").focus_ring(px(2),offset_px(-2),theme_color("focus_ring"));
            if selected{base=base.background(theme_color("surface_raised"));}
            let choice=row([text(label)]).with_key(id).accessibility_role("radio").accessibility_label(label)
                .accessibility_checked(selected).disabled(!config.editable).tab_stop(config.editable)
                .with_style(ctx.component_style(parts,base)).paint_transition(160,cubic_bezier(0.22,1.0,0.36,1.0));
            if config.editable{choice=choice.on_click(Fn("set_field").curry(value,config.controlled,"effort",id));}
            segments.push(choice);
        }
        let label=ctx.t_or("model.effort","Effort");
        let effort=row([text(label).with_style(ctx.component_style("effort_label",style().typography("caption"))),
            row(segments).accessibility_role("radiogroup").accessibility_label(label).tab_stop(false)
                .with_style(ctx.component_style("segments",style().gap(theme_length("space.spacing_unit").times(0.5)).padding(theme_length("space.spacing_unit").times(0.5)).items_center()
                    .radius(theme_radius("md")).background(theme_color("surface_hover"))))])
            .with_style(ctx.component_style("effort",style().items_center().justify_between().flex_wrap().gap(theme_length("space.spacing_unit"))));
        nodes.push(effort);
    }
    nodes.push(toggle::Switch(#{checked:value.search,label:ctx.t_or("model.search","Web search"),disabled:!config.editable,
        placement:"end",size:"sm",on_change:Fn("by_switch").curry(value,config.controlled,"search")})
        .with_key("search").with_style(ctx.component_style("switch",style())));
    column(nodes).with_key("settings").with_style(ctx.component_style("settings",style().gap(theme_length("space.spacing_unit").times(0.5)).padding_x(theme_length("space.spacing_unit").times(1.5))))
}
fn estimates(ctx,model,value){
    if model==(){return "";}
    let stats=[];let latency=optional(model,"latency_ms");let price=optional(model,"price_per_million");
    if latency!=(){
        // An application supplies measured estimates. Do not fabricate effort multipliers.
        stats.push(ctx.t_or("model.latency","~{seconds}s first token",#{seconds:ctx.format_number(latency.to_float()/1000.0)}));
    }
    if price!=(){
        stats.push(ctx.t_or("model.price","${price} / M tokens",#{price:ctx.format_number(price)}));
    }
    if stats.len==0{let description=optional(model,"description");return if description==(){model.name}else{description};}
    join_details(stats)
}
fn render_ModelPicker(ctx,props){
    let all=#{};let ids=[];let refs=#{};let row_index=0;
    for item in props.models{
        if item.id==""||item.id in all{throw "ModelPicker IDs must be unique and nonempty";}
        refs[item.id]="model-row:"+row_index.to_string();row_index+=1;
        all[item.id]=item;if optional(item,"disabled")!=true{ids.push(item.id);}
    }
    let provided=optional(props,"value");let local=ctx.get_state("local");
    let value=if provided!=(){provided}else if local!=(){local}else{#{model:"",thinking:false,search:false,effort:"medium"}};
    if provided==()&&!(value.model in all){value.model=if ids.len>0{ids[0]}else{""};}
    let chosen=if value.model in all{all[value.model]}else{()};
    let external_open=optional(props,"open");
    let open=!props.disabled&&(if external_open==(){ctx.get_state("local_open")}else{external_open});
    let config=#{value:value,controlled:provided!=(),open_controlled:external_open!=(),refs:refs,
        can_close:external_open==()||optional(props,"on_open_change")!=(),
        editable:!props.disabled&&(provided==()||optional(props,"on_change")!=())};
    let openable=!props.disabled&&config.can_close;
    let name=if chosen==(){ctx.t_or("model.choose","Choose a model")}else{chosen.name};
    let title=ctx.t_or("model.settings","Model settings");
    let trigger_nodes=[];if chosen!=(){trigger_nodes.push(model_icon(ctx,chosen));}
    trigger_nodes.push(text(name).with_key("name").with_style(ctx.component_style("name",style().font_weight(500)
        .min_width(px(0)).text_ellipsis().whitespace_nowrap()))
        .motion(motion_transition("opacity",0.65,1.0,#{duration_ms:220,intent:"feedback"})).motion_on_change(value.model));
    if value.thinking{trigger_nodes.push(glyph(ctx,"thinking").with_key("thought"));}
    trigger_nodes.push(glyph(ctx,"chevron").with_key("chevron"));
    let trigger=row(trigger_nodes).with_key("model-trigger").accessibility_role("button").accessibility_label(name)
        .accessibility_expanded(open).disabled(!openable)
        .with_style(ctx.component_style("trigger",style().items_center().min_width(px(0)).max_width(relative(1.0))
            .min_height(theme_length("space.unit").times(8.5)).gap(theme_length("space.spacing_unit").times(1.5)).padding_x(theme_length("space.spacing_unit").times(2)).typography("control").radius(theme_radius("md"))
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .on_key_value("down",Fn("trigger_key").curry(config.open_controlled,openable),());
    let cursor=ctx.get_state("cursor");let reference=element_ref("model-popup");
    let compact=ctx.element_width_class(reference,[292])==0;
    let rows=[];for item in props.models{rows.push(model_row(ctx,config,ids,item,item.id==cursor,compact));}
    if rows.len==0{rows.push(text(ctx.t_or("model.empty","No models available")).with_style(ctx.component_style("empty",style()
        .typography("caption").text_color(theme_color("text_muted")).padding(theme_length("space.spacing_unit").times(2)))));}
    let label=ctx.t_or("model.model","Model");
    let heading=[text(label).with_style(ctx.component_style("head_label",style().typography("caption").flex_grow()))];
    if !compact{for field in ["speed","quality","cost"]{
        heading.push(row([text(metric_name(ctx,field))]).with_style(style().width(theme_length("space.unit").times(10.5)).justify_center().flex_shrink(false)));
    }heading.push(row([]).with_style(style().width(theme_length("space.unit").times(4)).flex_shrink(false)));}
    let list=column(rows).with_key("models").tab_stop(false).accessibility_role("listbox").accessibility_label(label)
        .with_style(ctx.component_style("list",style().min_height(px(0)).min_width(px(0)).max_height(px(235)).overflow_y_scroll()));
    let observed=if cursor in all{all[cursor]}else{chosen};
    let caption=estimates(ctx,observed,value);
    let body=column([
        row(heading).with_key("heading").accessibility_hidden(true).with_style(ctx.component_style("heading",style().gap(theme_length("space.spacing_unit").times(0.5))
            .padding_x(theme_length("space.spacing_unit").times(1.5)).typography("caption").items_center())),
        list,settings(ctx,config),
        row([text(caption).with_key("estimate").with_style(ctx.component_style("stat",style().typography("caption").font_feature("tnum",1)))
            .motion(motion_transition("opacity",0.45,1.0,#{duration_ms:200,intent:"feedback"})).motion_on_change(caption)])
            .with_key("estimate-tray").accessibility_role("status").with_style(ctx.component_style("footer",style()
                .padding(theme_length("space.spacing_unit").times(1.5)).text_color(theme_color("text_muted")))),
    ]).with_key("model-popup").with_ref(reference).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit")));
    popover::Popover(#{key:props.key,label:title,open:open,placement:props.placement,align:props.align,trigger:trigger,content:body,
        initial_focus:"first",
        on_open_change:Fn("open_change").curry(config.open_controlled,openable),
        part_styles:#{content:ctx.component_style("panel",style().width(px(340)).min_width(px(0)).max_width(relative(1.0))
            .padding(theme_length("space.spacing_unit").times(1.5)).gap(px(0)).min_height(px(0)).overflow_y_scroll())}})
        .with_style(ctx.component_style("root",style().min_width(px(0))))
}
