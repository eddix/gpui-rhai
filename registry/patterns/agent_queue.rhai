/* gpui-rhai
{"id":"patterns/agent_queue","export":"AgentQueue","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/progress","components/scroll_area","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","selection","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted","typography.body","typography.caption","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// Worker dispatch, interactive job inspection and queue depth on a shared Host clock.
// State: locally pinned job and keyboard cursor only. Job actions are proposals.
// Example: AgentQueue(#{key:"pool",queue:"pool",revision:0,now_ms:0,jobs:[],workers:[]})
import "components/button" as button;
import "components/progress" as progress;
import "components/scroll_area" as scroll;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_queue","export":"AgentQueue",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/progress","components/scroll_area","patterns/approval_card"],capabilities:#{},
        tokens:["accent","border","focus_ring","selection","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted",
            "typography.body","typography.caption","space.spacing_unit","space.unit"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable instance key."},
        queue:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business queue identifier, included in proposals."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted queue revision. Advance it on business changes, not clock ticks."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional heading."},
        name:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional queue or provider name."},
        now_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Host clock shared with job and rate-limit time stamps."},
        window_ms:#{schema:#{type:"integer",min:1000,max:604800000},"default":#{type:"integer",value:120000},required:false,sensitive:false,doc:"Width of the depth chart's time window."},
        backpressure:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000}},required:false,sensitive:false,doc:"Optional line; the Waiting statistic is marked only when depth is above it."},
        jobs:#{schema:#{type:"array",max_items:128,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable job ID."},
            name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Job caption."},
            kind:#{schema:#{type:"string",allowed:["llm","tool","agent","human"]},required:true,sensitive:false,doc:"Type of work, used for labels and the job mark."},
            status:#{schema:#{type:"string",allowed:["queued","running","done","failed","cancelled"]},required:true,sensitive:false,doc:"Accepted job state."},
            priority:#{schema:#{type:"optional",value:#{type:"string",allowed:["high","normal","low"]}},required:false,sensitive:false,doc:"Queue priority; ties are chronological, then in caller order."},
            priority_order:#{schema:#{type:"optional",value:#{type:"integer"}},required:false,sensitive:false,doc:"Optional explicit order within a priority, before unranked jobs. Does not rewrite enqueue time."},
            enqueued_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"When this attempt entered the queue."},
            started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Optional start time; required for running jobs, absent for queued jobs."},
            ended_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Required for a terminal job; absent for active jobs."},
            worker:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Running job's accepted slot. Omit if it is running outside the declared slots."},
            progress:#{schema:#{type:"optional",value:#{type:"number",min:0.0,max:1.0}},required:false,sensitive:false,doc:"Authoritative running progress."},
            estimate_ms:#{schema:#{type:"optional",value:#{type:"integer",min:1}},required:false,sensitive:false,doc:"Estimate used when progress is absent. Finished runs do not extend the waiting interval."},
            attempt:#{schema:#{type:"optional",value:#{type:"integer",min:1,max:999}},required:false,sensitive:false,doc:"Attempt number, initially one."},
            max_attempts:#{schema:#{type:"optional",value:#{type:"integer",min:1,max:999}},required:false,sensitive:false,doc:"Maximum attempts. Disables Retry once exhausted."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Job evidence or a short explanation."},
            error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Reported failure, shown within inspection."},
        }}},required:true,sensitive:true,doc:"Live jobs and retained history. Retain jobs overlapping the chart window to reconstruct complete depth."},
        workers:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable and unique slot identity."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Slot name."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional caption below the slot name."},
        }}},required:true,sensitive:false,doc:"Accepted slots in presentation order. A running slot can hold at most one job."},
        rate_limits:#{schema:#{type:"optional",value:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            from:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Inclusive rate limit start."},
            until_ms:#{schema:#{type:"integer",min:1},required:true,sensitive:false,doc:"Exclusive rate limit end, after from."},
            label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional reason/provider detail."},
        }}}},required:false,sensitive:false,doc:"Throttle history; overlaps are painted once and subtracted once from estimated progress."},
        totals:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            done:#{schema:#{type:"integer",min:0},required:false,sensitive:false,doc:"Accepted lifetime success count."},
            failed:#{schema:#{type:"integer",min:0},required:false,sensitive:false,doc:"Accepted lifetime failure count."},
        }}},required:false,sensitive:false,doc:"Omit to count the retained terminal records instead."},
        paused:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Accepted pause; does not change running jobs or manufacture a rate limit."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Temporarily disables commands, not inspection."},
        min_workers:#{schema:#{type:"integer",min:0,max:32},"default":#{type:"integer",value:1},required:false,sensitive:false,doc:"Lowest proposed concurrency."},
        max_workers:#{schema:#{type:"integer",min:0,max:32},"default":#{type:"integer",value:16},required:false,sensitive:false,doc:"Highest proposed concurrency."},
        open_id:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Controlled inspection. Pass an empty string for closed, or omit for local inspection."},
        default_open_id:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initially inspected job in local mode."},
        allow_promote:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Allow a waiting job to propose moving to the front."},
        allow_cancel:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Allow waiting/running jobs to propose cancellation."},
        allow_retry:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Allow a failed job to propose retrying if attempts remain."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host feedback shown without changing accepted state."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a revisioned proposal. The component does not run jobs."},
        on_open_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Inspection change; in controlled mode the Host accepts it by updating open_id."},
    },state:#{fields:#{
        open:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        cursor:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
    }},events:#{
        action:#{payload:#{type:"object",allow_unknown:false,fields:#{
            queue:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business queue ID."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted queue version."},
            kind:#{schema:#{type:"string",allowed:["pause","resume","concurrency","promote","cancel","retry"]},required:true,sensitive:false,doc:"Requested operation."},
            job:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Job ID when applicable, or empty."},
            workers:#{schema:#{type:"integer",min:0,max:32},required:true,sensitive:false,doc:"Proposed worker count for concurrency, zero otherwise."},
        }},doc:"A proposal does not change queue state. Check the exact revision before accepting."},
        open_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            queue:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business queue ID."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted queue version."},
            job:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Proposed job to inspect, or empty for closed."},
        }},doc:"Inspection proposal."},
    },slots:#{},parts:["root","head","title","metadata","state","controls","card","stats","stat","stat_value","stat_label","flow","waiting","worker_group",
        "section","section_count","job","job_open","job_name","job_side","mark","priority","worker","worker_name","worker_detail","worker_state","progress",
        "bins","bin","bin_failed","detail","detail_title","detail_action","detail_meta","error","chart","chart_title","plot","depth_fill","scale","axis","empty","notice"]},
    render:Fn("render_AgentQueue")
});

fn AgentQueue(props){render_component("patterns/agent_queue",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,d){common::value_or(m,k,d)}
fn id_valid(id){id!=""&&utf8_byte_count(id)<=128}
fn priority(job){let p=value(job,"priority","normal");if p=="high"{0}else if p=="low"{2}else{1}}
fn job_order(a,b){
    let rank=priority(a)-priority(b);
    if rank!=0{return rank;}
    let ao=opt(a,"priority_order");let bo=opt(b,"priority_order");
    if ao!=bo{
        if ao==(){return 1;}if bo==(){return -1;}return time_order(ao,bo);
    }
    if a.enqueued_ms==b.enqueued_ms{return value(a,"ordinal",0)-value(b,"ordinal",0);}
    if a.enqueued_ms<b.enqueued_ms{-1}else{1}
}
fn end_order(a,b){if a.ended_ms==b.ended_ms{a.ordinal-b.ordinal}else if a.ended_ms>b.ended_ms{-1}else{1}}
fn time_order(a,b){if a<b{-1}else if a>b{1}else{0}}
fn range_order(a,b){time_order(a.from,b.from)}
fn limit_ranges(limits){
    let sorted=limits;sorted.sort(Fn("range_order"));
    let ranges=[];
    for r in sorted{
        if r.until_ms<=r.from{throw "Queue rate-limit end must be after its start";}
        if ranges.len==0||ranges[ranges.len-1].until_ms<r.from{ranges.push(r);}
        else{
            let i=ranges.len-1;let last=ranges[i];
            if r.until_ms>last.until_ms{last.until_ms=r.until_ms;}
            if opt(last,"label")==()&&opt(r,"label")!=(){last.label=r.label;}
            ranges[i]=last;
        }
    }
    ranges
}
fn duration_without_limits(start,now,ranges){
    let lost=0;for range in ranges{lost+=max(0,min(now,range.until_ms)-max(start,range.from));}
    max(0,now-start-lost)
}
fn pressure(starts,ends,t0,now){
    // Sweep two ordered streams. A span contributes on [enqueued, left).
    // Simultaneous starts/ends commit together, without a spurious peak.
    starts.sort();ends.sort();
    let si=0;let ei=0;
    while si<starts.len&&starts[si]<=t0{si+=1;}
    while ei<ends.len&&ends[ei]<=t0{ei+=1;}
    let depth=si-ei;let peak=depth;let area=0.0;let last=t0;
    let points=[#{ms:t0,depth:depth}];
    while si<starts.len||ei<ends.len{
        let next=if si==starts.len{ends[ei]}else if ei==ends.len{starts[si]}else{min(starts[si],ends[ei])};
        if next>now{break;}
        let prev=depth;
        while si<starts.len&&starts[si]==next{si+=1;}
        while ei<ends.len&&ends[ei]==next{ei+=1;}
        depth=si-ei;
        if depth==prev{continue;}
        area+=(next-last).to_float()*prev.to_float();last=next;
        points.push(#{ms:next,depth:depth});peak=max(peak,depth);
    }
    area+=(now-last).to_float()*depth.to_float();
    if points[points.len-1].ms!=now{points.push(#{ms:now,depth:depth});}
    #{points:points,peak:peak,depth:depth,average:area/(now-t0).to_float(),start:t0,now:now}
}
fn pressure_snapshot(jobs,now,window_ms){
    if window_ms<1{throw "Queue pressure needs a positive window";}
    let starts=[];let ends=[];
    for j in jobs{
        starts.push(j.enqueued_ms);
        let left=if j.status=="queued"{()}else{value(j,"started_ms",opt(j,"ended_ms"))};
        if left!=(){ends.push(max(j.enqueued_ms,left));}
    }
    pressure(starts,ends,now-window_ms,now)
}
fn project(props){
    if !id_valid(props.queue)||props.min_workers>props.max_workers{throw "Invalid queue identity or concurrency bounds";}
    let data=#{slots:#{},jobs:#{},waiting:[],running:[],loose:[],recent:[],done:0,failed:0,throughput:0,waits:[],active_limit:()};
    let now=props.now_ms;let starts=[];let ends=[];
    data.ranges=limit_ranges(value(props,"rate_limits",[]));
    for r in data.ranges{if r.from<=now&&now<r.until_ms{data.active_limit=r;}}
    for w in props.workers{
        if !id_valid(w.id)||w.id in data.slots{throw "Queue worker IDs must be unique and bounded";}
        data.slots[w.id]=();
    }
    for i in 0..props.jobs.len{
        let job=props.jobs[i];job.ordinal=i;
        if !id_valid(job.id)||job.id in data.jobs{throw "Queue job IDs must be unique and bounded";}
        let started=opt(job,"started_ms");let ended=opt(job,"ended_ms");
        if job.enqueued_ms>now||(started!=()&&(started<job.enqueued_ms||started>now))||
            (ended!=()&&(ended<job.enqueued_ms||ended>now||(started!=()&&ended<started))){
            throw "Queue job time stamps must be ordered on the Host clock";
        }
        let terminal=job.status in ["done","failed","cancelled"];
        if (job.status=="queued"&&started!=())||(job.status=="running"&&started==())||(terminal!=(ended!=())){
            throw "Queue job time stamps do not match its accepted status";
        }
        starts.push(job.enqueued_ms);
        if job.status=="queued"{
            data.waiting.push(job);data.waits.push(now-job.enqueued_ms);
        }else{
            ends.push(if started==(){ended}else{started});
            if started!=()&&started>=now-60000{data.waits.push(started-job.enqueued_ms);}
            if terminal{
                data.recent.push(job);
                if job.status=="done"{data.done+=1;if ended>now-60000{data.throughput+=1;}}
                else if job.status=="failed"{data.failed+=1;}
            }else{
                data.running.push(job); let worker=opt(job,"worker");
                if worker==(){data.loose.push(job);}
                else{
                    if !(worker in data.slots)||data.slots[worker]!=(){throw "Running jobs need a declared and unoccupied worker slot";}
                    data.slots[worker]=job;
                }
            }
        }
        data.jobs[job.id]=job;
    }
    data.waiting.sort(Fn("job_order"));
    data.recent.sort(Fn("end_order"));
    data.waits.sort();
    data.p95=if data.waits.len==0{()}else{data.waits[(data.waits.len*95+99)/100-1]};
    data.depth=pressure(starts,ends,now-props.window_ms,now);
    data.done=value(value(props,"totals",#{}),"done",data.done);data.failed=value(value(props,"totals",#{}),"failed",data.failed);
    data
}
fn act(token,kind,job,workers,ctx,event){
    ctx.emit("action",#{queue:token.queue,revision:token.revision,kind:kind,job:job,workers:workers});handled()
}
fn pin(cfg,id,ctx,event){
    if !cfg.external{ctx.set_state("open",id);}
    if cfg.notify{ctx.emit("open_change",#{queue:cfg.token.queue,revision:cfg.token.revision,job:id});}
    handled()
}
fn focus(id,index,ctx,event){ctx.set_state("cursor",id);ctx.focus("job-"+index.to_string());handled()}
fn text_part(ctx,caption,part){
    common::heading_note(ctx,caption,part).with_style(style().min_width(px(0)))
}
fn elapsed(ctx,ms){
    if ms<60000{return ctx.t_or("agent_queue.seconds","{n} s",#{n:ctx.format_number(ms/1000)});}
    ctx.t_or("agent_queue.minutes","{n} min",#{n:ctx.format_number(ms/60000)})
}
fn kind_label(ctx,job){
    if job.kind=="llm"{ctx.t_or("agent_queue.llm","LLM call")}else if job.kind=="tool"{ctx.t_or("agent_queue.tool","Tool call")}
    else if job.kind=="agent"{ctx.t_or("agent_queue.agent","Sub-agent")}else{ctx.t_or("agent_queue.human","Human review")}
}
fn percent(job,cfg,data){
    let defined=opt(job,"progress");if defined!=(){return defined.to_float();}
    let estimate=opt(job,"estimate_ms");if estimate==(){return ();}
    min(1.0,duration_without_limits(job.started_ms,cfg.now,data.ranges).to_float()/estimate.to_float())
}
fn command_button(ctx,key,title,variant,off,fn_ptr){
    button::Button(#{key:key,text:title,size:"xs",variant:variant,disabled:off,on_click:fn_ptr})
}
fn status_name(ctx,job){
    if job.status=="queued"{ctx.t_or("agent_queue.waiting","Waiting")}else if job.status=="running"{ctx.t_or("agent_queue.running","Running")}
    else if job.status=="done"{ctx.t_or("agent_queue.done","Done")}else if job.status=="failed"{ctx.t_or("agent_queue.failed","Failed")}
    else{ctx.t_or("agent_queue.cancelled","Cancelled")}
}
fn job_tile(ctx,job,cfg,data,walk,index){
    let selected=job.id==cfg.open;
    let locked=cfg.inspect.external&&!cfg.inspect.notify;
    let ink=if job.status=="failed"{theme_color("text.danger")}else if job.status=="done"{theme_color("text.success")}
        else if priority(job)==0{theme_color("text.warning")}else{theme_color("text_muted")};
    let caption=if job.status=="queued"{elapsed(ctx,cfg.now-job.enqueued_ms)}else if job.status=="running"&&data.active_limit!=(){
        ctx.t_or("agent_queue.limited","Rate limited")
    }else if job.status=="running"{elapsed(ctx,cfg.now-job.started_ms)}else{status_name(ctx,job)};
    let mark=if job.kind=="llm"{"✦"}else if job.kind=="tool"{"⌘"}else if job.kind=="agent"{"◈"}else{"●"};
    let parts=if selected{["job","job_open"]}else{["job"]};
    let children=[row([text(mark)]).with_key("mark").accessibility_hidden(true).with_style(ctx.component_style("mark",
        style().width(theme_length("space.unit").times(4.75)).height(theme_length("space.unit").times(4.75)).radius(px(5)).items_center().justify_center().flex_shrink(false).background(theme_color("surface_hover")).text_color(ink))),
        text(job.name).with_key("name").with_style(ctx.component_style("job_name",style().min_width(px(0)).flex_grow().typography("body")
            .whitespace_nowrap().text_ellipsis())),
        text_part(ctx,caption,"job_side").with_key("measure").with_style(style().text_color(ink).flex_shrink(false))];
    let description=kind_label(ctx,job)+" · "+status_name(ctx,job)+" · "+caption;
    if job.status=="running"{
        let p=percent(job,cfg,data);if p!=(){description+=" · "+ctx.format_number((p*100.0).round().to_int())+"%";}
    }
    let item=row(children).with_key("job-"+job.id).with_ref(element_ref("job-"+job.ordinal.to_string()))
        .accessibility_role("button").accessibility_label(job.name).accessibility_description(description)
        .accessibility_pressed(selected).tab_stop(!locked&&job.id==cfg.cursor).disabled(locked)
        .audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style(parts,
            style().min_width(px(0)).min_height(theme_length("space.unit").times(7.25)).gap(theme_length("space.spacing_unit").times(1.5)).padding_x(theme_length("space.spacing_unit").times(1.75)).padding_y(theme_length("space.spacing_unit")).radius(px(7)).items_center()
                .background(theme_color(if selected{"selection"}else{"surface_raised"})).hover(style().background(theme_color("surface_hover")))
                .focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))))
        .shared_layout(cfg.scope,job.id).layout_motion(ctx.motion_duration("normal"),ctx.motion_easing("standard"));
    if locked{return item;}
    let before=walk[max(0,index-1)];let after=walk[min(walk.len-1,index+1)];let rtl=ctx.text_direction()=="rtl";
    item.on_click(Fn("pin").curry(cfg.inspect,if selected{""}else{job.id}))
        .on_key_value("plain+up",Fn("focus").curry(before.id,before.ordinal),())
        .on_key_value("plain+down",Fn("focus").curry(after.id,after.ordinal),())
        .on_key_value("plain+left",Fn("focus").curry(if rtl{after.id}else{before.id},if rtl{after.ordinal}else{before.ordinal}),())
        .on_key_value("plain+right",Fn("focus").curry(if rtl{before.id}else{after.id},if rtl{before.ordinal}else{after.ordinal}),())
        .on_key_value("plain+home",Fn("focus").curry(walk[0].id,walk[0].ordinal),())
        .on_key_value("plain+end",Fn("focus").curry(walk[walk.len-1].id,walk[walk.len-1].ordinal),())
}
fn section(ctx,label,count){
    row([text(label).with_key("caption").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("section",style().typography("body").font_weight(500))),
        text_part(ctx,ctx.format_number(count),"section_count").with_key("count")]).with_key("section")
        .audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).items_center())
}
fn header(ctx,props,cfg,data){
    let title=value(props,"title",ctx.t_or("agent_queue.title","Agent queue"));
    let status=if data.active_limit!=(){ctx.t_or("agent_queue.limited","Rate limited")}else if props.paused{
        ctx.t_or("agent_queue.paused","Paused")}else if data.waiting.len+data.running.len>0{ctx.t_or("agent_queue.live","Live")}
        else{ctx.t_or("agent_queue.idle","Idle")};
    let ink=if data.active_limit!=()||props.paused{theme_color("text.warning")}else if data.waiting.len+data.running.len>0{theme_color("text.success")}else{theme_color("text_muted")};
    let left=[text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
        .with_style(ctx.component_style("title",style().min_width(px(0)).typography("body").font_weight(600)))];
    if opt(props,"name")!=(){left.push(text_part(ctx,props.name,"metadata").with_key("name"));}
    left.push(text_part(ctx,status,"state").with_key("state").accessibility_role("status").with_style(style().text_color(ink)));
    let buttons=[];let count=props.workers.len;
    if cfg.controls{
        let label=if props.paused{ctx.t_or("agent_queue.resume","Resume")}else{ctx.t_or("agent_queue.pause","Pause")};
        buttons.push(command_button(ctx,"pause",label,"secondary",props.busy,Fn("act").curry(cfg.token,if props.paused{"resume"}else{"pause"},"",0)));
        buttons.push(command_button(ctx,"decrease","−","ghost",props.busy||count<=props.min_workers,Fn("act").curry(cfg.token,"concurrency","",max(0,count-1)))
            .accessibility_label(ctx.t_or("agent_queue.decrease","Decrease concurrency")));
        buttons.push(text_part(ctx,ctx.format_number(count),"metadata").with_key("concurrency").accessibility_label(
            ctx.t_or("agent_queue.slots","{n} workers",#{n:ctx.format_number(count)})));
        buttons.push(command_button(ctx,"increase","+","ghost",props.busy||count>=props.max_workers,Fn("act").curry(cfg.token,"concurrency","",min(32,count+1)))
            .accessibility_label(ctx.t_or("agent_queue.increase","Increase concurrency")));
    }
    row([row(left).with_key("heading").audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.75)).flex_grow().items_center().flex_wrap()),
        row(buttons).with_key("controls").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("controls",style().gap(theme_length("space.spacing_unit").times(0.75)).items_center()))])
        .with_key("head").with_style(ctx.component_style("head",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.75)).items_center().flex_wrap()))
}
fn summary_item(ctx,key,label,reading,ink){
    column([text_part(ctx,label,"stat_label").with_key("label"),text(reading).with_key("number")
        .with_style(ctx.component_style("stat_value",style().typography("body").font_weight(600).text_color(ink)))])
        .with_key(key).accessibility_role("group").accessibility_label(label).accessibility_description(reading)
        .with_style(ctx.component_style("stat",style().min_width(px(0)).min_height(theme_length("space.unit").times(10)).gap(theme_length("space.spacing_unit").times(0.5)).flex_grow().padding(theme_length("space.spacing_unit").times(1.25))))
}
fn stats(ctx,props,data){
    let cap=opt(props,"backpressure");let over=cap!=()&&data.depth.depth>cap;
    let plain=theme_color("text_muted");
    let occupied=ctx.format_number(data.running.len-data.loose.len)+"/"+ctx.format_number(props.workers.len);
    if data.loose.len>0{occupied+=" · "+ctx.t_or("agent_queue.external","{n} external",#{n:ctx.format_number(data.loose.len)});}
    let view=[summary_item(ctx,"waiting",ctx.t_or("agent_queue.waiting","Waiting"),ctx.format_number(data.depth.depth),if over{theme_color("text.warning")}else{plain}),
        summary_item(ctx,"running",ctx.t_or("agent_queue.running","Running"),occupied,theme_color("accent")),
        summary_item(ctx,"throughput",ctx.t_or("agent_queue.throughput","Throughput"),
            ctx.t_or("agent_queue.per_minute","{n}/min",#{n:ctx.format_number(data.throughput)}),theme_color("text.success")),
        summary_item(ctx,"p95",ctx.t_or("agent_queue.p95","P95 wait"),if data.p95==(){"—"}else{elapsed(ctx,data.p95)},plain)];
    row(view).with_key("stats").with_style(ctx.component_style("stats",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).padding(theme_length("space.spacing_unit").times(1.75)).flex_wrap()))
}
fn flow(ctx,props,cfg,data,walk,slots){
    let waiting=[];for job in data.waiting{waiting.push(job_tile(ctx,job,cfg,data,walk,slots[job.id]));}
    if waiting.len==0{waiting.push(text_part(ctx,ctx.t_or("agent_queue.empty","No waiting jobs"),"empty").with_key("empty"));}
    let list=scroll::ScrollArea(#{key:"waiting-scroll",label:ctx.t_or("agent_queue.backlog","Waiting jobs"),height:px(270),
        content:column(waiting).with_key("waiting-items").with_style(style().gap(theme_length("space.spacing_unit")).min_width(px(0)))});
    if waiting.len<8{list=list.with_style(style().height(auto()).overflow_hidden());}
    let backlog=column([section(ctx,ctx.t_or("agent_queue.backlog","Waiting jobs"),data.waiting.len),list])
        .with_key("waiting").with_style(ctx.component_style("waiting",style().min_width(px(0)).width(if cfg.compact{relative(1.0)}else{relative(0.46)})
            .flex_shrink(false).gap(theme_length("space.spacing_unit").times(1.5)).padding(theme_length("space.spacing_unit").times(2)).background(theme_color("surface_hover")).radius(theme_length("space.unit").times(2))));
    let workers=[];
    for w in props.workers{
        let j=data.slots[w.id];let line=[text_part(ctx,w.name,"worker_name").with_key("name")];
        if opt(w,"detail")!=(){line.push(text_part(ctx,w.detail,"worker_detail").with_key("caption"));}
        let view=[row(line).with_key("identity").audit_allow(["mixed-type-in-row"]).with_style(style().gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap())];
        if j==(){
            let inactive=if data.active_limit!=(){ctx.t_or("agent_queue.limited","Rate limited")}else if props.paused{
                ctx.t_or("agent_queue.paused","Paused")}else{ctx.t_or("agent_queue.available","Available")};
            view.push(text_part(ctx,inactive,"worker_state").with_key("idle"));
        }else{
            view.push(job_tile(ctx,j,cfg,data,walk,slots[j.id]));
            let p=percent(j,cfg,data);
            if p!=(){view.push(progress::Progress(#{key:"progress-"+w.id,label:w.name+" progress",value:p,max:1.0,width:80}));}
        }
        workers.push(column(view).with_key("worker-"+w.id).accessibility_role("group").accessibility_label(w.name)
            .with_style(ctx.component_style("worker",style().min_width(px(0)).min_height(theme_length("space.unit").times(10.5)).padding(theme_length("space.spacing_unit").times(1.75)).gap(theme_length("space.spacing_unit"))
                .radius(theme_length("space.unit").times(2)).background(theme_color("surface_raised")))));
    }
    for j in data.loose{workers.push(job_tile(ctx,j,cfg,data,walk,slots[j.id]));}
    if workers.len==0{workers.push(text_part(ctx,ctx.t_or("agent_queue.no_workers","No worker slots"),"empty").with_key("empty"));}
    let group=column([section(ctx,ctx.t_or("agent_queue.workers","Workers"),props.workers.len),column(workers).with_key("worker-cards")
        .with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit")))])
        .with_key("workers").with_style(ctx.component_style("worker_group",style().min_width(px(0)).flex_grow().gap(theme_length("space.spacing_unit").times(1.5)).padding(theme_length("space.spacing_unit").times(2))));
    let body=if cfg.compact{column([backlog,group])}else{row([backlog,group]).with_style(style().items_start())};
    body.with_key("flow").with_style(ctx.component_style("flow",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5))))
}
fn bin(ctx,cfg,data,walk,slots,status,title,count){
    let shown=[];for j in data.recent{if j.status==status&&j.id in slots{shown.push(job_tile(ctx,j,cfg,data,walk,slots[j.id]));}}
    let parts=if status=="failed"{["bin","bin_failed"]}else{["bin"]};
    column([section(ctx,title,count),column(shown).with_key("recent").with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit")))])
        .with_key(status).accessibility_role("group").accessibility_label(title).accessibility_description(ctx.format_number(count))
        .with_style(ctx.component_style(parts,style().min_width(px(0)).gap(theme_length("space.spacing_unit")).padding(theme_length("space.spacing_unit").times(1.75)).flex_grow()
            .radius(theme_length("space.unit").times(2)).background(theme_color("surface_hover"))))
}
fn outcomes(ctx,cfg,data,walk,slots){
    let boxes=[bin(ctx,cfg,data,walk,slots,"done",ctx.t_or("agent_queue.done","Done"),data.done),
        bin(ctx,cfg,data,walk,slots,"failed",ctx.t_or("agent_queue.failed","Failed"),data.failed)];
    let n=0;for j in data.recent{if j.status=="cancelled"{n+=1;}}
    if n>0{boxes.push(bin(ctx,cfg,data,walk,slots,"cancelled",ctx.t_or("agent_queue.cancelled","Cancelled"),n));}
    let bins=if cfg.compact{column(boxes)}else{row(boxes)};
    bins.with_key("bins").with_style(ctx.component_style("bins",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(1.75)).gap(theme_length("space.spacing_unit").times(1.25))))
}
fn inspector(ctx,props,cfg,data){
    if !(cfg.open in data.jobs){return ();}
    let job=data.jobs[cfg.open];let attempt=value(job,"attempt",1);
    let lines=[text(job.name).with_key("title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("detail_title",style().typography("body").font_weight(600))),
        text_part(ctx,kind_label(ctx,job)+" · "+status_name(ctx,job)+" · "+ctx.t_or("agent_queue.attempt","Attempt {n}",#{n:ctx.format_number(attempt)}),"detail_meta")
            .with_key("details")];
    if opt(job,"detail")!=(){lines.push(text_part(ctx,job.detail,"detail_meta").with_key("info"));}
    if opt(job,"error")!=(){lines.push(text_part(ctx,job.error,"error").with_key("error").with_style(style().text_color(theme_color("text.danger"))));}
    let live=job.status=="queued"||job.status=="running";
    let actions=[];
    if cfg.controls&&props.allow_promote&&job.status=="queued"{
        actions.push(command_button(ctx,"promote",ctx.t_or("agent_queue.promote","Move to front"),"secondary",props.busy,Fn("act").curry(cfg.token,"promote",job.id,0)));
    }
    if cfg.controls&&props.allow_cancel&&live{
        actions.push(command_button(ctx,"cancel",ctx.t_or("agent_queue.cancel","Cancel job"),"ghost",props.busy,Fn("act").curry(cfg.token,"cancel",job.id,0)));
    }
    if cfg.controls&&props.allow_retry&&job.status=="failed"{
        actions.push(command_button(ctx,"retry",ctx.t_or("agent_queue.retry","Retry job"),"secondary",props.busy||attempt>=value(job,"max_attempts",1000),
            Fn("act").curry(cfg.token,"retry",job.id,0)));
    }
    if !cfg.inspect.external||cfg.inspect.notify{
        actions.push(command_button(ctx,"close-inspector",ctx.t_or("agent_queue.close","Close details"),"ghost",false,Fn("pin").curry(cfg.inspect,"")));
    }
    lines.push(row(actions).with_key("actions").with_style(ctx.component_style("detail_action",style().min_width(px(0)).gap(theme_length("space.spacing_unit")).flex_wrap())));
    column(lines).with_key("inspector").accessibility_role("region").accessibility_label(ctx.t_or("agent_queue.details","Job details"))
        .with_style(ctx.component_style("detail",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).padding(theme_length("space.spacing_unit").times(2.25)).radius(theme_length("space.unit").times(2)).background(theme_color("surface_hover"))))
}
fn depth_scale(depth,threshold){
    let needed=max(4,max(depth+1,if threshold==(){0}else{threshold+1}));
    for n in [4,6,8,10,12,16,20,24,30,40,50,60,80,100,120,160,200,300,400,500,600,800,1000,1500,2000]{
        if n>=needed{return n;}
    }
    ((needed+499)/500)*500
}
fn graph_scene(d,ranges,dim,limit,scale,rtl,depth_paint){
    let w=dim.width;let h=dim.height;let x0=1.0;let x1=w-3.0;let y0=h-3.0;
    let sx=(x1-x0)/(d.now-d.start).to_float();let sy=(y0-3.0)/scale.to_float();
    let begin=if rtl{x1}else{x0};let end=if rtl{x0}else{x1};let direction=if rtl{-sx}else{sx};
    let commands=[];
    for r in ranges{
        let a=max(r.from,d.start);let b=min(r.until_ms,d.now);
        if a<b{
            let left=begin+((if rtl{b}else{a})-d.start).to_float()*direction;
            commands.push(canvas_rect("limit-"+r.from.to_string(),left,3,(b-a).to_float()*sx,y0-3.0,
                alpha(theme_color("text.warning"),0.12)));
        }
    }
    commands.push(canvas_line("floor",x0,y0,x1,y0,1,theme_color("border")));
    commands.push(canvas_line("ceiling",x0,3,x1,3,1,theme_color("border")));
    let points=d.points;
    let py=y0-points[0].depth.to_float()*sy;
    let strokes=[path_move(begin,py)];
    for i in 1..points.len{
        let x=begin+(points[i].ms-d.start).to_float()*direction;
        let y=y0-points[i].depth.to_float()*sy;
        strokes.push(path_line(x,py));if y!=py{strokes.push(path_line(x,y));}
        py=y;
    }
    if d.peak>0{
        let fill=strokes;fill.push(path_line(end,y0));fill.push(path_line(begin,y0));fill.push(path_close());
        commands.push(canvas_fill_path("depth-fill",fill,depth_paint));
    }
    commands.push(canvas_stroke_path("depth",strokes,1.5,theme_color("accent")).stroke_join("round"));
    if limit!=(){
        let y=y0-limit.to_float()*sy;let span=(x1-x0)/18.0;
        for i in 0..18{
            let x=x0+i.to_float()*span;
            commands.push(canvas_line("threshold-"+i.to_string(),x,y,x+span*0.55,y,1,theme_color("text.warning")));
        }
    }
    commands.push(canvas_circle("now",end,py,2,theme_color("accent")));
    canvas_scene(commands).view_box(0,0,w,h)
}
fn chart(ctx,props,data){
    let b=opt(props,"backpressure");let scale=depth_scale(data.depth.peak,b);
    let ref_plot=element_ref("pressure-plot");let actual=ctx.element_size(ref_plot);
    let dim=if actual==()||actual.width<10.0||actual.height<10.0{#{width:380.0,height:102.0}}else{actual};
    let mean=ctx.format_number(data.depth.average,#{min_fraction_digits:1,max_fraction_digits:1});
    let summary=ctx.t_or("agent_queue.chart_reading","{waiting} waiting now · peak {peak} · mean {mean}",
        #{waiting:ctx.format_number(data.depth.depth),peak:ctx.format_number(data.depth.peak),mean:mean});
    let title=ctx.t_or("agent_queue.depth","Queue depth");
    let graph_label=title+" · "+summary;
    let count=0;for r in data.ranges{if max(r.from,data.depth.start)<min(r.until_ms,data.depth.now){count+=1;}}
    let extra=ctx.t_or("agent_queue.bands","Throttle intervals: {n}",#{n:ctx.format_number(count)});
    if b!=(){extra+=" · "+ctx.t_or("agent_queue.pressure","Backpressure: {n}",#{n:ctx.format_number(b)});}
    let labels=column([text_part(ctx,ctx.format_number(scale),"scale").with_key("max"),text_part(ctx,ctx.format_number(0),"scale").with_key("min")])
        .with_key("scale").with_style(style().justify_between().min_width(px(22)).flex_shrink(false));
    let depth_paint=ctx.component_style("depth_fill",style().background(alpha(theme_color("accent"),0.12)));
    let draw=canvas(graph_scene(data.depth,data.ranges,dim,b,scale,ctx.text_direction()=="rtl",depth_paint)).with_key("plot").with_ref(ref_plot).accessibility_hidden(true)
        .with_style(ctx.component_style("plot",style().min_width(px(0)).height(theme_length("space.unit").times(25.5)).flex_grow()));
    let foot=[text_part(ctx,ctx.t_or("agent_queue.ago","{time} ago",#{time:elapsed(ctx,props.window_ms)}),"axis").with_key("past")];
    if b!=(){foot.push(text_part(ctx,ctx.t_or("agent_queue.pressure","Backpressure: {n}",#{n:ctx.format_number(b)}),"axis")
        .with_key("threshold").with_style(style().text_color(theme_color("text.warning"))));}
    foot.push(text_part(ctx,ctx.t_or("agent_queue.now","Now"),"axis").with_key("now"));
    column([text(title).with_key("title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("chart_title",style().typography("body").font_weight(500))),
        row([labels,draw]).with_key("graph").accessibility_role("image").accessibility_label(graph_label).accessibility_description(extra)
            .with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit"))),
        row(foot).with_key("axis").with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap().justify_between())])
        .with_key("chart").with_style(ctx.component_style("chart",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).padding(theme_length("space.spacing_unit").times(2.25))))
}
fn render_AgentQueue(ctx,props){
    let data=project(props);let shell=element_ref("queue");let open=opt(props,"open_id");
    let inspect=#{external:open!=(),notify:opt(props,"on_open_change")!=(),token:#{queue:props.queue,revision:props.revision}};
    let selected=if open!=(){open}else{
        let chosen=ctx.get_state("open");if chosen!=(){chosen}else{value(props,"default_open_id","")}
    };
    let walk=[];for w in props.workers{if data.slots[w.id]!=(){walk.push(data.slots[w.id]);}}
    for j in data.loose{walk.push(j);}for j in data.waiting{walk.push(j);}
    for status in ["done","failed","cancelled"]{
        let n=0;for j in data.recent{if j.status==status&&n<3{walk.push(j);n+=1;}}
    }
    let slots=#{};for i in 0..walk.len{slots[walk[i].id]=i;}
    let cursor=ctx.get_state("cursor");if !(cursor in slots)&&walk.len>0{cursor=walk[0].id;}
    let cfg=#{scope:shell.semantic_id,token:inspect.token,inspect:inspect,controls:opt(props,"on_action")!=(),now:props.now_ms,
        open:selected,cursor:cursor,compact:ctx.element_width_class(shell,[620])==0};
    let face=[stats(ctx,props,data),flow(ctx,props,cfg,data,walk,slots)];
    let recent=outcomes(ctx,cfg,data,walk,slots);if recent!=(){face.push(recent);}
    let details=inspector(ctx,props,cfg,data);if details!=(){face.push(details);}
    face.push(chart(ctx,props,data));
    // The first glyph in a statistic, queue tray or outcome bin is inset
    // within its own surface. Align the panel edges, not those glyphs.
    let nodes=[header(ctx,props,cfg,data),column(face).with_key("card").audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("card",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).radius(theme_length("space.unit").times(2.5)).background(theme_color("surface_raised"))))];
    if data.active_limit!=(){
        let message=value(data.active_limit,"label",ctx.t_or("agent_queue.limited","Rate limited"));
        nodes.push(text_part(ctx,message+" · "+elapsed(ctx,data.active_limit.until_ms-props.now_ms),"notice").with_key("rate").accessibility_role("status")
            .with_style(style().text_color(theme_color("text.warning"))));
    }
    if opt(props,"notice")!=(){nodes.push(text_part(ctx,props.notice,"notice").with_key("notice").accessibility_role("status"));}
    let root=column(nodes).with_key(props.key).with_ref(shell).accessibility_role("group")
        .accessibility_label(value(props,"title",ctx.t_or("agent_queue.title","Agent queue")))
        .with_style(ctx.component_style("root",style().min_width(px(0)).max_width(px(950)).width(relative(1.0)).gap(theme_length("space.spacing_unit").times(1.5)).padding(theme_length("space.spacing_unit").times(1.25))));
    if details!=()&&(!inspect.external||inspect.notify){root=root.on_bubble("key:plain+escape",Fn("pin").curry(inspect,""));}
    root
}
