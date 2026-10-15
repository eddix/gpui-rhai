/* gpui-rhai
{"id":"components/calendar","export":"Calendar","version":"0.2.2","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"tokens":["surface_hover","accent","accent_hover","on_accent","text.accent","text_primary","text_muted","disabled","focus_ring","selection","metrics.control","metrics.marker","spacing.xs","spacing.sm","radius.md","typography.control","typography.label","typography.body"],"environment":["size","density","corners"]}
*/
// The single calendar engine shared by DatePicker and TimeRangePicker.
define_component(#{metadata:#{id:"components/calendar","export":"Calendar",version:"0.2.2",
    runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{},
    tokens:["surface_hover","accent","accent_hover","on_accent","text.accent","text_primary","text_muted","disabled","focus_ring","selection","metrics.control","metrics.marker","spacing.xs","spacing.sm","radius.md","typography.control","typography.label","typography.body"],environment:["size","density","corners"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable calendar identity."},
        label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accessible calendar name."},
        value:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Controlled selected ISO date, or range start."},
        end:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Controlled range end, inclusive; unset while choosing the end."},
        today:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Presentation-zone ISO today; unset follows ctx.today and the Host clock."},
        mode:#{schema:#{type:"string",allowed:["single","range"]},required:false,sensitive:false,"default":#{type:"string",value:"single"},doc:"Single date or inclusive date range."},
        months:#{schema:#{type:"integer",min:1,max:2},required:false,sensitive:false,"default":#{type:"integer",value:1},doc:"One or two consecutive calendar months."},
        min_date:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Inclusive earliest ISO date."},
        max_date:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Inclusive latest ISO date."},
        disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Disables calendar input."},
        size:#{schema:#{type:"optional",value:#{type:"string",allowed:["xs","sm","md","lg"]}},required:false,sensitive:false,doc:"Inherited control size unless explicitly set."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called with the chosen ISO date; range ownership stays with the caller."},
    },state:#{fields:#{
        focused:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        month:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        hover:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        anchor:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
    }},events:#{change:#{payload:#{type:"string"},doc:"Emitted when a day is activated by pointer or keyboard."}},slots:#{},
    parts:["root","header","month_title","previous","next","weekdays","weekday","grid","week","day","day_outside","day_today","day_selected","day_focused","day_disabled","day_range"],effects:["anchor"]},
    render:Fn("render_Calendar")});
fn Calendar(props){render_component("components/calendar",props)}
fn enabled(date,min_date,max_date){date!=()&&(min_date==()||date>=min_date)&&(max_date==()||date<=max_date)}
fn option(props,name){if name in props&&props[name]!=(){props[name]}else{()}}
fn weekday_index(name){if name=="monday"{1}else if name=="tuesday"{2}else if name=="wednesday"{3}else if name=="thursday"{4}else if name=="friday"{5}else if name=="saturday"{6}else{0}}
fn day_target(value,days,min_date,max_date){let target=date_checked_add_days(value,days);if target==(){value}else{date_clamp(target,min_date,max_date)}}
fn month_target(value,months,min_date,max_date){let target=date_checked_add_months(value,months);if target==()||!date_month_intersects(target,min_date,max_date){value}else{date_clamp(target,min_date,max_date)}}
fn week_target(value,first,end,min_date,max_date){let target=date_week_edge(value,first,end);if target==(){value}else{date_clamp(target,min_date,max_date)}}
fn navigated(ctx,date){ctx.set_state("focused",date);ctx.set_state("month",date_month_start(date));handled()}
fn chosen(ctx,date){ctx.set_state("focused",date);ctx.emit("change",date);handled()}
fn hovered(ctx,payload){ctx.set_state("hover",if payload.hovered{payload.value}else{""});}
fn anchor_changed(ctx,value){ctx.set_state("anchor",value);ctx.set_state("focused",value);ctx.set_state("month",date_month_start(value));}
fn anchor_cleanup(ctx,value){}
fn control_style(){style().width(theme_length("metrics.control")).height(theme_length("metrics.control"))
    .items_center().justify_center().border(px(2)).border_color(rgba(0x00000000)).radius(theme_radius("md"))
    .typography("control").hover(style().background(theme_color("surface_hover")))
    .focus(style().border_color(theme_color("focus_ring")))}
