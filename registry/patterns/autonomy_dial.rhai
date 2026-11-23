/* gpui-rhai
{"id":"patterns/autonomy_dial","export":"AutonomyDial","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/icon_button","components/radio_group","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","selection","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted","text_primary","typography.body","typography.caption","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// AutonomyDial groups actions under the currently accepted policy, with a three-way selector and per-action pins.
// State: controlled. The Host owns policy and execution, including mandatory confirmation.
// Example: AutonomyDial(#{key:"mode",policy:"review",revision:0,level:"risky",pinned:[],actions:[]})
import "components/icon_button" as icon_button;
import "components/radio_group" as radio;
import "patterns/approval_card" as review;

define_component(#{
    metadata:#{id:"patterns/autonomy_dial","export":"AutonomyDial",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/icon_button","components/radio_group","patterns/approval_card"],capabilities:#{},
        tokens:["border","focus_ring","selection","surface_hover","surface_raised","text.danger","text.success","text.warning","text_muted","text_primary","typography.body","typography.caption","space.spacing_unit","space.unit"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable control identity."},
        policy:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Host policy identity."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision seen by the user."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional name above the dial."},
        subtitle:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional context under the name."},
        level:#{schema:#{type:"string",allowed:["always","risky","auto"]},required:true,sensitive:false,
            doc:"Accepted policy: always ask, ask if risky, or follow the Host's auto policy."},
        pinned:#{schema:#{type:"array",max_items:256,items:#{type:"string"}},required:true,sensitive:false,doc:"IDs explicitly configured to ask first."},
        actions:#{schema:#{type:"array",max_items:128,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable ID of the action in this policy."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Plain action name; not a command."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional scope or description."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative art instead of the default dot."},
            risk:#{schema:#{type:"string",allowed:["safe","risky","destructive"]},required:true,sensitive:false,doc:"Risk accepted by the Host."},
            required_confirmation:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,
                doc:"Always asks due to an independent Host rule; this row cannot be unlocked here."},
        }}},required:true,sensitive:true,doc:"Actions to present, in the Host's relative order."},
        busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disable proposals during a pending change."},
        message:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional Host result."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,
            doc:"Receives a versioned level or one pin proposal. Nothing grants permission within the dial."},
    },state:#{fields:#{}},events:#{
        change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            policy:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Target policy."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Expected current revision."},
            kind:#{schema:#{type:"string",allowed:["level","pin"]},required:true,sensitive:false,doc:"Only one change per proposal."},
            level:#{schema:#{type:"string",allowed:["always","risky","auto"]},required:true,sensitive:false,doc:"Proposed level, or current level for a pin request."},
            action:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Action ID for a pin proposal; otherwise empty."},
            pinned:#{schema:#{type:"bool"},required:true,sensitive:false,doc:"Desired pin value, used only for kind pin."},
        }},doc:"Propose a change; render only the policy accepted by the Host."},
    },slots:#{},parts:["root","card","header","mark","title","subtitle","levels","level","hint","groups","group_head","group_title",
        "count","row","row_icon","label","detail","risk","risk_low","risk_medium","risk_high","pin","empty","message","icon"]},
    render:Fn("render_AutonomyDial")
});

fn AutonomyDial(props){render_component("patterns/autonomy_dial",props)}
fn target(props){
    let t=review::target(#{request:props.policy,revision:props.revision});
    #{policy:t.request,revision:t.revision,level:props.level}
}
fn asks_first(action,level,pins){
    review::value_or(action,"required_confirmation",false)||action.id in pins||level=="always"||(level=="risky"&&action.risk!="safe")
}
fn level_changed(t,ctx,next){
    if next!=t.level{ctx.emit("change",#{policy:t.policy,revision:t.revision,kind:"level",level:next,action:"",pinned:false});}
}
fn pin(t,ctx,event){review::send("change",t,ctx,event)}
fn glyph(ctx,name){
    let path=if name=="locked"{"M5 9h10v8H5zM7 9V6a3 3 0 1 1 6 0v3m-3 3v2"}
        else if name=="open"{"M5 9h10v8H5zM7 9V6a3 3 0 0 1 5.6-1.5M10 12v2"}
        else if name=="spark"{"M10 2c.8 4.6 2.6 6.4 7.2 7.2-4.6.8-6.4 2.6-7.2 7.2C9.2 11.8 7.4 10 2.8 9.2 7.4 8.4 9.2 6.6 10 2Z"}
        else{"M10 10h.1"};
    svg(`<svg viewBox='0 0 20 20' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("icon",style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4))))
}
fn note(ctx,label,part){review::heading_note(ctx,label,part)}
fn heading(ctx,title,part){
    text(title).accessibility_role("heading").accessibility_level(3).with_style(ctx.component_style(part,
        style().min_width(px(0)).typography("body").font_weight(500)))
}
fn action_row(ctx,t,g,locked,enabled,asks){
    let mandatory=review::value_or(g,"required_confirmation",false);
    let description=review::opt(g,"detail");let paint=review::value_or(g,"icon",glyph(ctx,"dot"));
    let mark=row([paint]).accessibility_hidden(true).disabled(true).with_style(ctx.component_style("row_icon",style().width(theme_length("space.unit").times(5)).height(theme_length("space.unit").times(5))
        .flex_shrink(false).items_center().justify_center()));
    let information=[text(g.label).with_style(ctx.component_style("label",style().min_width(px(0)).typography("body").font_weight(500)))];
    if g.risk!="safe"{
        information.push(review::risk_badge(ctx,if g.risk=="destructive"{"high"}else{"medium"},
            if g.risk=="destructive"{ctx.t_or("permission.destructive","Destructive")}else{ctx.t_or("permission.risky","Risky")}));
    }
    let middle=[row(information).audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).items_center().flex_wrap())];
    if description!=(){middle.push(note(ctx,description,"detail").with_style(style().min_width(px(0)).whitespace_nowrap().text_ellipsis()));}
    let pin_label=if mandatory{ctx.t_or("dial.required","Confirmation required for {action}",#{action:g.label})}
        else if locked{ctx.t_or("dial.unlock","Unlock {action}",#{action:g.label})}
        else{ctx.t_or("dial.lock","Always ask before {action}",#{action:g.label})};
    let selected=mandatory||locked;
    let button=icon_button::IconButton(#{key:"pin:"+g.id,label:pin_label,icon:glyph(ctx,if selected{"locked"}else{"open"}),
        size:"xs",selected:selected,disabled:mandatory||!enabled,
        on_click:Fn("pin").curry(#{policy:t.policy,revision:t.revision,kind:"pin",level:t.level,action:g.id,pinned:!locked})})
        .with_style(ctx.component_style("pin",style().flex_shrink(false)));
    row([mark,column(middle).with_style(style().min_width(px(0)).flex_grow()),button])
        .with_key("action:"+g.id).accessibility_role("listitem").accessibility_label(g.label)
        .accessibility_description(if asks{ctx.t_or("dial.asks","Asks you first")}else{ctx.t_or("dial.own","Runs on its own")})
        .with_style(ctx.component_style("row",style().min_width(px(0)).padding(theme_length("space.spacing_unit")).gap(theme_length("space.spacing_unit").times(1.25)).items_center()))
        .shared_layout(g.id).layout_motion(ctx.motion_duration("normal"),ctx.motion_easing("standard"))
}
fn group_head(ctx,id,title,amount){
    let value=ctx.format_number(amount);
    let count=text(value).with_key("count").with_style(ctx.component_style("count",style().typography("caption")
        .padding_x(theme_length("space.spacing_unit").times(1.25)).radius(px(4)).background(theme_color("surface_hover")).font_feature("tnum",1)))
        .motion(motion_transition("translate_y_fraction",0.35,0.0,#{duration_ms:220,intent:"feedback"}))
        .motion_on_change(value);
    row([heading(ctx,title,"group_title"),count]).with_key(id).audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("group_head",style().min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(1.5)).padding(theme_length("space.spacing_unit").times(1.25))))
}
fn render_AutonomyDial(ctx,props){
    let policy=target(props);let enabled=!props.busy&&review::opt(props,"on_change")!=();
    let own=[];let ask=[];let seen=#{};
    for g in props.actions{
        if g.id==""||utf8_byte_count(g.id)>128||g.id in seen{throw "Autonomy actions require unique 1–128 byte IDs";}
        seen[g.id]=true;
        let locked=g.id in props.pinned;let asks=asks_first(g,props.level,props.pinned);
        let row=action_row(ctx,policy,g,locked,enabled,asks);
        if asks{ask.push(row);}else{own.push(row);}
    }
    let options=[#{value:"always",label:ctx.t_or("dial.always","Always ask")},#{value:"risky",label:ctx.t_or("dial.risky","If risky")},
        #{value:"auto",label:ctx.t_or("dial.auto","Full auto")}];
    let level=radio::RadioGroup(#{key:"level",label:ctx.t_or("dial.level","Autonomy"),value:props.level,appearance:"chip",
        orientation:"horizontal",options:options,disabled:!enabled,on_change:Fn("level_changed").curry(policy),
        part_styles:#{root:ctx.component_style("levels",style().min_width(px(0)).flex_wrap().gap(theme_length("space.spacing_unit"))),
            option:ctx.component_style("level",style().flex_grow())}});
    let labels=[heading(ctx,review::value_or(props,"title",ctx.t_or("dial.title","Agent permissions")),"title"),
        note(ctx,review::value_or(props,"subtitle",ctx.t_or("dial.subtitle","What it may do without asking you")),"subtitle")];
    let head=row([row([glyph(ctx,"spark")]).with_style(ctx.component_style("mark",style().width(theme_length("space.unit").times(6.75)).height(theme_length("space.unit").times(6.75))
        .radius(px(7)).flex_shrink(false).items_center().justify_center().background(theme_color("surface_hover")))),
        column(labels).with_style(style().min_width(px(0)).flex_grow())]).with_key("header")
        .with_style(ctx.component_style("header",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).items_center()));
    let card=column([head,level]).with_key("card").with_style(ctx.component_style("card",style().min_width(px(0))
        .padding(theme_length("space.spacing_unit").times(2.25)).gap(theme_length("space.spacing_unit").times(2)).radius(theme_length("space.unit").times(3)).background(theme_color("surface_raised"))));
    // Keep both headings and all keyed rows in one parent so reclassification
    // doesn't replace the focused pin button or invent a second row instance.
    let rows=[group_head(ctx,"own",ctx.t_or("dial.own","Runs on its own"),own.len)];
    if own.len>0{for row in own{rows.push(row);}}
    else{rows.push(note(ctx,ctx.t_or("dial.none_own","Every action waits for you"),"empty").with_key("empty-own"));}
    rows.push(group_head(ctx,"ask",ctx.t_or("dial.asks","Asks you first"),ask.len));
    if ask.len>0{for row in ask{rows.push(row);}}
    else{rows.push(note(ctx,ctx.t_or("dial.none_ask","Nothing waits for confirmation"),"empty").with_key("empty-ask"));}
    let groups=column(rows).with_key("groups").audit_allow(["text-edge-misaligned"])
        .accessibility_role("list").accessibility_label(ctx.t_or("dial.actions","Actions by confirmation policy"))
        .with_style(ctx.component_style("groups",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(1.5)).gap(theme_length("space.spacing_unit").times(0.75))));
    let children=[card,motion_group("policy-actions",[groups])];let message=review::opt(props,"message");
    if message!=(){children.push(note(ctx,message,"message").with_key("message").accessibility_role("status"));}
    column(children).with_key(props.key).accessibility_role("region").accessibility_label(ctx.t_or("dial.title","Agent permissions"))
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(450))
            .padding(theme_length("space.spacing_unit")).radius(theme_length("space.unit").times(4)).gap(theme_length("space.spacing_unit")).background(theme_color("surface_hover"))))
}
