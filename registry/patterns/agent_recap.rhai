/* gpui-rhai
{"id":"patterns/agent_recap","export":"AgentRecap","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/spinner","patterns/agent_tree","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["agent_recap.area","agent_recap.cursor","agent_recap.dot","agent_recap.failed","agent_recap.grid","agent_recap.needs","agent_recap.stroke","border","button.primary_bg","button.primary_fg","focus_ring","space.unit","surface_hover","surface_raised","text.success","text_muted","text_primary","typography.body","typography.caption"],"environment":[]}
*/
// Replay an accepted interval of agent activity and send versioned event commands to the Host.
// Example: AgentRecap(#{key:"night",run:"night/1",revision:0,since_ms:0,until_ms:3600000,events:[]})
import "components/button" as button;
import "components/spinner" as spinner;
import "patterns/agent_tree" as ids;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_recap","export":"AgentRecap",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/spinner","patterns/agent_tree","patterns/approval_card"],capabilities:#{},
        tokens:["agent_recap.area","agent_recap.cursor","agent_recap.dot","agent_recap.failed","agent_recap.grid","agent_recap.needs",
            "agent_recap.stroke","border","button.primary_bg","button.primary_fg","focus_ring","space.unit","surface_hover","surface_raised",
            "text.success","text_muted","text_primary","typography.body","typography.caption"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable surface key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of the event snapshot series."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision of accepted events and outcomes."},
        since_ms:#{schema:#{type:"integer",min:0,max:253402300799999},required:true,sensitive:false,doc:"First epoch millisecond in the replay interval."},
        until_ms:#{schema:#{type:"integer",min:1,max:253402300799999},required:true,sensitive:false,doc:"Last epoch millisecond; must be later than since_ms."},
        time_zone:#{schema:#{type:"string"},"default":#{type:"string",value:"UTC"},required:false,sensitive:false,doc:"Display zone for epoch timestamps, not an offset for curve positions."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Recap title."},
        limit:#{schema:#{type:"integer",min:1,max:8},"default":#{type:"integer",value:3},required:false,sensitive:false,doc:"Maximum visible cards in each lane. Summary and plot always include the whole interval."},
        board:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Show the three lanes after the activity plot."},
        replay_ms:#{schema:#{type:"integer",min:0,max:8000},"default":#{type:"integer",value:1800},required:false,sensitive:false,doc:"Decorative replay length; reduced and disabled motion show the whole snapshot immediately."},
        replay_serial:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Increment to explicitly replay the current series. Resizing, theme and ordinary revision changes do not restart it."},
        events:#{schema:#{type:"array",max_items:72,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique business key; stable across revisions."},
            at_ms:#{schema:#{type:"integer",min:0,max:253402300799999},required:true,sensitive:false,doc:"Event epoch milliseconds. Events outside the supplied interval do not appear."},
            kind:#{schema:#{type:"string",allowed:["needs","failed","done"]},required:true,sensitive:false,doc:"Original lane. Successful commands do not change a card's lane or key."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Event's accessible title."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Secondary context, or failure explanation."},
            logo:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative mark; defaults to an initial."},
            count:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000}},required:false,sensitive:false,doc:"Completed work quantity. Affects peak height, not the event count."},
            action:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Custom primary label for a needs event."},
            done_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Custom accepted outcome label for a needs event."},
            state:#{schema:#{type:"optional",value:#{type:"string",allowed:["open","pending","resolved","undone"]}},required:false,sensitive:false,
                doc:"Host-accepted state. For an originally done event use open or undone. Pending does not accept another command."},
            can_undo:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Set false to omit Undo for an irreversible outcome."},
        }}},required:true,sensitive:true,doc:"Up to 72 events, sortable by epoch time and stable ID."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives primary, review, undo or redo proposals; the application owns all business outcomes."},
        on_inspect:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the versioned inspected event ID (empty means inspection ended)."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional Host response, presented as a semantic status."},
    },state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{
        action:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run owning the snapshot."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision observed by the user."},
            event:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business ID of the event."},
            action:#{schema:#{type:"string",allowed:["primary","review","undo","redo"]},required:true,sensitive:false,doc:"Requested action, independent of its visible label."}}},
            doc:"Proposal against an accepted revision."},
        inspect:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run owning the inspected snapshot."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision observed by the user."},
            event:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business ID, or an empty string when inspection ends."}}},doc:"Local dot/card inspection changed."},
    },slots:#{},parts:["root","panel","head","identity","title","since","pill","summary","summary_item","summary_count","summary_label",
        "plot","curve_fill","times","time","dot","tip","board","lane","lane_head","lane_label","lane_count","stack","card","card_hot","card_head","mark",
        "card_title","card_quiet","meta","detail","failure","quantity","footer","status","working","undo_state","primary","quiet","empty","notice"],
        effects:["reconcile"]},
    render:Fn("render_recap")
});

