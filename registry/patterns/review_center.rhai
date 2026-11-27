/* gpui-rhai
{"id":"patterns/review_center","export":"ReviewCenter","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/diff_viewer","components/icon_button","components/input","components/progress","components/scroll_area","components/textarea","patterns/approval_card","patterns/decision_card","patterns/decision_tree"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","radius.lg","selection","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted","text_primary","typography.body","typography.caption","space.spacing_unit","space.unit","typography.review_title"],"environment":["density"]}
*/
// Prioritized review queue with versioned proposals and independently inspectable evidence.
// State: filter, selection, one edit session, file and reason disclosure. Accepted decisions and receipts are Host-owned.
// Example: ReviewCenter(#{key:"review",now_ms:1000,items:[]})
import "components/button" as button;
import "components/diff_viewer" as document;
import "components/icon_button" as icon_button;
import "components/input" as input;
import "components/progress" as progress;
import "components/scroll_area" as scroll;
import "components/textarea" as textarea;
import "patterns/approval_card" as review;
import "patterns/decision_card" as cards;
import "patterns/decision_tree" as evidence;

define_component(#{
    metadata:#{id:"patterns/review_center","export":"ReviewCenter",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/diff_viewer","components/icon_button","components/input","components/progress","components/scroll_area",
            "components/textarea","patterns/approval_card","patterns/decision_card","patterns/decision_tree"],
        capabilities:#{},tokens:["accent","border","focus_ring","radius.lg","selection","surface_hover","surface_raised","text.danger","text.success","text.warning",
            "text_muted","text_primary","typography.body","typography.caption","space.spacing_unit","space.unit","typography.review_title"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable instance key."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Queue heading."},
        items:#{schema:#{type:"array",max_items:48,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable request ID."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Increase when business evidence changes."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Request name in both panels."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Who proposed this request."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional model name."},
            kind:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Task type caption such as Refund, Code change or Plan."},
            risk:#{schema:#{type:"string",allowed:["low","medium","high"]},required:true,sensitive:false,doc:"Risk category; higher risks sort first."},
            confidence:#{schema:#{type:"number",min:0.0,max:1.0},required:true,sensitive:false,doc:"Agent confidence, from 0 to 1."},
            created_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Wait start on the Host clock; breaks ties by oldest first."},
            reasoning:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Explanation for the proposal."},
            body_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional caption above proposal text."},
            body:#{schema:#{type:"ui_value"},required:false,sensitive:true,doc:"Proposal text or a list of plain and #{text,note,source?} evidence spans. Text edits propose a plain string."},
            fields:#{schema:#{type:"optional",value:#{type:"array",max_items:12,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique editable field identity, independent of its label."},
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible field caption."},
                value:#{schema:#{type:"ui_value"},required:true,sensitive:true,doc:"Plain text or flagged evidence spans, like body."},
            }}}},required:false,sensitive:true,doc:"Facts supplied by the Host; only changed values are included in approval proposals."},
            files:#{schema:#{type:"optional",value:#{type:"array",max_items:12,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique file ID."},
                name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Display path and syntax hint; never read from disk."},
                before:#{schema:#{type:"one_of",variants:[#{type:"string"},#{type:"document"}]},required:true,sensitive:true,doc:"Before text or Host-owned native document."},
                after:#{schema:#{type:"one_of",variants:[#{type:"string"},#{type:"document"}]},required:true,sensitive:true,doc:"Proposed text or Host-owned native document."},
                added:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Optional added line count supplied by the Host."},
                removed:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Optional removed line count supplied by the Host."},
            }}}},required:false,sensitive:true,doc:"File comparisons are read-only and use DiffViewer; changes are never applied by this component."},
            steps:#{schema:#{type:"optional",value:#{type:"array",max_items:20,items:#{type:"object",allow_unknown:false,fields:#{
                title:#{schema:#{type:"ui_value"},required:true,sensitive:true,doc:"Plain or flagged step title."},
                tool:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Tool caption; never invoked by the viewer."},
                detail:#{schema:#{type:"ui_value"},required:false,sensitive:true,doc:"Optional plain or flagged step explanation."},
            }}}},required:false,sensitive:true,doc:"Ordered plan evidence."},
            sources:#{schema:#{type:"optional",value:#{type:"array",max_items:12,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique source identity."},
                label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Visible source name."},
            }}}},required:false,sensitive:true,doc:"Evidence labels; optional source activation is a Host proposal, not external navigation."},
            busy:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Host is considering a proposal; decision actions are disabled for this request."},
            notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Non-final Host feedback. A refused proposal stays in this queue."},
        }}},required:true,sensitive:true,doc:"Pending requests. The Host removes an accepted decision."},
        now_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Host clock for wait time, SLA and undo expiry; no component-owned wall timer."},
        sla_ms:#{schema:#{type:"integer",min:1},"default":#{type:"integer",value:1800000},required:false,sensitive:false,doc:"Wait limit; 75% is near deadline and 100% is overdue."},
        low_confidence:#{schema:#{type:"number",min:0.0,max:1.0},"default":#{type:"float",value:0.7},required:false,sensitive:false,doc:"Threshold for the Unsure filter."},
        order:#{schema:#{type:"string",allowed:["risk","oldest","given"]},"default":#{type:"string",value:"risk"},
            required:false,sensitive:false,doc:"Queue ordering: risk then oldest, oldest regardless of risk, or caller order."},
        filter:#{schema:#{type:"optional",value:#{type:"string",allowed:["all","high","unsure"]}},required:false,sensitive:false,
            doc:"Controlled filter. The queue changes only when this value changes."},
        default_filter:#{schema:#{type:"string",allowed:["all","high","unsure"]},"default":#{type:"string",value:"all"},
            required:false,sensitive:false,doc:"Initial filter when filter is not controlled."},
        on_filter_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Optional filter selection proposal."},
        reviewed:#{schema:#{type:"integer",min:0,max:9000000000000000},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted review count."},
        total:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Optional total for progress; at least reviewed plus pending."},
        reasons:#{schema:#{type:"optional",value:#{type:"array",max_items:8,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable reject reason ID."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Caption shown to the reviewer."},
        }}}},required:false,sensitive:false,doc:"Omit for preset reasons. An empty list enables rejecting without a reason."},
        receipt:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted receipt identity, never the request title."},
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Original request ID."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted decision revision."},
            outcome:#{schema:#{type:"string",allowed:["approved","rejected"]},required:true,sensitive:false,doc:"Outcome the Host accepted."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Request caption in the receipt."},
            expires_ms:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Undo no longer offered when now_ms reaches this time."},
        }}},required:false,sensitive:true,doc:"Most recent accepted decision that the Host still permits undoing."},
        on_decide:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Versioned approval or rejection proposal."},
        on_skip:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Optional notification of a presentation-only skip."},
        on_undo:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Proposed undo of the exact accepted receipt, never optimistic restoration."},
        on_source:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Activation of a source identity for the current revision."},
    },state:#{fields:#{
        filter:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        active:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        file:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        editor:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
        reason:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        focus_plan:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        decide:#{payload:#{type:"object",allow_unknown:false,fields:#{
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Request identity."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision displayed on the selected item."},
            action:#{schema:#{type:"string",allowed:["approve","reject"]},required:true,sensitive:false,doc:"Proposed action."},
            reason:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Reason ID on rejection, or empty if no reason is required."},
            body:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:true,doc:"Changed body on edited approval, otherwise null. Empty string is a valid edit."},
            fields:#{schema:#{type:"map",values:#{type:"string"}},required:true,sensitive:true,doc:"Edited approval values indexed by field ID, never by caption."},
        }},doc:"Approval and rejection are proposals; only the Host changes the queue."},
        skip:#{payload:#{type:"object",allow_unknown:false,fields:#{
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Skipped request."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision displayed."},
        }},doc:"Skip moves selection without resolving or removing the request."},
        undo:#{payload:#{type:"object",allow_unknown:false,fields:#{
            receipt:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Exact accepted receipt ID."},
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Original request."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"The accepted revision."},
        }},doc:"The Host validates expiry and decides whether to restore the request."},
        source:#{payload:#{type:"object",allow_unknown:false,fields:#{
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Request whose source was activated."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision displayed."},
            source:#{schema:#{type:"string"},required:true,sensitive:false,doc:"The exact declared source ID."},
        }},doc:"The Host resolves an allowed source target, if any."},
        filter_change:#{payload:#{type:"string",allowed:["all","high","unsure"]},doc:"Filter chosen by the user."},
    },slots:#{},parts:["root","head","title","progress","count","oldest","receipt","card","queue","filters","filter_active","list","row","row_active","row_title",
        "meta","avatar","risk_dot","detail_pane","detail_scroll","detail","item_title","why","section","section_title","field","field_label","field_value","body","editor",
        "files","file","file_active","diff","step","step_no","steps","tool","sources","source_button","actions","pager","empty","notice","reason",
        "effect","flag","flag_label","flag_tip","source","risk","risk_low","risk_medium","risk_high","icon"],effects:["present_focus"]},
    render:Fn("render_ReviewCenter")
});

