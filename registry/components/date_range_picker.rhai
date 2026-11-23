/* gpui-rhai
{"id":"components/date_range_picker","export":"DateRangePicker","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/calendar","components/button","components/toggle_group"],"capabilities":{},"assets":["icons/calendar.svg","icons/disclosure_down.svg"],"tokens":["surface_raised","surface_hover","border","focus_ring","text_primary","text_muted","selection","accent","spacing.xs","spacing.sm","spacing.md","radius.sm","radius.lg","typography.control","typography.body","typography.caption","space.spacing_unit","space.unit"],"environment":["size","density"]}
*/
// DateRangePicker proposes an inclusive civil-date interval and a comparison only on Apply.
// State: an open-only range draft and the roving preset position. The Host owns committed dates and comparison mode.
// Example: DateRangePicker(#{key:"period",label:"Reporting period",value:#{start:"2026-10-01",end:"2026-10-08"},today:"2026-10-16"})
import "components/calendar" as calendar;
import "components/button" as button;
import "components/toggle_group" as toggle_group;

define_component(#{metadata:#{
    id:"components/date_range_picker","export":"DateRangePicker",version:"0.1.0",
    runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/calendar","components/button","components/toggle_group"],
    capabilities:#{},assets:["icons/calendar.svg","icons/disclosure_down.svg"],
    tokens:["surface_raised","surface_hover","border","focus_ring","text_primary","text_muted","selection","accent","spacing.xs","spacing.sm","spacing.md","radius.sm","radius.lg","typography.control","typography.body","typography.caption","space.spacing_unit","space.unit"],
    environment:["size","density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable picker and popover identity."},
        label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accessible name of the trigger and panel."},
        value:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            start:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Inclusive first ISO date."},
            end:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Inclusive last ISO date."},
        }}},required:false,sensitive:false,doc:"Controlled ascending range. Unset presents the last 30 days inside the available bounds."},
        today:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Anchor for Today and trailing presets. Defaults to the Host's local civil day."},
        min_date:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Earliest selectable day, inclusive."},
        max_date:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Latest selectable day, inclusive. Defaults to today."},
        first_weekday:#{schema:#{type:"optional",value:#{type:"string",allowed:["sunday","monday","tuesday","wednesday","thursday","friday","saturday"]}},required:false,sensitive:false,doc:"Overrides the locale's first weekday."},
        compare:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:true},doc:"Exposes the comparison selector and secondary calendar band."},
        compare_mode:#{schema:#{type:"string",allowed:["off","previous","year"]},required:false,sensitive:false,
            "default":#{type:"string",value:"off"},doc:"Controlled comparison mode. Store the returned mode together with the range."},
        presets:#{schema:#{type:"optional",value:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique preset key."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Localized preset label."},
            start:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Inclusive ISO start date."},
            end:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Inclusive ISO end date."},
        }}}},required:false,sensitive:false,doc:"Overrides the default list. An empty list hides presets."},
        disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Disables opening, editing and applying."},
        default_open:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Opens on first mount only."},
        revision:#{schema:#{type:"integer"},required:false,sensitive:false,"default":#{type:"integer",value:0},doc:"Change to invalidate a draft even when its controlled range is equal."},
        parent_overlay:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Parent overlay when opened from a dialog."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the complete range, compare mode and derived optional comparison on Apply."},
    },state:#{fields:#{
        began:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        open:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        draft:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        base:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        choosing_end:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        cursor:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    }},events:#{
        change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            range:#{schema:#{type:"object",allow_unknown:false,fields:#{
                start:#{schema:#{type:"string"},required:true,sensitive:false},
                end:#{schema:#{type:"string"},required:true,sensitive:false}}},required:true,sensitive:false},
            compare_mode:#{schema:#{type:"string",allowed:["off","previous","year"]},required:true,sensitive:false},
            comparison:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
                start:#{schema:#{type:"string"},required:true,sensitive:false},
                end:#{schema:#{type:"string"},required:true,sensitive:false}}}},required:true,sensitive:false},
        }},doc:"A complete inclusive range and optional derived comparison. Cancel and preset preview never emit."},
    },slots:#{},parts:["root","trigger","trigger_icon","trigger_value","trigger_comparison","panel","body","presets","presets_compact","preset","preset_compact","preset_selected","preset_custom","calendar",
        "footer","summary","count","comparison","compare_swatch","actions"],effects:["draft"]},
    render:Fn("render_DateRangePicker")
});

