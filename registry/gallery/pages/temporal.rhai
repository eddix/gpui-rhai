// Temporal workflows: controlled state stays in the Gallery root; no business polling.
import "gallery/kit" as kit;
import "components/calendar" as calendar;
import "components/time_input" as time_input;
import "components/timeline" as timeline;
import "components/time_range_picker" as time_range_picker;
import "components/button" as button;

fn pages(){[
    #{id:"calendar",title:"Calendar",group:"Fields"},
    #{id:"time_input",title:"TimeInput",group:"Fields"},
    #{id:"timeline",title:"Timeline",group:"Lists"},
    #{id:"time_range_picker",title:"TimeRangePicker",group:"Fields"},
]}
fn view(ctx,h,v,id){
    if id=="calendar"{calendar_page(h,v)}else if id=="time_input"{time_input_page(ctx,h,v)}
    else if id=="timeline"{timeline_page(ctx,h,v)}else if id=="time_range_picker"{range_page(h,v)}else{()}
}
fn calendar_sample(h,v,key,density){calendar::Calendar(#{key:key,label:"Calendar date",value:kit::val(v,key,"2026-10-14"),
    min_date:"2026-10-01",max_date:"2026-12-31",on_change:kit::store(h,key)}).env(#{density:density})}
fn calendar_page(h,v){kit::page(h,v,"calendar","Calendar","One locale-aware grid shared by date and date-time range fields.",[
    kit::sec("Keyboard",calendar_sample(h,v,"calendar.date","comfortable")),
    kit::sec("Density",kit::densities(calendar_sample(h,v,"calendar.comfortable","comfortable"),calendar_sample(h,v,"calendar.compact","compact"))),
    kit::note("Arrows move by day or week; Home/End use the locale week. Page Up/Down changes month. Enter selects."),
])}
fn time_input_page(ctx,h,v){let editable_error=kit::val(v,"clock.invalid","99:99");
    kit::page(h,v,"time_input","TimeInput","Native clock editing follows the locale hour cycle and digits.",[
    kit::sec("Precision",kit::line([
        time_input::TimeInput(#{key:"clock-minute",label:"Minute precision",value:kit::val(v,"clock.minute",ctx.format_time(0,"short","UTC")),on_change:kit::store(h,"clock.minute")}),
        time_input::TimeInput(#{key:"clock-second",label:"Second precision",value:kit::val(v,"clock.second",ctx.format_time(0,"medium","UTC")),precision:"second",on_change:kit::store(h,"clock.second")}),
    ])),
    kit::sec("States",kit::line([
        time_input::TimeInput(#{key:"clock-invalid",label:"Invalid time",value:editable_error,
            error:ctx.parse_time(editable_error,"minute")==(),on_change:kit::store(h,"clock.invalid")}),
        time_input::TimeInput(#{key:"clock-disabled",label:"Disabled time",value:ctx.format_time(0,"short","UTC"),disabled:true}),
    ])),kit::note("Up/Down steps one minute or second. Incomplete text is a draft, not a substituted time."),
])}
fn timeline_items(now){[
    #{key:"build",title:"Build",status:"success",start:now-5400000,end:now-5100000,meta:"3/3",children:[
        #{key:"compile",title:"Compile",status:"success",start:now-5400000,end:now-5220000},
        #{key:"test",title:"Test",status:"success",start:now-5220000,end:now-5100000},
    ]},
    #{key:"canary",title:"Canary",status:"running",start:now-300000,current:true,
        description:"Waiting for health checks. The description wraps without hiding the event or changing its stable identity."},
    #{key:"rollout",title:"Full rollout",status:"pending"},
]}
fn timeline_page(ctx,h,v){
    // Anchor the sample history to a stable instant, not to each presentation tick.
    let anchor=1791947400000;
    let feed=[];for i in 0..300{feed.push(#{key:`run-${i}`,title:`Job run ${i+1}`,status:if i%17==0{"warning"}else{"success"},
        start:anchor-i*3600000,end:anchor-i*3600000+120000,meta:ctx.format_duration(120000)});}
    kit::page(h,v,"timeline","Timeline","Stages and event histories share controlled expansion, selection, time and accessibility.",[
        kit::sec("Time display",kit::line([
            button::Button(#{key:"timeline-absolute",text:"Absolute",variant:"ghost",on_click:kit::put(h,"timeline.mode","absolute")}),
            button::Button(#{key:"timeline-relative",text:"Relative",variant:"ghost",on_click:kit::put(h,"timeline.mode","relative")}),
            button::Button(#{key:"timeline-duration",text:"Duration",variant:"ghost",on_click:kit::put(h,"timeline.mode","duration")}),
        ])),
        kit::sec("Stages",timeline::Timeline(#{key:"pipeline",label:"Release pipeline",items:timeline_items(ctx.now()),
            expanded:kit::val(v,"timeline.expanded",["build"]),on_expanded_change:kit::store(h,"timeline.expanded"),
            selection_mode:"single",selected_keys:kit::val(v,"timeline.selected",[]),on_selection_change:kit::store(h,"timeline.selected"),
            details:#{canary:kit::note("A rich detail node retains its caller ownership.")},
            time_display:kit::val(v,"timeline.mode","relative"),time_zone:"UTC",height:300})),
        kit::sec("Long history",timeline::Timeline(#{key:"history",label:"Job run history",items:feed,selection_mode:"single",
            selected_keys:kit::val(v,"timeline.feed",[]),on_selection_change:kit::store(h,"timeline.feed"),time_display:"absolute",time_zone:"UTC",day_headers:true,height:260})),
        kit::note("Up/Down moves selection; Left/Right folds stages. The current step is announced independently from selection. Reduce motion from the Gallery toolbar."),
    ])
}
fn range_page(h,v){kit::page(h,v,"time_range_picker","TimeRangePicker","Relative durations remain relative; absolute drafts take effect only on Apply.",[
    kit::sec("Relative window",time_range_picker::TimeRangePicker(#{key:"query-range",label:"Query time range",
        value:kit::val(v,"range.value",#{kind:"relative",duration_ms:86400000}),on_change:kit::store(h,"range.value"),
        time_zone:"UTC",zone_switch:true,shift_buttons:true,max_span_ms:2678400000})),
    kit::sec("Exact interval",time_range_picker::TimeRangePicker(#{key:"absolute-range",label:"Exact time interval",
        value:kit::val(v,"range.absolute",#{kind:"absolute",start:1791947400000,end:1791952200000}),on_change:kit::store(h,"range.absolute"),
        time_zone:"Asia/Shanghai",precision:"second"})),
    kit::sec("Daylight saving",time_range_picker::TimeRangePicker(#{key:"dst-range",label:"New York transition interval",
        value:kit::val(v,"range.dst",#{kind:"absolute",start:1793511000000,end:1793518200000}),on_change:kit::store(h,"range.dst"),
        time_zone:"America/New_York"})),
    kit::note("Escape cancels. Nonexistent local times show an inline error; repeated times require choosing an offset. Future times remain valid unless max forbids them."),
])}