fn AgentRecap(props){render_component("patterns/agent_recap",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,v){common::value_or(m,k,v)}
fn recap_space(n){theme_length("space.unit").times(n)}
fn kind_list(){["needs","failed","done"]}
fn state(e){value(e,"state","open")}
fn stable_order(a,b){if a.at_ms==b.at_ms{if a.id==b.id{0}else if a.id<b.id{-1}else{1}}else if a.at_ms<b.at_ms{-1}else{1}}
fn complete(e){(e.kind=="done"&&state(e)!="undone")||state(e)=="resolved"}
fn active_in_lane(e){!(state(e) in ["resolved","undone"])}
fn project(props){
    if !ids::valid(props.run)||props.until_ms<=props.since_ms{throw "AgentRecap needs a stable run and a positive time interval";}
    let span=props.until_ms-props.since_ms;
    let all=#{};let events=[];let bytes=0;
    for e in props.events{
        if !ids::valid(e.id)||e.id in all{throw "Recap event IDs must be distinct and bounded";}
        let s=state(e);
        if (e.kind=="done"&&!(s in ["open","undone"]))||(e.kind!="done"&&s=="undone"){
            throw "Recap event state does not match its original lane";
        }
        if e.title==""{throw "Recap event titles must not be empty";}
        for name in ["title","detail","action","done_label"]{bytes+=utf8_byte_count(value(e,name,""));}
        all[e.id]=true;
        if e.at_ms>=props.since_ms&&e.at_ms<=props.until_ms{events.push(e);}
    }
    if bytes>131072{throw "Recap text exceeds 128 KiB";}
    events.sort(Fn("stable_order"));
    let lanes=#{needs:[],failed:[],done:[]};let by_id=#{};let ordered=[];
    for e in events{
        e.t=(e.at_ms-props.since_ms).to_float()/span.to_float();
        let lane=lanes[e.kind];lane.push(e);lanes[e.kind]=lane;by_id[e.id]=e;
        ordered.push(e);
    }
    #{events:ordered,lanes:lanes,keys:by_id,span:span}
}
fn epoch(props){utf8_byte_count(props.run).to_string()+":"+props.run+":"+props.replay_serial.to_string()+":"+props.replay_ms.to_string()}
fn ui(ctx,token){
    let u=ctx.get_state("ui");
    if u!=()&&u.epoch==token{return u;}
    #{epoch:token,head:0,hover:"",hover_source:"",focus:"",focus_source:"",pin:""}
}
fn chosen(u){if u.hover!=""{u.hover}else if u.focus!=""{u.focus}else{u.pin}}
fn pulse(token,head,ctx,e){
    let u=ui(ctx,token);
    if u.head<head{u.head=head;ctx.set_state("ui",u);}
}
fn release(ctx,p){}
fn reconcile(ctx,p){
    let u=ui(ctx,p.epoch);let next=u;
    if p.skip{next.head=8;}
    for name in ["hover","focus","pin"]{if next[name]!=""&&!(next[name] in p.ids){next[name]="";}}
    if next!=u{ctx.set_state("ui",next);}
}
fn save_inspection(ctx,cfg,u,old){
    let now=chosen(u);ctx.set_state("ui",u);
    if cfg.inspect&&now!=old{ctx.emit("inspect",#{run:cfg.run,revision:cfg.revision,event:now});}
}
fn inspect(cfg,id,source,is_focus,ctx,inside){
    let u=ui(ctx,cfg.epoch);let old=chosen(u);
    let kind=if is_focus{"focus"}else{"hover"};let owner=kind+"_source";
    if inside{u[kind]=id;u[owner]=source;}
    else if u[kind]==id&&u[owner]==source{u[kind]="";u[owner]="";}
    else{return;}
    save_inspection(ctx,cfg,u,old);
}
fn activate_dot(cfg,id,ctx,e){
    let u=ui(ctx,cfg.epoch);let old=chosen(u);
    u.pin=if u.pin==id{""}else{id};
    save_inspection(ctx,cfg,u,old);handled()
}
fn propose(cfg,id,action,ctx,e){
    ctx.emit("action",#{run:cfg.run,revision:cfg.revision,event:id,action:action});handled()
}
fn note(ctx,words,part){
    text(words).with_style(ctx.component_style(part,style().typography("caption").text_color(theme_color("text_muted")).min_width(px(0)).whitespace_nowrap().text_ellipsis()))
}
fn time(ctx,ms,props){ctx.format_time(ms,"short",props.time_zone)}
fn lane_name(ctx,key){
    switch key{
        "needs"=>ctx.t_or("agent_recap.needs","Needs you"),
        "failed"=>ctx.t_or("agent_recap.failed","Failed"),
        _=>ctx.t_or("agent_recap.done","Done"),
    }
}
fn icon(ctx,key){
    let shape=switch key{
        "needs"=>"M5 14v-1a4 4 0 0 1 4-4h2a4 4 0 0 1 4 4v1M12.7 5.3a2.7 2.7 0 1 1-5.4 0 2.7 2.7 0 0 1 5.4 0",
        "failed"=>"M9 2.8 2 15h14L9 2.8zm0 5v3.5m0 1.6v.3",
        _=>"M5.2 9.1 7.8 12l5-5M16 9a7 7 0 1 1-14 0 7 7 0 0 1 14 0",
    };
    svg(`<svg viewBox='0 0 18 18' fill='none' stroke='currentColor' stroke-width='1.4' stroke-linecap='round' stroke-linejoin='round'><path d='${shape}'/></svg>`)
        .with_key("icon").accessibility_hidden(true).with_style(style().width(recap_space(3.5)).height(recap_space(3.5)).flex_shrink(false))
}
fn summary(ctx,props,data,cfg,title){
    let counts=#{needs:0,failed:0,done:0};
    for e in data.events{
        if !seen(cfg,e){continue;}
        if complete(e){counts.done+=1;}
        else if e.kind!="done"{counts[e.kind]+=1;}
    }
    let stats=[];
    for k in ["done","needs","failed"]{
        let label=lane_name(ctx,k);
        stats.push(row([icon(ctx,k),note(ctx,ctx.format_number(counts[k]),"summary_count").with_key("amount"),
            note(ctx,label,"summary_label").with_key("name")]).with_key(k).audit_allow(["mixed-type-in-row"])
            .accessibility_role("listitem").accessibility_label(label+": "+ctx.format_number(counts[k]))
            .with_style(ctx.component_style("summary_item",style().gap(recap_space(1)).items_center().flex_shrink(false))));
    }
    let clock=time(ctx,props.since_ms,props);
    let identity=column([text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
        .with_style(ctx.component_style("title",style().typography("body").font_weight(600).text_color(theme_color("text_primary")).min_width(px(0)))),
        row([note(ctx,ctx.t_or("agent_recap.since","Since {time}",#{time:clock}),"since").with_key("since"),
            note(ctx,ctx.format_duration(data.span),"pill").with_key("duration")]).with_key("period").audit_allow(["mixed-type-in-row"])
            .with_style(style().min_width(px(0)).items_center().gap(recap_space(1.5)).flex_wrap())
    ]).with_key("identity").with_style(ctx.component_style("identity",style().min_width(px(0)).gap(recap_space(0.5))));
    row([identity,row(stats).with_key("summary").accessibility_role("list").accessibility_label(ctx.t_or("agent_recap.summary","Recap summary"))
        .with_style(ctx.component_style("summary",style().min_width(px(0)).items_center().flex_wrap().gap(recap_space(3))))])
        .with_key("head").with_style(ctx.component_style("head",style().min_width(px(0)).justify_between().items_start().gap(recap_space(2)).flex_wrap()))
}

fn seen(cfg,e){cfg.head==8||e.t<=cfg.head.to_float()/8.0}
fn gaussian(distance){exp(-(distance*distance)/(2.0*0.022*0.022))}
fn activity(data){
    // Nearby samples only (four standard deviations each side). This work is
    // tied to semantic steps/inspection, never to native animation frames.
    let height=[];for i in 0..121{height.push(0.04);}
    for e in data.events{
        let weight=if e.kind=="done"{0.7+log(value(e,"count",1).to_float()+1.0)*0.45}else{1.05};
        let center=(e.t*120.0).floor().to_int();
        for i in max(0,center-11)..min(121,center+12){height[i]+=weight*gaussian(i.to_float()/120.0-e.t);}
    }
    let peak=0.2;for y in height{peak=max(peak,y);}
    for i in 0..height.len{height[i]=0.98-height[i]/peak*0.84;}
    height
}
fn ordinate(ys,t){
    let x=t*120.0;let i=min(119,x.floor().to_int());let f=x-i.to_float();
    ys[i]+(ys[i+1]-ys[i])*f
}
fn curve(ctx,w,h,ys){
    let dx=w/120.0;let controls=[path_move(0,ys[0]*h)];
    for i in 0..120{
        let x=i.to_float()*dx;let a=ys[max(i-1,0)];let y=ys[i];let z=ys[i+1];let b=ys[min(i+2,120)];
        controls.push(path_cubic(x+dx,z*h,x+dx/3.0,(y+(z-a)/6.0)*h,x+dx*2.0/3.0,(z-(b-y)/6.0)*h));
    }
    let area=controls;area.push(path_line(w,h));area.push(path_line(0,h));area.push(path_close());
    let wash=ctx.component_style("curve_fill",style().background(theme_color("agent_recap.area")));
    [canvas_fill_path("wash",area,wash),
        canvas_stroke_path("activity",controls,1.5,theme_color("agent_recap.stroke")).stroke_join("round")]
}
fn grid(data,props,w,h){
    let strokes=[];let color=theme_color("agent_recap.grid");
    for i in 0..5{
        let y=h*i.to_float()/4.0;
        strokes.push(canvas_stroke_path("row:"+i.to_string(),[path_move(0,y),path_line(w,y)],1,color));
    }
    // Use hourly divisions for ordinary recaps and keep long intervals bounded.
    let hour=3600000;let stride=hour*max(1,(data.span+hour*12-1)/(hour*12));
    let tick=(props.since_ms/stride+1)*stride;
    while tick<props.until_ms {
        let x=w*(tick-props.since_ms).to_float()/data.span.to_float();
        strokes.push(canvas_stroke_path("tick:"+tick.to_string(),[path_move(x,0),path_line(x,h)],1,color));tick+=stride;
    }
    strokes
}
fn painted(commands,w,h,key){
    canvas(canvas_scene(commands)).with_key(key).accessibility_hidden(true)
        .with_style(style().absolute().left(px(0)).top(px(0)).width(px(w)).height(px(h)))
}
fn plot(ctx,props,data,cfg){
    let reference=element_ref("plot");let s=ctx.element_size(reference);let items=[];
    if s!=()&&s.width>1.0&&s.height>1.0{
        let w=s.width;let h=s.height;let ys=activity(data);
        items.push(painted(grid(data,props,w,h),w,h,"grid"));
        let layer=stack([painted(curve(ctx,w,h,ys),w,h,"curve")]).with_key("activity-reveal:"+cfg.epoch).accessibility_hidden(true)
            .with_style(style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)).clip());
        if cfg.animate{
            layer=layer.motion(motion_transition("width_fraction",0.0,1.0,#{duration_ms:props.replay_ms,
                easing:cubic_bezier(0.333333333,1.0,0.666666667,1.0),intent:"decorative"}));
        }
        items.push(layer);
        for e in data.events{
            if !seen(cfg,e){continue;}
            let hot=cfg.hot==e.id;let r=if hot{4.5}else{3.0};
            let paint=if complete(e){ "agent_recap.dot" }else if e.kind=="needs"{"agent_recap.needs"}
                else if e.kind=="failed"{"agent_recap.failed"}else{"agent_recap.dot"};
            let x=min(w-7.0,max(7.0,e.t*w));let y=min(h-7.0,max(7.0,ordinate(ys,e.t)*h));
            let dot=row([]).with_key("point").accessibility_hidden(true).with_style(style().width(px(r*2.0)).height(px(r*2.0))
                .radius(px(r)).background(theme_color(paint)));
            let phrase=time(ctx,e.at_ms,props)+" · "+e.title;
            items.push(row([dot]).with_key("dot:"+e.id).accessibility_role("button").accessibility_label(phrase)
                .accessibility_pressed(cfg.pin==e.id)
                .with_style(ctx.component_style("dot",style().absolute().left(offset_px(x-7.0)).top(offset_px(y-7.0))
                    .items_center().justify_center().width(px(14)).height(px(14)).radius(px(7))
                    .focus_ring(px(1),offset_px(0),theme_color("focus_ring"))))
                .on_hover_change(Fn("inspect").curry(cfg,e.id,"dot",false))
                .on_focus_within_change(Fn("inspect").curry(cfg,e.id,"dot",true))
                .on_click(Fn("activate_dot").curry(cfg,e.id)));
        }
        if cfg.hot in data.keys{
            let e=data.keys[cfg.hot];
            let x=min(w-7.0,max(7.0,e.t*w));
            items.insert(2,painted([canvas_stroke_path("hair",[path_move(x,0),path_line(x,h)],1,theme_color("agent_recap.cursor"))],w,h,"cursor"));
            let span=min(270.0,max(1.0,w-4.0));let left=max(0.0,min(w-span,x-span/2.0));
            items.push(row([note(ctx,time(ctx,e.at_ms,props)+" · "+e.title,"tip").with_key("tip-text")]).with_key("tooltip")
                .accessibility_hidden(true).with_style(style().absolute().left(px(left)).bottom(px(h-2.0)).width(px(span))));
        }
    }
    stack(items).with_key("plot").with_ref(reference).accessibility_role("group")
        .accessibility_label(ctx.t_or("agent_recap.plot","Activity timeline"))
        .with_style(ctx.component_style("plot",style().relative().width(relative(1.0)).height(recap_space(24))))
}
fn marker(cfg,props,bin){
    let progress=bin.to_float()/8.0;
    let ms=if bin==8{props.replay_ms}else{max(1,((1.0-exp(ln(1.0-progress)/3.0))*props.replay_ms.to_float()).round().to_int())};
    let track=motion_track(".",motion_transition("opacity",0.0,0.0,#{duration_ms:ms,intent:"decorative"}));
    row([]).with_key("replay:"+cfg.epoch+":"+bin.to_string()).accessibility_hidden(true)
        .with_style(style().absolute().width(px(0)).height(px(0)))
        .timeline(motion_timeline("recap-"+bin.to_string(),track,#{on_complete:Fn("pulse").curry(cfg.epoch,bin)}))
}

fn command(ctx,cfg,e,key,action,label,part,working){
    button::Button(#{key:key+":"+e.id,text:label,size:"xs",variant:if part=="primary"{"primary"}else{"ghost"},loading:working,
        on_click:Fn("propose").curry(cfg,e.id,action)}).with_key(key)
        .tab_stop(true)
        .accessibility_label(ctx.t_or("agent_recap.command","{action}: {title}",#{action:label,title:e.title}))
        .with_style(ctx.component_style(part,style().min_width(px(0))))
}
fn commands(ctx,cfg,e){
    let s=state(e);let body=[];
    if e.kind=="done"||s=="resolved"{
        let message=if s=="undone"{ctx.t_or("agent_recap.undone","Undone")}
            else if e.kind=="failed"{ctx.t_or("agent_recap.fixed","Fixed")}
            else{value(e,"done_label",ctx.t_or("agent_recap.done","Done"))};
        let role=if s=="undone"{"undo_state"}else{"status"};
        body.push(row([note(ctx,message,role).with_key("status")]).with_key("receipt")
            .with_style(style().min_width(px(0)).flex_grow()));
        if cfg.connected&&value(e,"can_undo",true){
            body.push(command(ctx,cfg,e,"action",if s=="undone"{"redo"}else{"undo"},
                if s=="undone"{ctx.t_or("agent_recap.redo","Redo")}else{ctx.t_or("agent_recap.undo","Undo")},"quiet",false));
        }
    }else if cfg.connected{
        if e.kind=="needs"{body.push(command(ctx,cfg,e,"review","review",ctx.t_or("agent_recap.review","Review"),"quiet",s=="pending"));}
        let label=if e.kind=="failed"{ctx.t_or("agent_recap.retry","Retry")}else{value(e,"action",ctx.t_or("agent_recap.approve","Approve"))};
        body.push(command(ctx,cfg,e,"action","primary",label,"primary",s=="pending"));
    }
    let actions=row(body).with_key("footer").with_style(ctx.component_style("footer",style().min_width(px(0)).min_height(recap_space(7))
        .items_center().justify_end().gap(recap_space(1)).flex_wrap()))
        .with_style(style().relative());
    if s=="pending"{
        let label=if e.kind=="failed"{ctx.t_or("agent_recap.retrying","Retrying…")}else{ctx.t_or("agent_recap.working","Working…")};
        // Keep the same primary focus owner in place. The accepted pending
        // state blocks input while a separate decorative mark turns.
        actions=stack([actions,row([spinner::Spinner(#{key:"progress:"+e.id,size:"xs",label:label}).with_key("progress")])
            .with_key("pending").with_style(style().absolute().left(px(0)).top(px(0)))])
            .with_key("footer-pending").with_style(style().relative());
    }else{
        actions=stack([actions]).with_key("footer-pending").with_style(style().relative());
    }
    actions
}
fn card(ctx,props,cfg,e){
    let quiet=state(e) in ["resolved","undone"];
    let mark=if complete(e)&&e.kind!="done"{icon(ctx,"done")}else{value(e,"logo",text(e.title.sub_string(0,1).to_upper()))};
    let header=row([row([mark]).with_key("mark").accessibility_hidden(true)
            .with_style(ctx.component_style("mark",style().width(recap_space(5)).height(recap_space(5)).flex_shrink(false).items_center().justify_center())),
        text(e.title).with_key("title").with_style(ctx.component_style(if quiet{["card_title","card_quiet"]}else{["card_title"]},
            style().typography("body").font_weight(500).text_color(theme_color("text_primary")).min_width(px(0)).whitespace_nowrap().text_ellipsis()))])
        .with_key("head").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("card_head",style().gap(recap_space(2)).items_center()));
    let detail=note(ctx,value(e,"detail",""),if e.kind=="failed"&&state(e)=="open"{"failure"}else{"detail"}).with_key("detail");
    let trail=[detail.with_style(style().flex_grow())];
    if opt(e,"count")!=(){trail.push(note(ctx,ctx.format_number(e.count),"quantity").with_key("quantity").with_style(style().flex_shrink(false)));}
    trail.push(note(ctx,time(ctx,e.at_ms,props),"time").with_key("time").with_style(style().flex_shrink(false)));
    let meta=row(trail).with_key("meta").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("meta",style().gap(recap_space(2)).padding_start(recap_space(7)).min_width(px(0)).items_center()));
    let body=column([header,meta,commands(ctx,cfg,e)]).with_key("event:"+e.id).accessibility_role("listitem")
        .accessibility_label(e.title).accessibility_description(value(e,"detail",""))
        .with_style(ctx.component_style(if cfg.hot==e.id{["card","card_hot"]}else{["card"]},style().min_width(px(0))
            .min_height(recap_space(24)).gap(recap_space(0.5)).padding(recap_space(2.5)).background(theme_color("surface_raised"))))
        .on_hover_change(Fn("inspect").curry(cfg,e.id,"card",false))
        .on_focus_within_change(Fn("inspect").curry(cfg,e.id,"card",true));
    if cfg.animate{body=body.motion(motion_transition("opacity",0.0,1.0,#{duration_ms:260,intent:"decorative"}))
        .motion(motion_transition("translate_y",4.0,0.0,#{duration_ms:320,intent:"decorative"}));}
    body
}
fn lane(ctx,props,data,cfg,k){
    let name=lane_name(ctx,k);let events=data.lanes[k];let rows=[];let shown=0;let count=0;
    for e in events{
        if seen(cfg,e)&&active_in_lane(e){count+=1;}
        if shown<props.limit{if seen(cfg,e){rows.push(card(ctx,props,cfg,e));}shown+=1;}
    }
    if events.len==0{
        let empty=switch k {
            "needs"=>ctx.t_or("agent_recap.empty_needs","Nothing needs you"),
            "failed"=>ctx.t_or("agent_recap.empty_failed","Nothing failed"),
            _=>ctx.t_or("agent_recap.empty_done","Nothing done yet"),
        };
        rows.push(note(ctx,empty,"empty").with_key("empty"));
    }
    column([row([row([icon(ctx,k),note(ctx,name,"lane_label").with_key("label")]).with_key("label")
            .with_style(style().items_center().gap(recap_space(2))),
            note(ctx,ctx.format_number(count),"lane_count").with_key("count")]).with_key("lane-head").audit_allow(["mixed-type-in-row"])
            .accessibility_role("heading").accessibility_level(3).accessibility_label(name)
            .with_style(ctx.component_style("lane_head",style().min_height(recap_space(8)).padding_x(recap_space(2.5)).justify_between().items_center())),
        column(rows).with_key("cards").accessibility_role("list").accessibility_label(name)
            .with_style(ctx.component_style("stack",style().min_width(px(0)).min_height(recap_space(25*props.limit)).gap(recap_space(1))))
    ]).with_key(k).with_style(ctx.component_style("lane",style().min_width(px(0))))
}
fn render_recap(ctx,props){
    let data=project(props);let token=epoch(props);let u=ui(ctx,token);
    let skip=props.replay_ms==0||ctx.motion_preference()!="normal";
    effect("reconcile",#{epoch:token,skip:skip,ids:keys_map(data.events)},Fn("reconcile"),Fn("release"));
    let cfg=#{run:props.run,revision:props.revision,epoch:token,head:if skip{8}else{u.head},
        animate:!skip&&u.head<8,inspect:opt(props,"on_inspect")!=(),connected:opt(props,"on_action")!=(),pin:u.pin,hot:chosen(u)};
    let title=value(props,"title",ctx.t_or("agent_recap.title","While you were away"));
    let shell=element_ref("recap");
    let t_start=time(ctx,props.since_ms,props);let t_end=time(ctx,props.until_ms,props);
    let start_note=note(ctx,t_start,"time").with_key("since")
        .accessibility_label(ctx.t_or("agent_recap.start_time","Start: {time}",#{time:t_start}));
    let end_note=note(ctx,t_end,"time").with_key("until")
        .accessibility_label(ctx.t_or("agent_recap.end_time","End: {time}",#{time:t_end}));
    // The Canvas x axis increases in physical pixel order in every locale.
    // Mirror visual board chrome, not the two labels of that numeric axis.
    let times=row(if ctx.text_direction()=="rtl"{[end_note,start_note]}else{[start_note,end_note]}).with_key("times")
        .with_style(ctx.component_style("times",style().min_width(px(0)).justify_between().flex_wrap()));
    let out=[column([summary(ctx,props,data,cfg,title),plot(ctx,props,data,cfg),times]).with_key("panel")
        .with_style(ctx.component_style("panel",style().min_width(px(0)).gap(recap_space(1)).padding(recap_space(4)).background(theme_color("surface_raised"))))];
    if props.board{
        let wide=ctx.element_width_class(shell,[640])==1;
        let lanes=[];for k in kind_list(){lanes.push(lane(ctx,props,data,cfg,k));}
        out.push(box(lanes).with_key("board").with_style(ctx.component_style("board",
            style().min_width(px(0)).grid().grid_cols(if wide{3}else{1}).gap(recap_space(1)))));
    }
    let notice=opt(props,"notice");if notice!=(){out.push(note(ctx,notice,"notice").with_key("notice").accessibility_role("status"));}
    if cfg.animate{for bin in 1..9{out.push(marker(cfg,props,bin));}}
    column(out).with_key(props.key).with_ref(shell).accessibility_role("group").accessibility_label(title)
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).relative().gap(recap_space(1))))
}
fn keys_map(events){let map=#{};for e in events{map[e.id]=true;}map}
