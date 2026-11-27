/* gpui-rhai
{"id":"patterns/approval_card","export":"ApprovalCard","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/checkbox"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","surface_raised","surface_hover","text.danger","text.success","text.warning","text_primary","text_muted","typography.body","typography.caption","space.spacing_unit","space.unit","typography.title"],"environment":["density"]}
*/
// ApprovalCard presents a versioned request, its evidence and the accepted receipt.
// State: a revision-scoped checkbox; the Host supplies deadlines and all decisions.
// Example: ApprovalCard(#{key:"review",request:"review/1",revision:3,title:"Approve the release note"})
import "components/button" as button;
import "components/checkbox" as checkbox;

define_component(#{
    metadata:#{id:"patterns/approval_card","export":"ApprovalCard","version":"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/checkbox"],capabilities:#{},
        tokens:["border","focus_ring","surface_raised","surface_hover","text.danger","text.success","text.warning","text_primary","text_muted","typography.body","typography.caption","space.spacing_unit","space.unit","typography.title"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visual identity of this card."},
        request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted request ID. Use a new ID for a different request."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted request revision echoed in every proposal."},
        title:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Action the person is deciding on."},
        description:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional context under the title."},
        label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional tag above the card."},
        icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative mark instead of the shield."},
        preview:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:true,doc:"Application-owned preview node, not executable action metadata."},
        details:#{schema:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Field label."},
            value:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Displayed value."},
        }}},"default":#{type:"array",value:[]},required:false,sensitive:true,doc:"Accepted facts about the request."},
        risk:#{schema:#{type:"optional",value:#{type:"string",allowed:["low","medium","high"]}},required:false,sensitive:false,doc:"Optional risk badge."},
        status:#{schema:#{type:"string",allowed:["pending","approved","denied","expired"]},"default":#{type:"string",value:"pending"},
            required:false,sensitive:false,doc:"Accepted status. Only the Host can publish a terminal receipt."},
        remaining_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Time left as seen by the Host; zero disables new approval, without changing the accepted status."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Suppress another proposal while the Host is processing one."},
        message:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Host-owned feedback or receipt subtitle."},
        allow_always:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Opt in to offering a durable choice; it is not an authorization by itself."},
        editable:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Offer a separate Edit proposal."},
        actions:#{schema:#{type:"string",allowed:["tray","card"]},"default":#{type:"string",value:"tray"},required:false,sensitive:false,doc:"Action placement."},
        shortcuts:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Show and enable card-scoped Cmd/Ctrl+Enter and Escape."},
        on_decision:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a versioned proposal; never executes an action in the component."},
        on_activity:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives distinct hover and focus-within edges. A Host may pause a soft reminder; do not use this to extend a hard deadline."},
    },state:#{fields:#{always:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{
        decision:#{payload:#{type:"object",allow_unknown:false,fields:#{
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Request being decided."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision seen when the proposal was made."},
            action:#{schema:#{type:"string",allowed:["approve","deny","edit"]},required:true,sensitive:false,doc:"Intent, not its result."},
            always:#{schema:#{type:"bool"},required:true,sensitive:false,doc:"Only an approval may request a durable choice."},
        }},doc:"Proposes approval, denial or editing."},
        activity:#{payload:#{type:"object",allow_unknown:false,fields:#{
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Request receiving activity."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision when the edge was delivered."},
            channel:#{schema:#{type:"string",allowed:["pointer","focus"]},required:true,sensitive:false,doc:"Two independent activity channels."},
            active:#{schema:#{type:"bool"},required:true,sensitive:false,doc:"Entered or left."},
        }},doc:"Presentation hint for soft timers; no authority over expiration."},
    },slots:#{},parts:["root","label","surface","header","mark","title","description","risk","risk_low","risk_medium","risk_high",
        "preview","detail","detail_label","detail_value","always","footer","footer_inside","buttons","timer","timer_urgent","feedback",
        "approve","deny","edit","receipt","receipt_title","receipt_note","receipt_mark","receipt_ok","receipt_no","receipt_expired","icon"]},
    render:Fn("render_ApprovalCard")
});

