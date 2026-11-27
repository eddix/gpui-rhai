/* gpui-rhai
{"id":"patterns/approval_gates","export":"ApprovalGates","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/code_viewer","components/diff_viewer","components/icon_button","components/textarea","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted","text_primary","typography.body","typography.caption","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// ApprovalGates presents the pending gates, blocked steps and accepted history of one run.
// State: selected gate, disclosures and a revision-checked edit. Nothing executes in the component.
// Example: ApprovalGates(#{key:"review",run:"release/1",gates:[]})
import "components/button" as button;
import "components/code_viewer" as code;
import "components/diff_viewer" as diff;
import "components/icon_button" as icon_button;
import "components/textarea" as textarea;
import "patterns/approval_card" as review;

define_component(#{
    metadata:#{id:"patterns/approval_gates","export":"ApprovalGates",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/code_viewer","components/diff_viewer","components/icon_button","components/textarea","patterns/approval_card"],
        capabilities:#{},tokens:["border","focus_ring","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted","text_primary","typography.body","typography.caption","space.spacing_unit","space.unit"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable visual identity of the stack."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Host run identity, included in every proposal."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Summary heading."},
        task:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional task caption."},
        live:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Whether the run can receive further gates."},
        run_state:#{schema:#{type:"optional",value:#{type:"string",allowed:["waiting","running","stopped","done"]}},required:false,sensitive:false,
            doc:"Optional authoritative run label. Otherwise inferred from accepted gates and live."},
        gates:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique gate ID within the run."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision of the presented facts and proposed action."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Display name of the requester."},
            kind:#{schema:#{type:"string",allowed:["tool","file","agent","message"]},required:true,sensitive:false,doc:"Evidence type. No tool call is interpreted or invoked."},
            action:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Plain description of the requested action."},
            reason:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Reason the requester supplied."},
            status:#{schema:#{type:"string",allowed:["open","approved","edited","rejected","changes_requested"]},required:true,sensitive:false,
                doc:"Accepted phase; only open gates accept new decisions."},
            busy:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"A proposal is waiting for the Host."},
            editable:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Show edit, or ask for file changes, while open."},
            input:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Read-only evidence and the initial text for a non-file edit."},
            diff:#{schema:#{type:"optional",value:#{type:"array",max_items:8,items:#{type:"object",allow_unknown:false,fields:#{
                name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Display filename, not read by the component."},
                before:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Original text provided by the Host."},
                after:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Proposed text provided by the Host."},
            }}}},required:false,sensitive:true,doc:"Neutral, read-only file comparisons."},
            files:#{schema:#{type:"optional",value:#{type:"array",max_items:32,items:#{type:"string"}}},required:false,sensitive:true,doc:"Resources touched."},
            context:#{schema:#{type:"optional",value:#{type:"array",max_items:24,items:#{type:"string"}}},required:false,sensitive:true,doc:"Earlier steps."},
            holds:#{schema:#{type:"optional",value:#{type:"array",max_items:24,items:#{type:"string"}}},required:false,sensitive:false,doc:"Steps waiting on this gate."},
            at:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Caller-formatted time."},
            note:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Accepted decision or rejection feedback."},
        }}},required:true,sensitive:true,doc:"All gates; keep accepted receipts in this list to retain history."},
        on_decide:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a versioned approval, rejection, edited approval or request for file changes."},
    },state:#{fields:#{
        page:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        more:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        history:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        draft:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
        focus_edit:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        decide:#{payload:#{type:"object",allow_unknown:false,fields:#{
            run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Target run."},
            gate:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Exact gate ID."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Gate revision shown to the user."},
            action:#{schema:#{type:"string",allowed:["approve","reject","approve_edited","request_changes"]},required:true,sensitive:false,
                doc:"A file change request does not authorize arbitrary new content."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Edited input or requested changes; empty for approve or reject."},
        }},doc:"User proposal, not a result or execution directive."},
    },slots:#{},parts:["root","summary","bar","title","task","status","stats","stat","stat_label","stat_value","stack","peek","peek_far",
        "front","body","head","avatar","agent","kind","pager","page","question","reason","section","section_title","code","diff","files",
        "context","extra","more","actions","approve","reject","edit","editor","holds","hold","history","receipt","receipt_body","date",
        "note","empty","icon"],effects:["restore_edit"]},
    render:Fn("render_ApprovalGates")
});

fn ApprovalGates(props){render_component("patterns/approval_gates",props)}
fn token(run,g){
    let t=review::target(#{request:g.id,revision:g.revision});
    #{run:run,gate:t.request,revision:t.revision}
}
fn act(t,action,text){#{run:t.run,gate:t.gate,revision:t.revision,action:action,text:text}}
fn send(p,ctx,event){review::send("decide",p,ctx,event)}
fn page(id,run,ctx,event){
    ctx.set_state("page",#{id:id,run:run});ctx.set_state("draft",());ctx.set_state("more",());handled()
}
fn disclose(field,target,ctx,event){
    ctx.set_state(field,if ctx.get_state(field)==target{()}else{target});handled()
}
fn edit(t,value,ctx,event){ctx.set_state("draft",#{target:t,value:value});handled()}
fn change(t,ctx,value){
    let d=ctx.get_state("draft");
    if d!=()&&d.target==t&&utf8_byte_count(value)<=64000{ctx.set_state("draft",#{target:t,value:value});}
}
fn cancel(t,ctx,event){ctx.set_state("draft",());ctx.set_state("focus_edit",t);handled()}
fn restore_edit(ctx,t){
    if t!=(){ctx.set_state("focus_edit",());ctx.focus("edit");}
}
fn cleanup_focus(ctx,t){}
fn edited(t,action,ctx,value){
    let clean=value;clean.trim();
    if clean!=""&&utf8_byte_count(value)<=64000{ctx.emit("decide",act(t,action,value));}
    handled()
}
fn save_edit(t,action,value,ctx,event){edited(t,action,ctx,value)}
fn glyph(ctx,name){
    let path=if name=="prev"{"m12 5-5 5 5 5"}else if name=="next"{"m8 5 5 5-5 5"}
        else{"M4 8h12v8H4zm2-3h8M8 2h4"};
    svg(`<svg viewBox='0 0 20 20' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("icon",style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4))))
}
fn kind(ctx,v){
    if v=="file"{ctx.t_or("gates.file","File change")}else if v=="agent"{ctx.t_or("gates.agent","Sub-agent")}
    else if v=="message"{ctx.t_or("gates.message","Message")}else{ctx.t_or("gates.tool","Tool call")}
}
fn outcome(ctx,v){
    if v=="edited"{ctx.t_or("gates.edited","Approved with changes")}
    else if v=="rejected"{ctx.t_or("gates.rejected","Rejected")}
    else if v=="changes_requested"{ctx.t_or("gates.requested","Changes requested")}
    else{ctx.t_or("review.approved","Approved")}
}
fn heading(ctx,title,part){
    text(title).accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style(part,style().min_width(px(0)).typography("body").font_weight(500)))
}
fn stat(ctx,label,value){
    column([review::heading_note(ctx,label,"stat_label"),text(value).with_style(ctx.component_style("stat_value",style()
        .min_width(px(0)).typography("body").font_weight(500)))])
        .with_style(ctx.component_style("stat",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(1.5)).gap(theme_length("space.spacing_unit").times(0.5)).flex_grow()))
}
fn summary(ctx,props,open,history,total){
    let rejected=false;for g in history{if g.status=="rejected"{rejected=true;break;}}
    let phase=review::value_or(props,"run_state",if rejected{"stopped"}else if open>0{"waiting"}else if props.live{"running"}else{"done"});
    let words=if phase=="stopped"{ctx.t_or("gates.stopped","Stopped")}else if phase=="waiting"{ctx.t_or("gates.waiting","Waiting on you")}
        else if phase=="running"{ctx.t_or("gates.running","Running")}else{ctx.t_or("gates.done","Done")};
    let tag=if phase=="stopped"{["status","note"]}else{["status"]};
    let nodes=[glyph(ctx,"stack"),heading(ctx,review::value_or(props,"title",ctx.t_or("gates.title","Approvals")),"title")];
    if review::opt(props,"task")!=(){nodes.push(review::heading_note(ctx,props.task,"task").with_style(style().min_width(px(0)).flex_grow()));}
    nodes.push(text(words).with_key("phase").accessibility_role("status")
        .with_style(ctx.component_style(tag,style().min_width(px(0)).typography("caption").padding_x(theme_length("space.spacing_unit").times(1.5)).padding_y(theme_length("space.spacing_unit").times(0.5))
            .radius(px(5)).text_color(theme_color(if phase=="stopped"{"text.danger"}else if phase=="waiting"{"text.warning"}else{"text.success"})))));
    let bar=row(nodes).with_key("bar").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("bar",style()
        .min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(1.75)).padding(theme_length("space.spacing_unit").times(1.75)).flex_wrap()));
    let stats=row([stat(ctx,ctx.t_or("gates.pending","Pending gates"),ctx.format_number(open)),
        stat(ctx,ctx.t_or("gates.decided","Decided"),ctx.format_number(history.len)+"/"+ctx.format_number(total)),
        stat(ctx,ctx.t_or("gates.run","Run"),words)]).with_key("stats").with_style(ctx.component_style("stats",style()
            .min_width(px(0)).gap(theme_length("space.spacing_unit")).flex_wrap().background(theme_color("surface_raised")).radius(theme_length("space.unit").times(2.25))));
    column([bar,stats]).with_key("summary").with_style(ctx.component_style("summary",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(0.75))
        .padding(theme_length("space.spacing_unit")).radius(theme_length("space.unit").times(3.5)).background(theme_color("surface_hover"))))
}
fn section(ctx,key,title,body){
    column([heading(ctx,title,"section_title"),body]).with_key(key).with_style(ctx.component_style("section",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25))))
}
fn evidence(ctx,g,full){
    let files=review::value_or(g,"diff",[]);
    if files.len>0{
        let entries=[];let count=if full{files.len}else{1};
        for i in 0..count{
            let f=files[i];
            entries.push(column([review::heading_note(ctx,f.name,"files"),
                diff::DiffViewer(#{key:"diff-"+g.id+"-"+g.revision.to_string()+"-"+i.to_string(),
                    left:#{source:f.before,label:ctx.t_or("gates.before","Before"),file_name:f.name},
                    right:#{source:f.after,label:ctx.t_or("gates.proposed","Proposed"),file_name:f.name},
                    mode:"unified",wrap:"viewport"}).with_style(ctx.component_style("diff",style().height(theme_length("space.unit").times(38.75)).min_width(px(0))))])
                    .with_key("file-"+i.to_string()).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit"))));
        }
        return section(ctx,"evidence",ctx.t_or("gates.changes","Changes"),column(entries).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2))));
    }
    let content=review::opt(g,"input");
    if content==()||content==""{return ();}
    let viewer=code::CodeViewer(#{key:"code-"+g.id+"-"+g.revision.to_string(),source:content,
        label:kind(ctx,g.kind),language:if g.kind=="tool"{"json"}else{()},height:"content",max_height:if full{270}else{130},wrap:"viewport"})
        .with_style(ctx.component_style("code",style().min_width(px(0))));
    section(ctx,"evidence",if g.kind=="tool"{ctx.t_or("gates.arguments","Arguments")}else{kind(ctx,g.kind)},viewer)
}
fn extras(ctx,g){
    let result=[];
    for pair in [["context","Before this","gates.before_steps"],["files","Resources touched","gates.resources"]]{
        let values=review::value_or(g,pair[0],[]);
        if values.len==0{continue;}
        let rows=[];
        for i in 0..values.len{rows.push(review::heading_note(ctx,(i+1).to_string()+". "+values[i],pair[0]).with_key(i.to_string()));}
        result.push(section(ctx,pair[0],ctx.t_or(pair[2],pair[1]),column(rows).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(0.5)))));
    }
    result
}
fn control(key,title,variant,enabled,on_click){
    button::Button(#{key:key,text:title,variant:variant,size:"sm",disabled:!enabled,on_click:on_click}).with_key(key)
}
fn pager(ctx,open,index,run){
    let count=open.len;
    if count<2{return ();}
    let prev=(index+count-1)%count;let next=(index+1)%count;
    row([
        icon_button::IconButton(#{key:"prev",label:ctx.t_or("gates.previous","Previous gate"),icon:glyph(ctx,if ctx.text_direction()=="rtl"{"next"}else{"prev"}),
            size:"xs",on_click:Fn("page").curry(open[prev].id,run)}).with_style(ctx.component_style("page",style())),
        review::heading_note(ctx,ctx.format_number(index+1)+" / "+ctx.format_number(count),"date"),
        icon_button::IconButton(#{key:"next",label:ctx.t_or("gates.next","Next gate"),icon:glyph(ctx,if ctx.text_direction()=="rtl"{"prev"}else{"next"}),
            size:"xs",on_click:Fn("page").curry(open[next].id,run)}).with_style(ctx.component_style("page",style())),
    ]).with_key("pager").on_capture("key:plain+left",Fn("page").curry(open[prev].id,run))
        .on_capture("key:plain+right",Fn("page").curry(open[next].id,run))
        .with_style(ctx.component_style("pager",style().gap(theme_length("space.spacing_unit").times(0.75)).items_center()))
}
fn front_card(ctx,props,open,index){
    let g=open[index];let t=token(props.run,g);let stored=ctx.get_state("draft");
    let editing=stored!=()&&stored.target==t;let full=ctx.get_state("more")==t;
    let ready=!review::value_or(g,"busy",false)&&review::opt(props,"on_decide")!=();
    let can_edit=review::value_or(g,"editable",false);
    let avatar=row([text(text_initial(g.agent))]).accessibility_hidden(true)
        .with_style(ctx.component_style("avatar",style().width(theme_length("space.unit").times(7.5)).height(theme_length("space.unit").times(7.5)).radius(px(7)).flex_shrink(false)
            .items_center().justify_center().background(theme_color("surface_hover")).typography("caption").font_weight(500)));
    let title=[avatar,text(g.agent).with_style(ctx.component_style("agent",style().min_width(px(0)).flex_grow().typography("body"))),
        review::heading_note(ctx,kind(ctx,g.kind),"kind")];
    let paging=pager(ctx,open,index,props.run);if paging!=(){title.push(paging);}
    let header=row(title).with_key("head").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("head",style()
        .min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap()));
    let body=[header,heading(ctx,g.action,"question"),review::heading_note(ctx,g.reason,"reason")];let actions=[];
    let p=review::opt(g,"note");if p!=(){body.push(review::heading_note(ctx,p,"note").accessibility_role("status").with_key("feedback"));}
    if editing{
        let action=if g.kind=="file"{"request_changes"}else{"approve_edited"};
        let label=if g.kind=="file"{ctx.t_or("gates.change_note","Requested changes")}else{ctx.t_or("gates.edit_input","Edit request")};
        let field=textarea::Textarea(#{key:"edit-"+g.id+"-"+g.revision.to_string(),label:label,value:stored.value,autofocus:true,
            read_only:!ready,min_rows:2,max_rows:7,submit_key:if ready{"mod_enter"}else{()},
            on_change:if ready{Fn("change").curry(t)}else{()},on_submit:if ready{Fn("edited").curry(t,action)}else{()}})
            .with_style(ctx.component_style("editor",style().min_width(px(0))));
        body.push(field);
        actions.push(control("cancel",ctx.t_or("common.cancel","Cancel"),"ghost",ready,Fn("cancel").curry(t)));
        let clean=stored.value;clean.trim();
        actions.push(control("save",if g.kind=="file"{ctx.t_or("gates.ask_changes","Request changes")}else{ctx.t_or("gates.edited","Approve with changes")},
            "primary",ready&&clean!=""&&utf8_byte_count(stored.value)<=64000,Fn("save_edit").curry(t,action,stored.value)));
    }else{
        let view=evidence(ctx,g,full);if view!=(){body.push(view);}
        if can_edit{
            actions.push(control("edit",if g.kind=="file"{ctx.t_or("gates.ask_changes","Request changes")}else{ctx.t_or("review.edit","Edit")},
                "ghost",ready,Fn("edit").curry(t,if g.kind=="file"{""}else{review::value_or(g,"input","")}))
                .with_key("edit").with_ref(element_ref("edit")).with_style(ctx.component_style("edit",style())));
        }
        actions.push(control("reject",ctx.t_or("gates.reject","Reject"),"secondary",ready,Fn("send").curry(act(t,"reject",""))).with_style(ctx.component_style("reject",style())));
        actions.push(button::Button(#{key:"approve",text:ctx.t_or("review.approve","Approve"),size:"sm",disabled:!ready,
            loading:review::value_or(g,"busy",false),loading_text:ctx.t_or("review.processing","Processing…"),
            shortcut:review::confirm_chord(),on_click:Fn("send").curry(act(t,"approve",""))}).with_style(ctx.component_style("approve",style())));
    }
    let facts=extras(ctx,g);
    if facts.len>0||review::value_or(g,"diff",[]).len>1{
        let label=if full{ctx.t_or("gates.less","Less context")}else{ctx.t_or("gates.full","Full context")};
        body.push(control("more",label,"ghost",true,Fn("disclose").curry("more",t)).accessibility_expanded(full)
            .with_style(ctx.component_style("more",style().self_start())));
        if full&&facts.len>0{body.push(column(facts).with_key("extras").with_style(ctx.component_style("extra",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)))));}
    }
    let root=column([
        column(body).with_key("body").audit_allow(["text-edge-misaligned"]).with_style(ctx.component_style("body",style()
            .min_width(px(0)).padding(theme_length("space.spacing_unit").times(2.5)).gap(theme_length("space.spacing_unit").times(1.5)).radius(theme_length("space.unit").times(2.75)).background(theme_color("surface_raised")))),
        row(actions).with_key("actions").with_style(ctx.component_style("actions",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).padding(theme_length("space.spacing_unit"))
            .justify_end().flex_wrap())),
    ]).with_key(g.id).accessibility_role("group").accessibility_label(g.action).audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("front",style().min_width(px(0)).padding(theme_length("space.spacing_unit")).gap(theme_length("space.spacing_unit")).radius(theme_length("space.unit").times(3.5)).background(theme_color("surface_hover"))));
    effect("restore_edit",if !editing&&ready&&can_edit&&ctx.get_state("focus_edit")==t{t}else{()},Fn("restore_edit"),Fn("cleanup_focus"));
    if editing&&ready {root=root.on_bubble("key:plain+escape",Fn("cancel").curry(t));}
    else if ready{
        root=root.on_capture("key:cmd+enter",Fn("send").curry(act(t,"approve","")))
            .on_capture("key:ctrl+enter",Fn("send").curry(act(t,"approve","")));
    }
    root
}
fn stack_view(ctx,props,open){
    if open.len==0{return row([review::heading_note(ctx,ctx.t_or("gates.clear","No approvals waiting"),"empty")]).with_key("empty")
        .with_style(style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(3)));}
    let index=0;let p=ctx.get_state("page");
    if p!=()&&p.run==props.run{for i in 0..open.len{if open[i].id==p.id{index=i;break;}}}
    let peeks=[];
    for i in 1..min(open.len,3){
        let rule=ctx.component_style(if i==2{["peek","peek_far"]}else{["peek"]},style().absolute().top(px(8-i*4))
            .inset_start(px(i*8)).inset_end(px(i*8)).height(theme_length("space.unit").times(10)).radius(theme_length("space.unit").times(3.5)).background(theme_color("surface_hover")));
        peeks.insert(0,row([]).with_key("peek-"+open[(index+i)%open.len].id).accessibility_hidden(true).with_style(rule));
    }
        let inset=if open.len>1{theme_length("space.spacing_unit").times(2.75)}else{px(0)};
        peeks.push(front_card(ctx,props,open,index).with_style(style().margin_top(inset)));
    stack(peeks).with_key("stack").with_style(ctx.component_style("stack",style().min_width(px(0)).relative()))
}
fn history_view(ctx,props,history){
    let opened=ctx.get_state("history");let rows=[heading(ctx,ctx.t_or("gates.history","Decided gates"),"section_title")];
    for g in history{
        let t=token(props.run,g);let visible=opened==t;
        let label=outcome(ctx,g.status);
        let mark=review::glyph(ctx,if g.status=="rejected"{"close"}else if g.status=="changes_requested"{"warning"}else{"check"},14);
        let line=row([mark,text(g.action).with_style(style().typography("body").font_weight(500).min_width(px(0)).flex_grow()),
            review::heading_note(ctx,label,"date"),review::glyph(ctx,if visible{"up"}else{"down"},13)])
            .with_key("toggle").accessibility_role("button").accessibility_label(g.action).accessibility_description(label)
            .accessibility_expanded(visible).on_click(Fn("disclose").curry("history",t)).audit_allow(["mixed-type-in-row"])
            .with_style(ctx.component_style("receipt",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(1.5)).gap(theme_length("space.spacing_unit").times(1.25)).items_center().flex_wrap()
                .focus_ring(px(2),offset_px(-2),theme_color("focus_ring")).hover(style().background(theme_color("surface_hover")))));
        let opened_rows=[line];
        if visible{
            let body=[review::heading_note(ctx,g.reason,"reason")];
            if review::opt(g,"at")!=(){body.push(review::heading_note(ctx,g.at,"date"));}
            if review::opt(g,"note")!=(){body.push(review::heading_note(ctx,g.note,"note"));}
            let data=evidence(ctx,g,true);if data!=(){body.push(data);}
            for item in extras(ctx,g){body.push(item);}
            opened_rows.push(column(body).with_key("receipt-body").with_style(ctx.component_style("receipt_body",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).padding(theme_length("space.spacing_unit").times(2)))));
        }
        rows.push(column(opened_rows).with_key(g.id).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(0.5))));
    }
    column(rows).with_key("history").audit_allow(["text-edge-misaligned"]).with_style(ctx.component_style("history",style().min_width(px(0))
        .padding(theme_length("space.spacing_unit").times(2.25)).gap(theme_length("space.spacing_unit").times(1.25)).radius(theme_length("space.unit").times(3)).background(theme_color("surface_raised"))))
}
fn render_ApprovalGates(ctx,props){
    let run=review::target(#{request:props.run,revision:0}).request;
    let open=[];let history=[];let waiting=[];let seen=#{};let bytes=0;
    for g in props.gates{
        let id=token(run,g).gate;if id in seen{throw "Approval gates must have unique IDs";}
        seen[id]=true;bytes+=utf8_byte_count(g.action)+utf8_byte_count(g.reason)+utf8_byte_count(review::value_or(g,"input",""));
        for f in review::value_or(g,"diff",[]){bytes+=utf8_byte_count(f.before)+utf8_byte_count(f.after);}
        if bytes>524288{throw "Approval gate evidence exceeds 512 KiB";}
        if g.status=="open"{
            open.push(g);
            for held in review::value_or(g,"holds",[]){if !(held in waiting){waiting.push(held);}}
        }else{history.insert(0,g);}
    }
    let parts=[summary(ctx,props,open.len,history,props.gates.len),stack_view(ctx,props,open)];
    if waiting.len>0{
        let items=[heading(ctx,ctx.t_or("gates.holds","Waiting on approval"),"section_title")];
        for item in waiting{items.push(review::heading_note(ctx,item,"hold"));}
        parts.push(column(items).with_key("holds").with_style(ctx.component_style("holds",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2)).gap(theme_length("space.spacing_unit").times(0.75)))));
    }
    if history.len>0{parts.push(history_view(ctx,props,history));}
    column(parts).with_key(props.key).accessibility_role("region").accessibility_label(ctx.t_or("gates.region","Approval gates"))
        .audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).max_width(px(640)).width(relative(1.0)).gap(theme_length("space.spacing_unit").times(2.25))))
}
