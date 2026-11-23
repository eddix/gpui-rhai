/* gpui-rhai
{"id":"components/filter_menu","export":"FilterMenu","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/accordion","components/checkbox","components/kbd","components/tag"],"capabilities":{},"assets":[],"tokens":["surface_raised","surface_hover","text_primary","text_muted","border","focus_ring","accent","radius.sm","radius.md","typography.control","typography.caption","space.spacing_unit","space.unit","typography.filter_title"],"environment":["density"]}
*/
// A grouped multi-select filter surface. The caller owns filtering and data.
// Example: FilterMenu(#{ key:"filters",groups:[#{id:"state",label:"State",options:[#{id:"open",label:"Open"}]}],value:#{} })
import "components/accordion" as accordion;
import "components/checkbox" as checkbox;
import "components/kbd" as kbd;
import "components/tag" as tag;

define_component(#{
    metadata:#{id:"components/filter_menu","export":"FilterMenu",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/accordion","components/checkbox","components/kbd","components/tag"],
        capabilities:#{},assets:[],
        tokens:["surface_raised","surface_hover","text_primary","text_muted","border","focus_ring","accent","radius.sm","radius.md","typography.control","typography.caption","space.spacing_unit","space.unit","typography.filter_title"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity for this filter surface."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Visible and accessible title. Defaults to localized Filters."},
        groups:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable, unique group ID."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Display name of the group."},
            chip_prefix:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional prefix in the selected chip, not part of the value."},
            options:#{schema:#{type:"array",max_items:128,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable ID unique within this group."},
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Display name of the option."},
                disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Remains visible but cannot be toggled or used as a roving target."},
            }}},required:true,sensitive:false,doc:"Ordered options. At most 256 options across the surface."},
        }}},required:true,sensitive:false,doc:"Groups and options in display and proposal order."},
        value:#{schema:#{type:"optional",value:#{type:"map",values:#{type:"array",max_items:128,items:#{type:"string"}}}},
            required:false,sensitive:false,doc:"Controlled selections by group ID. Empty groups are omitted in proposals. Unknown keys are not presented."},
        default_value:#{schema:#{type:"map",values:#{type:"array",max_items:128,items:#{type:"string"}}},
            "default":#{type:"map",value:#{}},required:false,sensitive:false,doc:"Initial selections when value is unset."},
        default_open:#{schema:#{type:"optional",value:#{type:"array",max_items:32,items:#{type:"string"}}},
            required:false,sensitive:false,doc:"Initially expanded groups; unset opens the first group, [] opens none."},
        hints:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Draw keyboard hints below the groups."},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disables navigation and all edits without hiding applied selections."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the complete proposed selection map from a toggle, removal or clear."},
    },state:#{fields:#{
        local:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        opened:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        cursor:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        model:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        pending:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{change:#{payload:#{type:"map",values:#{type:"array",max_items:128,items:#{type:"string"}}},
        doc:"A complete selection proposal, in definition order within each group. Expand/collapse does not emit."}},
        slots:#{},parts:["root","card","header","title","clear","chips","chip","chip_body","chip_label","chip_remove","chip_icon","more",
            "groups","group","trigger","chevron","group_label","total","selected_count","options","option","option_label",
            "choice_control","choice_box","choice_on","choice_mark","hints","hint","hint_key"],effects:["committed"]},
    render:Fn("render_FilterMenu")
});

