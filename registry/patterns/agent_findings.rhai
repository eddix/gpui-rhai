/* gpui-rhai
{"id":"patterns/agent_findings","export":"AgentFindings","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/avatar","components/progress","patterns/agent_tree","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","agent_findings.critical","agent_findings.faint","agent_findings.heading_case","agent_findings.note","agent_findings.quiet","agent_findings.warning","border","focus_ring","selection","space.unit","surface_hover","surface_raised","text_primary","typography.body","typography.caption","typography.code"],"environment":[]}
*/
// Present a team of checking agents, live progress and a bounded newest-first feed of findings.
// Example: AgentFindings(#{key:"scan",run:"check/1",revision:0,now_ms:0,agents:[],findings:[]})
import "components/avatar" as avatar;
import "components/progress" as progress;
import "patterns/agent_tree" as ids;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_findings","export":"AgentFindings",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/avatar","components/progress","patterns/agent_tree","patterns/approval_card"],capabilities:#{},
        tokens:["accent","agent_findings.critical","agent_findings.faint","agent_findings.heading_case","agent_findings.note","agent_findings.quiet",
            "agent_findings.warning","border","focus_ring","selection","space.unit","surface_hover","surface_raised","text_primary",
            "typography.body","typography.caption","typography.code"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable retained surface key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable business identity of the verification run. Replace it to start a new run."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision of the accepted agents and findings snapshot."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Heading; defaults to Verification."},
        now_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:253402300799999}},required:false,sensitive:false,
            doc:"Accepted epoch millisecond clock. If omitted a view-owned five-second wall-clock refresh runs only when the shown feed has timed findings."},
        rows:#{schema:#{type:"integer",min:1,max:12},"default":#{type:"integer",value:4},required:false,sensitive:false,
            doc:"Maximum active rows in the newest-first feed; totals count the full input."},
        agents:#{schema:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique agent ID within the run."},
            name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Displayed and accessible agent name."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional agent role or model."},
            avatar:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative logo or a picture; omit for initials."},
            avatar_image:#{schema:#{type:"optional",value:#{type:"handle",kind:"image"}},required:false,sensitive:false,doc:"Native avatar image with initials on load failure. Cannot be combined with avatar."},
            step:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Current activity, when running."},
            done:#{schema:#{type:"integer",min:0,max:1000000000},required:true,sensitive:false,doc:"Completed checks; display caps at total."},
            total:#{schema:#{type:"integer",min:0,max:1000000000},required:true,sensitive:false,doc:"Planned checks."},
            status:#{schema:#{type:"optional",value:#{type:"string",allowed:["running","done","failed"]}},required:false,sensitive:false,
                doc:"Accepted phase; defaults to done once the planned count is reached."},
        }}},required:true,sensitive:true,doc:"Up to 24 agents in the Host's presentation order."},
        findings:#{schema:#{type:"array",max_items:256,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable finding ID; keep it through ledger updates."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Owning agent ID. Findings remain visible after their agent leaves; an empty ID denotes an unassigned finding."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Visible and accessible title."},
            severity:#{schema:#{type:"string",allowed:["critical","warning","note"]},required:true,sensitive:false,doc:"Severity shape and text."},
            location:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional source location or application-specific reference."},
            at_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:253402300799999}},required:false,sensitive:false,
                doc:"Finding epoch milliseconds. Missing times sort last, ties retain Host input order."},
        }}},required:true,sensitive:true,doc:"Up to 256 accepted findings for the feed and counts."},
        on_open:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,
            doc:"Receives a versioned open request. Omit to show a read-only feed."},
    },state:#{fields:#{
        selection:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        clock:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        open:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run owning the accepted snapshot."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision observed by the user."},
            finding:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Finding ID to open, not its title or row index."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Original agent ID, also for departed or unassigned agents."},
        }},doc:"Emitted only on activation of a displayed finding; the Host opens the correct detail."},
    },slots:#{},parts:["root","bar","spark","title","completion","totals","total","severity","critical","warning","note","faint",
        "agents","agent","selected","dim","avatar","avatar_logo","name","who","step","track","indicator","running","done","failed",
        "sheen","glow","count","counts","counter","feed_head","label","chip","chip_label","feed","feed_clip","feed_content","finding",
        "finding_body","finding_title","meta","author","location","separator","time","empty"],effects:["prune_filter"]},
    render:Fn("render_findings")
});