fn ReviewCenter(props){render_component("patterns/review_center",props)}
fn target(item){#{request:item.id,revision:item.revision}}
fn scoped(ctx,key,t){let s=ctx.get_state(key);if s!=()&&s.target==t{s.value}else{()}}
fn keep(ctx,key,t,value){ctx.set_state(key,#{target:t,value:value});}
fn id_check(records){
    let seen=#{};for item in records{
        if item.id==""||utf8_byte_count(item.id)>128||item.id in seen{throw "Review IDs must be nonempty, bounded and unique within their scope";}
        seen[item.id]=true;
    }
}
fn priority(risk){if risk=="high"{0}else if risk=="medium"{1}else{2}}
fn rank(order,a,b){
    if order=="risk"{
        let risk=priority(a.risk)-priority(b.risk);
        if risk!=0{return risk;}
    }
    if a.created_ms!=b.created_ms{return if a.created_ms<b.created_ms{-1}else{1};}
    a.index-b.index
}
fn project(items,filter,threshold,order){
    id_check(items);let data=[];let counts=#{all:items.len,high:0,unsure:0};let oldest=();
    for i in 0..items.len{
        let item=items[i];item.index=i;let high=item.risk=="high";let unsure=item.confidence<threshold;
        if high{counts.high+=1;}
        if unsure{counts.unsure+=1;}
        if oldest==()||item.created_ms<oldest{oldest=item.created_ms;}
        if filter=="all"||filter=="high"&&high||filter=="unsure"&&unsure{data.push(item);}
    }
    if order!="given"{data.sort(Fn("rank").curry(order));}
    #{items:data,counts:counts,oldest:oldest}
}
fn selection(ctx,items){
    let active=ctx.get_state("active");if active==()||items.len==0{return 0;}
    for i in 0..items.len{if active.id==items[i].id{return i;}}
    min(active.index,items.len-1)
}
fn select_outcome(t,index,focus,ctx,p){
    ctx.set_state("active",#{id:t.request,index:index});
    if focus!=""{ctx.focus(focus);}
    handled()
}
fn filter_change(id,controlled,notify,ctx,p){
    if !controlled{ctx.set_state("filter",id);}
    if notify{ctx.emit("filter_change",id);}
    handled()
}
fn row_ref(i){"queue-"+i.to_string()}
fn change_file(t,file,ctx,p){keep(ctx,"file",t,file);handled()}
fn edit(t,original,ctx,p){
    let current=scoped(ctx,"editor",t);if current==(){keep(ctx,"editor",t,#{body:original.body,fields:original.fields});}
    ctx.set_state("reason",());keep(ctx,"focus_plan",t,original.focus);handled()
}
fn edit_value(t,id,ctx,value){
    if utf8_byte_count(value)>16000{return;}
    let draft=scoped(ctx,"editor",t);if draft==(){return;}
    if id==""{draft.body=value;}
    else if id in draft.fields{draft.fields[id]=value;}
    else{return;}
    keep(ctx,"editor",t,draft);
}
fn edit_context(ctx,plan){
    if plan.focus!=""&&scoped(ctx,"focus_plan",plan.target)==plan.focus{
        ctx.set_state("focus_plan",());ctx.focus(plan.focus);
    }
}
fn no_effect(ctx,plan){}
fn cancel(t,kind,ctx,p){
    if kind=="reason"&&scoped(ctx,"reason",t)==true{ctx.set_state("reason",());}
    else if kind=="edit"&&scoped(ctx,"editor",t)!=(){ctx.set_state("editor",());}
    else{return;}
    keep(ctx,"focus_plan",t,if kind=="reason"{"reject"}else{"edit"});handled()
}
fn show_reasons(t,ctx,p){keep(ctx,"reason",t,true);keep(ctx,"focus_plan",t,"reason-first");handled()}
fn decide(t,action,reason,original,ctx,p){
    let body=();let fields=#{};
    if action=="approve"&&original!=(){
        let draft=scoped(ctx,"editor",t);if draft==(){return;}
        if draft.body!=original.body{body=draft.body;}
        for id in original.fields.keys(){if draft.fields[id]!=original.fields[id]{fields[id]=draft.fields[id];}}
    }
    ctx.emit("decide",#{request:t.request,revision:t.revision,action:action,reason:reason,body:body,fields:fields});handled()
}
fn skip(t,next,index,notify,ctx,p){
    ctx.set_state("reason",());
    if notify{ctx.emit("skip",t);}
    select_outcome(next,index,"",ctx,p)
}
fn send(name,payload,ctx,p){ctx.emit(name,payload);handled()}
fn format_age(ctx,age){
    let n=age/60000;
    if n<1{ctx.t_or("review_center.now","Just now")}
    else if n<60{ctx.t_or("review_center.minutes","{n} min",#{n:ctx.format_number(n)})}
    else{ctx.t_or("review_center.hours","{n} h",#{n:ctx.format_number(n/60)})}
}
fn age_ink(age,sla){
    if age>=sla{theme_color("text.danger")}else if age.to_float()>=sla.to_float()*0.75{theme_color("text.warning")}else{theme_color("text_muted")}
}
fn label(ctx,value,part){review::heading_note(ctx,value,part).with_style(style().min_width(px(0)))}
fn risk_badge(ctx,item){review::risk_badge(ctx,item.risk,review::risk_label(ctx,item.risk))}
fn avatar(ctx,agent){
    let initial=if agent==""{"AI"}else{agent.sub_string(0,1).to_upper()};
    row([text(initial)]).with_key("avatar").accessibility_hidden(true).with_style(ctx.component_style("avatar",style().width(theme_length("space.unit").times(6.75)).height(theme_length("space.unit").times(6.75))
        .radius(px(7)).background(theme_color("surface_hover")).justify_center().items_center().flex_shrink(false).typography("caption")))
}
fn head(ctx,props,data){
    let title=review::value_or(props,"title",ctx.t_or("review_center.title","Review center"));
    let total=max(props.reviewed+props.items.len,review::value_or(props,"total",0));
    let count=ctx.t_or("review_center.count","{done} of {total} reviewed",#{done:ctx.format_number(props.reviewed),total:ctx.format_number(total)});
    let content=[text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
        .with_style(ctx.component_style("title",style().min_width(px(0)).flex_grow().typography("body").font_weight(600))),
        row([label(ctx,count,"count").with_key("count"),progress::Progress(#{key:"rollup",label:count,value:props.reviewed,max:max(1,total),width:65})])
            .with_key("progress").audit_allow(["mixed-type-in-row"])
            .with_style(ctx.component_style("progress",style().gap(theme_length("space.spacing_unit").times(2)).items_center().flex_wrap()))];
    if data.oldest!=(){
        let age=max(0,props.now_ms-data.oldest);
        content.push(label(ctx,ctx.t_or("review_center.oldest","Oldest: {age}",#{age:format_age(ctx,age)}),"oldest")
            .with_key("oldest").with_style(style().text_color(age_ink(age,props.sla_ms))));
    }
    row(content).with_key("head").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("head",style().items_center().gap(theme_length("space.spacing_unit").times(3)).flex_wrap().min_width(px(0))))
}
fn receipt(ctx,props){
    let r=review::opt(props,"receipt");if r==()||r.expires_ms<=props.now_ms{return ();}
    let text=ctx.t_or(if r.outcome=="approved"{"review_center.approved"}else{"review_center.rejected"},
        if r.outcome=="approved"{"Approved: {title}"}else{"Rejected: {title}"},#{title:r.title});
    let parts=[label(ctx,text,"notice").with_key("accepted").accessibility_role("status").with_style(style().flex_grow())];
    if review::opt(props,"on_undo")!=(){
        parts.push(button::Button(#{key:"undo",text:ctx.t_or("review_center.undo","Undo"),size:"sm",variant:"secondary",
            on_click:Fn("send").curry("undo",#{receipt:r.id,request:r.request,revision:r.revision})}));
    }
    row(parts).with_key("receipt").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("receipt",style().gap(theme_length("space.spacing_unit").times(2)).items_center().flex_wrap().padding(theme_length("space.spacing_unit").times(2)).radius(theme_length("space.unit").times(2)).background(theme_color("surface_hover"))))
}
fn queued(ctx,props,items,i,current){
    let item=items[i];let age=max(0,props.now_ms-item.created_ms);let name=ctx.t_or("review_center.queue_item","Review {title}",#{title:item.title});
    let tag=review::risk_label(ctx,item.risk);
    let ink=theme_color(if item.risk=="high"{"text.danger"}else if item.risk=="medium"{"text.warning"}else{"text_muted"});
    let line=[text(item.title).with_key("title").with_style(ctx.component_style("row_title",style().typography("body").font_weight(500)
        .whitespace_nowrap().text_ellipsis().min_width(px(0)).flex_grow())),
        row([]).with_key("risk").accessibility_hidden(true).with_style(ctx.component_style("risk_dot",style().width(px(5)).height(px(5))
            .radius(px(4)).background(ink).flex_shrink(false))),
        label(ctx,format_age(ctx,age),"meta").with_key("age").with_style(style().text_color(age_ink(age,props.sla_ms)))];
    let meta=item.kind+" · "+item.agent;
    let row=row([avatar(ctx,item.agent),column([row(line).with_key("name").audit_allow(["mixed-type-in-row"]).with_style(style().items_center().gap(theme_length("space.spacing_unit").times(1.25)).min_width(px(0))),
        label(ctx,meta,"meta").with_key("meta").with_style(style().text_ellipsis().whitespace_nowrap())])
            .with_key("label").with_style(style().gap(theme_length("space.spacing_unit").times(0.5)).flex_grow().min_width(px(0)))])
        .with_key("item-"+item.id).with_ref(element_ref(row_ref(i))).accessibility_role("button").accessibility_label(name)
        .accessibility_description(tag+" · "+meta).tab_stop(true).audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style(if current{["row","row_active"]}else{["row"]},
            style().min_width(px(0)).min_height(theme_length("space.unit").times(12.25)).items_center().gap(theme_length("space.spacing_unit").times(2)).padding(theme_length("space.spacing_unit").times(1.75)).radius(theme_length("space.unit").times(2.25))
                .background(theme_color(if current{"surface_raised"}else{"surface_hover"}))
                .hover(style().background(theme_color("selection"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .on_click(Fn("select_outcome").curry(target(item),i,""));
    if current{row=row.accessibility_current("true");}
    let above=max(0,i-1);let below=min(items.len-1,i+1);
    row.on_key_value("plain+up",Fn("select_outcome").curry(target(items[above]),above,row_ref(above)),())
        .on_key_value("plain+down",Fn("select_outcome").curry(target(items[below]),below,row_ref(below)),())
}
fn queue(ctx,props,data,index,filter,compact){
    let choices=[["all",ctx.t_or("review_center.all","All")],["high",ctx.t_or("review_center.high","High risk")],["unsure",ctx.t_or("review_center.unsure","Unsure")]];
    let controlled=review::opt(props,"filter")!=();let listen=review::opt(props,"on_filter_change")!=();
    let switches=[];for opt in choices{
        let key=opt[0];let name=opt[1];let selected=filter==key;
        switches.push(button::Button(#{key:key,text:name+" "+ctx.format_number(data.counts[key]),size:"xs",variant:if selected{"secondary"}else{"ghost"},
            disabled:controlled&&!listen,on_click:Fn("filter_change").curry(key,controlled,listen)}).accessibility_label(name).accessibility_pressed(selected)
            .with_style(if selected{ctx.component_style("filter_active",style())}else{style()}));
    }
    let nodes=[];for i in 0..data.items.len{nodes.push(queued(ctx,props,data.items,i,i==index));}
    if nodes.len==0{nodes.push(label(ctx,ctx.t_or("review_center.none","No matching requests"),"empty").with_key("empty"));}
    let body=column(nodes).with_key("list").with_style(ctx.component_style("list",style().gap(theme_length("space.spacing_unit").times(0.75)).padding(theme_length("space.spacing_unit").times(2)).min_width(px(0))));
    let viewport=scroll::ScrollArea(#{key:"queue-list",label:ctx.t_or("review_center.queue","Pending review"),
        height:if compact{px(168)}else{relative(1.0)},content:body});
    if !compact{viewport=viewport.with_style(style().min_height(px(0)).flex_shrink(true).flex_grow());}
    // A separate toolbar aligns its control frames; avatars lead the list's
    // own column. Their label baselines are not shared across the divider.
    column([row(switches).with_key("filters").with_style(ctx.component_style("filters",style().min_width(px(0))
            .gap(theme_length("space.spacing_unit").times(0.75)).padding(theme_length("space.spacing_unit").times(2)).flex_wrap().flex_shrink(false))),
        viewport]).with_key("queue").audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("queue",style().min_width(px(0)).min_height(px(0)).flex_shrink(false)
            .width(if compact{relative(1.0)}else{relative(0.34)}).background(theme_color("surface_hover"))))
}
fn section(ctx,key,title,nodes){
    column([label(ctx,title,"section_title").with_key("heading").accessibility_role("heading").accessibility_level(4),column(nodes).with_key("content")
        .with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)))])
        .with_key(key).with_style(ctx.component_style("section",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25))))
}
fn changes(item){
    let fields=review::value_or(item,"fields",[]);let fields_text=#{};id_check(fields);
    for f in fields{fields_text[f.id]=evidence::plain_evidence(f.value);}
    let body=review::opt(item,"body");
    #{body:if body==(){()}else{evidence::plain_evidence(body)},fields:fields_text,
        focus:if body!=(){"body-editor"}else if fields.len>0{"field-0"}else{""}}
}
fn proposal(ctx,item,cfg){
    let nodes=[];let t=cfg.target;let fields=review::value_or(item,"fields",[]);
    if fields.len>0{
        let rows=[];
        for i in 0..fields.len{
            let f=fields[i];let id="field-"+i.to_string();
            let value=if cfg.edit==(){evidence::evidence_line(ctx,"value",f.value)}else{
                input::Input(#{key:id,label:f.label,value:cfg.edit.fields[f.id],disabled:!cfg.ready,
                    on_change:Fn("edit_value").curry(t,f.id)}).with_key(id).with_ref(element_ref(id))
                    .with_style(ctx.component_style("editor",style().min_width(px(0)).width(relative(1.0))))
            };
            let children=[label(ctx,f.label,"field_label").with_key("caption"),value.with_style(ctx.component_style("field_value",style().min_width(px(0))))];
            rows.push(column(children).with_style(ctx.component_style("field",style().min_width(px(0)).gap(theme_length("space.spacing_unit")))).with_key("field-"+f.id));
        }
        nodes.push(section(ctx,"facts",ctx.t_or("review_center.facts","Key facts"),rows));
    }
    let body=review::opt(item,"body");
    if body!=(){
        let caption=review::value_or(item,"body_label",ctx.t_or("review_center.proposal","Proposal"));
        let view=if cfg.edit==(){evidence::evidence_line(ctx,"proposal-text",body)}else{
            textarea::Textarea(#{key:"body-editor",label:caption,value:cfg.edit.body,rows:4,max_length:4000,disabled:!cfg.ready,
                editor_ref:element_ref("body-editor"),on_change:Fn("edit_value").curry(t,"")})
                .with_style(ctx.component_style("editor",style().min_width(px(0)).width(relative(1.0))))
        };
        nodes.push(section(ctx,"proposal",caption,[view.with_style(ctx.component_style("body",style().min_width(px(0))))]));
    }
    nodes
}
fn files(ctx,item,t){
    let files=review::value_or(item,"files",[]);if files.len==0{return ();}
    id_check(files);let selected=scoped(ctx,"file",t);let index=0;
    for i in 0..files.len{if files[i].id==selected{index=i;}}
    let tabs=[];for i in 0..files.len{
        let file=files[i];
        let mark=(if "added" in file&&file.added!=(){" +"+ctx.format_number(file.added)}else{""})
            +(if "removed" in file&&file.removed!=(){" −"+ctx.format_number(file.removed)}else{""});
        tabs.push(button::Button(#{key:"file-"+file.id,text:file.name+mark,size:"xs",variant:if i==index{"secondary"}else{"ghost"},
            on_click:Fn("change_file").curry(t,file.id)}).accessibility_label(file.name).accessibility_pressed(i==index)
            .with_style(ctx.component_style(if i==index{["file","file_active"]}else{["file"]},style().min_width(px(0)))));
    }
    let file=files[index];
    let diff=document::DiffViewer(#{key:"diff-"+file.id,left:#{source:file.before,label:ctx.t_or("gates.before","Before"),file_name:file.name},
        right:#{source:file.after,label:ctx.t_or("gates.proposed","Proposed"),file_name:file.name},mode:"unified",wrap:"viewport"})
        .with_style(ctx.component_style("diff",style().min_width(px(0)).height(theme_length("space.unit").times(47))));
    section(ctx,"files",ctx.t_or("review_center.files","Changed files"),[
        row(tabs).with_key("files").with_style(ctx.component_style("files",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).flex_wrap())),diff])
}
fn plan(ctx,item){
    let steps=review::value_or(item,"steps",[]);if steps.len==0{return ();}
    let rows=[];for i in 0..steps.len{
        let step=steps[i];let body=[evidence::evidence_line(ctx,"step-title",step.title)];
        let tool=review::opt(step,"tool");
        if tool!=(){body.push(label(ctx,tool,"tool").with_key("tool"));}
        let detail=review::opt(step,"detail");
        if detail!=(){body.push(evidence::evidence_line(ctx,"step-detail",detail));}
        rows.push(row([label(ctx,ctx.format_number(i+1),"step_no").with_key("number").accessibility_hidden(true)
                .with_style(style().width(theme_length("space.unit").times(5)).flex_shrink(false)),
            column(body).with_key("description").with_style(style().min_width(px(0)).flex_grow().gap(theme_length("space.spacing_unit").times(0.5)))])
            .with_key("step-"+i.to_string()).audit_allow(["mixed-type-in-row"])
            .with_style(ctx.component_style("step",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).items_start())));
    }
    section(ctx,"plan",ctx.t_or("review_center.plan","Plan"),rows)
}
fn sources(ctx,item,props,t){
    let sources=review::value_or(item,"sources",[]);if sources.len==0{return ();}
    id_check(sources);let nodes=[];
    for source in sources{
        let node=if review::opt(props,"on_source")==(){label(ctx,source.label,"meta").with_key("src-"+source.id)}else{
            button::Button(#{key:"src-"+source.id,text:source.label,size:"xs",variant:"ghost",on_click:Fn("send").curry("source",
                #{request:t.request,revision:t.revision,source:source.id})}).with_style(ctx.component_style("source_button",style().min_width(px(0))))
        };nodes.push(node);
    }
    section(ctx,"sources",ctx.t_or("review_center.sources","Sources"),[row(nodes).with_key("source-links").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("sources",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).flex_wrap()))])
}
fn detail(ctx,props,item,cfg){
    let age=max(0,props.now_ms-item.created_ms);
    let name=item.agent+if review::opt(item,"model")==(){""}else{" · "+item.model};
    let identity=row([avatar(ctx,item.agent),label(ctx,name,"meta").with_key("name"),risk_badge(ctx,item),
        label(ctx,ctx.t_or("review_center.confidence","{n}% confident",#{n:ctx.format_number((item.confidence*100.0).round().to_int())}),"meta")
            .with_key("confidence"),label(ctx,format_age(ctx,age),"meta").with_key("wait").with_style(style().text_color(age_ink(age,props.sla_ms)))])
        .with_key("identity").audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(1.75)).flex_wrap());
    let content=[identity,text(item.title).with_key("title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("item_title",style().min_width(px(0)).typography("review_title")))];
    let why=review::opt(item,"reasoning");
    if why!=(){content.push(section(ctx,"why",ctx.t_or("review_center.why","Why"),[label(ctx,why,"why").with_key("reasoning")]));}
    for block in proposal(ctx,item,cfg){content.push(block);}
    let file=files(ctx,item,cfg.target);if file!=(){content.push(file);}
    let steps=plan(ctx,item);if steps!=(){content.push(steps);}
    let sources=sources(ctx,item,props,cfg.target);if sources!=(){content.push(sources);}
    column(content).with_key("request-"+item.id+"@"+item.revision.to_string())
        .with_style(ctx.component_style("detail",style().padding(theme_length("space.spacing_unit").times(4.25)).gap(theme_length("space.spacing_unit").times(3.25)).min_width(px(0))))
}
fn reasons(ctx,props){
    let reasons=review::value_or(props,"reasons",[
        #{id:"incorrect",label:ctx.t_or("review_center.incorrect","Incorrect details")},
        #{id:"out_of_scope",label:ctx.t_or("review_center.scope","Out of scope")},
        #{id:"needs_evidence",label:ctx.t_or("review_center.evidence","Needs more evidence")},
    ]);
    id_check(reasons);reasons
}
fn action(ctx,key,title,variant,enabled,callback,ref_key){
    let node=button::Button(#{key:key,text:title,variant:variant,size:"sm",disabled:!enabled,on_click:callback});
    if ref_key!=""{node=node.with_key(key).with_ref(element_ref(ref_key));}
    node
}
fn buttons(ctx,props,items,index,cfg,original){
    let t=cfg.target;let ready=cfg.ready;let list=[];
    let choices=reasons(ctx,props);let choosing=cfg.reason&&choices.len>0;
    if choosing{
        for i in 0..choices.len{
            let reason=choices[i];
            list.push(action(ctx,"reason-"+reason.id,reason.label,"secondary",ready,
                Fn("decide").curry(t,"reject",reason.id,()),if i==0{"reason-first"}else{""})
                .with_style(ctx.component_style("reason",style())));
        }
        list.push(action(ctx,"cancel-reason",ctx.t_or("review_center.cancel","Cancel"),"ghost",ready,Fn("cancel").curry(t,"reason"),""));
    }else if cfg.edit!=(){
        list.push(action(ctx,"save",ctx.t_or("review_center.save","Approve with edits"),"primary",ready,Fn("decide").curry(t,"approve","",original),""));
        list.push(action(ctx,"cancel-edit",ctx.t_or("review_center.cancel","Cancel"),"ghost",ready,Fn("cancel").curry(t,"edit"),""));
    }else{
        list.push(action(ctx,"approve",ctx.t_or("review.approve","Approve"),"primary",ready,Fn("decide").curry(t,"approve","",()),""));
        list.push(action(ctx,"reject",ctx.t_or("gates.reject","Reject"),"secondary",ready,
            if choices.len>0{Fn("show_reasons").curry(t)}else{Fn("decide").curry(t,"reject","",())},"reject"));
        if original.focus!=""{list.push(action(ctx,"edit",ctx.t_or("review_center.edit","Edit"),"ghost",ready,Fn("edit").curry(t,original),"edit"));}
        let next=(index+1)%items.len;
        list.push(action(ctx,"skip",ctx.t_or("review_center.skip","Skip"),"ghost",items.len>1||review::opt(props,"on_skip")!=(),
            Fn("skip").curry(t,target(items[next]),next,review::opt(props,"on_skip")!=()),""));
    }
    let nav=[];if items.len>1{
        let rtl=ctx.text_direction()=="rtl";let prev=max(0,index-1);let next=min(items.len-1,index+1);
        nav.push(icon_button::IconButton(#{key:"previous",label:ctx.t_or("review_center.previous","Previous request"),icon:cards::icon(ctx,if rtl{"forward"}else{"back"}),
            size:"sm",disabled:prev==index,on_click:Fn("select_outcome").curry(target(items[prev]),prev,"")}));
        nav.push(label(ctx,ctx.format_number(index+1)+" / "+ctx.format_number(items.len),"count").with_key("position"));
        nav.push(icon_button::IconButton(#{key:"next",label:ctx.t_or("review_center.next","Next request"),icon:cards::icon(ctx,if rtl{"back"}else{"forward"}),
            size:"sm",disabled:next==index,on_click:Fn("select_outcome").curry(target(items[next]),next,"")}));
    }
    let footer=row([row(list).with_key("commands").with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit")).items_center().flex_wrap().flex_grow()),
        row(nav).with_key("pager").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("pager",style().gap(theme_length("space.spacing_unit")).items_center().flex_shrink(false)))])
        .with_key("actions").audit_allow(["mixed-type-in-row"]).with_style(ctx.component_style("actions",style().min_width(px(0))
        .gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap().padding(theme_length("space.spacing_unit").times(2)).border_top(px(1)).border_color(theme_color("border")).flex_shrink(false)));
    if choosing&&ready{
        for i in 0..choices.len{
            footer=footer.on_bubble("key:plain+"+(i+1).to_string(),Fn("decide").curry(t,"reject",choices[i].id,()));
        }
    }
    footer
}
fn render_ReviewCenter(ctx,props){
    let shell_ref=element_ref("shell");let width=ctx.element_width_class(shell_ref,[640]);let compact=width==0;
    let local=ctx.get_state("filter");let filter=review::value_or(props,"filter",if local==(){props.default_filter}else{local});
    let data=project(props.items,filter,props.low_confidence,props.order);let index=selection(ctx,data.items);
    let side=queue(ctx,props,data,index,filter,compact);
    let root=[head(ctx,props,data)];let ack=receipt(ctx,props);if ack!=(){root.push(ack);}
    let stage=();
    if data.items.len==0{
        stage=column([label(ctx,ctx.t_or("review_center.empty","Nothing to review here"),"empty").with_key("empty")]);
    }else{
        let item=data.items[index];let t=target(item);let original=changes(item);
        let ready=!review::value_or(item,"busy",false)&&review::opt(props,"on_decide")!=();
        let cfg=#{target:t,edit:scoped(ctx,"editor",t),reason:scoped(ctx,"reason",t)==true,ready:ready};
        let content=detail(ctx,props,item,cfg);
        let viewport=scroll::ScrollArea(#{key:"detail-content",label:ctx.t_or("review_center.evidence_view","Request evidence"),
            height:relative(1.0),content:content})
            .with_style(ctx.component_style("detail_scroll",style().min_width(px(0)).min_height(px(0))));
        // The compact detail expands in its caller's scroll surface, so a
        // nested diff cannot squeeze the evidence into a few visible lines.
        // Keep the same formal owner and editor keys on both sides of reflow.
        if compact{viewport=viewport.with_style(style().height(auto()).overflow_hidden());}
        else{viewport=viewport.with_style(style().flex_shrink(true).flex_grow());}
        let notice=review::opt(item,"notice");let nodes=[viewport];
        if notice!=(){nodes.push(label(ctx,notice,"notice").with_key("notice").accessibility_role("status").with_style(style().flex_shrink(false)));}
        nodes.push(buttons(ctx,props,data.items,index,cfg,original));
        stage=column(nodes);
        if cfg.edit!=(){stage=stage.on_bubble("key:plain+escape",Fn("cancel").curry(t,"edit"));}
        if cfg.reason{stage=stage.on_bubble("key:plain+escape",Fn("cancel").curry(t,"reason"));}
        let focus=scoped(ctx,"focus_plan",t);
        let can_focus=focus!=()&&ready&&(focus!="reason-first"||reasons(ctx,props).len>0);
        effect("present_focus",#{target:t,focus:if can_focus{focus}else{""}},Fn("edit_context"),Fn("no_effect"));
    }
    let panes=[side,stage.with_key("stage").with_style(ctx.component_style("detail_pane",style().min_width(px(0)).min_height(px(0)).flex_grow()
        .background(theme_color("surface_raised"))))];
    let shell=if compact{column(panes)}else{row(panes).with_style(style().height(theme_length("space.unit").times(130)))};
    root.push(shell.with_key("card").with_style(ctx.component_style("card",style().min_width(px(0)).width(relative(1.0))
        .background(theme_color("surface_raised")).radius(theme_radius("lg")).clip())));
    column(root).with_key(props.key).with_ref(shell_ref)
        .accessibility_role("group").accessibility_label(review::value_or(props,"title",ctx.t_or("review_center.title","Review center")))
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(1100)).gap(theme_length("space.spacing_unit").times(1.25))))
}
