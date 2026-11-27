/* gpui-rhai
{"id":"components/calendar","export":"Calendar","version":"0.4.3","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"tokens":["surface_hover","accent","accent_hover","on_accent","text.accent","text_primary","text_muted","disabled","focus_ring","selection","metrics.control","metrics.marker","spacing.xs","spacing.sm","radius.md","typography.control","typography.label","typography.body","radius.sm","space.spacing_unit","space.unit"],"environment":["size","density"]}
*/
// The single calendar engine shared by DatePicker and TimeRangePicker.
// Example: Calendar(#{key:"day",label:"Date",value:"2026-10-16",on_change:Fn("date_changed")})
define_component(#{metadata:#{id:"components/calendar","export":"Calendar",version:"0.4.3",
    runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{},
    tokens:["surface_hover","accent","accent_hover","on_accent","text.accent","text_primary","text_muted","disabled","focus_ring","selection","metrics.control","metrics.marker","spacing.xs","spacing.sm","radius.md","typography.control","typography.label","typography.body","radius.sm","space.spacing_unit","space.unit"],environment:["size","density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable calendar identity."},
        label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accessible calendar name."},
        value:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Controlled selected ISO date, or range start."},
        end:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Controlled range end, inclusive; unset while choosing the end."},
        today:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Presentation-zone ISO today; unset follows ctx.today and the Host clock."},
        month_anchor:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initial visible month and cursor, independent of the selected range start."},
        mode:#{schema:#{type:"string",allowed:["single","range"]},required:false,sensitive:false,"default":#{type:"string",value:"single"},doc:"Single date or inclusive date range."},
        months:#{schema:#{type:"integer",min:1,max:2},required:false,sensitive:false,"default":#{type:"integer",value:1},doc:"One or two consecutive calendar months."},
        range_bands:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Draw continuous range and preview bands, with only in-month dates presented."},
        comparison_start:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"First date of the optional secondary range band."},
        comparison_end:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Last date of the optional secondary range band, inclusive."},
        first_weekday:#{schema:#{type:"optional",value:#{type:"string",allowed:["sunday","monday","tuesday","wednesday","thursday","friday","saturday"]}},required:false,sensitive:false,doc:"Overrides the locale's first weekday for the grid and Home/End."},
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
    parts:["root","header","month_title","previous","next","weekdays","weekday","grid","week","day","day_outside","day_today","day_selected","day_focused","day_disabled","day_range","day_preview","day_comparison",
        "banded_range","banded_selected","banded_today","banded_weekday","banded_grid","banded_cell","banded_nav","banded_month_title"],effects:["anchor"]},
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
fn banded_chosen(ctx,date){ctx.focus("calendar-focus");chosen(ctx,date)}
fn banded_navigated(ctx,date){ctx.focus("calendar-focus");navigated(ctx,date)}
fn hovered(ctx,payload){
    if payload.hovered{ctx.set_state("hover",payload.value);}
    else if ctx.get_state("hover")==payload.value{ctx.set_state("hover","");}
}
fn anchor_changed(ctx,value){ctx.set_state("anchor",value);ctx.set_state("focused",value);ctx.set_state("month",date_month_start(value));}
fn anchor_cleanup(ctx,value){}
fn control_style(){style().width(theme_length("metrics.control")).height(theme_length("metrics.control"))
    .items_center().justify_center().border(px(2)).border_color(rgba(0x00000000)).radius(theme_radius("md"))
    .typography("control").hover(style().background(theme_color("surface_hover")))
    .focus(style().border_color(theme_color("focus_ring")))}
