/* gpui-rhai
{"id":"patterns/agent_steering","export":"AgentSteering","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/spinner","components/textarea","patterns/agent_monitor","patterns/agent_tree","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","space.spacing_unit","space.unit","surface_hover","surface_raised","text.success","text_muted","text_primary","typography.body","typography.caption","typography.metadata"],"environment":[]}
*/
// Send versioned steering intent without editing the Host's accepted plan or queue.
// Example: AgentSteering(#{key:"steer",run:"preview",revision:0,now_ms:0,title:"Preview",steps:[],queue:[]})
import "components/button" as button;
import "components/spinner" as spinner;
import "components/textarea" as textarea;
import "patterns/agent_monitor" as monitor;
import "patterns/agent_tree" as team;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_steering","export":"AgentSteering",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/spinner","components/textarea","patterns/agent_monitor","patterns/agent_tree","patterns/approval_card"],
        capabilities:#{},tokens:["accent","border","focus_ring","space.spacing_unit","space.unit","surface_hover","surface_raised","text.success",
            "text_muted","text_primary","typography.body","typography.caption","typography.metadata"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable instance key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business run identity."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision, independent from typing or the clock."},
        now_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Shared Host clock."},
        title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Work being steered."},
        agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Agent or model name."},
        status:#{schema:#{type:"string",allowed:["running","paused","done"]},"default":#{type:"string",value:"running"},
            required:false,sensitive:false,doc:"Accepted run state."},
        started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Run clock start."},
        paused_at_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Authoritative freeze point while paused."},
        paused_ms:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Completed pause intervals."},
        ended_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Fixed endpoint for a started completed run."},
        steps:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable step identity."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Step caption."},
            status:#{schema:#{type:"optional",value:#{type:"string",allowed:["done","active","pending"]}},required:false,sensitive:false,doc:"Default pending; at most one active step."},
            started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Clock start for the active step."},
            elapsed_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Accepted fixed time for a completed step."},
            paused_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Pause time already completed during this step."},
            notes:#{schema:#{type:"optional",value:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Same note ID when it moves from the queue into the step."},
                text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Accepted note text."},
                picked:#{schema:#{type:"optional",value:#{type:"string",allowed:["next","now"]}},required:false,sensitive:false,doc:"Whether picked up at a step boundary or applied immediately."},
            }}}},required:false,sensitive:true,doc:"Notes the Host accepted into this step."},
        }}},required:true,sensitive:true,doc:"Accepted plan order."},
        queue:#{schema:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable queued note ID, unique across the entire steering surface."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Note waiting to be applied."},
        }}},required:true,sensitive:true,doc:"Accepted waiting notes, oldest first."},
        draft:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Use a string for a controlled draft, including an empty string. Omit for local edits."},
        on_draft_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"A controlled editor receives run and the full changed text."},
        placeholder:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Hint for the steering editor."},
        compose:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Hide the editor when the application already provides one."},
        highlight:#{schema:#{type:"optional",value:#{type:"array",max_items:32,items:#{type:"string"}}},required:false,sensitive:false,doc:"Spotlight accepted step IDs."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disable new actions while a proposal is processing. The draft can still be edited."},
        actions:#{schema:#{type:"optional",value:#{type:"array",max_items:5,items:#{type:"string",allowed:["queue","interrupt","remove","pause","resume"]}}},
            required:false,sensitive:false,doc:"Restrict visible proposals, or omit for every valid action."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a request to queue, apply, remove, pause or resume."},
        receipt:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            proposal:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Exact proposal identity reported by on_action."},
            accepted:#{schema:#{type:"bool"},required:true,sensitive:false,doc:"Whether the Host accepted it."},
        }}},required:false,sensitive:false,doc:"Only a matching accepted receipt clears an unchanged local draft. Controlled drafts are never cleared here."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host feedback about a proposal."},
    },state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},events:#{
        draft_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run identity."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Full native draft."},
        }},doc:"Controlled draft edit, not a request to steer."},
        action:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run being acted on."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Observed revision."},
            proposal:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity to echo in receipt."},
            action:#{schema:#{type:"string",allowed:["queue","interrupt","remove","pause","resume"]},required:true,sensitive:false,doc:"Requested intent."},
            active:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Observed active step ID, or empty."},
            note:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Existing queued note ID, or empty for new text."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Normalized draft or queued note text; empty for non-text actions."},
        }},doc:"Validate version, active step and queued note before updating accepted data. The SDK never executes a note."},
    },slots:#{},parts:["root","card","header","heading","title","agent","state","state_running","state_paused","state_done","clock","steer","plan","rail",
        "step","step_active","step_done","step_quiet","step_lead","step_title","step_clock","marker","marker_done","marker_active","notes","note","note_applied","note_arrow","note_next","note_now","note_text",
        "note_tag","queue","queue_title","queued","queue_actions","compose","field","send","hints","hint","keycap","empty","notice"],effects:["settle_receipt"]},
    render:Fn("render_AgentSteering")
});

