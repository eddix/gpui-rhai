/* gpui-rhai
{"id":"patterns/agent_checkpoints","export":"AgentCheckpoints","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","space.spacing_unit","surface_hover","surface_raised","text.danger","text_muted","text_primary","typography.body","typography.caption","space.unit"],"environment":[]}
*/
// An immutable chronological history with a controlled head. Rewind is a proposal.
// Example: AgentCheckpoints(#{key:"history",run:"preview",revision:0,checkpoints:[]})
import "components/button" as button;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_checkpoints","export":"AgentCheckpoints","version":"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","patterns/approval_card"],capabilities:#{},
        tokens:["accent","border","focus_ring","space.spacing_unit","surface_hover","surface_raised","text.danger","text_muted","text_primary","typography.body","typography.caption","space.unit"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable history surface key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business identity of the run."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"History heading."},
        task:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short task caption."},
        head:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Existing checkpoint ID. Omit for the last checkpoint."},
        height:#{schema:#{type:"optional",value:#{type:"length"}},required:false,sensitive:false,doc:"Maximum height of the scrollable history; default 456px."},
        checkpoints:#{schema:#{type:"array",max_items:96,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable checkpoint ID."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Checkpoint title; does not identify the checkpoint."},
            parent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"ID of an earlier checkpoint; omit for an independent root."},
            status:#{schema:#{type:"optional",value:#{type:"string",allowed:["done","running","failed","stopped"]}},
                required:false,sensitive:false,doc:"Accepted status; defaults to done."},
            elapsed_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Elapsed time from the start of its attempt."},
            files:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Number of changed files."},
            cost_cents:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000}},required:false,sensitive:false,doc:"Cost in hundredths of the display currency."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative step icon."},
        }}},required:true,sensitive:true,doc:"Up to 96 checkpoints, oldest first. Parents must precede children."},
        currency:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Short ISO currency label for cents; defaults to USD."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Keep history navigable, but disable rewind while the Host resolves a proposal."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host feedback for a pending, accepted or declined proposal."},
        on_rewind:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a revision- and head-bound proposal to start a new attempt."},
    },state:#{fields:#{cursor:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{
        rewind:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run that owns the checkpoint."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Snapshot the user acted on."},
            checkpoint:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Checkpoint the new attempt should start from."},
            head:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Expected current checkpoint when the proposal was sent."},
        }},doc:"Validate run, revision, checkpoint and head in the Host before branching. No history is modified by the component."},
    },slots:#{},parts:["root","head","title","task","summary","body","stage","track","row","row_current","row_archived","content","title_text","meta","icon","attempt",
        "failed","current","marker","marker_current","marker_live","marker_archived","rail","rewind","empty","notice"],effects:["reveal_head"]},
    render:Fn("render_AgentCheckpoints")
});