fn calendar_month(ctx,props,month,focus,start,finish,min_date,max_date){
    let calendar=ctx.calendar();let first=weekday_index(calendar.first_weekday);let weekday_nodes=[];
    for offset in 0..7{let label=calendar.weekdays.short[(first+offset)%7];weekday_nodes.push(row([text(label.to_upper())])
        .with_style(ctx.component_style("weekday",style().width(theme_length("metrics.control")).height(theme_length("metrics.marker"))
            .items_center().justify_center().typography("label").text_color(theme_color("text_muted"))))
        .accessibility_role("columnheader").accessibility_label(label));}
    let cells=date_month_grid(month,calendar.first_weekday);let weeks=[];
    let hover=ctx.get_state("hover");let range_end=if finish==()&&hover!=""{hover}else{finish};
    for w in 0..6{let days=[];for d in 0..7{
        let cell=cells[w*7+d];let date=cell.date;let disabled=!enabled(date,min_date,max_date);
        let selected=date!=()&&(date==start||date==finish);let in_range=props.mode=="range"&&date!=()&&start!=()&&range_end!=()
            &&date>=start&&date<=range_end;
        let s=ctx.component_style("day",control_style());
        if cell.outside{s=ctx.component_style("day_outside",s.text_color(theme_color("text_muted")));}
        if date==if option(props,"today")==(){ctx.today()}else{props.today}{s=ctx.component_style("day_today",s.text_color(theme_color("text.accent")));}
        if in_range{s=ctx.component_style("day_range",s.background(theme_color("selection")));}
        if selected{s=ctx.component_style("day_selected",s.background(theme_color("accent")).text_color(theme_color("on_accent"))
            .hover(style().background(theme_color("accent_hover"))));}
        if date==focus{let focused_style=s.border_color(rgba(0x00000000)).group_focus(style().border_color(theme_color("focus_ring")));
            s=ctx.component_style("day_focused",focused_style);}
        if disabled{s=ctx.component_style("day_disabled",s.text_color(theme_color("disabled")));}
        let label=if date==(){""}else{ctx.format_date(date,"long")};
        if date!=()&&props.mode=="range"{if date==start{label+=`, ${ctx.t("time.start")}`;}if date==finish{label+=`, ${ctx.t("time.end")}`;}}
        let day=row([text(if cell.day==(){""}else{ctx.format_number(cell.day,#{grouping:false})})])
            .with_key(if date==(){`blank-${w}-${d}`}else{date}).with_style(s).disabled(disabled).tab_stop(false)
            .accessibility_role("gridcell").accessibility_label(label).accessibility_selected(selected||in_range).accessibility_value(date);
        if !disabled{day=day.on_click_value(Fn("chosen"),date).tab_stop(false).on_hover_value(Fn("hovered"),date);}
        days.push(day);
    }weeks.push(row(days).with_key(`week-${w}`).with_style(ctx.component_style("week",style())));}
    let children=[];
    if props.months==2{children.push(text(ctx.format_month_year(month)).with_style(ctx.component_style("month_title",style().typography("body").font_weight(600))));}
    children.push(row(weekday_nodes).with_style(ctx.component_style("weekdays",style())));
    children.push(column(weeks).with_style(ctx.component_style("grid",style())).accessibility_role("grid").accessibility_label(ctx.format_month_year(month)));
    column(children)
        .with_key(month).with_style(style().gap(theme_spacing("xs")))
}
fn render_Calendar(ctx,props){
    let start=option(props,"value");let finish=option(props,"end");let min_date=option(props,"min_date");let max_date=option(props,"max_date");
    if start!=(){date_info(start);}if finish!=(){date_info(finish);}if min_date!=(){date_info(min_date);}if max_date!=(){date_info(max_date);}
    if option(props,"today")!=(){date_info(props.today);}
    if min_date!=()&&max_date!=()&&min_date>max_date{throw "Calendar min_date cannot be after max_date";}
    let anchor=date_clamp(if start==(){ctx.today()}else{start},min_date,max_date);
    effect("anchor",anchor,Fn("anchor_changed"),Fn("anchor_cleanup"));
    let changed=ctx.get_state("anchor")!=anchor;let stored=ctx.get_state("focused");
    let focus=if stored==""||changed{anchor}else{date_clamp(stored,min_date,max_date)};
    let month=if ctx.get_state("month")==""||changed{date_month_start(focus)}else{ctx.get_state("month")};
    let previous=month_target(focus,-1,min_date,max_date);let next=month_target(focus,1,min_date,max_date);
    let head=row([row([text("‹")]).with_key("previous").with_style(ctx.component_style("previous",control_style()))
        .disabled(previous==focus).accessibility_role("button").accessibility_label(ctx.t("calendar.previous_month"))
        .on_click_value(Fn("navigated"),previous).tab_stop(false),
        text(if props.months==1{ctx.format_month_year(month)}else{props.label})
            .with_style(ctx.component_style("month_title",style().typography("body").font_weight(600))),
        row([text("›")]).with_key("next").with_style(ctx.component_style("next",control_style()))
        .disabled(next==focus).accessibility_role("button").accessibility_label(ctx.t("calendar.next_month"))
        .on_click_value(Fn("navigated"),next).tab_stop(false)]).with_style(ctx.component_style("header",style().justify_between()));
    let months=[calendar_month(ctx,props,month,focus,start,finish,min_date,max_date)];
    if props.months==2{let next_month=date_checked_add_months(month,1);if next_month!=(){months.push(calendar_month(ctx,props,next_month,focus,start,finish,min_date,max_date));}}
    let first=ctx.calendar().first_weekday;
    let root=column([head,row(months).with_style(style().gap(theme_spacing("sm")))])
        .with_key(props.key).with_style(ctx.component_style("root",style().gap(theme_spacing("xs")).text_color(theme_color("text_primary")).focus(style())))
        .disabled(props.disabled).tab_stop(true).accessibility_role("group").accessibility_label(props.label)
        .on_key_value("left",Fn("navigated"),day_target(focus,if ctx.text_direction()=="rtl"{1}else{-1},min_date,max_date))
        .on_key_value("right",Fn("navigated"),day_target(focus,if ctx.text_direction()=="rtl"{-1}else{1},min_date,max_date))
        .on_key_value("up",Fn("navigated"),day_target(focus,-7,min_date,max_date))
        .on_key_value("down",Fn("navigated"),day_target(focus,7,min_date,max_date))
        .on_key_value("home",Fn("navigated"),week_target(focus,first,false,min_date,max_date))
        .on_key_value("end",Fn("navigated"),week_target(focus,first,true,min_date,max_date))
        .on_key_value("pageup",Fn("navigated"),previous).on_key_value("pagedown",Fn("navigated"),next)
        .on_key_value("enter",Fn("chosen"),focus).on_key_value("space",Fn("chosen"),focus);
    if "size" in props&&props.size!=(){root.env(#{size:props.size})}else{root}
}