fn AgentSteering(props){render_component("patterns/agent_steering",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,d){common::value_or(m,k,d)}
fn steer_unit(n){theme_length("space.unit").times(n)}
fn steer_gap(n){theme_length("space.spacing_unit").times(n)}
fn note(ctx,s,part){common::heading_note(ctx,s,part)}
fn valid_text(text){text!=""&&utf8_byte_count(text)<=4096}
fn normalize(text){let s=text;s.trim();s}
fn project(props){
    let steps=#{};let notes=#{};let active="";let complete=0;let bytes=0;
    for s in props.steps{
        if !team::valid(s.id)||s.id in steps{throw "Steering needs distinct stable step IDs";}
        steps[s.id]=true;
        let phase=value(s,"status","pending");
        if phase=="active"{if active!=""{throw "Steering has only one active step";}active=s.id;}
        if phase=="done"{complete+=1;}
        for n in value(s,"notes",[]){
            if !team::valid(n.id)||n.id in notes||!valid_text(n.text){throw "Accepted steering notes need distinct IDs and bounded text";}
            notes[n.id]=true;bytes+=utf8_byte_count(n.text);
        }
    }
    for n in props.queue{
        if !team::valid(n.id)||n.id in notes||!valid_text(n.text){throw "A steering note cannot exist both in the queue and in the plan";}
        notes[n.id]=true;bytes+=utf8_byte_count(n.text);
    }
    if notes.len()>96||bytes>262144{throw "Steering note history exceeds its budget";}
    #{active:active,complete:complete}
}
fn clock(props){
    let phase=if props.status=="done"{"finished"}else{props.status};
    monitor::accepted_clock(#{state:phase,now_ms:props.now_ms,started_ms:opt(props,"started_ms"),
        paused_at_ms:opt(props,"paused_at_ms"),paused_ms:props.paused_ms,ended_ms:opt(props,"ended_ms")})
}
fn local(ctx,run){
    let s=ctx.get_state("ui");if s!=()&&s.run==run{return s;}
    #{run:run,draft:"",edit:0,sequence:0,pending:()}
}
fn draft_edit(cfg,ctx,text){
    let s=local(ctx,cfg.run);s.edit+=1;
    if !cfg.controlled{s.draft=text;}
    ctx.set_state("ui",s);
    ctx.emit("draft_change",#{run:cfg.run,text:text});
}
fn listed(cfg,op){cfg.connected&&(cfg.actions==()||op in cfg.actions)}
fn live(cfg,op){
    if cfg.busy||!listed(cfg,op){return false;}
    if op=="interrupt"{return cfg.status=="running"&&cfg.active!="";}
    if op=="pause"{return cfg.status=="running";}
    if op=="resume"{return cfg.status=="paused";}
    cfg.status!="done"
}
fn proposal(cfg,kind,id,raw,ctx){
    if !live(cfg,kind){return propagate();}
    let text=normalize(raw);
    if kind in ["queue","interrupt"]&&!valid_text(text){return handled();}
    let s=local(ctx,cfg.run);s.sequence+=1;
    // The window/component scope separates identical run IDs in two views.
    let ticket=cfg.scope+":"+cfg.revision.to_string()+":"+s.sequence.to_string();
    if !cfg.controlled&&id==""&&kind in ["queue","interrupt"]{
        s.pending=#{proposal:ticket,edit:s.edit,draft:raw};
    }
    ctx.set_state("ui",s);
    ctx.emit("action",#{run:cfg.run,revision:cfg.revision,proposal:ticket,action:kind,active:cfg.active,note:id,text:text});
    handled()
}
fn submit(cfg,kind,ctx,text){proposal(cfg,kind,"",text,ctx)}
fn command(cfg,kind,id,text,ctx,e){proposal(cfg,kind,id,text,ctx)}
fn settle(ctx,p){
    if p.receipt==()||p.controlled{return;}
    let s=local(ctx,p.run);let before=s.pending;
    if before==()||before.proposal!=p.receipt.proposal{return;}
    s.pending=();
    if p.receipt.accepted&&before.edit==s.edit&&before.draft==s.draft{s.draft="";s.edit+=1;}
    ctx.set_state("ui",s);
}
fn stop(ctx,p){}
fn ref_step(i){"steer-step-"+i.to_string()}
fn icon(ctx,s,n){
    common::glyph(ctx,s,n).accessibility_hidden(true)
}
fn run_state(ctx,phase){
    if phase=="paused"{ctx.t_or("agent_monitor.paused","Paused")}
    else if phase=="done"{ctx.t_or("agent_queue.done","Done")}
    else{ctx.t_or("agent_queue.running","Running")}
}
fn header(ctx,props,cfg,data,now){
    let name=text(props.title).with_key("title").accessibility_role("heading").accessibility_level(2)
        .with_style(ctx.component_style("title",style().min_width(px(0)).typography("body").font_weight(600)));
    let detail=[];let a=opt(props,"agent");if a!=(){detail.push(note(ctx,a,"agent").with_key("agent"));}
    let n=ctx.t_or("agent_steering.done_count","{done}/{all} done",#{done:ctx.format_number(data.complete),all:ctx.format_number(props.steps.len)});
    detail.push(note(ctx,n,"agent").with_key("progress"));
    let title=column([name,row(detail).with_key("subheading").with_style(style().min_width(px(0)).gap(steer_gap(1.25)).flex_wrap())])
        .with_key("heading").with_style(ctx.component_style("heading",style().min_width(px(0)).flex_grow().gap(steer_gap(0.5))));
    let label=run_state(ctx,props.status);
    let capsule=[text(label).with_key("state").accessibility_role("status").with_style(ctx.component_style(
        ["state","state_"+props.status],style().typography("caption").padding_x(steer_gap(1.25)).padding_y(steer_gap(0.25))
        .radius(px(5)).background(theme_color("surface_hover"))))];
    if opt(props,"started_ms")!=(){capsule.push(note(ctx,monitor::elapsed_phrase(ctx,now.elapsed),"clock").with_key("elapsed")
        .accessibility_role("status").accessibility_label(ctx.t_or("agent_monitor.elapsed","Elapsed: {time}",#{time:monitor::elapsed_phrase(ctx,now.elapsed)})));}
    let controls=[title,row(capsule).with_key("status").with_style(style().min_width(px(0)).gap(steer_gap(1)).items_center().flex_wrap())];
    let paused=props.status=="paused";let action=if paused{"resume"}else{"pause"};
    if props.status!="done"&&listed(cfg,action){
        controls.push(button::Button(#{key:"steer-toggle",text:if paused{ctx.t_or("agent_monitor.resume","Resume")}else{ctx.t_or("agent_monitor.pause","Pause")},
            size:"xs",variant:"secondary",disabled:cfg.busy,on_click:Fn("command").curry(cfg,action,"","")}).with_key("toggle")
            .with_style(ctx.component_style("steer",style().flex_shrink(false))));
    }
    row(controls).with_key("head").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("header",
        style().min_width(px(0)).gap(steer_gap(1.5)).items_center().flex_wrap()))
}
fn marker(ctx,status,anchor){
    let ink=theme_color(if status=="done"{"text.success"}else if status=="active"{"accent"}else{"border"});
    let content=if status=="active"{spinner::Spinner(#{key:"active-marker",label:ctx.t_or("agent_queue.running","Running"),size:"xs"})}
        else if status=="done"{icon(ctx,"check",13)}else{box([]).with_style(style().width(px(7)).height(px(7)).radius(px(4)).border(px(1)).border_color(ink))};
    let parts=if status=="pending"{["marker"]}else{["marker","marker_"+status]};
    row([content]).with_key("marker").with_ref(anchor).accessibility_hidden(true)
        .with_style(ctx.component_style(parts,style().width(steer_unit(4)).height(steer_unit(4)).items_center().justify_center().flex_shrink(false)
            .text_color(ink).background(theme_color("surface_raised")).radius(px(6))))
}
fn note_icon(ctx,part,check){
    let path=if check{"m2.8 6.2 2 2 4.4-4.6"}else{"M3 2.4v3.4c0 .9.7 1.6 1.6 1.6h4.6M7.6 5.6l1.8 1.8-1.8 1.8"};
    let weight=if check{1.5}else{1.2};
    let flip=if !check&&ctx.text_direction()=="rtl"{" transform='translate(12 0) scale(-1 1)'"}else{""};
    let art=svg(`<svg viewBox='0 0 12 12' fill='none' stroke='currentColor' stroke-width='${weight}' stroke-linecap='round' stroke-linejoin='round'><path${flip} d='${path}'/></svg>`)
        .accessibility_hidden(true);
    let base=style().width(steer_unit(3)).height(steer_unit(3)).flex_shrink(false);
    art.with_style(if check{base}else{ctx.component_style(part,base.margin_top(steer_unit(0.75)).text_color(theme_color("text_muted")))})
}
fn decorated_note(ctx,cfg,n,applied){
    let picked=value(n,"picked","");
    let bits=[text(n.text).with_key("message").with_style(ctx.component_style("note_text",style().min_width(px(0)).flex_grow().typography("caption")))];
    if applied&&picked!=""{
        let status=if picked=="now"{ctx.t_or("agent_steering.interrupted","Interrupted")}else{ctx.t_or("agent_steering.picked","Picked up")};
        bits.push(row([note_icon(ctx,"note_tag",true),text(status)]).with_key("tag")
            .with_style(ctx.component_style("note_tag",style().items_center().gap(steer_gap(0.75)).flex_shrink(false)
                .whitespace_nowrap().typography("metadata").font_weight(500).text_color(theme_color("accent")))));
    }
    let parts=if !applied{["note","queued"]}else if picked!=""{["note","note_applied","note_"+picked]}else{["note","note_applied"]};
    if applied{
        // Keep the pictogram out of the wrapping text column so that every
        // line, not only the first one, starts on the same inset.
        bits=[note_icon(ctx,"note_arrow",false),row(bits).with_key("copy")
            .with_style(style().min_width(px(0)).flex_grow().items_start().flex_wrap().gap_x(steer_gap(1.5)))];
    }
    row(bits).with_key("note:"+n.id).accessibility_role("listitem").accessibility_label(n.text)
        .with_style(ctx.component_style(parts,style().min_width(px(0)).gap(steer_gap(1))
            .padding_x(steer_gap(1.25)).padding_y(steer_gap(0.75)).radius(px(6)).background(theme_color("surface_hover"))
            .merge(if applied{style().items_start()}else{style().items_center().flex_wrap()})))
        .shared_layout(cfg.scope,n.id).layout_motion(ctx.motion_duration("normal"),ctx.motion_easing("standard"))
}
fn step_lead(ctx,children){
    row(children).accessibility_hidden(true).with_style(ctx.component_style("step_lead",
        style().width(steer_unit(7)).flex_shrink(false).items_center()))
}
fn step(ctx,props,cfg,s,anchor,now){
    let phase=value(s,"status","pending");
    let live=if phase=="active"{monitor::spent(#{state:"running",started_ms:opt(s,"started_ms"),elapsed_ms:opt(s,"elapsed_ms"),
        paused_ms:opt(s,"paused_ms")},now)}else{value(s,"elapsed_ms",0)};
    let mark=marker(ctx,phase,anchor);
    let title=text(s.title).with_key("title").with_style(ctx.component_style("step_title",style().min_width(px(0)).typography("body").flex_grow().font_weight(500)));
    let bits=[step_lead(ctx,[mark]).with_key("lead"),title];
    if phase!="pending"{bits.push(note(ctx,monitor::elapsed_phrase(ctx,live),"step_clock").with_key("clock")
        .with_style(style().margin_start(steer_gap(1.5))));}
    let parts=if phase=="pending"{["step"]}else{["step","step_"+phase]};
    if opt(props,"highlight")!=()&&!(s.id in props.highlight){parts.push("step_quiet");}
    let content=[row(bits).with_key("head").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style(parts,style().min_width(px(0)).min_height(steer_unit(8)).items_center().flex_wrap()))];
    let notes=[];for n in value(s,"notes",[]){notes.push(decorated_note(ctx,cfg,n,true));}
    if notes.len>0{
        let copy=column(notes).with_key("notes").accessibility_role("list")
            .accessibility_label(ctx.t_or("agent_steering.applied","Applied notes")).with_style(ctx.component_style("notes",
                style().min_width(px(0)).flex_grow().gap(steer_gap(0.75))));
        content.push(row([step_lead(ctx,[]).with_key("lead"),copy]).with_key("notes-layout").with_style(style().min_width(px(0))));
    }
    let node=column(content).with_key("step:"+s.id).accessibility_role("listitem").accessibility_label(s.title)
        .accessibility_description((if phase=="active"{ctx.t_or("agent_queue.running","Running")}
            else if phase=="done"{ctx.t_or("agent_queue.done","Done")}else{ctx.t_or("agent_plan.pending","Pending")})
            +(if phase=="pending"{""}else{" · "+monitor::elapsed_phrase(ctx,live)})).with_style(style().min_width(px(0)).gap(steer_gap(0.5)));
    if phase=="active"{node.accessibility_current("step")}else{node}
}
fn rail(ctx,props,stage,refs){
    let base=ctx.element_bounds(stage);if base==(){return ();}
    let rect=base.visual;if rect.width<=0.0||rect.height<=0.0{return ();}
    let points=[];
    for r in refs{
        let p=ctx.element_bounds(r);
        points.push(if p==(){()}else{#{x:p.visual.x+p.visual.width*0.5-rect.x,y:p.visual.y+p.visual.height*0.5-rect.y}});
    }
    let edges=[];
    for i in 1..points.len{
        let a=points[i-1];let b=points[i];
        if a==()||b==()||b.y<=a.y{continue;}
        let ink=theme_color(if value(props.steps[i-1],"status","pending")=="done"{"accent"}else{"border"});
        edges.push(canvas_line("rail-"+i.to_string(),a.x,a.y,b.x,b.y,1,ink));
    }
    if edges.len==0{return ();}
    canvas(canvas_scene(edges).view_box(0,0,rect.width,rect.height)).with_key("steering-rail").accessibility_hidden(true)
        .with_style(ctx.component_style("rail",style().absolute().top(px(0)).left(px(0)).width(relative(1.0)).height(relative(1.0))))
}
fn plan(ctx,props,cfg,now){
    let zone=element_ref("steer-plan");
    let rows=[];let pins=[];
    for i in 0..props.steps.len{
        let point=element_ref(ref_step(i));pins.push(point);rows.push(step(ctx,props,cfg,props.steps[i],point,now));
    }
    if rows.len==0{rows.push(note(ctx,ctx.t_or("agent_steering.no_steps","No steps yet"),"empty").with_key("empty"));}
    let group=column(rows).with_key("rows").accessibility_role("list").accessibility_label(ctx.t_or("agent_steering.plan","Plan"))
        .with_style(style().min_width(px(0)).gap(steer_gap(0.5)));
    let edge=rail(ctx,props,zone,pins);
    stack(if edge==(){[group]}else{[edge,group]}).with_key("stage").with_ref(zone)
        .with_style(ctx.component_style("plan",style().min_width(px(0)).relative()))
}
fn queued(ctx,props,cfg){
    let rows=[];
    for a in props.queue{
        let actions=[];
        if cfg.status=="running"&&cfg.active!=""&&listed(cfg,"interrupt"){
            actions.push(button::Button(#{key:"now:"+a.id,text:ctx.t_or("agent_steering.now","Now"),size:"xs",variant:"secondary",disabled:cfg.busy,
                on_click:Fn("command").curry(cfg,"interrupt",a.id,a.text)}).with_key("now")
                .accessibility_label(ctx.t_or("agent_steering.apply_note","Apply now: {note}",#{note:a.text})));
        }
        if cfg.status!="done"&&listed(cfg,"remove"){
            actions.push(button::Button(#{key:"remove:"+a.id,text:"×",size:"xs",variant:"ghost",disabled:cfg.busy,
                on_click:Fn("command").curry(cfg,"remove",a.id,"")}).with_key("remove")
                .accessibility_label(ctx.t_or("agent_steering.remove_note","Remove: {note}",#{note:a.text})));
        }
        let children=[decorated_note(ctx,cfg,a,false).with_style(style().flex_grow())];
        if actions.len>0{children.push(row(actions).with_key("actions").with_style(ctx.component_style("queue_actions",
            style().gap(steer_gap(0.5)).items_center().flex_wrap())));}
        rows.push(row(children).with_key("queued:"+a.id).with_style(style().min_width(px(0)).gap(steer_gap(1)).items_center().flex_wrap()));
    }
    let title=ctx.t_or("agent_steering.queued","Next step");
    let body=[text(title).with_key("title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("queue_title",style().typography("caption").font_weight(600)))];
    if rows.len==0{body.push(note(ctx,ctx.t_or("agent_steering.no_queue","Nothing queued"),"empty").with_key("empty"));}
    else{body.push(column(rows).with_key("items").accessibility_role("list").accessibility_label(title)
        .with_style(style().min_width(px(0)).gap(steer_gap(0.75))));}
    column(body).with_key("queue").with_style(ctx.component_style("queue",style().min_width(px(0)).gap(steer_gap(1.25))
        .padding_y(steer_gap(1.5)).border_top(px(1)).border_color(theme_color("border"))))
}
fn hint(ctx,key,words){
    row([text(key).with_key("key").with_style(ctx.component_style("keycap",style().typography("caption")
            .padding_x(steer_gap(0.75)).radius(px(3)).background(theme_color("surface_raised")))),
        note(ctx,words,"hint").with_key("words")]).with_key("hint:"+key).audit_allow(["mixed-type-in-row"])
        .with_style(style().min_width(px(0)).items_center().gap(steer_gap(0.75)))
}
fn editor(ctx,props,cfg,text){
    let editable=props.status!="done"&&(!cfg.controlled||opt(props,"on_draft_change")!=());
    let send=listed(cfg,"queue")&&props.status!="done";
    let immediate=listed(cfg,"interrupt")&&cfg.status=="running"&&cfg.active!="";
    let name=ctx.t_or("agent_steering.editor","Steer the agent");
    let placeholder=if props.status=="done"{ctx.t_or("agent_steering.finished","The run has finished")}
        else{value(props,"placeholder",ctx.t_or("agent_steering.placeholder","Leave a note for the agent…"))};
    let field=textarea::Textarea(#{key:"steering-draft",label:name,value:text,placeholder:placeholder,min_rows:1,max_rows:3,max_length:4096,
        read_only:!editable,submit_key:if send{"enter"}else{()},
        alt_submit_key:if immediate{"mod_enter"}else{()},
        on_change:if editable{Fn("draft_edit").curry(cfg)}else{()},
        on_submit:if editable&&send{Fn("submit").curry(cfg,"queue")}else{()},
        on_alt_submit:if editable&&immediate{Fn("submit").curry(cfg,"interrupt")}else{()}})
        .with_key("editor").with_style(ctx.component_style("field",style().min_width(px(0)).flex_grow()));
    let items=[field];
    if send{
        items.push(button::Button(#{key:"send",text:"↑",size:"xs",disabled:cfg.busy||!valid_text(normalize(text)),
            on_click:Fn("command").curry(cfg,"queue","",text)}).with_key("send")
            .accessibility_label(ctx.t_or("agent_steering.send","Queue for the next step"))
            .with_style(ctx.component_style("send",style().flex_shrink(false))));
    }
    let nodes=[row(items).with_key("input").with_style(style().min_width(px(0)).gap(steer_gap(1)).items_end())];
    let labels=[];
    if send{labels.push(hint(ctx,"↵",ctx.t_or("agent_steering.next_hint","next step")));}
    if immediate{labels.push(hint(ctx,if key_legend("mod").platform=="mac"{"⌘↵"}else{"Ctrl+↵"},ctx.t_or("agent_steering.now_hint","interrupt now")));}
    if props.status=="done"&&props.queue.len>0{labels.push(note(ctx,ctx.t_or("agent_steering.left","Queued notes were not used"),"hint").with_key("unused"));}
    if labels.len>0{nodes.push(row(labels).with_key("hints").with_style(ctx.component_style("hints",
        style().min_width(px(0)).gap(steer_gap(2)).items_center().flex_wrap())));}
    column(nodes).with_key("compose").with_style(ctx.component_style("compose",style().min_width(px(0)).gap(steer_gap(1))))
}
fn render_AgentSteering(ctx,props){
    if !team::valid(props.run){throw "AgentSteering needs a stable run ID";}
    let data=project(props);let now=clock(props);let u=local(ctx,props.run);let controlled=opt(props,"draft")!=();
    let draft=if controlled{props.draft}else{u.draft};let shell=element_ref("steering");
    let cfg=#{scope:shell.semantic_id+":"+utf8_byte_count(props.run).to_string()+":"+props.run,run:props.run,revision:props.revision,active:data.active,status:props.status,
        controlled:controlled,connected:opt(props,"on_action")!=(),actions:opt(props,"actions"),busy:props.busy};
    effect("settle_receipt",#{run:props.run,receipt:opt(props,"receipt"),controlled:controlled},Fn("settle"),Fn("stop"));
    let body=[header(ctx,props,cfg,data,now),plan(ctx,props,cfg,now)];
    if props.queue.len>0{body.push(queued(ctx,props,cfg));}
    let box=column(body)
        .with_key("card").audit_allow(["text-edge-misaligned"]).with_style(ctx.component_style("card",style().min_width(px(0))
            .padding(steer_gap(2)).gap(steer_gap(1.5)).radius(steer_unit(3)).background(theme_color("surface_raised"))));
    let result=[box];if props.compose{result.push(editor(ctx,props,cfg,draft));}
    let feedback=opt(props,"notice");
    if feedback!=(){result.push(note(ctx,feedback,"notice").with_key("notice").accessibility_role("status"));}
    column(result).with_key(props.key).with_ref(shell).accessibility_role("group").accessibility_label(props.title).audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).max_width(px(460)).width(relative(1.0))
            .padding(steer_gap(1.25)).gap(steer_gap(1.5)).radius(steer_unit(4)).background(theme_color("surface_hover"))))
}
