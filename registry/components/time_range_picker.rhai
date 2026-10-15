/* gpui-rhai
{"id":"components/time_range_picker","export":"TimeRangePicker","version":"0.2.2","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/calendar","components/time_input","components/input","components/button"],"capabilities":{},"tokens":["surface_raised","surface_hover","border","focus_ring","text_primary","text_muted","text.danger","selection","metrics.control","metrics.field_pad","metrics.inset","spacing.xs","spacing.sm","spacing.lg","radius.md","radius.lg","typography.control","typography.caption","typography.body"],"environment":["size","density","corners"]}
*/
import "components/calendar" as calendar;
import "components/time_input" as time_input;
import "components/input" as input;
import "components/button" as button;
// The controlled value changes only through presets, shifts, or Apply. A draft never queries.
define_component(#{metadata:#{id:"components/time_range_picker","export":"TimeRangePicker",version:"0.2.2",
    runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/calendar","components/time_input","components/input","components/button"],capabilities:#{},
    tokens:["surface_raised","surface_hover","border","focus_ring","text_primary","text_muted","text.danger","selection","metrics.control","metrics.field_pad","metrics.inset","spacing.xs","spacing.sm","spacing.lg","radius.md","radius.lg","typography.control","typography.caption","typography.body"],environment:["size","density","corners"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable picker and overlay identity."},
        label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accessible picker name."},
        value:#{schema:#{type:"one_of",variants:[
            #{type:"object",allow_unknown:false,fields:#{kind:#{schema:#{type:"string",allowed:["relative"]},required:true,sensitive:false,doc:"Relative range tag."},duration_ms:#{schema:#{type:"integer",min:1},required:true,sensitive:false,doc:"Trailing duration resolved by the caller at query time."}}},
            #{type:"object",allow_unknown:false,fields:#{kind:#{schema:#{type:"string",allowed:["absolute"]},required:true,sensitive:false,doc:"Absolute range tag."},start:#{schema:#{type:"integer"},required:true,sensitive:false,doc:"Inclusive start instant in epoch milliseconds."},end:#{schema:#{type:"integer"},required:true,sensitive:false,doc:"End instant in epoch milliseconds, after start."}}}]},required:true,sensitive:false,doc:"Controlled relative duration or absolute epoch-millisecond range."},
        presets:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Localized preset text."},
            duration_ms:#{schema:#{type:"integer",min:1},required:true,sensitive:false,doc:"Positive trailing duration in milliseconds."}}}},required:false,sensitive:false,"default":#{type:"array",value:[]},doc:"Quick relative presets; unset uses localized 15 minute, hour, day and week choices."},
        time_zone:#{schema:#{type:"string"},required:false,sensitive:false,"default":#{type:"string",value:"local"},doc:"local, UTC, offset:<minutes>, or IANA zone shared with charts."},
        precision:#{schema:#{type:"string",allowed:["minute","second"]},required:false,sensitive:false,"default":#{type:"string",value:"minute"},doc:"Native time field precision."},
        min:#{schema:#{type:"optional",value:#{type:"integer"}},required:false,sensitive:false,doc:"Earliest allowed absolute epoch instant, inclusive."},
        max:#{schema:#{type:"optional",value:#{type:"integer"}},required:false,sensitive:false,doc:"Latest allowed absolute epoch instant, inclusive; unset permits future instants."},
        max_span_ms:#{schema:#{type:"optional",value:#{type:"integer",min:1}},required:false,sensitive:false,doc:"Maximum duration for either range kind."},
        zone_switch:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Offers local and UTC draft presentation; does not mutate the controlled range."},
        shift_buttons:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Offers previous/next windows shifted by their span, as absolute snapshots."},
        disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Disables the picker."},
        parent_overlay:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Parent overlay identity when nested in a dialog."},
        size:#{schema:#{type:"optional",value:#{type:"string",allowed:["xs","sm","md","lg"]}},required:false,sensitive:false,doc:"Explicit size or inherited size."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called with an accepted relative preset or complete validated absolute range."},
    },state:#{fields:#{
        open:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        start_date:#{schema:#{type:"string"},"default":#{type:"string",value:""}},end_date:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        start_time:#{schema:#{type:"string"},"default":#{type:"string",value:""}},end_time:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        zone:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        start_choice:#{schema:#{type:"optional",value:#{type:"integer"}},"default":#{type:"null"}},end_choice:#{schema:#{type:"optional",value:#{type:"integer"}},"default":#{type:"null"}},
        selecting_end:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        preset_index:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    }},events:#{change:#{payload:#{type:"ui_value"},doc:"A validated controlled range proposal; no event fires for half-edited drafts or cancel."}},slots:#{},
    parts:["root","trigger","trigger_value","panel","presets","preset","calendar","fields","field","zone","error","actions","focus_frame"]},render:Fn("render_TimeRangePicker")});
