/* gpui-rhai
{"id":"patterns/agent_trail","export":"AgentTrail","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/spinner","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","space.spacing_unit","surface_hover","surface_raised","text.danger","text_muted","text_primary","typography.body","typography.caption","space.unit"],"environment":[]}
*/
// Follow execution and drill into recent substeps without transferring execution to the UI.
// Example: AgentTrail(#{key:"trail",run:"preview",revision:0,now_ms:0,steps:[]})
import "components/button" as button;
import "components/spinner" as spinner;
import "patterns/approval_card" as common;

define_component(#{
    metadata:#{id:"patterns/agent_trail","export":"AgentTrail","version":"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/spinner","patterns/approval_card"],capabilities:#{},
        tokens:["accent","border","focus_ring","space.spacing_unit","surface_hover","surface_raised","text.danger","text_muted","text_primary",
            "typography.body","typography.caption","space.unit"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable trail instance key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of the run represented by this snapshot."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision, independent of clock ticks."},
        now_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Host clock shared by all running steps."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional task title; omit to use the trail without a header."},
        agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Agent or model caption."},
        agent_icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative agent icon in the heading."},
        steps:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable step ID."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Step title."},
            state:#{schema:#{type:"string",allowed:["done","running","pending","waiting","failed","declined"]},required:true,sensitive:false,doc:"Accepted execution state."},
            elapsed_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Fixed duration of a completed step."},
            started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Start time in the Host clock domain."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Current activity, outcome, or short step summary."},
            requires_approval:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Show an upcoming approval before a pending step."},
            ask:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Evidence or question in the waiting step's approval disclosure."},
            tool:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Tool caption, also exposed when the caller supplies a logo."},
            tool_icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative logo for the step."},
            substeps:#{schema:#{type:"optional",value:#{type:"array",max_items:48,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable ID, unique within this step."},
                title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Substep caption."},
                state:#{schema:#{type:"optional",value:#{type:"string",allowed:["done","running","pending"]}},required:false,sensitive:false,doc:"Default done."},
                detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"File, count or other short detail."},
            }}}},required:false,sensitive:true,doc:"Ordered child progress; downsampled for initial folded display, never for business state."},
        }}},required:true,sensitive:true,doc:"Ordered steps from the accepted snapshot; maximum 256 substeps in total."},
        expanded:#{schema:#{type:"optional",value:#{type:"array",max_items:32,items:#{type:"string"}}},required:false,sensitive:false,doc:"Controlled expanded step IDs. Omit to expand running steps by default and allow local overrides."},
        on_expand:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives expansion intent in display order."},
        on_decision:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a versioned decision for a waiting step."},
        busy_step:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Waiting step whose proposal the Host is resolving."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host feedback; a refusal does not alter the accepted phase."},
    },state:#{fields:#{
        local:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        expand:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run identity."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Observed snapshot revision."},
            step:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Changed step ID."},
            expanded:#{schema:#{type:"array",max_items:32,items:#{type:"string"}},required:true,sensitive:false,doc:"Proposed expanded IDs in display order."},
        }},doc:"Adopt the list to accept controlled disclosure."},
        decision:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Run identity."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Observed snapshot revision."},
            step:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Waiting step ID."},
            choice:#{schema:#{type:"string",allowed:["approve","decline"]},required:true,sensitive:false,doc:"Proposed decision."},
        }},doc:"The Host must check the run, revision and waiting step. The component does not approve or start work locally."},
    },slots:#{},parts:["root","head","title","agent","agent_icon","count","body","stage","rail","pulse","entry","row","node","name","tool","detail",
        "status","clock","pending","waiting","failed","declined","ask","question","actions","sublist","sub","subnode","subnode_active","subtitle","subdetail","fold","empty","notice"],
        effects:["prune"]},render:Fn("render_AgentTrail")
});