fn ApprovalCard(props){render_component("patterns/approval_card",props)}
fn opt(map,key){if key in map{map[key]}else{()}}
fn value_or(map,key,fallback){let value=opt(map,key);if value==(){fallback}else{value}}
// Reveal only this retained viewport; all three measurements are in the
// same presented coordinate domain. Do not move an enclosing page.
fn reveal_in(ctx,target,viewport,contents){
    let row=ctx.element_bounds(target);let outer=ctx.element_bounds(viewport);let inner=ctx.element_bounds(contents);
    if row==()||outer==()||inner==(){return;}
    let a=row.visual;let b=outer.visual;
    let delta=if a.y<b.y{a.y-b.y}else{max(0.0,a.y+a.height-b.y-b.height)};
    if delta.abs()>0.5{ctx.scroll_to(viewport,0.0,max(0.0,b.y-inner.visual.y+delta));}
}
fn target(props){
    if props.request==""||utf8_byte_count(props.request)>128{throw "Review request IDs must be 1–128 UTF-8 bytes";}
    #{request:props.request,revision:props.revision}
}
fn send(event,decision,ctx,payload){ctx.emit(event,decision);handled()}
fn confirm_chord(){if key_legend("mod").platform=="mac"{"cmd-enter"}else{"ctrl-enter"}}
fn heading_note(ctx,text,part){
    text(text).with_style(ctx.component_style(part,style().min_width(px(0)).typography("caption").text_color(theme_color("text_muted"))))
}
fn glyph(ctx,name,n){
    let path=switch name{
        "check"=>"m4 10 3.5 3.5L16 5.5",
        "close"=>"m5 5 10 10M15 5 5 15",
        "clock"=>"M10 2.5a7.5 7.5 0 1 0 0 15 7.5 7.5 0 0 0 0-15Zm0 4v4l3 1.5",
        "warning"=>"M10 2 18 16H2L10 2Zm0 5v4m0 2.6v.1",
        "down"=>"m5 8 5 5 5-5",
        "up"=>"m5 12 5-5 5 5",
        _=>"M10 2.5 4 5v4.5c0 3.4 2.6 6.5 6 8 3.4-1.5 6-4.6 6-8V5l-6-2.5Zm-2.6 7 1.6 1.7 3.2-3.4",
    };
    svg(`<svg viewBox='0 0 20 20' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("icon",style().width(px(n)).height(px(n)).flex_shrink(false)))
}
fn risk_badge(ctx,risk,title){
    let tone=if risk=="high"{"text.danger"}else if risk=="medium"{"text.warning"}else{"text_muted"};
    let dot=row([]).accessibility_hidden(true).with_style(style().width(px(5)).height(px(5)).radius(px(4)).background(theme_color(tone)));
    row([dot,text(title).with_style(style().min_width(px(0)))]).with_key("risk")
        .with_style(ctx.component_style(["risk","risk_"+risk],style().min_width(px(0)).padding_x(theme_length("space.spacing_unit").times(1.5)).padding_y(theme_length("space.spacing_unit").times(0.5))
            .gap(theme_length("space.spacing_unit")).items_center().radius(px(5)).typography("caption").text_color(theme_color(tone)).background(theme_color("surface_hover"))))
}
fn risk_label(ctx,risk){
    if risk=="high"{ctx.t_or("review.high","High risk")}else if risk=="medium"{ctx.t_or("review.medium","Medium risk")}else{ctx.t_or("review.low","Low risk")}
}
fn header(ctx,title,subtitle,icon,risk){
    let mark=row([icon]).with_key("mark").accessibility_hidden(true).with_style(ctx.component_style("mark",style().width(theme_length("space.unit").times(9)).height(theme_length("space.unit").times(9))
        .radius(theme_length("space.unit").times(2.25)).background(theme_color("surface_hover")).justify_center().items_center().clip().flex_shrink(false)));
    let texts=[text(title).with_key("title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("title",style().min_width(px(0)).typography("title").font_weight(500)))];
    if subtitle!=(){texts.push(heading_note(ctx,subtitle,"description"));}
    let contents=[mark,column(texts).with_style(style().min_width(px(108)).flex_basis(px(108)).flex_grow())];
    if risk!=(){contents.push(risk);}
    row(contents).with_key("header").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("header",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).items_center().flex_wrap()))
}
fn receipt_mark(ctx,status){
    if status=="expired"{return glyph(ctx,"clock",17);}
    let points=if status=="ok"{[path_move(4,10),path_line(7.5,13.5),path_line(16,5.5)]}
        else{[path_move(5,5),path_line(15,15),path_move(15,5),path_line(5,15)]};
    canvas(canvas_scene([canvas_stroke_path("state",points,1.7,current_color()).stroke_cap("round").stroke_join("round")]).view_box(0,0,20,20))
        .with_key("receipt-stroke").accessibility_hidden(true).with_style(style().width(theme_length("space.unit").times(4.25)).height(theme_length("space.unit").times(4.25)))
        .motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:280,intent:"feedback"}))
}
fn receipt(ctx,kind,title,detail,action,drain){
    let status=row([receipt_mark(ctx,kind)]).with_key("mark").with_style(ctx.component_style(["receipt_mark","receipt_"+kind],
        style().width(theme_length("space.unit").times(7)).height(theme_length("space.unit").times(7)).radius(px(7)).flex_shrink(false).items_center().justify_center()
            .text_color(theme_color(if kind=="ok"{"text.success"}else{"text_muted"})).background(theme_color("surface_hover"))));
    let labels=[text(title).with_key("outcome").accessibility_role("status").accessibility_label(title)
        .with_style(ctx.component_style("receipt_title",style().min_width(px(0)).typography("body").font_weight(500)))];
    if detail!=(){labels.push(heading_note(ctx,detail,"receipt_note"));}
    let children=[status,column(labels).with_style(style().min_width(px(0)).flex_grow())];
    if action!=(){children.push(action);}
    let nodes=[row(children).with_key("line").with_style(style().min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(2.25)).flex_wrap())];
    if drain!=(){nodes.push(drain);}
    column(nodes).with_key("receipt").with_style(ctx.component_style("receipt",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2.5)).gap(theme_length("space.spacing_unit").times(1.25))
        .radius(theme_length("space.unit").times(2.5)).background(theme_color("surface_raised"))))
        .motion(motion_transition("opacity",0.45,1.0,#{duration_ms:200,intent:"feedback"}))
}
fn change_always(token,ctx,value){ctx.set_state("always",#{request:token.request,revision:token.revision,checked:value.checked});}
fn choice(token,action,durable){#{request:token.request,revision:token.revision,action:action,always:action=="approve"&&durable}}
fn activity(token,channel,ctx,value){
    token.channel=channel;token.active=value;ctx.emit("activity",token);
}
fn timer_label(seconds){
    let minutes=seconds/60;let tail=(seconds%60).to_string();
    minutes.to_string()+":"+if tail.len==1{"0"+tail}else{tail}
}
fn timer(ctx,value){
    let seconds=value/1000+if value%1000>0{1}else{0};
    let name=if value==0{ctx.t_or("review.deadline_elapsed","Time elapsed")}
        else{ctx.t_or("review.expires_in","Expires in {time}",#{time:timer_label(seconds)})};
    row([glyph(ctx,"clock",14),text(name).with_style(style().min_width(px(0)).typography("caption").font_feature("tnum",1))])
        .with_key("timer").with_style(ctx.component_style(if seconds<=10{["timer","timer_urgent"]}else{["timer"]},
            style().gap(theme_length("space.spacing_unit").times(1.25)).min_width(px(0)).items_center().text_color(theme_color(if seconds<=10{"text.warning"}else{"text_muted"}))))
}
fn footer(ctx,props,token,active,approvable,always,remaining){
    let buttons=[];
    if props.editable{
        buttons.push(button::Button(#{key:"edit",text:ctx.t_or("review.edit","Edit"),size:"sm",variant:"ghost",disabled:!active,
            on_click:Fn("send").curry("decision",choice(token,"edit",false))}).with_style(ctx.component_style("edit",style())));
    }
    buttons.push(button::Button(#{key:"deny",text:ctx.t_or("review.deny","Deny"),size:"sm",variant:"secondary",disabled:!active,
        shortcut:if props.shortcuts{"escape"}else{()},on_click:Fn("send").curry("decision",choice(token,"deny",false))})
        .with_style(ctx.component_style("deny",style())));
    buttons.push(button::Button(#{key:"approve",text:ctx.t_or("review.approve","Approve"),size:"sm",disabled:!approvable,
        loading:props.busy,loading_text:ctx.t_or("review.processing","Processing…"),shortcut:if props.shortcuts{confirm_chord()}else{()},
        on_click:Fn("send").curry("decision",choice(token,"approve",always))}).with_style(ctx.component_style("approve",style())));
    let children=[];
    if remaining!=(){children.push(timer(ctx,remaining));}
    children.push(row(buttons).with_key("buttons").with_style(ctx.component_style("buttons",style().gap(theme_length("space.spacing_unit").times(1.25)).min_width(px(0)).items_center().flex_wrap())));
    row(children).with_key("footer").with_style(ctx.component_style(if props.actions=="card"{["footer","footer_inside"]}else{["footer"]},
        style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.75)).padding(theme_length("space.spacing_unit").times(1.25)).justify_between().items_center().flex_wrap()))
}
fn render_ApprovalCard(ctx,props){
    let token=target(props);let pending=props.status=="pending";let remaining=opt(props,"remaining_ms");
    let active=pending&&!props.busy&&opt(props,"on_decision")!=();
    let approvable=active&&(remaining==()||remaining>0);
    let draft=ctx.get_state("always");
    let always=props.allow_always&&draft!=()&&draft.request==token.request&&draft.revision==token.revision&&draft.checked;
    let contents=[];let label=opt(props,"label");let feedback=opt(props,"message");
    if label!=(){contents.push(heading_note(ctx,label,"label").with_key("label"));}
    if pending{
        let risk=opt(props,"risk");let icon=value_or(props,"icon",glyph(ctx,"shield",19));
        let body=[header(ctx,props.title,opt(props,"description"),icon,if risk==(){()}else{risk_badge(ctx,risk,risk_label(ctx,risk))})];
        if opt(props,"preview")!=(){body.push(props.preview.with_key("preview").with_style(ctx.component_style("preview",style().min_width(px(0)))));}
        for i in 0..props.details.len{
            let d=props.details[i];
            body.push(row([heading_note(ctx,d.label,"detail_label").with_style(style().min_width(px(0)).max_width(relative(0.45))),
                text(d.value).with_style(ctx.component_style("detail_value",style().min_width(px(0)).flex_grow().typography("body")))])
                .with_key("detail-"+i.to_string()).audit_allow(["mixed-type-in-row"])
                .with_style(ctx.component_style("detail",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).items_start())));
        }
        if props.allow_always{
            body.push(checkbox::Checkbox(#{key:"always",label:ctx.t_or("review.always_action","Always allow this action"),checked:always,
                disabled:!approvable,on_change:Fn("change_always").curry(token)})
                .with_style(ctx.component_style("always",style().min_width(px(0)))));
        }
        let foot=footer(ctx,props,token,active,approvable,always,remaining);
        if props.actions=="card"{body.push(foot);}
        contents.push(column(body).with_key("surface").with_style(ctx.component_style("surface",style().min_width(px(0))
            .padding(theme_length("space.spacing_unit").times(3)).gap(theme_length("space.spacing_unit").times(2)).radius(theme_length("space.unit").times(2.75)).background(theme_color("surface_raised")))));
        if props.actions=="tray"{contents.push(foot);}
        if feedback!=(){contents.push(heading_note(ctx,feedback,"feedback").accessibility_role("status").with_key("feedback"));}
    }else{
        let title=if props.status=="approved"{ctx.t_or("review.approved","Approved")}
            else if props.status=="expired"{ctx.t_or("review.expired","Expired")}else{ctx.t_or("review.denied","Denied")};
        let kind=if props.status=="approved"{"ok"}else if props.status=="expired"{"expired"}else{"no"};
        contents.push(receipt(ctx,kind,title,value_or(props,"message",props.title),(),()));
    }
    let root=column(contents).with_key(props.key).accessibility_role("group").accessibility_label(props.title)
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(560))
            .padding(theme_length("space.spacing_unit")).gap(theme_length("space.spacing_unit")).radius(theme_length("space.unit").times(4)).background(theme_color("surface_hover"))));
    if props.shortcuts&&approvable{
        // An explicit review chord takes precedence over a descendant's
        // generic Enter activation. Escape still yields to nested surfaces.
        root=root.on_capture("key:cmd+enter",Fn("send").curry("decision",choice(token,"approve",always)))
            .on_capture("key:ctrl+enter",Fn("send").curry("decision",choice(token,"approve",always)));
    }
    if props.shortcuts&&active{root=root.on_bubble("key:escape",Fn("send").curry("decision",choice(token,"deny",false)));}
    if pending&&opt(props,"on_activity")!=(){
        root=root.on_hover_change(Fn("activity").curry(token,"pointer")).on_focus_within_change(Fn("activity").curry(token,"focus"));
    }
    root
}