fn TimeRangePicker(props){render_component("components/time_range_picker",props)}
fn option(props,name){if name in props&&props[name]!=(){props[name]}else{()}}
fn clock_style(precision){if precision=="second"{"medium"}else{"short"}}
fn reset_draft(ctx,value,zone,precision){
    let resolved=time_range_resolve(value,ctx.now());let start=time_parts(resolved.start,zone);let finish=time_parts(resolved.end,zone);
    ctx.set_state("start_date",start.date);ctx.set_state("end_date",finish.date);
    ctx.set_state("start_time",ctx.format_time(resolved.start,clock_style(precision),zone));
    ctx.set_state("end_time",ctx.format_time(resolved.end,clock_style(precision),zone));
    ctx.set_state("start_choice",resolved.start);ctx.set_state("end_choice",resolved.end);
    ctx.set_state("zone",zone);ctx.set_state("selecting_end",false);
}
fn opened(value,zone,precision,ctx,open){ctx.set_state("open",open);if open{reset_draft(ctx,value,zone,precision);}handled()}
fn edited(field,ctx,value){ctx.set_state(field,value);ctx.set_state(if field=="start_date"||field=="start_time"{"start_choice"}else{"end_choice"},());}
fn calendar_changed(ctx,date){
    if !ctx.get_state("selecting_end")||date<ctx.get_state("start_date"){
        ctx.set_state("start_date",date);ctx.set_state("end_date",date);ctx.set_state("selecting_end",true);
    }else{ctx.set_state("end_date",date);ctx.set_state("selecting_end",false);}
    ctx.set_state("start_choice",());ctx.set_state("end_choice",());
}
fn chosen_instant(field,ctx,instant){ctx.set_state(field,instant);}
fn apply_range(ctx,value){ctx.emit("change",value);ctx.set_state("open",false);handled()}
fn apply_button(value,ctx,payload){apply_range(ctx,value)}
fn occurrence_button(field,instant,ctx,payload){chosen_instant(field,ctx,instant);}
fn preset_cursor(ctx,index){ctx.set_state("preset_index",index);handled()}
fn switch_zone(value,precision,zone,ctx,payload){reset_draft(ctx,value,zone,precision);handled()}
fn last_label(ctx,duration){let label=ctx.t("time.last");label.replace("{value}",ctx.format_duration(duration));label}
fn valid_date_or(value,fallback){try{date_info(value);return value;}catch(error){return fallback;}}
fn parse_draft(ctx,start_date,start_time,end_date,end_time,zone,precision,start_choice,end_choice,min,max,span){
    let start_parsed=ctx.parse_time(start_time,precision);let end_parsed=ctx.parse_time(end_time,precision);
    let result=#{value:(),error:"",start_candidates:[],end_candidates:[]};
    if start_parsed==()||end_parsed==(){result.error=ctx.t("time.invalid");return result;}
    try{result.start_candidates=time_local_candidates(start_date,start_parsed,zone);result.end_candidates=time_local_candidates(end_date,end_parsed,zone);}
    catch(error){result.error=ctx.t("time.invalid");return result;}
    if result.start_candidates.len==0||result.end_candidates.len==0{result.error=ctx.t("time.gap");return result;}
    let start=if result.start_candidates.len==1{result.start_candidates[0]}else if start_choice in result.start_candidates{start_choice}else{()};
    let finish=if result.end_candidates.len==1{result.end_candidates[0]}else if end_choice in result.end_candidates{end_choice}else{()};
    if start==()||finish==(){result.error=ctx.t("time.fold");return result;}
    if finish<=start{result.error=ctx.t("time.order");return result;}
    let value=#{kind:"absolute",start:start,end:finish};
    if time_range_error(value,min,max,span)!=""{result.error=ctx.t("time.bounds");return result;}
    result.value=value;result
}
fn occurrence_choices(ctx,label,candidates,choice,field,zone){
    let nodes=[];if candidates.len>1{for instant in candidates{nodes.push(button::Button(#{key:`${field}-${instant}`,
        text:ctx.format_time(instant,"long",zone),variant:if instant==choice{"secondary"}else{"outline"},
        on_click:Fn("occurrence_button").curry(field,instant)})
        .accessibility_label(`${label}, ${ctx.t("time.fold")}, ${ctx.format_time(instant,"long",zone)}`));}}
    column(nodes).with_style(style().gap(theme_spacing("xs")))
}
fn draft_field(ctx,props,label,date_field,time_field,candidates,zone){
    let date=ctx.get_state(date_field);let clock=ctx.get_state(time_field);let choice_field=if date_field=="start_date"{"start_choice"}else{"end_choice"};
    column([text(label).with_style(style().typography("body")),
        input::Input(#{key:`${props.key}-${date_field}`,label:`${label} ${ctx.t("time.custom")}`,value:date,
            on_change:Fn("edited").curry(date_field)}),
        time_input::TimeInput(#{key:`${props.key}-${time_field}`,label:label,value:clock,precision:props.precision,
            on_change:Fn("edited").curry(time_field)}),
        occurrence_choices(ctx,label,candidates,ctx.get_state(choice_field),choice_field,zone)])
        .with_style(ctx.component_style("field",style().gap(theme_spacing("xs"))))
}
fn picker_presets(ctx,props,presets){
    let eligible=[];for i in 0..presets.len{if time_range_error(#{kind:"relative",duration_ms:presets[i].duration_ms},(),(),option(props,"max_span_ms"))==""{eligible.push(i);}}
    let index=ctx.get_state("preset_index");if !(index in eligible)&&eligible.len>0{index=eligible[0];}let rows=[];
    for i in 0..presets.len{let preset=presets[i];let value=#{kind:"relative",duration_ms:preset.duration_ms};
        let invalid=time_range_error(value,(),(),option(props,"max_span_ms"))!="";
        let preset_style=style().min_height(theme_length("metrics.control"))
                .padding_x(theme_length("metrics.inset")).items_center().typography("control")
                .background(if i==index{theme_color("selection")}else{rgba(0x00000000)})
                .border(px(2)).border_color(rgba(0x00000000));
        if i==index{preset_style=preset_style.group_focus(style().border_color(theme_color("focus_ring")));}
        rows.push(row([text(preset.label)]).with_key(`preset-${preset.duration_ms}`)
            .with_style(ctx.component_style("preset",preset_style))
            .disabled(invalid).accessibility_role("option").accessibility_label(preset.label)
            .accessibility_selected(props.value.kind=="relative"&&props.value.duration_ms==preset.duration_ms)
            .on_click_value(Fn("apply_range"),value).tab_stop(false));
    }
    let group=column(rows).with_key("presets").with_style(ctx.component_style("presets",style().gap(theme_spacing("xs")).focus(style())))
        .tab_stop(true).accessibility_role("listbox").accessibility_label(ctx.t("time.presets"));
    if eligible.len>0{let position=0;for i in 0..eligible.len{if eligible[i]==index{position=i;}}
        group=group.on_key_value("up",Fn("preset_cursor"),eligible[if position>0{position-1}else{eligible.len-1}])
        .on_key_value("down",Fn("preset_cursor"),eligible[if position+1<eligible.len{position+1}else{0}]);
        let selected=#{kind:"relative",duration_ms:presets[index].duration_ms};
        if time_range_error(selected,(),(),option(props,"max_span_ms"))==""{group=group.on_key_value("enter",Fn("apply_range"),selected);}}
    group
}
fn render_TimeRangePicker(ctx,props){
    let min=option(props,"min");let max=option(props,"max");let span=option(props,"max_span_ms");
    let error=time_range_error(props.value,min,max,span);if error!=""{throw `TimeRangePicker controlled range: ${error}`;}
    let resolved=time_range_resolve(props.value,ctx.now());let parts=time_parts(resolved.start,props.time_zone);
    let zone_label=if props.time_zone=="local"{`${ctx.t("time.local")} UTC${parts.zone}`}else{props.time_zone};
    let presets=props.presets;if presets.len==0{for duration in [900000,3600000,86400000,604800000]{
        presets.push(#{label:last_label(ctx,duration),duration_ms:duration});}}
    let preset_keys=#{};for preset in presets{let label=preset.label;label.trim();
        if label==""{throw "TimeRangePicker preset label cannot be blank";}
        let key=`${preset.duration_ms}`;if key in preset_keys{throw "TimeRangePicker preset durations must be unique";}preset_keys[key]=true;}
    let display=if props.value.kind=="relative"{last_label(ctx,props.value.duration_ms)}
        else{`${ctx.format_time(resolved.start,"long",props.time_zone)} – ${ctx.format_time(resolved.end,"long",props.time_zone)}`};
    for preset in presets{if props.value.kind=="relative"&&preset.duration_ms==props.value.duration_ms{display=preset.label;}}
    let open=ctx.get_state("open");
    let trigger=row([text(`${display} · ${zone_label}`).with_style(ctx.component_style("trigger_value",style()))])
        .with_key(`${props.key}-trigger`).with_style(ctx.component_style("trigger",style().min_height(theme_length("metrics.control"))
            .padding_x(theme_length("metrics.field_pad")).items_center().border(px(2)).border_color(theme_color("border"))
            .radius(theme_radius("md")).background(theme_color("surface_raised")).typography("control")
            .focus(style().border_color(theme_color("focus_ring")))))
        .disabled(props.disabled).tab_stop(true).accessibility_role("combobox").accessibility_label(props.label).accessibility_expanded(open)
        .accessibility_value(`${ctx.format_time(resolved.start,"long",props.time_zone)} – ${ctx.format_time(resolved.end,"long",props.time_zone)} ${zone_label}`);
    let zone=ctx.get_state("zone");if zone==""{zone=props.time_zone;}
    let draft=parse_draft(ctx,ctx.get_state("start_date"),ctx.get_state("start_time"),ctx.get_state("end_date"),ctx.get_state("end_time"),zone,props.precision,
        ctx.get_state("start_choice"),ctx.get_state("end_choice"),min,max,span);
    let date_min=if min==(){()}else{time_parts(min,zone).date};let date_max=if max==(){()}else{time_parts(max,zone).date};
    let dates=[];
    if open{dates.push(calendar::Calendar(#{key:`${props.key}-calendar`,label:ctx.t("time.custom"),mode:"range",months:2,
        value:valid_date_or(ctx.get_state("start_date"),time_parts(resolved.start,zone).date),
        end:if ctx.get_state("selecting_end"){()}else{valid_date_or(ctx.get_state("end_date"),time_parts(resolved.end,zone).date)},
        min_date:date_min,max_date:date_max,today:time_parts(ctx.now(),zone).date,on_change:Fn("calendar_changed")})
        .with_style(ctx.component_style("calendar",style())).tab_stop(false));}
    let custom=column(dates+[row([
        draft_field(ctx,props,ctx.t("time.start"),"start_date","start_time",draft.start_candidates,zone),
        draft_field(ctx,props,ctx.t("time.end"),"end_date","end_time",draft.end_candidates,zone)])
        .with_style(ctx.component_style("fields",style().gap(theme_spacing("sm")).items_start())),
        text(draft.error).with_style(ctx.component_style("error",style().typography("caption").text_color(theme_color("text.danger"))))
            .accessibility_role("status"),
        button::Button(#{key:`${props.key}-apply`,text:ctx.t("time.apply"),variant:"primary",disabled:draft.value==(),
            on_click:Fn("apply_button").curry(draft.value)})])
        .with_style(style().gap(theme_spacing("sm")));
    let draft_offset=time_parts(if draft.value==(){resolved.start}else{draft.value.start},zone).zone;
    let zones=[text(`${ctx.t("time.zone")}: ${zone} UTC${draft_offset}`).with_style(ctx.component_style("zone",style().typography("caption")))];
    if props.zone_switch{for candidate in ["local","UTC"]{zones.push(button::Button(#{key:`zone-${candidate}`,text:if candidate=="local"{ctx.t("time.local")}else{"UTC"},variant:"ghost",
        on_click:Fn("switch_zone").curry(if draft.value==(){props.value}else{draft.value},props.precision,candidate)}));}}
    let panel=column([row(zones).with_style(style().items_center().gap(theme_spacing("xs"))),
        row([picker_presets(ctx,props,presets),custom]).with_style(style().items_start().gap(theme_spacing("lg")))])
        .with_key(`${props.key}-panel`).with_style(ctx.component_style("panel",style().padding(theme_length("metrics.inset"))
            .gap(theme_spacing("sm")).border(px(1)).border_color(theme_color("border")).radius(theme_radius("lg"))
            .background(theme_color("surface_raised")).text_color(theme_color("text_primary"))))
        .accessibility_role("dialog").accessibility_label(props.label);
    let config=#{id:props.key,kind:"popover",open:open,placement:"bottom",align:"start",gap:4,modal:false,
        initial_focus:"first",dismiss_on_escape:true,dismiss_on_outside:true};
    if option(props,"parent_overlay")!=(){config.parent=props.parent_overlay;}
    let picker=overlay(trigger,panel,config).with_key(props.key).on_open_change(Fn("opened").curry(props.value,props.time_zone,props.precision));
    let nodes=[picker];if props.shift_buttons{let width=resolved.end-resolved.start;
        for direction in [-1,1]{let shifted=#{kind:"absolute",start:resolved.start+direction*width,end:resolved.end+direction*width};
            nodes.push(button::Button(#{key:`${props.key}-shift-${direction}`,text:if direction<0{"‹"}else{"›"},variant:"ghost",
                disabled:time_range_error(shifted,min,max,span)!="",on_click:Fn("apply_button").curry(shifted)})
                .accessibility_label(ctx.t(if direction<0{"time.previous"}else{"time.next"})));}}
    let root=row(nodes).with_style(ctx.component_style("root",style().gap(theme_spacing("xs")).items_center())).disabled(props.disabled);
    if "size" in props&&props.size!=(){root.env(#{size:props.size})}else{root}
}
