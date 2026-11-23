/* gpui-rhai
{"id":"patterns/agent_plan","export":"AgentPlan","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/input","components/sortable","components/spinner","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","space.spacing_unit","surface_hover","surface_raised","text.danger","text.success","text_muted","text_primary","typography.body","typography.caption","typography.code","space.unit"],"environment":["density"]}
*/
// Review and edit an ordered plan, then present the Host's execution and recovery.
// State: an uncommitted text editor only; step order and phase are Host inputs.
// Example: AgentPlan(#{key:"run",plan:"release",revision:0,now_ms:0,steps:[]})
import "components/button" as button;
import "components/input" as input;
import "components/sortable" as sortable;
import "components/spinner" as spinner;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_plan","export":"AgentPlan",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/input","components/sortable","components/spinner","patterns/approval_card"],capabilities:#{},
        tokens:["accent","border","focus_ring","space.spacing_unit","surface_hover","surface_raised","text.danger","text.success","text_muted","text_primary",
            "typography.body","typography.caption","typography.code","space.unit"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable instance key."},
        plan:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business identity of the plan."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Current accepted revision. Advance when the plan changes, not on clock ticks."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Goal heading."},
        estimate:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Estimated duration in the draft header."},
        status:#{schema:#{type:"string",allowed:["draft","running","failed","done","cancelled"]},"default":#{type:"string",value:"draft"},
            required:false,sensitive:false,doc:"Accepted plan phase."},
        now_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"A single Host clock for active step durations."},
        steps:#{schema:#{type:"array",max_items:48,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable step ID."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Step title."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short step explanation."},
            tool:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Name of the tool, for display only."},
            status:#{schema:#{type:"optional",value:#{type:"string",allowed:["pending","active","done","failed","skipped"]}},
                required:false,sensitive:false,doc:"Accepted step status; default pending."},
            started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Host start of the active step."},
            elapsed_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Accepted fixed duration of a terminal step."},
            error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Reason for failure."},
        }}},required:true,sensitive:true,doc:"Steps in the current plan order. Task identity, not array index, owns edit/retry actions."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Host is resolving a proposal; local editor text remains visible."},
        allow_cancel:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Show a Cancel proposal in draft, processing or failure."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host feedback from accepting or rejecting proposals."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives versioned edits, run, cancel and retry proposals."},
    },state:#{fields:#{editor:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
        refocus:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{
        action:#{payload:#{type:"object",allow_unknown:false,fields:#{
            plan:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business plan ID."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision read by the control sending this proposal."},
            kind:#{schema:#{type:"string",allowed:["add","edit","move","remove","run","cancel","retry"]},required:true,sensitive:false,doc:"Requested action."},
            step:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Affected step ID, or empty for plan-level and add proposals."},
            anchor:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Move destination anchor ID, otherwise empty."},
            placement:#{schema:#{type:"string",allowed:["","before","after"]},required:true,sensitive:false,doc:"Position relative to the anchor for a move."},
            order:#{schema:#{type:"array",max_items:48,items:#{type:"string"}},required:true,sensitive:false,doc:"Accepted order at the point of a Run request; empty for other actions."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Text of an Add or Edit proposal, otherwise empty."},
            detail:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Step explanation for Add/Edit, otherwise empty."},
            tool:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Tool caption for Add/Edit, otherwise empty."},
        }},doc:"Validate the plan, revision and identities in the Host. This component does not run tools."},
    },slots:#{},parts:["root","head","title","estimate","status","workspace","draft","sort_item","step","step_head","step_body","step_title","detail","tool",
        "number","marker","marker_active","marker_done","marker_failed","marker_skipped","elapsed","error","retry","controls","editor","editor_title",
        "field","field_label","footer","summary","empty","notice","rail"],effects:["settle_editor"]},
    render:Fn("render_AgentPlan")
});