fn FilterMenu(props){render_component("components/filter_menu",props)}
fn opt(map,key){if key in map{map[key]}else{()}}
fn group_key(id){`group:${id}`}
fn option_key(g,o){`option:${g.len}:${g}${o.len}:${o}`}
fn group_ref(i){`group-${i}`}
fn option_ref(i,j){`option-${i}-${j}`}
fn validates(groups){
    let seen=#{};
    let total=0;
    for group in groups{
        if group.id==""||group.id in seen{throw "FilterMenu group IDs must be unique and nonempty";}
        seen[group.id]=true;
        let options=#{};
        for item in group.options{
            if item.id==""||item.id in options{throw "FilterMenu option IDs must be unique and nonempty within a group";}
            options[item.id]=true;
            total+=1;
        }
    }
    if total>256{throw "FilterMenu supports at most 256 options";}
}
fn selected(groups,raw){
    let value=#{};
    for group in groups{
        let picked=opt(raw,group.id);
        if picked==(){continue;}
        let flags=#{};for id in picked{flags[id]=true;}
        let row=[];
        for option in group.options{if option.id in flags{row.push(option.id);}}
        if row.len>0{value[group.id]=row;}
    }
    value
}
fn should_show(value,g,o){g in value&&o in value[g]}
fn same_selection(a,b){
    let keys=a.keys();
    // Map dot access reads business fields; count the actual key array.
    if keys.len!=b.keys().len{return false;}
    for key in keys{if !(key in b)||a[key]!=b[key]{return false;}}
    true
}
fn disclosed(ctx,props){
    let values=ctx.get_state("opened");
    if values!=(){return values;}
    let defaults=opt(props,"default_open");
    if defaults!=(){return defaults;}
    if props.groups.len==0{[]}else{[props.groups[0].id]}
}
fn build_rows(groups,opened){
    let rows=[];let headers=#{};
    for i in 0..groups.len{
        let g=groups[i];let header=#{key:group_key(g.id),target:group_ref(i),group:g.id,header:true};
        headers[g.id]=header;rows.push(header);
        if g.id in opened{
            for j in 0..g.options.len{
                let o=g.options[j];
                if opt(o,"disabled")==true{continue;}
                rows.push(#{key:option_key(g.id,o.id),target:option_ref(i,j),group:g.id,header:false});
            }
        }
    }
    #{rows:rows,headers:headers}
}
fn current_row(model,key){
    for row in model.rows{if row.key==key{return row;}}
    if model.rows.len>0{model.rows[0]}else{()}
}
fn focus_row(ctx,row){
    if row==(){return;}
    ctx.set_state("cursor",row.key);ctx.focus(row.target);
    if !row.header{ctx.scroll_into_view(row.target);}
}
fn focused(ctx,event){
    if event.focused&&ctx.get_state("cursor")!=event.value{ctx.set_state("cursor",event.value);}
}
fn synchronize(ctx,data){
    let model=data[0];let pending=data[1];
    ctx.set_state("model",model);
    if ctx.get_state("opened")==(){ctx.set_state("opened",model.opened);}
    if pending!=(){
        if same_selection(model.value,pending.value){
            let row=if pending.group in model.headers{model.headers[pending.group]}else{current_row(model,"")};
            focus_row(ctx,row);
        }
        ctx.set_state("pending",());
    }
}
fn cleanup(ctx,data){}
fn move_row(ctx,step){
    let model=ctx.get_state("model");
    if model==()||model.disabled||model.rows.len==0{return handled();}
    let key=ctx.get_state("cursor");
    let at=0;for i in 0..model.rows.len{if model.rows[i].key==key{at=i;break;}}
    let next=min(max(0,at+step),model.rows.len-1);
    focus_row(ctx,model.rows[next]);handled()
}
fn fold_to(ctx,id,open){
    let model=ctx.get_state("model");
    if model==()||model.disabled||!(id in model.headers){return handled();}
    let values=ctx.get_state("opened");if values==(){values=model.opened;}
    let next=[];for value in values{if value!=id{next.push(value);}}
    if open{next.push(id);}
    ctx.set_state("opened",next);
    focus_row(ctx,model.headers[id]);
    handled()
}
fn toggle_group(ctx,id){
    let model=ctx.get_state("model");
    if model==(){return handled();}
    fold_to(ctx,id,!(id in model.opened))
}
fn fold_focused(ctx,open){
    let model=ctx.get_state("model");
    if model==()||model.rows.len==0{return handled();}
    let row=current_row(model,ctx.get_state("cursor"));
    if open&&row.group in model.opened{return handled();}
    fold_to(ctx,row.group,open)
}
fn propose(ctx,model,value,return_focus){
    if !model.editable||same_selection(model.value,value){return handled();}
    if !model.controlled{ctx.set_state("local",value);}
    if return_focus!=(){ctx.set_state("pending",#{value:value,group:return_focus});}
    ctx.emit("change",value);
    handled()
}
fn choose(g,o,removal,ctx,payload){
    let model=ctx.get_state("model");
    if model==()||!model.editable{return handled();}
    let next=model.value;
    for group in model.groups{
        if group.id!=g{continue;}
        let editing=removal||g in model.opened;
        if !editing{return handled();}
        let options=[];let exists=false;
        for item in group.options{
            let on=should_show(model.value,g,item.id);
            if item.id==o{
                if opt(item,"disabled")==true{return handled();}
                exists=true;
                if !on&&!removal{options.push(item.id);}
            }else if on{options.push(item.id);}
        }
        if !exists{return handled();}
        if options.len==0{if g in next{next.remove(g);}}else{next[g]=options;}
        return propose(ctx,model,next,if removal{g}else{()});
    }
    handled()
}
fn clear(ctx,payload){
    let model=ctx.get_state("model");
    if model==(){return handled();}
    // Disabled option values are immutable through every editing entry.
    let keep=#{};
    for group in model.groups{
        let values=[];
        for item in group.options{if opt(item,"disabled")==true&&should_show(model.value,group.id,item.id){values.push(item.id);}}
        if values.len>0{keep[group.id]=values;}
    }
    propose(ctx,model,keep,"")
}
fn short_label(s){text(s).with_style(style().min_width(px(0)).whitespace_nowrap().text_ellipsis())}
fn option_row(ctx,model,g,o,i,j,open,active){
    let checked=should_show(model.value,g.id,o.id);let key=option_key(g.id,o.id);
    let locked=opt(o,"disabled")==true;
    let variant=if checked{["choice_box","choice_on"]}else{["choice_box"]};
    let row_style=ctx.component_style("option",style().width(relative(1.0)).min_width(px(0)).height(theme_length("space.unit").times(8.5))
        .min_height(theme_length("space.unit").times(8.5)).padding_x(theme_length("space.spacing_unit").times(1.5)).gap(theme_length("space.spacing_unit").times(3)).items_center().radius(theme_radius("sm"))
        .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring")));
    let option=checkbox::Checkbox(#{key:key,label:o.label,checked:checked,disabled:model.disabled||!open||locked,
        on_change:if model.editable&&open&&!locked{Fn("choose").curry(g.id,o.id,false)}else{()},
        part_styles:#{
            root:row_style,
            control:ctx.component_style("choice_control",style().width(theme_length("space.unit").times(5)).height(theme_length("space.unit").times(5)).min_height(px(0))),
            box:ctx.component_style(variant,style().width(theme_length("space.unit").times(5)).height(theme_length("space.unit").times(5)).radius(theme_radius("sm"))),
            mark:ctx.component_style("choice_mark",style().width(px(14)).height(px(14))),
            label:ctx.component_style("option_label",style().min_width(px(0)).whitespace_nowrap().text_ellipsis()),
        }
    }).with_key(key).with_ref(element_ref(option_ref(i,j))).tab_stop(open&&!locked&&key==active&&model.editable);
    if open&&!model.disabled&&!locked{option=option.on_focus_within_value(Fn("focused"),key);}
    option
}
fn group_view(ctx,model,g,i,active,rtl){
    let expanded=g.id in model.opened;
    let count=if g.id in model.value{model.value[g.id].len}else{0};
    let total=ctx.format_number(g.options.len);
    let picked=if count==0{""}else{ctx.t_or(if count==1{"filter_menu.one"}else{"filter_menu.picked"},
        if count==1{"1 filter"}else{"{count} filters"},#{count:ctx.format_number(count)})};
    let degrees=if expanded{0.0}else if rtl{90.0}else{-90.0};
    let arrow=svg("<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'><path d='m6.5 9.5 5.5 5.5 5.5-5.5'/></svg>")
        .with_key("chevron").accessibility_hidden(true).with_style(ctx.component_style("chevron",
            style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4)).flex_shrink(false).text_color(theme_color("text_muted"))))
        .motion(motion_transition("rotate",degrees,degrees,#{duration_ms:260,easing:cubic_bezier(0.22,1.0,0.36,1.0),intent:"feedback"}));
    let id=group_key(g.id);
    let header=row([
        arrow,short_label(g.label).with_style(ctx.component_style("group_label",style().font_weight(500))),
        text(total).with_key("total").accessibility_role("status").with_style(ctx.component_style("total",style().typography("caption").text_color(theme_color("text_muted")).flex_shrink(false))),
        text(picked).with_key("selected-count").accessibility_role("status").with_style(ctx.component_style("selected_count",style().typography("caption")
            .text_color(theme_color("text_muted")).margin_start(auto()).whitespace_nowrap().flex_shrink(false))),
    ]).with_key("header").with_ref(element_ref(group_ref(i))).with_style(ctx.component_style("trigger",
        style().width(relative(1.0)).min_width(px(0)).min_height(theme_length("space.unit").times(9)).gap(theme_length("space.spacing_unit").times(1.75)).padding_x(theme_length("space.spacing_unit").times(1.5))
            .items_center().typography("control").radius(theme_radius("sm"))
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .paint_transition(160,cubic_bezier(0.22,1.0,0.36,1.0))
        .accessibility_role("button").accessibility_label(g.label).accessibility_expanded(expanded)
        .accessibility_description(if picked==""{total}else{`${total}, ${picked}`})
        .tab_stop(id==active&&!model.disabled).disabled(model.disabled);
    if !model.disabled{header=header.on_click_value(Fn("toggle_group"),g.id).on_focus_within_value(Fn("focused"),id);}
    let options=[];
    for j in 0..g.options.len{options.push(option_row(ctx,model,g,g.options[j],i,j,expanded,active));}
    let contents=column(options).with_key("options-scroll").with_style(ctx.component_style("options",
        style().gap(theme_length("space.spacing_unit").times(0.25)).max_height(px(340)).min_width(px(0)).overflow_y_scroll()))
        .accessibility_role("group").accessibility_label(g.label).accessibility_hidden(!expanded).tab_stop(false);
    let panel=accordion::disclosure_panel(ctx,`options-${i}`,expanded,(),contents);
    column([header,panel]).with_key(g.id).with_style(ctx.component_style("group",style().min_width(px(0))))
}
fn chips_view(ctx,model,clean){
    let cells=[];
    for g in model.groups{
        if !(g.id in model.value){continue;}
        let prefix=opt(g,"chip_prefix");if prefix==(){prefix="";}
        for o in g.options{
            if !(o.id in model.value[g.id]){continue;}
            let item=tag::Tag(#{key:option_key(g.id,o.id),text:prefix+o.label,
                closable:model.editable&&opt(o,"disabled")!=true,
                on_close:if model.editable&&opt(o,"disabled")!=true{Fn("choose").curry(g.id,o.id,true)}else{()},
                part_styles:#{
                    root:ctx.component_style("chip",style().height(theme_length("space.unit").times(7.5)).min_height(theme_length("space.unit").times(7.5)).max_width(relative(1.0))
                        .background(theme_color("surface_hover")).radius(theme_radius("sm"))),
                    value:ctx.component_style("chip_body",style().padding_start(theme_length("space.spacing_unit").times(2.75)).padding_end(theme_length("space.spacing_unit")).gap(theme_length("space.spacing_unit").times(0.5))),
                    label:ctx.component_style("chip_label",style().max_width(px(180))),
                    close:ctx.component_style("chip_remove",style().width(theme_length("space.unit").times(5.5)).height(theme_length("space.unit").times(5.5)).radius(theme_radius("sm"))),
                    close_icon:ctx.component_style("chip_icon",style().width(px(14)).height(px(14))),
                }
            });
            let shape=style().min_width(px(0)).max_width(relative(1.0));
            if clean{shape=shape.overflow_priority(256-cells.len);}
            cells.push(row([item]).with_key(option_key(g.id,o.id)).with_style(shape));
        }
    }
    let count=cells.len;
    if clean{
        for omitted in 1..count+1{
            cells.push(text("+"+ctx.format_number(omitted)).with_key(`more-${omitted}`).overflow_summary_for(omitted)
                .accessibility_role("status").accessibility_label(ctx.t_or("filter_menu.more","{count} more filters",#{count:ctx.format_number(omitted)}))
                .with_style(ctx.component_style("more",style().typography("caption").padding_x(theme_length("space.spacing_unit").times(2)).whitespace_nowrap()
                    .text_color(theme_color("text_muted")))));
        }
    }
    let layout=style().min_width(px(0)).width(relative(1.0)).items_center().gap(theme_length("space.spacing_unit").times(1.5));
    if clean{layout=layout.overflow_row();}else{layout=layout.flex_wrap();}
    let content=row(cells).with_key("chips-row").with_style(ctx.component_style("chips",layout));
    accordion::disclosure_panel(ctx,"chips",count>0,(),content)
}
fn hint_view(ctx,keys,label){
    let children=[];
    for key in keys{
        children.push(kbd::Kbd(#{key:key,keys:key,part_styles:#{root:ctx.component_style("hint_key",style())}}));
    }
    children.push(text(label).with_key("legend").with_style(style().min_width(px(0))));
    row(children).with_key(keys[0]).with_style(ctx.component_style("hint",style().items_center().gap(theme_length("space.spacing_unit").times(1.25)).min_width(px(0))))
        .accessibility_hidden(true);
}
fn render_FilterMenu(ctx,props){
    validates(props.groups);
    let opened=disclosed(ctx,props);
    let input=opt(props,"value");let controlled=input!=();
    let stored=ctx.get_state("local");
    let value=selected(props.groups,if controlled{input}else if stored!=(){stored}else{props.default_value});
    let route=build_rows(props.groups,opened);
    let model=#{value:value,groups:props.groups,opened:opened,rows:route.rows,headers:route.headers,disabled:props.disabled,
        controlled:controlled,editable:!props.disabled&&(!controlled||opt(props,"on_change")!=())};
    effect("committed",[model,ctx.get_state("pending")],Fn("synchronize"),Fn("cleanup"));
    let active=current_row(model,ctx.get_state("cursor"));let active_key=if active==(){""}else{active.key};
    let preset=ctx.theme_variant();let clean=preset!=()&&preset.family=="Clean";
    let title=opt(props,"title");if title==(){title=ctx.t_or("filter_menu.label","Filters");}
    let clearable=false;
    for group in model.groups{for o in group.options{
        if opt(o,"disabled")!=true&&should_show(value,group.id,o.id){clearable=true;}
    }}
    let clear_label=ctx.t_or("filter_menu.clear","Clear all");
    let button=row([text(clear_label)]).with_key("clear").with_style(ctx.component_style("clear",
        style().height(theme_length("space.unit").times(7)).items_center().padding_x(theme_length("space.spacing_unit").times(1.5)).flex_shrink(false).radius(theme_radius("sm"))
            .typography("control").text_color(theme_color("text_muted")).hover(style().background(theme_color("surface_hover")))
            .disabled(style().opacity(0.45)).focus_ring(px(2),px(2),theme_color("focus_ring"))))
        .accessibility_role("button").accessibility_label(clear_label).disabled(!clearable||!model.editable)
        .paint_transition(160,cubic_bezier(0.22,1.0,0.36,1.0));
    if clearable&&model.editable{button=button.on_click_value(Fn("clear"),());}
    let rtl=ctx.text_direction()=="rtl";let items=[];
    for i in 0..model.groups.len{items.push(group_view(ctx,model,model.groups[i],i,active_key,rtl));}
    let list=column(items).with_key("groups").with_style(ctx.component_style("groups",style().min_width(px(0))))
        .accessibility_role("group").accessibility_label(title).tab_stop(false)
        .on_key_value("up",Fn("move_row"),-1).on_key_value("down",Fn("move_row"),1)
        .on_key_value("home",Fn("move_row"),-288).on_key_value("end",Fn("move_row"),288)
        .on_key_value(if rtl{"right"}else{"left"},Fn("fold_focused"),false)
        .on_key_value(if rtl{"left"}else{"right"},Fn("fold_focused"),true);
    let header=row([short_label(title).with_style(ctx.component_style("title",style().typography("filter_title")
        .font_weight(600))).accessibility_role("heading").accessibility_level(2),button])
        .with_key("header").with_style(ctx.component_style("header",style().justify_between().items_center().gap(theme_length("space.spacing_unit").times(3))));
    let legends=row([hint_view(ctx,["up","down"],ctx.t_or("filter_menu.navigate","To navigate")),
        hint_view(ctx,["enter"],ctx.t_or("filter_menu.select","To select"))])
        .with_key("hints").with_style(ctx.component_style("hints",style().flex_wrap().justify_between()
            .gap(theme_length("space.spacing_unit").times(3)).typography("caption").text_color(theme_color("text_muted"))))
        .accessibility_hidden(true);
    let card=[header,chips_view(ctx,model,clean),list];
    if clean&&props.hints{card.push(legends);}
    let face=column(card).with_key("card").with_style(ctx.component_style("card",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(3))
        .radius(theme_radius("md")).background(theme_color("surface_raised"))));
    // Modern insets the title by 2px relative to the options and chip band.
    if preset!=()&&preset.family=="Modern"{face=face.audit_allow(["text-edge-misaligned"]);}
    let root=[face];
    if !clean&&props.hints{root.push(legends);}
    let surface=column(root).with_key(props.key).disabled(props.disabled)
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(420)).min_width(px(0))
            .typography("control").text_color(theme_color("text_primary"))));
    // The tray's keyboard legend and the inset card have their own gutters.
    if preset!=()&&preset.family=="Modern"{surface.audit_allow(["text-edge-misaligned"])}else{surface}
}