fn AgentFindings(props){render_component("patterns/agent_findings",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,v){common::value_or(m,k,v)}
fn af(n){theme_length("space.unit").times(n)}
fn token_for(severity){
    if severity=="critical"{"agent_findings.critical"}else if severity=="warning"{"agent_findings.warning"}else{"agent_findings.note"}
}
fn newer(a,b){if a.time==b.time{a.index-b.index}else if a.time>b.time{-1}else{1}}
fn project(props){
    if !ids::valid(props.run){throw "AgentFindings requires a stable run ID";}
    let names=#{};let refs=#{};let totals=#{};let found=#{critical:0,warning:0,note:0};let id_set=#{};let rows=[];let done=0;let total=0;let bytes=0;
    for i in 0..props.agents.len {
        let a=props.agents[i];
        if !ids::valid(a.id)||a.id in names{throw "AgentFindings needs unique agent IDs";}
        if a.name==""{throw "An agent must have a name";}
        if opt(a,"avatar")!=()&&opt(a,"avatar_image")!=(){throw "Choose an avatar node or image handle, not both";}
        names[a.id]=a.name;refs[a.id]="agent-filter:"+i.to_string();
        totals[a.id]=#{critical:0,warning:0,note:0};done+=min(a.done,a.total);total+=a.total;
        for field in ["name","step","model"]{bytes+=utf8_byte_count(value(a,field,""));}
    }
    for i in 0..props.findings.len{
        let f=props.findings[i];
        if !ids::valid(f.id)||f.id in id_set||(f.agent!=""&&!ids::valid(f.agent)){throw "AgentFindings needs valid unique finding IDs and valid owners";}
        if f.title==""{throw "A finding must have a title";}
        id_set[f.id]=true;
        bytes+=utf8_byte_count(f.title)+utf8_byte_count(value(f,"location",""));
        found[f.severity]+=1;
        if f.agent in totals{let count=totals[f.agent];count[f.severity]+=1;totals[f.agent]=count;}
        rows.push(#{item:f,time:value(f,"at_ms",-1),index:i});
    }
    if bytes>196608{throw "AgentFindings text exceeds 192 KiB";}
    rows.sort(Fn("newer"));
    #{names:names,refs:refs,totals:totals,found:found,rows:rows,done:done,total:total}
}
fn filter_id(ctx,run){
    let s=ctx.get_state("selection");if s!=()&&s.run==run{s.id}else{""}
}
fn clear_filter(ctx,p){
    let current=filter_id(ctx,p.run);
    if current!=""&&!(current in p.ids){ctx.set_state("selection",());}
}
fn stop_filter(ctx,p){}
fn choose(run,id,ctx,e){
    let previous=filter_id(ctx,run);
    let next=if previous==id{""}else{id};
    ctx.set_state("selection",#{run:run,id:next});
    handled()
}
fn clear(run,focus_key,ctx,e){
    ctx.set_state("selection",#{run:run,id:""});
    ctx.focus(focus_key);
    handled()
}
fn open_finding(run,revision,id,agent,ctx,e){
    ctx.emit("open",#{run:run,revision:revision,finding:id,agent:agent});handled()
}
fn tick(ctx,run){ctx.set_state("clock",#{run:run,at:ctx.now()});}
fn clock(ctx,props,needs_time){
    let provided=opt(props,"now_ms");
    if provided!=(){return provided;}
    if !needs_time{return 0;}
    interval("finding-clock",5000,false,Fn("tick"),props.run);
    let remembered=ctx.get_state("clock");
    if remembered!=()&&remembered.run==props.run{remembered.at}else{ctx.now()}
}
fn line(ctx,words,part,base){
    text(words).with_style(ctx.component_style(part,base.min_width(px(0)).whitespace_nowrap().text_ellipsis()))
}
fn caption(ctx,words,part){
    line(ctx,words,part,style().typography("caption").text_color(theme_color("agent_findings.quiet")))
}
fn severity_name(ctx,key){
    switch key{
        "critical"=>ctx.t_or("agent_findings.critical","Critical"),
        "warning"=>ctx.t_or("agent_findings.warning","Warning"),
        _=>ctx.t_or("agent_findings.note","Note"),
    }
}
fn count_text(ctx,sev,n){
    ctx.t_or("agent_findings.severity_count","{severity}: {count}",#{severity:severity_name(ctx,sev),count:ctx.format_number(n)})
}
fn count_word(ctx,sev,n){
    if sev=="critical"{ctx.t_or("agent_findings.stat_critical","critical")}
    else if n==1{ctx.t_or("agent_findings.stat_warning","warning")}
    else{ctx.t_or("agent_findings.stat_warnings","warnings")}
}
fn checks_text(ctx,done,total){
    ctx.t_or("agent_findings.checks","{done} of {total} checks",#{done:ctx.format_number(done),total:ctx.format_number(total)})
}
fn spoken(parts){
    let words="";
    for part in parts{
        if words!=""{words+=" · ";}
        words+=part;
    }
    words
}
fn compact_age(ctx,at,now){
    if at<0{return "";}
    let s=max(0,now-at)/1000;
    if s<5{return ctx.t_or("agent_findings.now","Now");}
    let unit=if s<60{[1,"s"]}else if s<3600{[60,"m"]}else if s<86400{[3600,"h"]}else{[86400,"d"]};
    let n=s/unit[0];
    try{return ctx.format_duration(n*unit[0]*1000);}
    catch{return ctx.format_number(n)+unit[1];}
}
fn spark(ctx){
    let ink=current_color();
    canvas(canvas_scene([
        canvas_fill_path("spark",[path_move(7,1.2),path_cubic(11.8,6,7.4,4.1,8.9,5.6),
            path_cubic(7,10.8,8.9,6.4,7.4,7.9),path_cubic(2.2,6,6.6,7.9,5.1,6.4),
            path_cubic(7,1.2,5.1,5.6,6.6,4.1),path_close()],ink),
        canvas_circle("glint",11.6,11.4,1.1,ink.alpha(0.55))
    ]).view_box(0,0,14,14))
        .with_key("spark").accessibility_hidden(true).with_style(ctx.component_style("spark",style()
            .width(af(4)).height(af(4)).flex_shrink(false).text_color(theme_color("accent"))))
}
fn mark(ctx,sev,zero,mini){
    let strengths=if sev=="critical"{[1.0,1.0,1.0,0.6]}else if sev=="warning"{[1.0,0.6,0.6,0.3]}else{[0.6,0.3,0.3,0.3]};
    let commands=[];let tone=theme_color(if zero{"agent_findings.faint"}else{token_for(sev)});
    for i in 0..4{
        commands.push(canvas_rect("point-"+i.to_string(),(i%2).to_float()*5.5,(i/2).to_float()*5.5,4.5,4.5,alpha(tone,strengths[i])));
    }
    let size=if mini{af(2.5)}else{af(3)};
    canvas(canvas_scene(commands).view_box(0,0,10,10)).with_key("mark").accessibility_hidden(true)
        .with_style(ctx.component_style("severity",style().width(size).height(size).flex_shrink(false)))
}
fn rolling(ctx,words,part,parts){
    let label=line(ctx,words,parts,style().typography("caption").font_weight(500).text_color(theme_color("agent_findings.quiet")));
    row([label.with_key("ink")]).with_key("number").with_style(ctx.component_style(part,style().min_width(px(0))))
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:420,intent:"decorative"}))
        .motion(motion_transition("translate_y",-5.0,0.0,#{duration_ms:420,intent:"decorative"}))
        .motion_on_change(words)
}
fn color_piece(ctx,severity,n,full){
    let zero=n==0;let paint=theme_color(if zero{"agent_findings.faint"}else{token_for(severity)});
    let nodes=[];
    if full{
        nodes.push(row([]).with_key("dot").accessibility_hidden(true).with_style(style().width(af(1.5)).height(af(1.5)).radius(af(0.75)).background(paint)));
    }else{nodes.push(mark(ctx,severity,zero,true));}
    let style_parts=if zero{["counter","faint"]}else{["counter",severity]};
    nodes.push(rolling(ctx,ctx.format_number(n),"count",style_parts));
    if full{nodes.push(caption(ctx,count_word(ctx,severity,n),style_parts).with_key("word"));}
    row(nodes).with_key(severity).accessibility_hidden(true)
        .with_style(ctx.component_style("total",style().items_center().gap(af(1)).flex_shrink(false)));
}
fn header(ctx,props,data,cfg,title){
    let checks=checks_text(ctx,data.done,data.total);
    let nodes=[spark(ctx),line(ctx,title,"title",style().typography("body").font_weight(600).text_color(theme_color("text_primary")).flex_grow()).with_key("title")
        .accessibility_role("heading").accessibility_level(2).accessibility_description(checks)];
    if !cfg.narrow&&data.total>0{nodes.push(caption(ctx,checks,"completion").with_key("checks"));}
    nodes.push(row([color_piece(ctx,"critical",data.found.critical,true),color_piece(ctx,"warning",data.found.warning,true)])
        .with_key("totals").accessibility_role("status")
        .accessibility_label(count_text(ctx,"critical",data.found.critical)+" · "+count_text(ctx,"warning",data.found.warning))
        .with_style(ctx.component_style("totals",style().items_center().gap(af(3)).flex_shrink(false))));
    row(nodes).with_key("bar").audit_allow(["mixed-type-in-row","text-edge-misaligned"])
        .with_style(ctx.component_style("bar",style().padding_x(af(3)).padding_y(af(2.5)).gap(af(2)).items_center().min_width(px(0))))
}
fn initials(name){
    let words=[];for word in name.split(" "){if word!=""{words.push(word);}}
    if words.len==0{return "";}
    let result=avatar::first_grapheme(words[0]);
    if words.len>1{result+=avatar::first_grapheme(words[words.len-1]);}
    result.to_upper()
}
fn face(ctx,a){
    let label=initials(a.name);let custom=opt(a,"avatar");let handle=opt(a,"avatar_image");
    let mark=if custom!=(){custom}else if handle!=(){image(handle).image_fallback_text(label)
        .with_style(style().width(relative(1.0)).height(relative(1.0)))}
        else{text(label)};
    row([mark]).with_key("avatar").accessibility_hidden(true).with_style(ctx.component_style(
        if custom!=()||handle!=(){["avatar","avatar_logo"]}else{["avatar"]},
        style().width(af(6.5)).height(af(6.5)).typography("caption").font_weight(500).radius(af(3.25))
            .background(theme_color("surface_hover")).items_center().justify_center().flex_shrink(false).clip()))
}
fn track(ctx,a,status,cfg){
    let done=min(a.done,a.total);let ratio=if a.total==0{0.0}else{done.to_float()/a.total.to_float()};
    let running=status=="running";let children=[];
    if running&&ratio>0.0{
        let start=if cfg.rtl{cfg.track+40.0}else{-40.0};let finish=if cfg.rtl{-40.0}else{cfg.track+40.0};
        children.push(row([]).with_key("sheen").with_style(ctx.component_style("sheen",style().absolute()
            .left(px(0)).top(px(0)).height(relative(1.0)).width(af(10))))
            .motion(motion_transition("translate_x",start,finish,#{duration_ms:1500,easing:"linear",iterations:"infinite",intent:"decorative"})));
        children.push(row([]).with_key("glow").with_style(ctx.component_style("glow",style().absolute()
            .inset_end(px(0)).top(px(0)).height(relative(1.0)).width(af(4.5))))
            .motion(motion_keyframes("opacity",[
                #{offset:0.0,value:0.45},#{offset:0.5,value:1.0},#{offset:1.0,value:0.45}],
                #{duration_ms:2200,iterations:"infinite",intent:"decorative"})));
    }
    let fill=ctx.component_style(["indicator",status],style().relative().height(relative(1.0))
        .background(theme_color("accent")).radius(af(1)).clip());
    let sample=progress::determinate_indicator("fill",ratio,fill,children,700,cubic_bezier(0.22,1.0,0.36,1.0))
        .paint_transition(300,"ease_out");
    row([sample]).with_key("track").accessibility_hidden(true)
        .with_style(ctx.component_style("track",style().width(px(cfg.track)).height(af(2)).radius(af(1)).clip().flex_shrink(false).background(theme_color("surface_hover"))))
}
fn agent_row(ctx,a,counts,focus_key,cfg){
    let status=value(a,"status",if a.done>=a.total{"done"}else{"running"});
    let step=if status=="done"{ctx.t_or("agent_findings.checked","All checks done")}
        else if status=="failed"{ctx.t_or("agent_findings.stopped","Stopped")}else{value(a,"step","")};
    let caption_step=caption(ctx,step,"step").with_key("step")
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:360,intent:"decorative"}))
        .motion(motion_transition("translate_y",3.0,0.0,#{duration_ms:360,intent:"decorative"}))
        .motion_on_change(step);
    let fields=[face(ctx,a),column([line(ctx,a.name,"name",style().typography("body").font_weight(500).text_color(theme_color("text_primary")))
        .with_key("name"),caption_step]).with_key("who").with_style(ctx.component_style("who",style().min_width(px(0)).flex_grow())),track(ctx,a,status,cfg)];
    if !cfg.narrow{
        fields.push(caption(ctx,ctx.format_number(min(a.done,a.total))+"/"+ctx.format_number(a.total),"count").with_key("count")
            .with_style(style().width(af(cfg.count_width)).flex_shrink(false).text_end()));
    }
    fields.push(row([color_piece(ctx,"critical",counts.critical,false),color_piece(ctx,"warning",counts.warning,false)])
        .with_key("counts").with_style(ctx.component_style("counts",style().width(af(cfg.found_width)).flex_shrink(false).justify_end().gap(af(1.5)).items_center())));
    let selected=cfg.pick==a.id;let parts=if selected{["agent","selected"]}else if cfg.pick!=""{["agent","dim"]}else{["agent"]};
    let desc=[checks_text(ctx,min(a.done,a.total),a.total),count_text(ctx,"critical",counts.critical),count_text(ctx,"warning",counts.warning)];
    for chunk in [value(a,"model",""),step]{if chunk!=""{desc.push(chunk);}}
    let accessible=spoken(desc);
    row([row(fields).with_key("filter").with_ref(element_ref(focus_key)).accessibility_role("button")
        .accessibility_label(a.name).accessibility_description(accessible).accessibility_pressed(selected)
        .on_click(Fn("choose").curry(cfg.run,a.id)).audit_allow(["mixed-type-in-row","text-edge-misaligned"])
        .with_style(ctx.component_style(parts,style().width(relative(1.0)).min_width(px(0)).min_height(af(13)).items_center()
            .padding_x(af(2.5)).gap(af(if cfg.narrow{2.5}else{3})).radius(af(2))
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))))
        .paint_transition(180,"ease_out")]).with_key("agent:"+cfg.run_key+a.id)
        .accessibility_role("listitem").with_style(style().min_width(px(0)).width(relative(1.0)))
}
fn feed_head(ctx,data,cfg){
    let label=ctx.t_or("agent_findings.recent","Latest findings");
    let uppercase=ctx.theme_variant()!=()&&ctx.theme_token("agent_findings.heading_case")=="upper";
    let children=[caption(ctx,if uppercase{label.to_upper()}else{label},"label").with_key("heading")
        .accessibility_role("heading").accessibility_level(3).accessibility_label(label)];
    if cfg.pick!=""{
        let name=data.names[cfg.pick];
        children.push(row([caption(ctx,name,"chip_label").with_key("label"),text("×").with_key("close").accessibility_hidden(true)])
            .with_key("clear").accessibility_role("button")
            .accessibility_label(ctx.t_or("agent_findings.clear","Show all agents (now only {name})",#{name:name}))
            .on_click(Fn("clear").curry(cfg.run,data.refs[cfg.pick])).audit_allow(["mixed-type-in-row"])
            .with_style(ctx.component_style("chip",style().min_width(px(0)).max_width(relative(0.6)).items_center().gap(af(1))
                .background(theme_color("surface_raised")).focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))))
            .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:260,intent:"decorative"})));
    }
    row(children).with_key("feed-heading").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("feed_head",style().min_width(px(0)).min_height(af(10)).padding_x(af(3)).gap(af(2)).items_center()))
}
fn finding_row(ctx,record,data,cfg,now){
    let f=record.item;let who=value(data.names,f.agent,"");let place=value(f,"location","");let time=compact_age(ctx,record.time,now);
    let fields=[mark(ctx,f.severity,false,false),line(ctx,f.title,"finding_title",style().typography("body").text_color(theme_color("text_primary"))
        .flex_grow()).with_key("title")];
    if !cfg.narrow&&(who!=""||place!=""){
        let parts=[];
        if who!=""{parts.push(caption(ctx,who,"author").with_key("author"));}
        if who!=""&&place!=""{parts.push(caption(ctx,"·","separator").with_key("separator").accessibility_hidden(true));}
        if place!=""{parts.push(line(ctx,place,"location",style().typography("code").text_color(theme_color("agent_findings.quiet"))).with_key("location"));}
        fields.push(row(parts).with_key("meta").audit_allow(["mixed-type-in-row"])
            .with_style(ctx.component_style("meta",style().min_width(px(0)).gap(af(1.5)).width(relative(0.40)).items_center())));
    }
    if time!=""{fields.push(caption(ctx,time,"time").with_key("time").with_style(style().flex_shrink(false).text_end()));}
    let details=[severity_name(ctx,f.severity)];for item in [who,place,time]{if item!=""{details.push(item);}}
    let row_body=row(fields).with_key("body").audit_allow(["mixed-type-in-row","text-edge-misaligned"])
        .accessibility_label(f.title).accessibility_description(spoken(details))
        .with_style(ctx.component_style("finding_body",style().min_width(px(0)).width(relative(1.0)).min_height(af(10)).gap(af(2.5))
            .padding_x(af(2.5)).items_center().radius(af(2)).hover(style().background(theme_color("surface_hover")))
            .focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))))
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:320,intent:"decorative"}))
        .motion(motion_transition("translate_y",-10.0,0.0,#{duration_ms:480,intent:"decorative"}))
        .motion(motion_transition("foreground_blur",2.0,0.0,#{duration_ms:360,intent:"decorative"}));
    if cfg.open{row_body=row_body.accessibility_role("button")
        .on_click(Fn("open_finding").curry(cfg.run,cfg.revision,f.id,f.agent));}
    else{row_body=row_body.accessibility_role("text");}
    row([row_body]).with_key("finding:"+cfg.run_key+f.id).accessibility_role("listitem")
        .with_style(ctx.component_style("finding",style().min_width(px(0)).width(relative(1.0)).flex_shrink(false)))
        .layout_motion(520,"ease_out")
        .exit_motion(motion_transition("opacity",1.0,0.0,#{duration_ms:320,intent:"decorative"}))
}
fn empty_row(ctx,words,key,units){
    row([caption(ctx,words,"empty").with_key("label")]).with_key(key)
        .with_style(style().min_height(af(units)).width(relative(1.0)).justify_center().items_center());
}
fn feed_panel(ctx,data,cfg,props){
    let feed=[];let has_clock=false;
    for r in data.rows {
        if cfg.pick!=""&&r.item.agent!=cfg.pick{continue;}
        feed.push(r);has_clock=has_clock||r.time>=0;
        if feed.len>=props.rows{break;}
    }
    let now=clock(ctx,props,has_clock);let children=[];
    for r in feed{children.push(finding_row(ctx,r,data,cfg,now));}
    if children.len==0{children.push(empty_row(ctx,ctx.t_or("agent_findings.none_found","Nothing found yet"),"empty",10));}
    let measure=element_ref("feed-content");
    let current=if ctx.motion_preference()=="normal"{ctx.element_size(measure)}else{()};
    let list=column(children).with_key("entries").with_ref(measure).accessibility_role("list")
        .accessibility_label(ctx.t_or("agent_findings.recent","Latest findings"))
        .with_style(ctx.component_style("feed_content",style().min_width(px(0)).width(relative(1.0)).flex_shrink(false)));
    let clip=column([list]).with_key("clip")
        .with_style(ctx.component_style("feed_clip",style().min_width(px(0)).width(relative(1.0)).clip()));
    if current!=()&&current.height>0.0{
        clip=clip.motion(motion_transition("clip_height",current.height,current.height,
            #{duration_ms:520,easing:cubic_bezier(0.22,1.0,0.36,1.0),intent:"decorative"}));
    }
    column([clip]).with_key("feed").with_style(ctx.component_style("feed",style().min_width(px(0)).padding(af(1.5))
        .background(theme_color("surface_raised")).radius(af(3))))
}
fn render_findings(ctx,props){
    let d=project(props);let selected=filter_id(ctx,props.run);let shell=element_ref("findings");
    let labels=ctx.element_width_class(shell,[280,480]);let narrow=labels==()||labels<2;
    let count_width=10.0;let found_width=13.0;
    for a in props.agents{
        let check=ctx.format_number(min(a.done,a.total))+"/"+ctx.format_number(a.total);
        count_width=max(count_width,grapheme_count(check).to_float()*1.65);
        let n=d.totals[a.id];
        found_width=max(found_width,8.0+(grapheme_count(ctx.format_number(n.critical))+grapheme_count(ctx.format_number(n.warning))).to_float()*1.55);
    }
    let cfg=#{run:props.run,run_key:utf8_byte_count(props.run).to_string()+":"+props.run+":",revision:props.revision,
        pick:if selected in d.names{selected}else{""},open:opt(props,"on_open")!=(),narrow:narrow,rtl:ctx.text_direction()=="rtl",
        track:if labels==0{36.0}else if narrow{64.0}else{128.0},count_width:count_width,found_width:found_width};
    effect("prune_filter",#{run:props.run,ids:d.names},Fn("clear_filter"),Fn("stop_filter"));
    let teams=[];
    for a in props.agents{teams.push(agent_row(ctx,a,d.totals[a.id],d.refs[a.id],cfg));}
    if teams.len==0{teams.push(empty_row(ctx,ctx.t_or("agent_findings.no_agents","No agents yet"),"no-agents",12));}
    let agents=column(teams).with_key("agents").accessibility_role("list")
        .accessibility_label(ctx.t_or("agent_findings.agents","Agents"))
        .with_style(ctx.component_style("agents",style().min_width(px(0)).padding(af(1.5))
            .background(theme_color("surface_raised")).radius(af(3))));
    let title=value(props,"title",ctx.t_or("agent_findings.title","Verification"));
    column([header(ctx,props,d,cfg,title),agents,feed_head(ctx,d,cfg),feed_panel(ctx,d,cfg,props)])
        .with_key(props.key).with_ref(shell).accessibility_role("group").accessibility_label(title)
        // Section headers, avatar rows and severity-marked discoveries
        // deliberately start on different content columns.
        .audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(af(160)).min_width(px(0)).gap(af(0.5))))
}