fn AgentTrail(props){render_component("patterns/agent_trail",props)}
fn opt(m,k){common::opt(m,k)}
fn value(m,k,d){common::value_or(m,k,d)}
fn valid(id){id!=""&&utf8_byte_count(id)<=96}
fn collect(props){
    if !valid(props.run){throw "Trail run ID must be nonempty and bounded";}
    let phases=#{};let expandable=#{};let completed=0;let total=0;
    for s in props.steps{
        if !valid(s.id)||s.id in phases{throw "Trail step IDs must be unique and bounded";}
        phases[s.id]=s.state;if s.state=="done"{completed+=1;}
        let seen=#{};
        for child in value(s,"substeps",[]){
            if !valid(child.id)||child.id in seen{throw "Trail substep IDs must be unique within their step";}
            seen[child.id]=true;total+=1;
        }
        if seen.len()>0{expandable[s.id]=true;}
    }
    if total>256{throw "Trail allows at most 256 substeps";}
    let controlled=opt(props,"expanded");
    if controlled!=(){
        let check=#{};
        for id in controlled{
            if id in check||!(id in expandable){throw "Expanded IDs must identify distinct steps with substeps";}
            check[id]=true;
        }
    }
    #{phases:phases,expandable:expandable,completed:completed,controlled:controlled}
}
fn local(ctx,run){
    let s=ctx.get_state("local");if s!=()&&s.run==run{return s;}
    #{run:run,open:#{},all:#{},ask:""}
}
fn prune(ctx,deps){
    let old=ctx.get_state("local");
    if old==(){return;}
    let state=local(ctx,deps.run);let open=#{};let all=#{};
    for id in state.open.keys(){
        if id in deps.expandable&&state.open[id].phase==deps.phases[id]{open[id]=state.open[id];}
    }
    for id in state.all.keys(){if id in deps.expandable{all[id]=state.all[id];}}
    state.open=open;state.all=all;
    if !(state.ask in deps.phases)||deps.phases[state.ask]!="waiting"{state.ask="";}
    if state!=old{ctx.set_state("local",state);}
}
fn stop(ctx,deps){}
fn opened(s,data,state){
    if !(s.id in data.expandable){return false;}
    if data.controlled!=(){return s.id in data.controlled;}
    if s.id in state.open&&state.open[s.id].phase==s.state{return state.open[s.id].value;}
    s.state=="running"
}
fn toggle(token,id,phase,show,ids,local_only,ctx,event){
    if local_only{
        let s=local(ctx,token.run);s.open[id]=#{value:show,phase:phase};ctx.set_state("local",s);
    }
    ctx.emit("expand",#{run:token.run,revision:token.revision,step:id,expanded:ids});handled()
}
fn set_all(run,id,anchor,show,ctx,event){
    let s=local(ctx,run);if show{s.all[id]=anchor;}else{s.all.remove(id);}
    ctx.set_state("local",s);handled()
}
fn ask(run,id,ctx,event){
    let s=local(ctx,run);s.ask=if s.ask==id{""}else{id};ctx.set_state("local",s);handled()
}
fn cancel_ask(run,ref_id,ctx,event){
    let s=local(ctx,run);s.ask="";ctx.set_state("local",s);ctx.focus(ref_id);handled()
}
fn answer(token,id,choice,ctx,event){
    ctx.emit("decision",#{run:token.run,revision:token.revision,step:id,choice:choice});handled()
}
fn status(ctx,phase){
    switch phase{
        "running"=>ctx.t_or("agent_queue.running","Running"),
        "waiting"=>ctx.t_or("agent_trail.waiting","Needs you"),
        "done"=>ctx.t_or("agent_queue.done","Done"),
        "failed"=>ctx.t_or("agent_queue.failed","Failed"),
        "declined"=>ctx.t_or("agent_trail.declined","Declined"),
        _=>ctx.t_or("agent_plan.pending","Pending"),
    }
}
fn elapsed(ctx,s,now){
    let ms=if s.state=="running"&&opt(s,"started_ms")!=(){max(0,now-s.started_ms)}else{opt(s,"elapsed_ms")};
    if ms==(){return "";}
    let sec=ms/1000;
    if sec<60{return ctx.t_or("agent_lanes.seconds","{n} s",#{n:ctx.format_number(sec)});}
    ctx.t_or("agent_inbox.minutes_seconds","{m} min {s} s",#{m:ctx.format_number(sec/60),s:ctx.format_number(sec%60)})
}
fn note(ctx,s,part){common::heading_note(ctx,s,part)}
fn wrap_icon(ctx,graphic,part){
    row([graphic]).with_key(part).accessibility_hidden(true).with_style(ctx.component_style(part,
        style().width(theme_length("space.unit").times(4.5)).height(theme_length("space.unit").times(4.5)).items_center().justify_center().flex_shrink(false)))
}
fn pin(ctx,s,reference,sub){
    let phase=value(s,"state","done");let tone=if phase=="failed"{"text.danger"}else if phase in ["running","done"]{"accent"}else{"text_muted"};
    let active=sub&&phase=="running";
    let mark=if sub{
        let size=if active{8}else{5};
        let part=if active{["subnode","subnode_active"]}else{["subnode"]};
        box([]).with_style(ctx.component_style(part,style().width(px(size)).height(px(size)).radius(px(5)).background(theme_color(tone))))
    }else if phase=="running"{
        spinner::Spinner(#{key:"current:"+s.id,label:status(ctx,phase),size:"xs"})
    }else if phase in ["done","failed","declined","waiting"]{
        common::glyph(ctx,if phase=="done"{"check"}else if phase=="waiting"{"clock"}else{"close"},15)
    }else{
        box([]).with_style(style().width(px(9)).height(px(9)).radius(px(5)).border(px(1)).border_color(theme_color("text_muted")))
    };
    let base=style().width(theme_length("space.unit").times(4.5)).height(theme_length("space.unit").times(4.5)).items_center().justify_center().flex_shrink(false).text_color(theme_color(tone));
    if !sub{base=ctx.component_style("node",base.radius(theme_length("space.unit").times(2.25)).background(theme_color("surface_raised")));}
    row([mark.with_key("mark")]).with_key("pin").with_ref(reference).accessibility_hidden(true).with_style(base)
}
fn slice_for(subs,all){
    let n=subs.len;if all||n<=5{return #{start:0,end:n};}
    let at=-1;
    for i in 0..n{if value(subs[i],"state","done")=="running"{at=i;break;}}
    let end=if at<0{n}else{min(n,at+2)};
    #{start:max(0,end-5),end:end}
}
fn sub_row(ctx,items){
    row(items).audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("sub",style().min_width(px(0))
        .min_height(theme_length("space.unit").times(6)).items_center().gap(theme_length("space.spacing_unit").times(1.75)).flex_wrap()))
}
fn fold_button(ctx,cfg,s,key,label,more){
    let button=button::Button(#{key:key+":"+s.id,text:label,size:"xs",variant:"ghost",on_click:Fn("set_all").curry(cfg.token.run,s.id,key,more)})
        .with_key("control").accessibility_expanded(!more).with_style(ctx.component_style("fold",style().padding_start(px(0)).border(px(0))));
    let gutter=box([]).with_key("gutter").accessibility_hidden(true).with_style(style().width(theme_length("space.unit").times(4.5)).flex_shrink(false));
    sub_row(ctx,[gutter,button]).with_key(key)
}
fn subrows(ctx,cfg,s,i,state){
    let subs=value(s,"substeps",[]);let all=opt(state.all,s.id);let range=slice_for(subs,all!=());
    let rows=[];let anchors=[];let pulse=-1;
    let counts=#{earlier:range.start,later:subs.len-range.end};
    if counts.earlier>0{rows.push(fold_button(ctx,cfg,s,"earlier",ctx.t_or("agent_trail.earlier","+{n} earlier",#{n:ctx.format_number(counts.earlier)}),true));}
    else if all=="earlier"&&subs.len>5{rows.push(fold_button(ctx,cfg,s,"earlier",ctx.t_or("agent_trail.fewer","Show fewer"),false));}
    for k in range.start..range.end{
        let q=subs[k];let ref_item=element_ref("sub-"+i.to_string()+"-"+k.to_string());
        anchors.push(ref_item);
        let active=value(q,"state","done")=="running";if pulse<0&&active{pulse=anchors.len-1;}
        let title=text(q.title).with_key("label").with_style(ctx.component_style("subtitle",style().min_width(px(0)).flex_grow().typography("caption")));
        let body=[pin(ctx,q,ref_item,true),title];let detail=opt(q,"detail");
        if detail!=(){body.push(note(ctx,detail,"subdetail").with_key("detail"));}
        let row=sub_row(ctx,body).with_key("sub:"+q.id).accessibility_role("listitem").accessibility_label(q.title)
            .accessibility_description(status(ctx,value(q,"state","done"))+(if detail==(){""}else{" · "+detail}))
            .accessibility_position_in_set(k+1).accessibility_size_of_set(subs.len);
        rows.push(if active{row.accessibility_current("step")}else{row});
    }
    if counts.later>0{rows.push(fold_button(ctx,cfg,s,"later",ctx.t_or("agent_trail.more","+{n} more",#{n:ctx.format_number(counts.later)}),true));}
    else if all=="later"&&subs.len>5{rows.push(fold_button(ctx,cfg,s,"later",ctx.t_or("agent_trail.fewer","Show fewer"),false));}
    let list=column(rows).with_key("substeps").accessibility_role("list")
        .accessibility_label(ctx.t_or("agent_trail.substeps","Substeps of {title}",#{title:s.title}))
        .with_style(ctx.component_style("sublist",style().min_width(px(0)).padding_start(theme_length("space.spacing_unit").times(5.25)).padding_end(theme_length("space.spacing_unit").times(1.5))));
    #{list:list,anchors:anchors,pulse:pulse,pulse_id:if pulse>=0{subs[range.start+pulse].id}else{""}}
}
fn approval(ctx,cfg,s,ref_name,locked){
    let q=value(s,"ask",ctx.t_or("agent_trail.question","Continue with this step?"));
    let opts=[];
    for decision in ["decline","approve"]{
        let yes=decision=="approve";let word=if yes{ctx.t_or("review.approve","Approve")}else{ctx.t_or("agent_trail.decline","Decline")};
        opts.push(button::Button(#{key:decision+":"+s.id,text:word,variant:if yes{"primary"}else{"secondary"},size:"xs",disabled:locked,
            on_click:Fn("answer").curry(cfg.token,s.id,decision)}));
    }
    column([note(ctx,q,"question").with_key("question"),
        row(opts).with_key("actions").with_style(ctx.component_style("actions",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).justify_end().flex_wrap()))])
        .with_key("ask").accessibility_role("region").accessibility_label(ctx.t_or("agent_trail.approval_for","Approval for {title}",#{title:s.title}))
        .on_bubble("key:plain+escape",Fn("cancel_ask").curry(cfg.token.run,ref_name))
        .with_style(ctx.component_style("ask",style().min_width(px(0)).margin_start(theme_length("space.spacing_unit").times(7.25)).padding(theme_length("space.spacing_unit").times(2)).gap(theme_length("space.spacing_unit").times(1.5))
            .radius(px(6)).background(theme_color("surface_hover"))))
}
fn entry(ctx,props,cfg,data,ui,i,s,main){
    let label=status(ctx,s.state);let stamp=elapsed(ctx,s,props.now_ms);let detail=opt(s,"detail");let tool=opt(s,"tool");let icon=opt(s,"tool_icon");
    let stack=[pin(ctx,s,main,false)];if icon!=(){stack.push(wrap_icon(ctx,icon,"tool"));}
    stack.push(text(s.title).with_key("title").with_style(ctx.component_style("name",style().min_width(px(0)).flex_grow().typography("body"))));
    let description=label;if tool!=(){description+=" · "+tool;if icon==(){stack.push(note(ctx,tool,"tool").with_key("tool"));}}
    if detail!=(){
        description+=" · "+detail;
        if !cfg.compact{stack.push(note(ctx,detail,"detail").with_key("detail"));}
    }
    if stamp!=""{description+=" · "+stamp;}
    let can_toggle=s.id in data.expandable&&(data.controlled==()||cfg.expand);
    let is_open=opened(s,data,ui);let asks=s.state=="waiting"&&cfg.decide;let is_asking=asks&&ui.ask==s.id;let locked=s.id==cfg.busy;
    let ask_ref="ask-"+i.to_string();
    if asks{
        stack.push(button::Button(#{key:"review:"+s.id,text:label,variant:"secondary",size:"xs",
            on_click:Fn("ask").curry(cfg.token.run,s.id)}).with_key("review").with_ref(element_ref(ask_ref))
            .accessibility_label(ctx.t_or("agent_trail.review","Review {title}",#{title:s.title})).accessibility_expanded(is_asking)
            .with_style(ctx.component_style("waiting",style().flex_shrink(false))));
    }else if s.state!="done"{
        let tag=if s.state=="pending"&&value(s,"requires_approval",false){ctx.t_or("agent_trail.approval","Approval")}else{label};
        stack.push(note(ctx,tag,if s.state=="running"{"status"}else{s.state}).with_key("status"));
    }
    if stamp!=""{stack.push(note(ctx,stamp,"clock").with_key("clock"));}
    if s.id in data.expandable{
        let proposed=[];for item in props.steps{
            let include=if item.id==s.id{!is_open}else{opened(item,data,ui)};
            if include{proposed.push(item.id);}
        }
        let name=if is_open{ctx.t_or("agent_trail.hide_steps","Hide substeps of {title}",#{title:s.title})}
            else{ctx.t_or("agent_trail.show_steps","Show substeps of {title}",#{title:s.title})};
        stack.push(button::Button(#{key:"steps:"+s.id,text:if is_open{"⌃"}else{"⌄"},variant:"ghost",size:"xs",disabled:!can_toggle,
            on_click:Fn("toggle").curry(cfg.token,s.id,s.state,!is_open,proposed,data.controlled==())})
            .with_key("expand").accessibility_label(name).accessibility_expanded(is_open).with_style(style().flex_shrink(false)));
    }
    let lines=[row(stack).with_key("step").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("row",
        style().min_width(px(0)).min_height(theme_length("space.unit").times(8)).items_center().flex_wrap().gap(theme_length("space.spacing_unit").times(1.75)).padding_end(theme_length("space.spacing_unit"))))];
    if cfg.compact&&detail!=(){
        lines.push(note(ctx,detail,"detail").with_key("compact-detail").with_style(
            style().padding_start(theme_length("space.spacing_unit").times(7.25)).padding_end(theme_length("space.spacing_unit")).padding_bottom(theme_length("space.spacing_unit").times(0.75))));
    }
    let sub=();
    if is_open{sub=subrows(ctx,cfg,s,i,ui);lines.push(sub.list);}
    if is_asking{lines.push(approval(ctx,cfg,s,ask_ref,locked));}
    let node=column(lines).with_key("step:"+s.id).accessibility_role("listitem").accessibility_label(s.title)
        .accessibility_description(description).accessibility_position_in_set(i+1).accessibility_size_of_set(props.steps.len)
        .with_style(ctx.component_style("entry",style().min_width(px(0))));
    if s.state=="running"{node=node.accessibility_current("step");}
    #{node:node,sub:sub}
}
fn center(ctx,reference,b){
    let measured=ctx.element_bounds(reference);if measured==(){return ();}
    let g=measured.visual;#{x:g.x+g.width*0.5-b.x,y:g.y+g.height*0.5-b.y}
}
fn path(points){
    let p=points[0];let nodes=[path_move(p.x,p.y)];
    for i in 1..points.len{
        let a=points[i-1];let z=points[i];let mid=(a.y+z.y)*0.5;
        if (a.x-z.x).abs()<0.5{nodes.push(path_line(z.x,z.y));}
        else{nodes.push(path_cubic(z.x,z.y,a.x,mid,z.x,mid));}
    }
    nodes
}
fn paint(ctx,part){
    ctx.component_style(part,style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)))
}
fn trace(ctx,props,cfg,stage_ref,refs,branches){
    let frame=ctx.element_bounds(stage_ref);if frame==(){return [];}
    let b=frame.visual;if b.width<=0.0||b.height<=0.0{return [];}
    let dots=[];for id in refs{dots.push(center(ctx,id,b));}
    let links=[];let pulses=[];
    for i in 0..dots.len{
        let top=dots[i];if top==(){continue;}
        let next=if i+1<dots.len{dots[i+1]}else{()};
        let trail=[top];let sub=branches[i];let spark=[];
        if sub!=(){
            let complete=true;
            for k in 0..sub.anchors.len{
                let p=center(ctx,sub.anchors[k],b);
                if p==(){complete=false;break;}
                trail.push(p);
                if k==sub.pulse{spark=trail;}
            }
            if !complete{trail=[top];spark=[];}
        }
        if next!=(){trail.push(next);}
        if trail.len>1{
            let reached=props.steps[i].state=="done"&&(i+1>=dots.len||props.steps[i+1].state!="pending");
            links.push(canvas_stroke_path("edge-"+i.to_string(),path(trail),1.2,theme_color(if reached{"accent"}else{"border"})).stroke_cap("round"));
        }
        if spark.len>1&&props.steps[i].state=="running"{
            pulses.push(canvas(canvas_scene([canvas_stroke_path("pulse",path(spark),1.6,theme_color("accent"))]).view_box(0,0,b.width,b.height))
                .with_key("pulse:"+props.steps[i].id).accessibility_hidden(true).with_style(paint(ctx,"pulse"))
                .motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:ctx.motion_duration("normal"),intent:"decorative"}))
                .motion(motion_transition("opacity",0.95,0.0,#{duration_ms:ctx.motion_duration("normal"),intent:"decorative"}))
                .motion_on_change(utf8_byte_count(cfg.token.run).to_string()+":"+cfg.token.run+sub.pulse_id));
        }
    }
    let out=[];
    if links.len>0{out.push(canvas(canvas_scene(links).view_box(0,0,b.width,b.height)).with_key("rail")
        .accessibility_hidden(true).with_style(paint(ctx,"rail")));}
    for pulse in pulses{out.push(pulse);}
    out
}
fn render_AgentTrail(ctx,props){
    let data=collect(props);let ui=local(ctx,props.run);let stage_ref=element_ref("stage");
    let cfg=#{token:#{run:props.run,revision:props.revision},expand:opt(props,"on_expand")!=(),decide:opt(props,"on_decision")!=(),
        busy:opt(props,"busy_step"),compact:ctx.element_width_class(stage_ref,[430])==0};
    let title=opt(props,"title");let name=if title==(){ctx.t_or("agent_trail.title","Progress")}else{title};
    let nodes=[];
    if title!=(){
        let header=[text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
            .with_style(ctx.component_style("title",style().min_width(px(0)).flex_grow().typography("body").font_weight(500)))];
        let agent=opt(props,"agent");if agent!=(){
            let mark=opt(props,"agent_icon");let bits=[];
            if mark!=(){bits.push(wrap_icon(ctx,mark,"agent_icon"));}
            bits.push(note(ctx,agent,"agent").with_key("agent-label"));
            header.push(row(bits).with_key("agent").with_style(style().items_center().gap(theme_length("space.spacing_unit"))));
        }
        header.push(note(ctx,ctx.format_number(data.completed)+"/"+ctx.format_number(props.steps.len),"count").with_key("count")
            .accessibility_role("status").accessibility_label(ctx.t_or("agent_trail.count","{done} of {total} steps done",
                #{done:ctx.format_number(data.completed),total:ctx.format_number(props.steps.len)})));
        nodes.push(row(header).with_key("head").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("head",
            style().min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(2)).flex_wrap())));
    }
    let refs=[];let branches=[];let rows=[];
    for i in 0..props.steps.len{
        let dot=element_ref("step-"+i.to_string());let result=entry(ctx,props,cfg,data,ui,i,props.steps[i],dot);
        refs.push(dot);branches.push(result.sub);rows.push(result.node);
    }
    let body=[];
    if rows.len==0{body.push(note(ctx,ctx.t_or("agent_trail.empty","No steps yet"),"empty").with_key("empty"));}
    else{
        let deck=trace(ctx,props,cfg,stage_ref,refs,branches);
        deck.push(column(rows).with_key("steps").accessibility_role("list").accessibility_label(name).with_style(style().min_width(px(0))));
        body.push(stack(deck).with_key("stage").with_ref(stage_ref).accessibility_role("group")
            .accessibility_label(ctx.t_or("agent_trail.timeline","Execution trail")).with_style(ctx.component_style("stage",style().min_width(px(0)).relative())));
    }
    let feedback=opt(props,"notice");
    if feedback!=(){body.push(note(ctx,feedback,"notice").with_key("notice").accessibility_role("status"));}
    nodes.push(column(body).with_key("body").with_style(ctx.component_style("body",
        style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(1.25)).radius(theme_length("space.unit").times(2.5)).background(theme_color("surface_raised")))));
    effect("prune",#{run:props.run,phases:data.phases,expandable:data.expandable},Fn("prune"),Fn("stop"));
    column(nodes).with_key(props.key).accessibility_role("group").accessibility_label(name).audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).padding(theme_length("space.spacing_unit"))
            .gap(theme_length("space.spacing_unit").times(0.75)).radius(theme_length("space.unit").times(3.5)).background(theme_color("surface_hover"))))
}
