/* gpui-rhai
{"id":"patterns/permission_request","export":"PermissionRequest","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/radio_group","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","surface_raised","surface_hover","text.danger","text.success","text.warning","text_primary","text_muted","typography.body","typography.caption","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// PermissionRequest displays a capability request and an accepted grant, denial or expiry.
// State: preview expansion and optional local duration choice. Only the Host grants or revokes access.
// Example: PermissionRequest(#{key:"access",request:"review/1",revision:1,agent:"Reviewer",action:"read release notes"})
import "components/button" as button;
import "components/radio_group" as radios;
import "patterns/approval_card" as review;

define_component(#{
    metadata:#{id:"patterns/permission_request","export":"PermissionRequest",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/radio_group","patterns/approval_card"],capabilities:#{},
        tokens:["accent","border","focus_ring","surface_raised","surface_hover","text.danger","text.success","text.warning","text_primary","text_muted","typography.body","typography.caption","space.spacing_unit","space.unit"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable visual key."},
        request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of this permission request."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted policy revision echoed by every proposal."},
        agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Who requests access."},
        action:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Plain display text describing the requested action, not a command."},
        target:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional target or model label."},
        avatar:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative avatar; otherwise initials are shown."},
        reason:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Plain explanation from the requester."},
        risk:#{schema:#{type:"string",allowed:["safe","risky","destructive"]},"default":#{type:"string",value:"risky"},required:false,sensitive:false,
            doc:"Destructive requests never offer a perpetual grant."},
        preview:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable preview identity."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"One expected effect of the action."},
            info:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional annotation to its end."},
        }}},"default":#{type:"array",value:[]},required:false,sensitive:true,doc:"What will happen if granted."},
        preview_title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional heading over preview entries."},
        preview_limit:#{schema:#{type:"integer",min:0,max:32},"default":#{type:"integer",value:3},required:false,sensitive:false,doc:"Maximum rows before disclosure."},
        scopes:#{schema:#{type:"array",max_items:3,items:#{type:"string",allowed:["once","timed","always"]}},
            "default":#{type:"array",value:[#{type:"string",value:"once"},#{type:"string",value:"timed"},#{type:"string",value:"always"}]},
            required:false,sensitive:false,doc:"Offered grant scopes. Empty disables Allow."},
        scope:#{schema:#{type:"optional",value:#{type:"string",allowed:["once","timed","always"]}},required:false,sensitive:false,
            doc:"Optional controlled scope. Otherwise the selection is local to this request revision."},
        duration_minutes:#{schema:#{type:"integer",min:1,max:10080},"default":#{type:"integer",value:60},required:false,sensitive:false,
            doc:"Duration offered for a timed request. The Host sets the actual deadline."},
        status:#{schema:#{type:"string",allowed:["pending","allowed","denied","expired","revoked"]},"default":#{type:"string",value:"pending"},
            required:false,sensitive:false,doc:"Accepted outcome, never computed from a UI clock."},
        grant:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted grant identity needed for revocation."},
            scope:#{schema:#{type:"string",allowed:["once","timed","always"]},required:true,sensitive:false,doc:"Actually granted duration, independent of a proposal."},
            remaining_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Remaining time displayed by the Host."},
            total_ms:#{schema:#{type:"optional",value:#{type:"integer",min:1}},required:false,sensitive:false,doc:"Initial duration, used for the visual time track."},
        }}},required:false,sensitive:false,doc:"Accepted grant record. Required for an allowed receipt."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Blocks repeat proposals during a policy operation."},
        can_revoke:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"The Host allows a revoke proposal for this grant."},
        can_renew:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"The Host allows a fresh request after denial or expiry."},
        shortcuts:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Allow deliberate Cmd/Ctrl+Enter and Escape when focus is inside the pending request."},
        message:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional status or receipt subtitle."},
        on_request:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,
            doc:"Receives versioned intents for allow, deny, scope selection, revoke or renew."},
    },state:#{fields:#{
        expanded:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        choice:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        request:#{payload:#{type:"object",allow_unknown:false,fields:#{
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Request being acted on."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Expected accepted revision."},
            action:#{schema:#{type:"string",allowed:["allow","deny","scope","revoke","renew"]},required:true,sensitive:false,doc:"User intent; not a resulting grant."},
            scope:#{schema:#{type:"string",allowed:["","once","timed","always"]},required:true,sensitive:false,doc:"Proposed scope, or accepted scope on revoke; otherwise empty."},
            duration_minutes:#{schema:#{type:"integer",min:0,max:10080},required:true,sensitive:false,doc:"Requested duration only for timed allow; zero otherwise."},
            grant:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Exact accepted ID when revoking, empty for other intents."},
        }},doc:"Versioned capability proposal. The component does not grant, extend or revoke access."},
    },slots:#{},parts:["root","surface","header","mark","title","description","risk","risk_low","risk_medium","risk_high",
        "reason","preview","preview_header","preview_title","preview_count","preview_row","preview_label","preview_info","preview_more",
        "answer","scopes","scope_option","hint","buttons","allow","allow_destructive","deny","feedback","receipt","receipt_title","receipt_note","receipt_mark","receipt_ok",
        "receipt_no","receipt_expired","receipt_action","time_track","time_fill","icon"]},
    render:Fn("render_PermissionRequest")
});

fn PermissionRequest(props){render_component("patterns/permission_request",props)}
fn duration_text(ctx,minutes){
    if minutes==60{return ctx.t_or("permission.one_hour","1 hour");}
    if minutes>=60&&minutes%60==0{
        return ctx.t_or("permission.hours","{count} hours",#{count:ctx.format_number(minutes/60)});
    }
    ctx.t_or("permission.minutes","{count} min",#{count:ctx.format_number(minutes)})
}
fn request(token,action,scope,duration,grant){
    #{request:token.request,revision:token.revision,action:action,scope:scope,
        duration_minutes:if action=="allow"&&scope=="timed"{duration}else{0},grant:grant}
}
fn offered(ctx,props){
    let values=[];let data=[];
    for id in props.scopes{
        if id in values||(id=="always"&&props.risk=="destructive"){continue;}
        values.push(id);
        let label=if id=="once"{ctx.t_or("permission.once","Just once")}
            else if id=="always"{ctx.t_or("permission.always","Always")}else{duration_text(ctx,props.duration_minutes)};
        data.push(#{value:id,label:label});
    }
    #{values:values,data:data}
}
fn same(token,stored){stored!=()&&stored.request==token.request&&stored.revision==token.revision}
fn choose_scope(token,controlled,ctx,value){
    if controlled{ctx.emit("request",request(token,"scope",value,0,""));}
    else{ctx.set_state("choice",#{request:token.request,revision:token.revision,value:value});}
}
fn expand(token,open,ctx,event){ctx.set_state("expanded",if open{token}else{()});handled()}
fn preview_row(ctx,payload){
    let item=payload.item;
    let dot=row([]).accessibility_hidden(true).with_style(style().width(px(5)).height(px(5)).radius(px(4)).background(theme_color("text_muted")).flex_shrink(false));
    let row_items=[dot,text(item.label).with_style(ctx.component_style("preview_label",style().min_width(px(0)).flex_grow().typography("body")))];
    if review::opt(item,"info")!=(){row_items.push(review::heading_note(ctx,item.info,"preview_info"));}
    row(row_items).with_key(item.key).audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("preview_row",style().min_width(px(0)).padding(theme_length("space.spacing_unit")).gap(theme_length("space.spacing_unit").times(2)).items_center().flex_wrap()))
}
fn preview(ctx,props,token){
    if props.preview.len==0{return ();}
    let open=same(token,ctx.get_state("expanded"));
    let keys=#{};let data=[];let bytes=0;
    for item in props.preview{
        if item.id==""||utf8_byte_count(item.id)>128||item.id in keys{throw "Permission preview requires unique 1–128 byte IDs";}
        keys[item.id]=true;item.key="preview:"+item.id;data.push(item);
        bytes+=utf8_byte_count(item.label)+utf8_byte_count(review::value_or(item,"info",""));
        if bytes>65536{throw "Permission preview exceeds 64 KiB";}
    }
    let title=review::value_or(props,"preview_title",ctx.t_or("permission.preview","What will happen"));
    let head=row([text(title).accessibility_role("heading").accessibility_level(4).with_style(ctx.component_style("preview_title",
        style().min_width(px(0)).typography("body").font_weight(500))),review::heading_note(ctx,ctx.format_number(data.len),"preview_count")])
        .with_style(ctx.component_style("preview_header",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).items_center())).audit_allow(["mixed-type-in-row"]);
    let nodes=[head];
    if open{
        nodes.push(virtual_collection(#{key:"preview-rows",label:title,data:data,estimated_height:30,max_height:px(320)},Fn("preview_row"))
            .accessibility_role("list").accessibility_label(title).tab_stop(true));
    }else{
        let rows=[];for i in 0..min(data.len,props.preview_limit){rows.push(preview_row(ctx,#{item:data[i]}));}
        nodes.push(column(rows).with_style(style().min_width(px(0))));
    }
    if data.len>props.preview_limit{
        let label=if open{ctx.t_or("permission.less","Show less")}
            else{ctx.t_or("permission.more","{count} more",#{count:ctx.format_number(data.len-props.preview_limit)})};
        nodes.push(button::Button(#{key:"expand",text:label,size:"xs",variant:"ghost",prefix:review::glyph(ctx,if open{"up"}else{"down"},14),
            on_click:Fn("expand").curry(token,!open)}).accessibility_expanded(open)
            .with_style(ctx.component_style("preview_more",style().self_start())));
    }
    column(nodes).with_key("preview").audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("preview",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2)).gap(theme_length("space.spacing_unit").times(1.25))))
}
fn receipt(ctx,props,token,active){
    let grant=review::opt(props,"grant");let note=review::value_or(props,"message",props.action);let action=();let line=();
    let title=ctx.t_or("review.denied","Denied");let kind="no";
    if props.status=="allowed"{
        if grant==()||grant.id==""||utf8_byte_count(grant.id)>128{throw "An allowed PermissionRequest requires its accepted grant identity";}
        kind="ok";
        title=if grant.scope=="once"{ctx.t_or("permission.allowed_once","Allowed once")}
            else if grant.scope=="always"{ctx.t_or("permission.allowed_always","Always allowed")}
            else{ctx.t_or("permission.allowed_timed","Allowed temporarily")};
        let remaining=review::opt(grant,"remaining_ms");let total=review::opt(grant,"total_ms");
        if grant.scope=="timed"&&remaining!=(){
            if review::opt(props,"message")==(){
                let minutes=remaining/60000+if remaining%60000>0{1}else{0};
                note=ctx.t_or("permission.time_left","{time} left",#{time:duration_text(ctx,minutes)});
            }
            if total!=(){
                let fill=min(1.0,remaining.to_float()/total.to_float());
                line=row([row([]).with_style(ctx.component_style("time_fill",style().height(px(3)).width(relative(fill)).background(theme_color("accent"))))])
                    .with_key("time-track").accessibility_hidden(true).with_style(ctx.component_style("time_track",style().height(px(3)).clip().background(theme_color("surface_hover"))));
            }
        }
        if props.can_revoke{action=button::Button(#{key:"revoke",text:ctx.t_or("permission.revoke","Revoke"),size:"sm",variant:"ghost",disabled:!active,
            on_click:Fn("request_click").curry(request(token,"revoke",grant.scope,0,grant.id))});}
    }else{
        if props.status=="expired"{title=ctx.t_or("permission.expired","Access expired");kind="expired";}
        if props.status=="revoked"{title=ctx.t_or("permission.revoked","Access revoked");}
        if props.can_renew{action=button::Button(#{key:"renew",text:ctx.t_or("permission.renew","Ask again"),size:"sm",variant:"ghost",disabled:!active,
            on_click:Fn("request_click").curry(request(token,"renew","",0,""))});}
    }
    if action!=(){action=action.with_style(ctx.component_style("receipt_action",style().flex_shrink(false)));}
    review::receipt(ctx,kind,title,note,action,line)
}
fn request_click(spec,ctx,event){review::send("request",spec,ctx,event)}
fn render_PermissionRequest(ctx,props){
    let token=review::target(props);let active=!props.busy&&review::opt(props,"on_request")!=();
    let children=[];let proposed_scope="";
    if props.status!="pending"{children.push(receipt(ctx,props,token,active));}
    else{
        let title=ctx.t_or("permission.title","{agent} wants to {action}",#{agent:props.agent,action:props.action});
        let avatar=review::value_or(props,"avatar",text(text_initial(props.agent)).with_style(style().font_weight(600)));
        let risk=if props.risk=="safe"{()}else{review::risk_badge(ctx,if props.risk=="destructive"{"high"}else{"medium"},
            if props.risk=="destructive"{ctx.t_or("permission.destructive","Destructive")}else{ctx.t_or("permission.risky","Risky")})};
        let display=[review::header(ctx,title,review::opt(props,"target"),avatar,risk)];
        if review::opt(props,"reason")!=(){
            display.push(text(props.reason).with_key("reason").with_style(ctx.component_style("reason",style().min_width(px(0))
                .padding_start(theme_length("space.spacing_unit").times(2)).border_start(px(2)).border_color(theme_color("border")).typography("caption"))));
        }
        let evidence=preview(ctx,props,token);if evidence!=(){display.push(evidence);}
        children.push(column(display).with_key("surface").with_style(ctx.component_style("surface",style().min_width(px(0))
            .padding(theme_length("space.spacing_unit").times(3)).gap(theme_length("space.spacing_unit").times(2)).radius(theme_length("space.unit").times(2.75)).background(theme_color("surface_raised")))));
        let offers=offered(ctx,props);let pending=ctx.get_state("choice");let controlled=review::opt(props,"scope")!=();
        let intent=if controlled{props.scope}else if same(token,pending){pending.value}else{"once"};
        // A withdrawn selection must not silently become a more durable scope.
        let chosen=if intent in offers.values{intent}else if offers.values.len==1&&offers.values[0]=="once"{"once"}else{""};
        proposed_scope=chosen;
        let answers=[];
        if offers.values.len>0{
            answers.push(radios::RadioGroup(#{key:"scope",value:chosen,label:ctx.t_or("permission.scope","How long"),
                orientation:"horizontal",appearance:"chip",options:offers.data,disabled:!active,on_change:Fn("choose_scope").curry(token,controlled),
                part_styles:#{root:ctx.component_style("scopes",style().min_width(px(0)).flex_wrap()),option:ctx.component_style("scope_option",style())}}));
        }
        if props.risk=="destructive"&&"always" in props.scopes{
            answers.push(review::heading_note(ctx,ctx.t_or("permission.no_perpetual","Destructive actions can’t be allowed permanently."),"hint"));
        }
        let allow_label=if chosen=="once"{ctx.t_or("permission.allow_once","Allow once")}
            else if chosen=="always"{ctx.t_or("permission.allow_always","Always allow")}
            else if chosen=="timed"{ctx.t_or("permission.allow_timed","Allow for {time}",#{time:duration_text(ctx,props.duration_minutes)})}
            else{ctx.t_or("permission.allow","Allow")};
        let buttons=[
            button::Button(#{key:"deny",text:ctx.t_or("review.deny","Deny"),variant:"secondary",size:"sm",disabled:!active,
                shortcut:if props.shortcuts{"escape"}else{()},
                on_click:Fn("request_click").curry(request(token,"deny","",0,""))}).with_style(ctx.component_style("deny",style())),
            button::Button(#{key:"allow",text:allow_label,size:"sm",variant:if props.risk=="destructive"{"danger"}else{"primary"},disabled:!active||chosen=="",loading:props.busy,
                loading_text:ctx.t_or("review.processing","Processing…"),shortcut:if props.shortcuts{review::confirm_chord()}else{()},
                on_click:Fn("request_click").curry(request(token,"allow",chosen,props.duration_minutes,""))})
                .with_style(ctx.component_style(if props.risk=="destructive"{["allow","allow_destructive"]}else{["allow"]},style()))
        ];
        answers.push(row(buttons).with_key("buttons").with_style(ctx.component_style("buttons",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).justify_end().flex_wrap())));
        if review::opt(props,"message")!=(){answers.push(review::heading_note(ctx,props.message,"feedback").accessibility_role("status"));}
        children.push(column(answers).with_key("answer").with_style(ctx.component_style("answer",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2)).gap(theme_length("space.spacing_unit").times(2)))));
    }
    let root=column(children).with_key(props.key).accessibility_role("group").accessibility_label(props.action)
        .audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(420))
            .padding(theme_length("space.spacing_unit")).radius(theme_length("space.unit").times(4)).background(theme_color("surface_hover"))));
    if props.status=="pending"&&active&&props.shortcuts{
        root=root.on_bubble("key:escape",Fn("request_click").curry(request(token,"deny","",0,"")));
        if proposed_scope!=""{
            let allow=request(token,"allow",proposed_scope,props.duration_minutes,"");
            root=root.on_capture("key:cmd+enter",Fn("request_click").curry(allow))
                .on_capture("key:ctrl+enter",Fn("request_click").curry(allow));
        }
    }
    root
}