fn AgentPlan(props){render_component("patterns/agent_plan",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,d){common::value_or(m,k,d)}
fn valid(id){
    if id==""||utf8_byte_count(id)>96{return false;}
    for c in id.chars(){if c<' '||(c>='\u007f'&&c<='\u009f'){return false;}}
    true
}
fn state(step){value(step,"status","pending")}
fn terminal(step){state(step) in ["done","skipped"]}
fn project(props){
    if !valid(props.plan){throw "Plan needs a stable business identity";}
    let rows=#{};
    let order=[];let complete=0;
    for i in 0..props.steps.len{
        let s=props.steps[i];if !valid(s.id)||s.id in rows{throw "Plan step IDs must be unique and bounded";}
        s.ordinal=i;rows[s.id]=s;order.push(s.id);
        if terminal(s){complete+=1;}
    }
    #{by_id:rows,order:order,complete:complete}
}
fn fields(t){#{title:t.title,detail:value(t,"detail",""),tool:value(t,"tool","")}}
fn blank(){#{title:"",detail:"",tool:""}}
fn put(ctx,token,kind,step,order,anchor,placement,fields){
    ctx.emit("action",#{plan:token.plan,revision:token.revision,kind:kind,step:step,order:order,anchor:anchor,placement:placement,
        title:fields.title,detail:fields.detail,tool:fields.tool});handled()
}
fn act(token,kind,step,order,ctx,event){put(ctx,token,kind,step,order,"","",blank())}
fn moved(token,ctx,p){
    if p.source_key==p.anchor_key{return;}
    put(ctx,token,"move",p.source_key,[],p.anchor_key,p.placement,blank())
}
fn start_edit(token,id,from,known,ctx,event){
    ctx.set_state("refocus",());
    ctx.set_state("editor",#{plan:token.plan,step:id,original:from,values:from,known:known,proposed:()});handled()
}
fn edit_active(e,props,data){
    if e==()||props.status!="draft"||e.plan!=props.plan||opt(props,"on_action")==(){return false;}
    if e.step!=""{return e.step in data.by_id&&fields(data.by_id[e.step])==e.original;}
    if props.steps.len>=48{return false;}
    if e.proposed!=(){
        for step in props.steps{if !(step.id in e.known)&&fields(step)==e.proposed{return false;}}
    }
    true
}
fn normalized(v){
    let title=v.title;let detail=v.detail;let tool=v.tool;
    title.trim();detail.trim();tool.trim();
    #{title:title,detail:detail,tool:tool}
}
fn good(v){v.title!=""&&utf8_byte_count(v.title)<=512&&utf8_byte_count(v.detail)<=4096&&utf8_byte_count(v.tool)<=120}
fn change(token,id,field,ctx,new_value){
    let e=ctx.get_state("editor");if e==()||e.plan!=token.plan||e.step!=id||utf8_byte_count(new_value)>8192{return;}
    e.values[field]=new_value;ctx.set_state("editor",e);
}
fn send(token,id,ctx){
    let e=ctx.get_state("editor");if e==()||e.plan!=token.plan||e.step!=id{return;}
    let clean=normalized(e.values);
    if !good(clean)||(id!=""&&clean==e.original){return;}
    e.proposed=clean;ctx.set_state("editor",e);
    put(ctx,token,if id==""{"add"}else{"edit"},id,[],"","",clean)
}
fn save(token,id,ctx,event){send(token,id,ctx)}
fn submit(token,id,field,ctx,event){change(token,id,field,ctx,event);send(token,id,ctx)}
fn focus_edit_row(ordinal){"edit-row-"+ordinal.to_string()}
fn discard(token,id,ctx,event){
    ctx.set_state("editor",());ctx.set_state("refocus",#{plan:token.plan,step:id});handled()
}
fn settle(ctx,p){
    if p==(){return;}
    if ctx.get_state("editor")==p.previous{
        ctx.set_state("editor",());
        ctx.set_state("refocus",());
        if p.focus!=""{ctx.focus(p.focus);}
    }
}
fn no_effect(ctx,p){}
fn caption(ctx,v,part){common::heading_note(ctx,v,part).with_style(style().min_width(px(0)))}
fn phase_word(ctx,s){
    if s=="draft"{ctx.t_or("agent_plan.draft","Draft")}else if s=="running"{ctx.t_or("agent_queue.running","Running")}
    else if s=="done"{ctx.t_or("agent_queue.done","Done")}else if s=="failed"{ctx.t_or("agent_queue.failed","Failed")}
    else{ctx.t_or("agent_queue.cancelled","Cancelled")}
}
fn step_word(ctx,s){
    if s=="active"{ctx.t_or("agent_queue.running","Running")}else if s=="done"{ctx.t_or("agent_queue.done","Done")}
    else if s=="failed"{ctx.t_or("agent_queue.failed","Failed")}else if s=="skipped"{ctx.t_or("agent_plan.skipped","Skipped")}
    else{ctx.t_or("agent_plan.pending","Pending")}
}
fn duration(ctx,ms){
    let seconds=ms/1000;
    if seconds<60{return ctx.t_or("agent_queue.seconds","{n} s",#{n:ctx.format_number(seconds)});}
    ctx.t_or("agent_inbox.minutes_seconds","{m} min {s} s",#{m:ctx.format_number(seconds/60),s:ctx.format_number(seconds%60)})
}
fn step_time(ctx,s,clock){
    let elapsed=opt(s,"elapsed_ms");let start=opt(s,"started_ms");
    if state(s)=="active"{elapsed=if start==(){()}else{max(0,clock-start)};}
    if elapsed==(){""}else{duration(ctx,elapsed)}
}
fn body(ctx,s,cfg){
    let rows=[text(s.title).with_key("title").with_style(ctx.component_style("step_title",style().min_width(px(0)).typography("body").font_weight(500)))];
    let detail=opt(s,"detail");if detail!=()&&detail!=""{rows.push(caption(ctx,detail,"detail").with_key("detail"));}
    let tool=opt(s,"tool");if tool!=()&&tool!=""{rows.push(text(tool).with_key("tool")
        .with_style(ctx.component_style("tool",style().min_width(px(0)).typography("code").text_color(theme_color("text_muted")))));}
    column(rows).with_key("body").with_style(ctx.component_style("step_body",style().min_width(px(0)).flex_grow().gap(theme_length("space.spacing_unit").times(0.5))))
}
fn small_button(key,label,variant,disabled,f){
    button::Button(#{key:key,text:label,size:"xs",variant:variant,disabled:disabled,on_click:f})
}
fn draft_row(ctx,s,cfg){
    let number=caption(ctx,ctx.format_number(s.ordinal+1),"number").with_key("number").accessibility_hidden(true)
        .with_style(style().flex_shrink(false));
    let controls=[];
    if cfg.actions{
        controls.push(small_button("edit-"+s.id,ctx.t_or("agent_plan.edit","Edit"),"ghost",cfg.busy||cfg.editing,
            Fn("start_edit").curry(cfg.token,s.id,fields(s),[])).with_key("edit").with_ref(element_ref(focus_edit_row(s.ordinal)))
            .accessibility_label(ctx.t_or("agent_plan.edit_step","Edit {step}",#{step:s.title})));
        controls.push(small_button("remove-"+s.id,ctx.t_or("agent_plan.remove","Remove"),"ghost",cfg.busy||cfg.editing,
            Fn("act").curry(cfg.token,"remove",s.id,[])).with_key("remove")
            .accessibility_label(ctx.t_or("agent_plan.remove_step","Remove {step}",#{step:s.title})));
    }
    let bits=[number,body(ctx,s,cfg)];
    if controls.len>0{bits.push(row(controls).with_key("buttons").with_style(ctx.component_style("controls",
        style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(0.5)).flex_wrap())));}
    row(bits).with_key("review-row").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("step_head",
        style().min_width(px(0)).gap(theme_length("space.spacing_unit")).padding_y(theme_length("space.spacing_unit"))
        .padding_end(theme_length("space.spacing_unit")).items_center().flex_wrap()))
}
fn draft_list(ctx,props,cfg,data){
    let label=ctx.t_or("agent_plan.steps","Plan steps");let items=[];
    for id in data.order{let t=data.by_id[id];items.push(#{key:t.id,label:t.title,content:draft_row(ctx,t,cfg)});}
    if items.len==0{return caption(ctx,ctx.t_or("agent_plan.empty","No steps in this plan"),"empty").with_key("empty");}
    sortable::Sortable(#{key:"draft",label:label,items:items,disabled:cfg.busy||!cfg.actions||cfg.editing,
        gap:theme_length("space.spacing_unit").times(0.75),on_reorder:Fn("moved").curry(cfg.token),
        part_styles:#{item:ctx.component_style("sort_item",style().border(px(0)).background(theme_color("surface_raised")).radius(px(6))),
            item_disabled:style().opacity(1.0)}})
        .with_style(ctx.component_style("draft",style().min_width(px(0))))
}
fn editor(ctx,cfg,e){
    let labels=[["title",ctx.t_or("agent_plan.step_title","Step title")],["detail",ctx.t_or("agent_plan.step_detail","Step detail")],
        ["tool",ctx.t_or("agent_plan.step_tool","Tool name")]];
    let adding=e.step=="";let text=if adding{ctx.t_or("agent_plan.add","Add step")}else{ctx.t_or("agent_plan.edit_title","Edit step")};
    let rows=[text(text).with_key("title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("editor_title",style().typography("body").font_weight(600)))];
    let clean=normalized(e.values);
    let locked=cfg.busy||!good(clean)||(!adding&&clean==e.original);
    for spec in labels{
        let field=spec[0];let label=spec[1];
        let entry=input::Input(#{key:"editor-"+e.step+":"+field,label:label,value:e.values[field],read_only:cfg.busy,
            autofocus:field=="title",size:"sm",on_change:Fn("change").curry(cfg.token,e.step,field),
            on_submit:if cfg.busy{()}else{Fn("submit").curry(cfg.token,e.step,field)}})
            .with_key("entry").with_style(style().width(relative(1.0)).min_width(px(0)));
        rows.push(column([caption(ctx,label,"field_label").with_key("label"),entry]).with_key("field-"+field)
            .with_style(ctx.component_style("field",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(0.5)))));
    }
    rows.push(row([small_button("discard",ctx.t_or("agent_plan.discard","Discard edit"),"ghost",cfg.busy,Fn("discard").curry(cfg.token,e.step)),
        small_button("save",ctx.t_or("agent_plan.save","Save step"),"primary",locked,Fn("save").curry(cfg.token,e.step))])
        .with_key("actions").with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit")).flex_wrap()));
    column(rows).with_key("editor").accessibility_role("region").accessibility_label(text)
        .on_bubble("key:plain+escape",if cfg.busy{Fn("nothing")}else{Fn("discard").curry(cfg.token,e.step)})
        .with_style(ctx.component_style("editor",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).padding(theme_length("space.spacing_unit").times(2.25))
            .background(theme_color("surface_hover")).radius(px(7))))
}
fn nothing(ctx,p){handled()}
fn mark(ctx,s,anchor){
    let current=state(s);
    let node=if current=="active"{spinner::Spinner(#{key:"spin-"+s.id,label:ctx.t_or("agent_queue.running","Running"),size:"xs"})}
        else{text(if current=="done"{"✓"}else if current=="failed"{"!"}else if current=="skipped"{"–"}else{ctx.format_number(s.ordinal+1)})};
    let ink=theme_color(if current=="done"{"text.success"}else if current=="failed"{"text.danger"}else if current=="active"{"accent"}else{"text_muted"});
    let part=if current=="pending"{["marker"]}else{["marker","marker_"+current]};
    row([node]).with_key("marker").with_ref(anchor).accessibility_hidden(true)
        .with_style(ctx.component_style(part,style().width(theme_length("space.unit").times(5.75)).height(theme_length("space.unit").times(5.75)).items_center().justify_center().self_start().flex_shrink(false)
            .radius(theme_length("space.unit").times(3)).background(theme_color("surface_raised")).border(px(1)).border_color(ink).text_color(ink).typography("caption")))
}
fn execution_row(ctx,s,cfg,anchor){
    let stage=state(s);let time=step_time(ctx,s,cfg.now);let label=step_word(ctx,stage)+(if time==""{""}else{" · "+time});
    let content=[row([body(ctx,s,cfg),caption(ctx,label,"elapsed").with_key("elapsed")]).with_key("head")
        .audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit")).flex_wrap().items_center())];
    if stage=="failed"{
        let error=opt(s,"error");if error!=(){content.push(caption(ctx,error,"error").with_key("error").with_style(style().text_color(theme_color("text.danger"))));}
        if cfg.actions&&cfg.mode in ["running","failed"]{
            content.push(small_button("retry-"+s.id,ctx.t_or("agent_plan.retry","Retry from here"),"secondary",cfg.busy,Fn("act").curry(cfg.token,"retry",s.id,[]))
                .with_key("retry").with_style(ctx.component_style("retry",style().self_start())));
        }
    }
    row([mark(ctx,s,anchor),column(content).with_key("contents").with_style(style().min_width(px(0)).flex_grow()
        .gap(theme_length("space.spacing_unit").times(0.75)))])
        .with_key("step-"+s.id).accessibility_role("listitem").accessibility_label(s.title).accessibility_description(label)
        .audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("step",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).items_start()))
}
fn rail(ctx,stage,anchors,steps){
    let actual=ctx.element_bounds(stage);if actual==()||anchors.len<2{return ();}
    let box=actual.visual;if box.width<5.0||box.height<5.0{return ();}
    let commands=[];
    for i in 0..anchors.len-1{
        let a=ctx.element_bounds(anchors[i]);let b=ctx.element_bounds(anchors[i+1]);if a==()||b==(){continue;}
        let from=a.visual;let to=b.visual;
        let done=terminal(steps[i]);
        commands.push(canvas_line((if done{"complete-"}else{"remaining-"})+i.to_string(),from.x+from.width*0.5-box.x,from.y+from.height*0.5-box.y,
            to.x+to.width*0.5-box.x,to.y+to.height*0.5-box.y,if done{2}else{1},theme_color(if done{"text.success"}else{"border"})));
    }
    if commands.len==0{return ();}
    canvas(canvas_scene(commands).view_box(0,0,box.width,box.height)).with_key("rail").accessibility_hidden(true)
        .with_style(ctx.component_style("rail",style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0))))
}
fn execution(ctx,props,cfg,data){
    if props.steps.len==0{return caption(ctx,ctx.t_or("agent_plan.empty","No steps in this plan"),"empty").with_key("empty");}
    let stage=element_ref("execution");let refs=[];let rows=[];let i=0;
    for id in data.order{
        let anchor=element_ref("marker-"+i.to_string());refs.push(anchor);
        rows.push(execution_row(ctx,data.by_id[id],cfg,anchor));i+=1;
    }
    let decoration=rail(ctx,stage,refs,props.steps);
    let body=column(rows).with_key("steps").with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)))
        .accessibility_role("list").accessibility_label(ctx.t_or("agent_plan.steps","Plan steps"));
    let all=if decoration==(){[body]}else{[decoration,body]};
    stack(all).with_key("execution").with_ref(stage).with_style(style().min_width(px(0)).relative())
}
fn footer(ctx,props,cfg,data){
    let group=[];
    if cfg.actions&&props.status=="draft"&&props.steps.len<48{
        group.push(small_button("add-step",ctx.t_or("agent_plan.add","Add step"),"secondary",cfg.busy||cfg.editing,
            Fn("start_edit").curry(cfg.token,"",blank(),data.order)).with_key("add-step").with_ref(element_ref("add-step")));
    }
    if cfg.actions&&props.allow_cancel&&props.status in ["draft","running","failed"]{
        group.push(small_button("cancel-plan",ctx.t_or("agent_plan.cancel","Cancel plan"),"ghost",cfg.busy,
            Fn("act").curry(cfg.token,"cancel","",[])));
    }
    if cfg.actions&&props.status=="draft"{
        group.push(small_button("run-plan",ctx.t_or("agent_plan.run","Run plan"),"primary",cfg.busy||cfg.editing||data.order.len==0,
            Fn("act").curry(cfg.token,"run","",data.order)));
    }
    let reading=ctx.t_or("agent_plan.progress","{done} of {total} steps complete",
        #{done:ctx.format_number(data.complete),total:ctx.format_number(data.order.len)});
    row([caption(ctx,reading,"summary").with_key("progress").accessibility_role("status").with_style(style().flex_grow()),
        row(group).with_key("buttons").with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit")).flex_wrap())])
        .with_key("footer").with_style(ctx.component_style("footer",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2))
            .items_center().flex_wrap())).audit_allow(["mixed-type-in-row"])
}
fn render_AgentPlan(ctx,props){
    let data=project(props);let own=element_ref("plan");let proposed=ctx.get_state("editor");
    let editing=edit_active(proposed,props,data);
    let cfg=#{token:#{plan:props.plan,revision:props.revision},busy:props.busy,actions:opt(props,"on_action")!=(),editing:editing,mode:props.status,now:props.now_ms};
    let focus="";let refocus=ctx.get_state("refocus");
    let source=if proposed!=()&&!editing{proposed}else{refocus};
    if source!=()&&!editing&&source.plan==props.plan&&props.status=="draft"&&cfg.actions&&!cfg.busy{
        if source.step!=""&&source.step in data.by_id{focus=focus_edit_row(data.by_id[source.step].ordinal);}
        else if props.steps.len<48{focus="add-step";}
    }
    effect("settle_editor",if source!=()&&!editing{#{previous:proposed,focus:focus}}else{()},Fn("settle"),Fn("no_effect"));
    let title=value(props,"title",ctx.t_or("agent_plan.title","Agent plan"));
    let info=[text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
        .with_style(ctx.component_style("title",style().min_width(px(0)).flex_grow().typography("body").font_weight(600))),
        caption(ctx,phase_word(ctx,props.status),"status").with_key("status").accessibility_role("status")];
    if props.status=="draft"&&opt(props,"estimate")!=(){info.push(caption(ctx,props.estimate,"estimate").with_key("estimate"));}
    let top=row(info).with_key("head").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("head",
        style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).items_center().flex_wrap()));
    let content=if props.status=="draft"{draft_list(ctx,props,cfg,data)}else{execution(ctx,props,cfg,data)};
    let parts=[top,column([content]).with_key("workspace").with_style(ctx.component_style("workspace",style().min_width(px(0))))];
    if editing{parts.push(editor(ctx,cfg,proposed));}
    parts.push(footer(ctx,props,cfg,data));
    if opt(props,"notice")!=(){parts.push(caption(ctx,props.notice,"notice").with_key("notice").accessibility_role("status"));}
    column(parts).with_key(props.key).with_ref(own).accessibility_role("group").accessibility_label(title)
        // The inset work surface begins with a grip or execution marker;
        // its decorative glyph is deliberately not the heading's text edge.
        .audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(760))
            .padding(theme_length("space.spacing_unit").times(2)).gap(theme_length("space.spacing_unit").times(3))))
}