fn calendar_first(ctx,props){
    let first=option(props,"first_weekday");
    if first==(){ctx.calendar().first_weekday}else{first}
}
fn is_between(date,left,right){
    date!=()&&left!=()&&right!=()&&((date>=left&&date<=right)||(date>=right&&date<=left))
}
fn band(ctx,part,default_ink,start_edge,end_edge,split){
    let base=style().absolute().inset_start(px(if start_edge{2}else{0})).inset_end(px(if end_edge{2}else{0}))
        .top(if split{relative(0.76)}else{px(2)}).bottom(px(2)).background(default_ink);
    let styled=ctx.component_style(part,base);
    if start_edge{styled=styled.radius_start(theme_radius("sm"));}
    if end_edge{styled=styled.radius_end(theme_radius("sm"));}
    row([]).with_style(styled).accessibility_hidden(true)
}
fn banded_day(ctx,s,number,flags){
    let pieces=[];
    if flags.primary{
        pieces.push(band(ctx,if flags.preview{"day_preview"}else{"banded_range"},theme_color("selection"),
            flags.left,flags.right,false));
    }
    if flags.compare{pieces.push(band(ctx,"day_comparison",alpha(theme_color("accent"),0.20),
        flags.compare_left,flags.compare_right,flags.primary));}
    pieces.push(row([text(number)]).with_style(s.width(relative(1.0)).height(relative(1.0)).min_width(px(0))));
    if flags.today{
        pieces.push(row([]).with_style(ctx.component_style("banded_today",style().absolute().bottom(px(3)).left(relative(0.5))
            .translate_x(-1.5).width(px(3)).height(px(3)).radius(theme_length("space.unit").times(249.75)).background(theme_color("accent"))))
            .accessibility_hidden(true));
    }
    row(pieces).with_style(ctx.component_style("banded_cell",style().relative().width(relative(1.0)).min_width(px(0))
        .height(theme_length("space.unit").times(8))))
}
fn calendar_month(ctx,props,month,focus,start,finish,min_date,max_date){
    let calendar=ctx.calendar();let first_name=calendar_first(ctx,props);let first=weekday_index(first_name);let weekday_nodes=[];
    for offset in 0..7{
        let label=calendar.weekdays.short[(first+offset)%7];
        let part=if props.range_bands{"banded_weekday"}else{"weekday"};
        let weekday_style=style().width(theme_length("metrics.control")).height(theme_length("metrics.marker"))
            .items_center().justify_center().typography("label").text_color(theme_color("text_muted"));
        weekday_nodes.push(row([text(label)])
            .with_style(ctx.component_style(part,weekday_style
                .merge(if props.range_bands{style().width(relative(1.0)).min_width(px(0))}else{style()})))
            .accessibility_role("columnheader").accessibility_label(label));
    }
    let today=if option(props,"today")==(){ctx.today()}else{props.today};
    let cells=date_month_grid(month,first_name);let weeks=[];
    let hover=ctx.get_state("hover");
    let range_end=if finish==()&&hover!=""{hover}else if finish==()&&props.range_bands{focus}else{finish};
    let compare_start=option(props,"comparison_start");let compare_end=option(props,"comparison_end");
    let week_layout=if props.range_bands{style().width(relative(1.0)).grid_cols(7)}else{style()};
    for w in 0..6{
        if props.range_bands&&w>=4&&cells[w*7].outside{break;}
        let days=[];for d in 0..7{
        let cell=cells[w*7+d];let date=if props.range_bands&&cell.outside{()}else{cell.date};let disabled=!enabled(date,min_date,max_date);
        let selected=date!=()&&(date==start||date==finish);let in_range=props.mode=="range"&&date!=()&&start!=()&&range_end!=()
            &&is_between(date,start,range_end);
        let in_comparison=props.range_bands&&is_between(date,compare_start,compare_end);
        let s=ctx.component_style("day",control_style());
        if cell.outside{s=ctx.component_style("day_outside",s.text_color(theme_color("text_muted")));}
        if date==today{s=ctx.component_style("day_today",s.text_color(theme_color("text.accent")));}
        if in_range&&!props.range_bands{s=ctx.component_style("day_range",s.background(theme_color("selection")));}
        if selected {
            let part=if props.range_bands{"banded_selected"}else{"day_selected"};
            s=ctx.component_style(part,s.background(theme_color("accent")).text_color(theme_color("on_accent"))
                .hover(style().background(theme_color("accent_hover"))));
        }
        if date==focus{let focused_style=s.border_color(rgba(0x00000000)).group_focus(style().border_color(theme_color("focus_ring")));
            s=ctx.component_style("day_focused",focused_style);}
        if disabled{s=ctx.component_style("day_disabled",s.text_color(theme_color("disabled")));}
        let label=if date==(){""}else{ctx.format_date(date,"long")};
        if date!=()&&props.mode=="range"{if date==start{label+=`, ${ctx.t_or("time.start", "Start")}`;}if date==finish{label+=`, ${ctx.t_or("time.end", "End")}`;}}
        let number=if date==(){""}else{ctx.format_number(cell.day,#{grouping:false})};
        let day=if props.range_bands{
            let at=w*7+d;let prev=if d==0||cells[at-1].outside{()}else{cells[at-1].date};
            let tail=if d==6||cells[at+1].outside{()}else{cells[at+1].date};
            banded_day(ctx,s,number,#{primary:in_range,preview:finish==(),compare:in_comparison,today:date==today,
                left:!is_between(prev,start,range_end),right:!is_between(tail,start,range_end),
                compare_left:!is_between(prev,compare_start,compare_end),compare_right:!is_between(tail,compare_start,compare_end)})
        }else{row([text(number)]).with_style(s)};
        day=day.with_key(if date==(){`blank-${w}-${d}`}else{date}).disabled(disabled).tab_stop(false)
            .accessibility_role("gridcell").accessibility_label(label)
            .accessibility_selected(selected||(in_range&&finish!=())).accessibility_value(date);
        if in_comparison{day=day.accessibility_description(ctx.t_or("date_range.comparison_day","Comparison period"));}
        if date==()&&props.range_bands{day=day.accessibility_hidden(true);}
        if !disabled{
            day=day.on_click_value(if props.range_bands{Fn("banded_chosen")}else{Fn("chosen")},date).tab_stop(false);
            if !props.range_bands||finish==(){day=day.on_hover_value(Fn("hovered"),date);}
        }
        days.push(day);
    }weeks.push(row(days).with_key(`week-${w}`).with_style(ctx.component_style("week",week_layout)));}
    let children=[];
    if props.months==2{
        let title=ctx.format_month_year(month);
        if props.range_bands{
            children.push(row([text(title)]).with_style(ctx.component_style("banded_month_title",style()
                .height(theme_length("space.unit").times(7)).items_center().justify_center().typography("control").font_weight(600))));
        }else{
            children.push(text(title).with_style(ctx.component_style("month_title",style().typography("body").font_weight(600))));
        }
    }
    children.push(row(weekday_nodes).with_style(ctx.component_style("weekdays",week_layout)));
    children.push(column(weeks).with_style(ctx.component_style(if props.range_bands{"banded_grid"}else{"grid"},style()))
        .accessibility_role("grid").accessibility_label(ctx.format_month_year(month)));
    let pane=column(children).with_key(month);
    if props.range_bands{pane.with_style(style().min_width(px(0)).max_width(px(224)).flex_grow().flex_basis(px(0)))}else{pane}
}
fn render_Calendar(ctx,props){
    let start=option(props,"value");let finish=option(props,"end");let min_date=option(props,"min_date");let max_date=option(props,"max_date");
    if start!=(){date_info(start);}if finish!=(){date_info(finish);}if min_date!=(){date_info(min_date);}if max_date!=(){date_info(max_date);}
    if option(props,"today")!=(){date_info(props.today);}
    let preferred=option(props,"month_anchor");if preferred!=(){date_info(preferred);}
    if option(props,"comparison_start")!=(){date_info(props.comparison_start);}
    if option(props,"comparison_end")!=(){date_info(props.comparison_end);}
    if min_date!=()&&max_date!=()&&min_date>max_date{throw "Calendar min_date cannot be after max_date";}
    let today=if option(props,"today")==(){ctx.today()}else{props.today};
    let anchor=date_clamp(if preferred!=(){preferred}else if start==(){today}else{start},min_date,max_date);
    effect("anchor",anchor,Fn("anchor_changed"),Fn("anchor_cleanup"));
    let changed=ctx.get_state("anchor")!=anchor;let stored=ctx.get_state("focused");
    let focus=if stored==""||changed{anchor}else{date_clamp(stored,min_date,max_date)};
    let month=if ctx.get_state("month")==""||changed{date_month_start(focus)}else{ctx.get_state("month")};
    let previous=month_target(focus,-1,min_date,max_date);let next=month_target(focus,1,min_date,max_date);
    let browse=if props.range_bands{Fn("banded_navigated")}else{Fn("navigated")};
    let nav_style=if props.range_bands{ctx.component_style("banded_nav",control_style().width(theme_length("space.unit").times(7)).height(theme_length("space.unit").times(7))
        .text_color(theme_color("text_muted")))}else{control_style()};
    let wide_banded=props.range_bands&&props.months==2;
    let head=row([row([text("‹")]).with_key("previous").with_style(ctx.component_style("previous",nav_style))
        .disabled(previous==focus).accessibility_role("button").accessibility_label(ctx.t_or("calendar.previous_month", "Previous month"))
        .on_click_value(browse,previous).tab_stop(false),
        text(if wide_banded{""}else if props.months==1{ctx.format_month_year(month)}else{props.label})
            .with_style(ctx.component_style("month_title",style().typography("body").font_weight(600))),
        row([text("›")]).with_key("next").with_style(ctx.component_style("next",nav_style))
        .disabled(next==focus).accessibility_role("button").accessibility_label(ctx.t_or("calendar.next_month", "Next month"))
        .on_click_value(browse,next).tab_stop(false)]);
    let months=[calendar_month(ctx,props,month,focus,start,finish,min_date,max_date)];
    if props.months==2{let next_month=date_checked_add_months(month,1);if next_month!=(){months.push(calendar_month(ctx,props,next_month,focus,start,finish,min_date,max_date));}}
    let first=calendar_first(ctx,props);
    let panel_style=style().gap(theme_spacing("sm"));
    if props.range_bands{panel_style=panel_style.width(relative(1.0)).min_width(px(0)).justify_center().gap(theme_length("space.spacing_unit").times(6));}
    let header=if props.range_bands{style().justify_between().items_center().height(theme_length("space.unit").times(7))}
        else{ctx.component_style("header",style().justify_between())};
    let layout=if wide_banded{
        row([row(months).with_style(panel_style),head.with_style(header.absolute().top(px(0)).left(px(0)).right(px(0)))])
            .with_style(style().relative().width(relative(1.0)))
    }else{column([head.with_style(header),row(months).with_style(panel_style)])};
    let root=layout
        .with_key(props.key).with_style(ctx.component_style("root",style().gap(theme_spacing("xs")).text_color(theme_color("text_primary")).focus(style())))
        .disabled(props.disabled).tab_stop(true).accessibility_role("group").accessibility_label(props.label)
        .on_key_value("left",Fn("navigated"),day_target(focus,-1,min_date,max_date))
        .on_key_value("right",Fn("navigated"),day_target(focus,1,min_date,max_date))
        .on_key_value("up",Fn("navigated"),day_target(focus,-7,min_date,max_date))
        .on_key_value("down",Fn("navigated"),day_target(focus,7,min_date,max_date))
        .on_key_value("home",Fn("navigated"),week_target(focus,first,false,min_date,max_date))
        .on_key_value("end",Fn("navigated"),week_target(focus,first,true,min_date,max_date))
        .on_key_value("pageup",Fn("navigated"),previous).on_key_value("pagedown",Fn("navigated"),next)
        .on_key_value("enter",Fn("chosen"),focus).on_key_value("space",Fn("chosen"),focus);
    if props.range_bands{root=root.with_ref(element_ref("calendar-focus"));}
    if "size" in props&&props.size!=(){root.env(#{size:props.size})}else{root}
}