fn AgentCheckpoints(props){render_component("patterns/agent_checkpoints",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,d){common::value_or(m,k,d)}
fn valid(id){
    if id==""||utf8_byte_count(id)>96{return false;}
    for c in id.chars(){if c<' '||(c>=''&&c<=''){return false;}}
    true
}
fn row_key(i){"row-"+i.to_string()}
fn marker_key(i){"checkpoint-"+i.to_string()}
fn project(props){
    if !valid(props.run){throw "Checkpoint history needs a stable run ID";}
    let by_id=#{};let rows=[];let chains=[];let extended=#{};let cents=0;let cost=false;
    for cp in props.checkpoints{
        let i=rows.len;
        if !valid(cp.id)||cp.id in by_id{throw "Checkpoint IDs must be distinct and bounded";}
        let parent=opt(cp,"parent");
        if parent!=()&&!(parent in by_id){throw "Checkpoint parent must precede its child";}
        let from=if parent==(){-1}else{by_id[parent]};
        let start=from<0||parent in extended;
        let chain=if start{chains.len}else{rows[from].chain};
        if start{chains.push(#{begin:if from<0{i}else{from},end:i});}
        else{chains[chain].end=i;}
        if from>=0{extended[parent]=true;}
        by_id[cp.id]=i;
        rows.push(#{item:cp,ordinal:i,from:from,chain:chain,attempt:if start{chain+1}else{0}});
        let count=opt(cp,"cost_cents");if count!=(){cents+=count;cost=true;}
    }
    let head=opt(props,"head");
    if head!=()&&!(head in by_id){throw "Checkpoint head must name an existing checkpoint";}
    let at=if head==(){rows.len-1}else{by_id[head]};
    let live=#{};let cur=at;
    while cur>=0{live[rows[cur].item.id]=true;cur=rows[cur].from;}
    // Color complete chain intervals, including the incoming fork segment.
    // A lane is reusable only when every interval on it is disjoint.
    let lanes=[];let positions=[];
    for span in chains{
        let lane=0;
        while lane<lanes.len{
            let fits=true;
            for other in lanes[lane]{if span.begin<=other.end&&other.begin<=span.end{fits=false;break;}}
            if fits{break;}
            lane+=1;
        }
        if lane==lanes.len{lanes.push([]);}
        lanes[lane].push(span);positions.push(lane);
    }
    #{rows:rows,by_id:by_id,head:at,live:live,lanes:positions,lane_count:lanes.len,attempts:chains.len,
        cents:if cost{cents}else{()}}
}
fn choose(run,id,key,ctx,event){
    ctx.set_state("cursor",#{run:run,id:id});ctx.focus(key);reveal_row(ctx,key);handled()
}
fn focus_in(run,id,ctx,inside){if inside{ctx.set_state("cursor",#{run:run,id:id});}}
fn reveal(ctx,deps){if deps[3]{reveal_row(ctx,row_key(deps[2]));}}
fn reveal_row(ctx,key){common::reveal_in(ctx,key,"viewport","stage");}
fn stop(ctx,deps){}
fn rewind(token,id,ctx,event){
    ctx.emit("rewind",#{run:token.run,revision:token.revision,head:token.head,checkpoint:id});handled()
}
fn note(ctx,s,part){common::heading_note(ctx,s,part)}
fn elapsed(ctx,ms){
    let sec=ms/1000;
    if sec<60{return ctx.t_or("agent_lanes.seconds","{n} s",#{n:ctx.format_number(sec)});}
    ctx.t_or("agent_inbox.minutes_seconds","{m} min {s} s",#{m:ctx.format_number(sec/60),s:ctx.format_number(sec%60)})
}
fn icon(ctx,v,part){
    row([v]).with_key("symbol").accessibility_hidden(true).with_style(ctx.component_style(part,
        style().width(theme_length("space.unit").times(5)).height(theme_length("space.unit").times(5)).radius(px(5)).items_center().justify_center().flex_shrink(false).typography("caption")))
}
fn marker(ctx,cfg,r,ref_mark,is_head,live){
    let diameter=if is_head{9}else{7};
    let mark=box([]).with_key("dot").with_style(ctx.component_style(["marker",if is_head{"marker_current"}else if live{"marker_live"}else{"marker_archived"}],
        style().width(px(diameter)).height(px(diameter)).radius(px(6))
        .background(theme_color(if is_head||live{"accent"}else{"surface_raised"})).border(px(1))
        .border_color(theme_color(if live{"surface_raised"}else{"text_muted"}))));
    if is_head{
        mark=mark.motion(motion_transition("opacity",0.45,1.0,#{duration_ms:ctx.motion_duration("fast"),intent:"decorative"}))
            .motion_on_change(r.item.id);
    }
    row([row([mark]).with_key("measure").with_ref(ref_mark).with_style(style().width(px(10)).height(px(14)).items_center().justify_center())])
        .with_key("anchor").accessibility_hidden(true).with_style(ctx.component_style("track",style().items_center().flex_shrink(false))
        .padding_start(theme_length("space.unit").times((8.0+cfg.lanes[r.chain].to_float()*cfg.step)/4.0))
        .width(theme_length("space.unit").times(cfg.gutter/4.0)))
}
fn nav(node,combo,run,row){
    node.on_key_value(combo,Fn("choose").curry(run,row.item.id,row_key(row.ordinal)),())
}
fn history_row(ctx,data,cfg,r,ref_row,ref_mark){
    let p=r.item;let i=r.ordinal;let is_head=i==data.head;let live=p.id in data.live;let stage=value(p,"status","done");
    let running=stage=="running";let failed=stage=="failed";let can_rewind=cfg.actions&&!is_head;
    let state=if running{ctx.t_or("agent_queue.running","Running")}else if failed{ctx.t_or("agent_queue.failed","Failed")}
        else if stage=="stopped"{ctx.t_or("agent_checkpoints.stopped","Stopped")}else{ctx.t_or("agent_queue.done","Done")};
    let desc=state;let meta=[];let content=[];
    let img=opt(p,"icon");if img!=(){content.push(icon(ctx,img,"icon"));}
    content.push(text(p.title).with_key("title").with_style(ctx.component_style("title_text",
        style().min_width(px(0)).flex_grow().text_ellipsis().whitespace_nowrap().typography("body"))));
    if r.attempt>1{
        let label=ctx.t_or("agent_checkpoints.attempt","Attempt {n}",#{n:ctx.format_number(r.attempt)});
        desc+=" · "+label;content.push(note(ctx,label,"attempt").with_key("attempt"));
    }
    if r.from>=0{
        desc+=" · "+ctx.t_or("agent_checkpoints.parent","From {name}",#{name:data.rows[r.from].item.title});
    }
    if is_head{content.push(note(ctx,ctx.t_or("agent_checkpoints.current","Current"),"current").with_key("current"));}
    else if !live{desc+=" · "+ctx.t_or("agent_checkpoints.archived","Earlier attempt");}
    if stage!="done"{content.push(note(ctx,state,if failed{"failed"}else{"meta"}).with_key("state"));}
    let files=opt(p,"files");if files!=()&&files>0{
        let label=ctx.t_or("agent_checkpoints.files","Files: {n}",#{n:ctx.format_number(files)});
        desc+=" · "+label;meta.push(note(ctx,label,"meta").with_key("files"));
    }
    let time=opt(p,"elapsed_ms");if time!=(){let label=elapsed(ctx,time);desc+=" · "+label;meta.push(note(ctx,label,"meta").with_key("time"));}
    if !cfg.tight{for metric in meta{content.push(metric);}}
    if can_rewind{
        let label=ctx.t_or("agent_checkpoints.rewind","Rewind");
        let accessible=ctx.t_or("agent_checkpoints.rewind_to","Rewind to {name}",#{name:p.title});
        content.push(button::Button(#{key:"rewind:"+p.id,text:label,variant:"secondary",size:"xs",disabled:cfg.busy,
            on_click:Fn("rewind").curry(cfg.token,p.id)}).with_key("rewind").tab_stop(false).accessibility_label(accessible)
            .with_style(ctx.component_style("rewind",style().flex_shrink(false))));
    }
    let parts=["row"];if is_head{parts.push("row_current");}else if !live{parts.push("row_archived");}
    let inner=row(content).with_key("content").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("content",style().min_width(px(0))
        .flex_grow().items_center().flex_wrap().gap(theme_length("space.spacing_unit")).padding_end(theme_length("space.spacing_unit"))));
    let n=row([marker(ctx,cfg,r,ref_mark,is_head,live),inner]).with_key("checkpoint:"+p.id).with_ref(ref_row).accessibility_role("listitem")
        .accessibility_label(p.title).accessibility_description(desc).tab_stop(p.id==cfg.focus)
        .accessibility_position_in_set(i+1).accessibility_size_of_set(data.rows.len)
        .with_style(ctx.component_style(parts,style().min_width(px(0)).min_height(theme_length("space.unit").times(8)).items_center().padding_y(theme_length("space.spacing_unit").times(0.5))
            .focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))))
        .on_click(Fn("choose").curry(cfg.token.run,p.id,row_key(i)))
        .on_focus_within_change(Fn("focus_in").curry(cfg.token.run,p.id));
    n=nav(n,"plain+up",cfg.token.run,data.rows[max(0,i-1)]);
    n=nav(n,"plain+down",cfg.token.run,data.rows[min(data.rows.len-1,i+1)]);
    n=nav(n,"plain+home",cfg.token.run,data.rows[0]);
    n=nav(n,"plain+end",cfg.token.run,data.rows[data.rows.len-1]);
    if is_head{n=n.accessibility_current("step");}
    if can_rewind&&!cfg.busy{
        n=n.on_key_value("plain+enter",Fn("rewind").curry(cfg.token,p.id),())
            .on_key_value("plain+space",Fn("rewind").curry(cfg.token,p.id),());
    }
    n
}
fn point_at(ctx,ref_item,stage){
    let b=ctx.element_bounds(ref_item);if b==(){return ();}
    let q=b.visual;#{x:q.x+q.width*0.5-stage.x,y:q.y+q.height*0.5-stage.y}
}
fn rail(ctx,data,stage_ref,marks){
    let frame=ctx.element_bounds(stage_ref);
    if frame==()||frame.visual.width<1.0||frame.visual.height<1.0{return ();}
    let b=frame.visual;let commands=[];let points=[];
    for mark in marks{points.push(point_at(ctx,mark,b));}
    for r in data.rows{
        if r.from<0{continue;}
        let a=points[r.from];let z=points[r.ordinal];if a==()||z==()||z.y<=a.y{continue;}
        let ink=theme_color(if r.item.id in data.live{"accent"}else{"border"});
        let key="link-"+r.ordinal.to_string();
        if data.lanes[r.chain]==data.lanes[data.rows[r.from].chain]{
            commands.push(canvas_line(key,a.x,a.y,z.x,z.y,1.1,ink));
        }else{
            let bend=min(z.y,a.y+27.0);let middle=(bend+a.y)*0.5;
            commands.push(canvas_stroke_path(key,[path_move(a.x,a.y),path_cubic(z.x,bend,a.x,middle,z.x,middle),path_line(z.x,z.y)],1.1,ink)
                .stroke_cap("round"));
        }
    }
    if commands.len==0{return ();}
    canvas(canvas_scene(commands).view_box(0,0,b.width,b.height)).with_key("rail").accessibility_hidden(true)
        .with_style(ctx.component_style("rail",style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0))))
}
fn render_AgentCheckpoints(ctx,props){
    let data=project(props);let shell=element_ref("history");let stage_ref=element_ref("stage");let viewport=element_ref("viewport");
    let head=if data.head<0{""}else{data.rows[data.head].item.id};
    let cursor=ctx.get_state("cursor");let focus=if cursor!=()&&cursor.run==props.run&&cursor.id in data.by_id{cursor.id}else{head};
    let step=min(12.0,54.0/max(1,data.lane_count-1).to_float());
    let cfg=#{token:#{run:props.run,revision:props.revision,head:head},focus:focus,busy:props.busy,actions:opt(props,"on_rewind")!=(),
        tight:ctx.element_width_class(shell,[505])==0,step:step,gutter:26.0+max(0,data.lane_count-1).to_float()*step,lanes:data.lanes};
    let title=value(props,"title",ctx.t_or("agent_checkpoints.title","Checkpoints"));
    let count=ctx.t_or("agent_checkpoints.count","Checkpoints: {n}",#{n:ctx.format_number(data.rows.len)});
    if data.attempts>1{count+=" · "+ctx.t_or("agent_checkpoints.attempts","Attempts: {n}",#{n:ctx.format_number(data.attempts)});}
    if data.cents!=(){
        count+=" · "+ctx.format_number(data.cents.to_float()/100.0,#{min_fraction_digits:2,max_fraction_digits:2})+" "+value(props,"currency","USD");
    }
    let mark=svg("<svg viewBox='0 0 18 18' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round'><path d='M4.5 4.5v9m0-5c0 2 1.5 3 4 3h4'/><circle cx='4.5' cy='4.5' r='1.7' fill='currentColor'/><circle cx='13' cy='11.5' r='1.7' fill='currentColor'/></svg>")
        .with_style(style().width(theme_length("space.unit").times(3.75)).height(theme_length("space.unit").times(3.75)).text_color(theme_color("accent")));
    let header=[icon(ctx,mark,"icon"),text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
        .with_style(ctx.component_style("title",style().min_width(px(0)).typography("body").font_weight(600)))];
    let task=opt(props,"task");if task!=(){header.push(note(ctx,task,"task").with_key("task").with_style(style().flex_grow()));}
    header.push(note(ctx,count,"summary").with_key("summary").accessibility_role("status"));
    let output=[row(header).with_key("header").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("head",
        style().min_width(px(0)).gap(theme_length("space.spacing_unit")).items_center().flex_wrap()))];
    let ready=false;
    if data.rows.len==0{
        output.push(note(ctx,ctx.t_or("agent_checkpoints.empty","No checkpoints yet"),"empty").with_key("empty"));
    }else{
        let refs=[];let marks=[];let rows=[];
        for r in data.rows{refs.push(element_ref(row_key(r.ordinal)));marks.push(element_ref(marker_key(r.ordinal)));}
        ready=ctx.element_bounds(refs[data.head])!=()&&ctx.element_bounds(viewport)!=()&&ctx.element_bounds(stage_ref)!=();
        for r in data.rows{rows.push(history_row(ctx,data,cfg,r,refs[r.ordinal],marks[r.ordinal]));}
        let list=column(rows).with_key("entries").accessibility_role("list").accessibility_label(title)
            .with_style(style().min_width(px(0)).padding_y(theme_length("space.spacing_unit")).flex_shrink(false));
        let wires=rail(ctx,data,stage_ref,marks);
        let children=if wires==(){[list]}else{[wires,list]};
        let stage=stack(children).with_key("stage").with_ref(stage_ref).accessibility_role("group")
            .accessibility_label(ctx.t_or("agent_checkpoints.history","Checkpoint history"))
            .with_style(ctx.component_style("stage",style().min_width(px(0)).relative().flex_shrink(false)));
        output.push(column([stage]).with_key("history-scroll").with_ref(viewport).scrollbars("hidden","auto")
            .with_style(ctx.component_style("body",style().min_width(px(0)).max_height(value(props,"height",px(456))).overflow_y_scroll()
                .background(theme_color("surface_raised")).radius(theme_length("space.unit").times(2.5)))));
    }
    let notice=opt(props,"notice");
    if notice!=(){output.push(note(ctx,notice,"notice").with_key("notice").accessibility_role("status"));}
    // A clock tick or style change is not a head change and must not scroll.
    effect("reveal_head",[props.run,head,data.head,ready],Fn("reveal"),Fn("stop"));
    // The header mark and the history's variable-width track own different gutters.
    column(output).with_key(props.key).with_ref(shell).accessibility_role("group").accessibility_label(title)
        .audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).padding(theme_length("space.spacing_unit")).gap(theme_length("space.spacing_unit"))
            .background(theme_color("surface_hover")).radius(theme_length("space.unit").times(3.25))))
}