fn DateRangePicker(props){render_component("components/date_range_picker",props)}
fn opt(props,name){if name in props {props[name]}else{()}}
fn order_range(a,b){if a<=b{#{start:a,end:b}}else{#{start:b,end:a}}}
fn date_or(value,fallback){if value==(){fallback}else{value}}
fn range_text(ctx,value){
    let a=ctx.format_date(value.start,"medium");
    if value.end==value.start{a}else{`${a} – ${ctx.format_date(value.end,"medium")}`}
}
fn comparison_for(value,mode){
    if value==()||mode=="off" {return ();}
    let n=date_days_between(value.start,value.end)+1;
    let start=if mode=="previous"{date_checked_add_days(value.start,-n)}else{date_checked_add_months(value.start,-12)};
    let finish=if mode=="previous"{date_checked_add_days(value.start,-1)}else{date_checked_add_months(value.end,-12)};
    if start==()||finish==(){()}else{#{start:start,end:finish}}
}
fn clip_preset(key,label,start,finish,min,max){
    let invalid=start==()||finish==()||(min!=()&&finish<min)||start>max;
    let value=if invalid{()}else{#{start:date_clamp(start,min,max),end:date_clamp(finish,min,max)}};
    #{key:key,label:label,value:value}
}
fn trailing_label(ctx,days){
    let phrase=ctx.t_or("date_range.trailing","Last {days} days");
    phrase.replace("{days}",ctx.format_number(days));
    phrase
}
fn all_presets(ctx,props,today,min,max){
    let normal=[];
    let custom=opt(props,"presets");
    if custom!=() {
        let keys=#{};
        for preset in custom {
            let label=preset.label;label.trim();
            if preset.key==""||label==""||preset.key in keys{throw "DateRangePicker presets require unique, nonempty keys and labels";}
            keys[preset.key]=true;
            date_info(preset.start);date_info(preset.end);
            if preset.start>preset.end {throw "DateRangePicker preset start is after end";}
            normal.push(clip_preset(preset.key,label,preset.start,preset.end,min,max));
        }
        return normal;
    }
    let previous=date_checked_add_days(today,-1);
    normal.push(clip_preset("today",ctx.t_or("date_range.today","Today"),today,today,min,max));
    normal.push(clip_preset("yesterday",ctx.t_or("date_range.yesterday","Yesterday"),previous,previous,min,max));
    for days in [7,30,90] {
        normal.push(clip_preset(days.to_string(),trailing_label(ctx,days),
            date_or(date_checked_add_days(today,1-days),"0001-01-01"),today,min,max));
    }
    let month=date_month_start(today);let m=date_info(today).month-1;
    normal.push(clip_preset("month",ctx.t_or("date_range.month","This month"),month,today,min,max));
    normal.push(clip_preset("last-month",ctx.t_or("date_range.last_month","Last month"),
        date_checked_add_months(month,-1),date_checked_add_days(month,-1),min,max));
    normal.push(clip_preset("quarter",ctx.t_or("date_range.quarter","This quarter"),date_checked_add_months(month,-(m%3)),today,min,max));
    normal.push(clip_preset("ytd",ctx.t_or("date_range.ytd","Year to date"),date_checked_add_months(month,-m),today,min,max));
    normal
}
fn reset_draft(ctx,model){
    ctx.set_state("draft",#{range:model.value,mode:model.mode});
    ctx.set_state("base",model.base);
    ctx.set_state("choosing_end",false);
    ctx.set_state("cursor",0);
}
fn opened(model,ctx,open){
    if open&&!model.disabled{reset_draft(ctx,model);ctx.set_state("open",true);}
    else{ctx.set_state("open",false);}
    handled()
}
fn synchronize(ctx,model){
    if !ctx.get_state("began") {
        ctx.set_state("began",true);
        if model.default_open&&!model.disabled{opened(model,ctx,true);}
    }
    if model.disabled&&ctx.get_state("open"){ctx.set_state("open",false);}
    if ctx.get_state("open")&&ctx.get_state("base")!=model.base {reset_draft(ctx,model);}
}
fn cleanup(ctx,payload){}
fn current(ctx,base){ctx.get_state("open")&&ctx.get_state("base")==base}
fn choose_date(base,ctx,date){
    if !current(ctx,base){return handled();}
    let draft=ctx.get_state("draft");
    if ctx.get_state("choosing_end"){
        draft.range=order_range(draft.range.start,date);
        ctx.set_state("choosing_end",false);
    }else{
        draft.range=#{start:date,end:date};
        ctx.set_state("choosing_end",true);
    }
    ctx.set_state("draft",draft);
    handled()
}
fn choose_preset(base,ctx,item){
    if !current(ctx,base)||item.value==(){return handled();}
    let draft=ctx.get_state("draft");
    draft.range=item.value;
    ctx.set_state("draft",draft);
    ctx.set_state("cursor",item.index);
    ctx.set_state("choosing_end",false);
    handled()
}
fn move_preset(base,ctx,index){
    if current(ctx,base){
        ctx.set_state("cursor",index);
        ctx.scroll_into_view(`preset-${index}`);
    }
    handled()
}
fn change_comparison(base,ctx,values){
    if current(ctx,base)&&values.len==1{
        let draft=ctx.get_state("draft");draft.mode=values[0];ctx.set_state("draft",draft);
    }
}
fn cancel(ctx,payload){ctx.set_state("open",false);handled()}
fn apply(model,ctx,payload){
    if !current(ctx,model.base)||ctx.get_state("choosing_end"){return handled();}
    let draft=ctx.get_state("draft");
    if draft.range.start>draft.range.end||(model.min!=()&&draft.range.start<model.min)||draft.range.end>model.max{return handled();}
    let mode=if model.compare{draft.mode}else{"off"};
    ctx.emit("change",#{range:draft.range,compare_mode:mode,comparison:comparison_for(draft.range,mode)});
    ctx.set_state("open",false);
    handled()
}

fn same_range(a,b){a!=()&&b!=()&&a.start==b.start&&a.end==b.end}
fn preset_strip(ctx,model,choices,draft,compact){
    let entries=[];let eligible=[];let selected=-1;
    let cursor=ctx.get_state("cursor");
    for i in 0..choices.len{
        let preset=choices[i];
        if preset.value!=(){eligible.push(i);}
        if selected<0&&same_range(preset.value,draft.range)&&!ctx.get_state("choosing_end"){selected=i;}
    }
    if !(cursor in eligible)&&eligible.len>0{cursor=eligible[0];}
    for i in 0..choices.len {
        let item=choices[i];
        let ref_key=`preset-${i}`;
        let s=style().items_center().height(theme_length("space.unit").times(7.5)).padding_x(theme_length("space.spacing_unit").times(2.5)).min_width(px(0)).radius(theme_radius("sm"))
            .typography("control").text_color(theme_color("text_muted")).border(px(1)).border_color(rgba(0x00000000))
            .hover(style().background(theme_color("surface_hover")));
        s=ctx.component_style("preset",s);
        if compact{s=ctx.component_style("preset_compact",s.flex_shrink(false).padding_x(theme_length("space.spacing_unit").times(3)).radius(theme_length("space.unit").times(249.75)));}
        if i==selected{s=ctx.component_style("preset_selected",s.background(theme_color("selection")).text_color(theme_color("text_primary")));}
        if i==cursor{s=s.group_focus(style().border_color(theme_color("focus_ring")));}
        entries.push(row([text(item.label).with_style(style().whitespace_nowrap().text_ellipsis().min_width(px(0)))])
            .with_key(item.key).with_ref(element_ref(ref_key)).with_style(s)
            .accessibility_role("option").accessibility_label(item.label).accessibility_selected(i==selected)
            .on_click_value(Fn("choose_preset").curry(model.base),#{index:i,value:item.value})
            .tab_stop(false).disabled(item.value==()));
    }
    if !compact && selected<0{
        entries.push(row([text(ctx.t_or("date_range.custom","Custom range"))])
            .with_style(ctx.component_style("preset_custom",style().items_center().min_height(theme_length("space.unit").times(7.5))
                .padding_x(theme_length("space.spacing_unit").times(2.5)).typography("control").text_color(theme_color("text_muted"))))
            .accessibility_hidden(true));
    }
    let style=if compact{style().gap(theme_spacing("xs")).width(relative(1.0)).min_width(px(0))
        .overflow_x_scroll().padding(theme_length("space.spacing_unit").times(2)).border_bottom(px(1)).border_color(theme_color("border"))}
        else{style().width(theme_length("space.unit").times(34)).flex_shrink(false).padding(theme_length("space.spacing_unit").times(1.5)).gap(theme_length("space.spacing_unit").times(0.25))
            .border_end(px(1)).border_color(theme_color("border"))};
    let group=(if compact{row(entries)}else{column(entries)}).with_key("presets")
        .with_style(ctx.component_style(if compact{"presets_compact"}else{"presets"},style.focus(style())))
        .tab_stop(eligible.len>0).accessibility_role("listbox").accessibility_label(ctx.t_or("date_range.presets","Date presets"));
    if eligible.len==0{return group;}
    let position=0;for i in 0..eligible.len{if eligible[i]==cursor{position=i;}}
    let previous=eligible[(position+eligible.len-1)%eligible.len];let next=eligible[(position+1)%eligible.len];
    group.on_key_value(if compact{"left"}else{"up"},Fn("move_preset").curry(model.base),previous)
        .on_key_value(if compact{"right"}else{"down"},Fn("move_preset").curry(model.base),next)
        .on_key_value("home",Fn("move_preset").curry(model.base),eligible[0])
        .on_key_value("end",Fn("move_preset").curry(model.base),eligible[eligible.len-1])
        .on_key_value("enter",Fn("choose_preset").curry(model.base),#{index:cursor,value:choices[cursor].value});
}
fn range_footer(ctx,model,draft,comparison,partial,compact){
    let fields=[row([
        text(range_text(ctx,draft.range)).with_style(style().min_width(px(0)).text_ellipsis().whitespace_nowrap()),
        text(if partial {ctx.t_or("date_range.choose_end","Choose an end date")}else{
            let phrase=ctx.t_or("date_range.day_count","{days} days");
            phrase.replace("{days}",ctx.format_number(date_days_between(draft.range.start,draft.range.end)+1));
            phrase
        }).with_style(ctx.component_style("count",style().typography("caption").text_color(theme_color("text_muted")))),
    ]).with_style(ctx.component_style("summary",style().gap(theme_spacing("xs")).items_center().flex_wrap().typography("control").font_weight(500)))];
    if model.compare{
        let compare_label=ctx.t_or("date_range.compare","Compare to");
        fields.push(row([text(compare_label).with_style(style().typography("caption").text_color(theme_color("text_muted"))),
            toggle_group::ToggleGroup(#{key:"comparison-mode",label:compare_label,values:[draft.mode],allow_empty:false,size:"sm",
                items:[#{value:"off",label:ctx.t_or("date_range.off","Off")},
                    #{value:"previous",label:ctx.t_or("date_range.previous","Previous period")},
                    #{value:"year",label:ctx.t_or("date_range.year","Previous year")}],
                on_change:Fn("change_comparison").curry(model.base)})])
            .with_style(style().items_center().gap(theme_spacing("xs")).flex_wrap().min_width(px(0))));
    }
    if comparison!=(){
        fields.push(row([row([]).with_style(ctx.component_style("compare_swatch",style().width(px(14)).height(px(8))
            .radius(px(3)).background(alpha(theme_color("accent"),0.20)))).accessibility_hidden(true),
            text(range_text(ctx,comparison)).with_style(style().min_width(px(0)).text_ellipsis().whitespace_nowrap())])
            .with_style(ctx.component_style("comparison",style().items_center().gap(theme_spacing("xs")).typography("caption").text_color(theme_color("text_muted")))));
    }
    let summary=column(fields).with_style(style().gap(theme_spacing("xs")).min_width(px(0)));
    let actions=row([
        button::Button(#{key:"cancel",text:ctx.t_or("common.cancel","Cancel"),variant:"ghost",on_click:Fn("cancel")}),
        button::Button(#{key:"apply",text:ctx.t_or("time.apply","Apply"),variant:"primary",disabled:partial,
            on_click:Fn("apply").curry(model)}),
    ]).with_style(ctx.component_style("actions",style().items_center().justify_end().gap(theme_spacing("xs")).flex_shrink(false)));
    let alignment=if compact{style()}else{style().items_end().justify_between()};
    (if compact{column([summary,actions])}else{row([summary,actions])})
        .with_style(ctx.component_style("footer",alignment.gap(theme_spacing("sm"))
            .padding(theme_length("space.spacing_unit").times(2.5)).border_top(px(1)).border_color(theme_color("border"))))
}

fn render_DateRangePicker(ctx,props){
    let today=date_or(opt(props,"today"),ctx.today());date_info(today);
    let min=opt(props,"min_date");let max=date_or(opt(props,"max_date"),today);
    if min!=(){date_info(min);}date_info(max);
    if min!=()&&min>max{throw "DateRangePicker min_date cannot be after max_date";}
    let value=opt(props,"value");
    if value==(){
        let end=date_clamp(today,min,max);
        value=#{start:date_clamp(date_or(date_checked_add_days(end,-29),"0001-01-01"),min,max),end:end};
    } else {
        date_info(value.start);date_info(value.end);
        if value.start>value.end||(min!=()&&value.start<min)||value.end>max{throw "DateRangePicker controlled interval is outside bounds or reversed";}
    }
    let mode=if props.compare{props.compare_mode}else{"off"};
    let base=`${value.start}|${value.end}|${mode}|${props.revision}|${min}|${max}|${props.compare}`;
    let model=#{base:base,value:value,mode:mode,min:min,max:max,disabled:props.disabled,compare:props.compare,default_open:props.default_open};
    effect("draft",[model,ctx.get_state("open")],Fn("synchronize_effect"),Fn("cleanup"));
    let open=ctx.get_state("open")&&!props.disabled;
    let displayed=range_text(ctx,value);
    let committed_comparison=comparison_for(value,mode);
    let lines=[text(displayed).with_style(ctx.component_style("trigger_value",style().min_width(px(0)).font_weight(500)
        .text_ellipsis().whitespace_nowrap()))];
    let accessible_value=displayed;
    if committed_comparison!=(){
        let phrase=ctx.t_or("date_range.versus","vs. {range}");
        phrase.replace("{range}",range_text(ctx,committed_comparison));
        lines.push(text(phrase).with_style(ctx.component_style("trigger_comparison",style().min_width(px(0)).typography("caption")
            .text_color(theme_color("text_muted")).text_ellipsis().whitespace_nowrap())));
        accessible_value+=` ${phrase}`;
    }
    let trigger=row([
        date_icon(ctx,"app/icons/calendar"),
        column(lines).with_style(style().min_width(px(0))),
        date_icon(ctx,"app/icons/disclosure_down"),
    ]).with_key("trigger").with_style(ctx.component_style("trigger",style().gap(theme_spacing("xs")).items_center().min_width(px(0))
        .min_height(theme_length("space.unit").times(8.5)).padding_x(theme_length("space.spacing_unit").times(2.5)).radius(theme_radius("sm")).typography("control")
        .border(px(1)).border_color(theme_color("border")).background(theme_color("surface_raised"))
        .focus_ring(px(2),px(2),theme_color("focus_ring"))))
        .accessibility_role("combobox").accessibility_label(props.label).accessibility_value(accessible_value).accessibility_expanded(open)
        .tab_stop(!props.disabled).disabled(props.disabled);
    let panel=row([]);
    if open{
        let stale=ctx.get_state("base")!=base;
        let draft=if stale{#{range:value,mode:mode}}else{ctx.get_state("draft")};
        let partial=!stale&&ctx.get_state("choosing_end");
        let comparison=if partial{()}else{comparison_for(draft.range,draft.mode)};
        let panel_ref=element_ref("panel");
        let width_class=ctx.element_width_class(panel_ref,[600]);
        let compact=if width_class==(){ctx.viewport_class()=="compact"}else{width_class==0};
        let presets=all_presets(ctx,props,today,min,max);
        let calendar_view=calendar::Calendar(#{
            key:"range-calendar",label:ctx.t_or("date_range.calendar","Range calendar"),mode:"range",range_bands:true,months:if compact{1}else{2},
            value:draft.range.start,end:if partial{()}else{draft.range.end},
            month_anchor:if compact{draft.range.end}else{draft.range.start},
            comparison_start:if comparison==(){()}else{comparison.start},comparison_end:if comparison==(){()}else{comparison.end},
            today:today,min_date:min,max_date:max,first_weekday:opt(props,"first_weekday"),on_change:Fn("choose_date").curry(base),
        }).with_style(ctx.component_style("calendar",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2.5)).flex_grow().flex_basis(px(0))));
        let choices=[];if presets.len>0{choices.push(preset_strip(ctx,model,presets,draft,compact));}choices.push(calendar_view);
        let body=if compact{column(choices)}else{row(choices)};
        panel=column([column([body,range_footer(ctx,model,draft,comparison,partial||stale,compact)])
            .with_style(ctx.component_style("body",style().min_width(px(0)).background(theme_color("surface_raised")).radius(theme_radius("lg"))))])
            .with_key("panel").with_ref(panel_ref)
            .with_style(ctx.component_style("panel",style().width(px(652)).max_width(relative(1.0)).min_width(px(0)).padding(theme_length("space.spacing_unit"))
                .max_height(relative(1.0)).overflow_y_scroll()
                .radius(theme_radius("lg")).background(theme_color("surface_raised")).text_color(theme_color("text_primary"))))
            .accessibility_role("dialog").accessibility_label(props.label);
    }
    let config=#{id:props.key,kind:"popover",open:open,placement:"bottom",align:"start",gap:8,modal:false,
        initial_focus:"first",dismiss_on_escape:true,dismiss_on_outside:true};
    if opt(props,"parent_overlay")!=(){config.parent=props.parent_overlay;}
    overlay(trigger,panel,config).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0))))
        .on_open_change(Fn("opened").curry(model))
        .motion(motion_transition("overlay_reveal",0.0,1.0,#{duration_ms:260,easing:cubic_bezier(0.22,1.0,0.36,1.0)}))
        .motion_on_change(if open{"open"}else{"closed"});
}
fn synchronize_effect(ctx,data){synchronize(ctx,data[0]);}
fn date_icon(ctx,id){
    image_source(asset(id)).with_style(ctx.component_style("trigger_icon",style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4))
        .flex_shrink(false).text_color(theme_color("text_muted")))).accessibility_hidden(true)
}
