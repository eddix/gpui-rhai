/* gpui-rhai
{"id":"components/message_actions","export":"MessageActions","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/accordion","components/button","components/textarea"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","surface_raised","surface_hover","text_muted","text_primary","text.success","radius.sm","typography.caption","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// MessageActions presents clipboard, version, speech request and feedback controls for one message.
// State: local feedback and transient indicators; version and playback can be application-owned.
// Example: MessageActions(#{key:"response",text:"The tests passed",versions:2})
import "components/accordion" as accordion;
import "components/button" as button;
import "components/textarea" as textarea;

define_component(#{
    metadata:#{id:"components/message_actions","export":"MessageActions",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/accordion","components/button","components/textarea"],capabilities:#{},assets:[],
        tokens:["accent","border","focus_ring","surface_raised","surface_hover","text_muted","text_primary","text.success","radius.sm","typography.caption","space.spacing_unit","space.unit"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable message identity; changing it resets local feedback."},
        text:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Text for the Copy action. Omit to hide Copy."},
        label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accessible group name."},
        actions:#{schema:#{type:"array",max_items:4,items:#{type:"string",allowed:["copy","regenerate","feedback","speak"]}},
            "default":#{type:"array",value:[#{type:"string",value:"copy"},#{type:"string",value:"regenerate"},#{type:"string",value:"feedback"},#{type:"string",value:"speak"}]},
            required:false,sensitive:false,doc:"Action groups in display order."},
        versions:#{schema:#{type:"integer",min:1,max:1000},"default":#{type:"integer",value:1},required:false,sensitive:false,doc:"Number of versions."},
        version:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:999}},required:false,sensitive:false,doc:"Accepted zero-based version. Omit for local navigation."},
        default_version:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:999}},required:false,sensitive:false,doc:"Initial local version; defaults to newest."},
        regenerating:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Application-owned busy state."},
        speaking:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Application-owned speech playback state."},
        quiet:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Deemphasize the inactive bar, restoring ink on hover or keyboard focus."},
        reasons:#{schema:#{type:"optional",value:#{type:"array",max_items:8,items:#{type:"string"}}},required:false,sensitive:false,doc:"Quick reasons; a free-text Other option is always available."},
        on_copy:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the version after queuing the transaction's clipboard write."},
        on_regenerate:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Request another version."},
        on_version_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Proposed version index."},
        on_feedback:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Feedback map with version, kind, and optional reason."},
        on_speak:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Proposed version and playing state. No speech engine is invoked."},
    },state:#{fields:#{
        local_version:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        seen_versions:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
        votes:#{schema:#{type:"ui_value"},"default":#{type:"map",value:#{}}},
        panel:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        panel_focused:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        reason:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
        other:#{schema:#{type:"string"},"default":#{type:"string",value:""},sensitive:true},
        sent:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        copied:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        copy_serial:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    }},events:#{
        copy:#{payload:#{type:"integer"},doc:"Version copied; message contents are not included."},
        version_change:#{payload:#{type:"integer"},doc:"Proposed zero-based version."},
        regenerate:#{payload:#{type:"integer"},doc:"Version to regenerate."},
        speak:#{payload:#{type:"object",allow_unknown:false,fields:#{
            version:#{schema:#{type:"integer"},required:true,sensitive:false},
            playing:#{schema:#{type:"bool"},required:true,sensitive:false},
        }},doc:"Playback request."},
        feedback:#{payload:#{type:"object",allow_unknown:false,fields:#{
            version:#{schema:#{type:"integer"},required:true,sensitive:false},
            kind:#{schema:#{type:"optional",value:#{type:"string",allowed:["up","down"]}},required:true,sensitive:false},
            reason:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:true},
        }},doc:"Vote or reason. Unit kind clears a vote."},
    },slots:#{},parts:["root","bar","quiet","action","glyph","pressed","copied","counter","current","total",
        "feedback","heading","reasons","reason","other","send","thanks"],effects:["sync_versions"]},
    render:Fn("render_MessageActions")
});

fn MessageActions(props){render_component("components/message_actions",props)}
fn optional(props,name){if name in props{props[name]}else{()}}
fn enabled(props,name){optional(props,"on_"+name)!=()}
fn sync_versions(ctx,pair){
    ctx.set_state("seen_versions",pair[0]);ctx.set_state("local_version",pair[1]);
    if ctx.get_state("panel")!=()&&ctx.get_state("panel")!=pair[2]{dismiss(ctx,());}
}
fn cleanup(ctx,pair){}
fn choose(index,controlled,ctx,payload){
    if !controlled{ctx.set_state("local_version",index);}
    ctx.emit("version_change",index);
    handled()
}
fn copy(version,value,ctx,payload){
    ctx.copy_text(value);ctx.emit("copy",version);
    let n=ctx.get_state("copy_serial")+1;ctx.set_state("copy_serial",n);
    ctx.set_state("copied",#{version:version,serial:n});handled()
}
fn expire_copy(ctx,expected){if ctx.get_state("copied")==expected{ctx.set_state("copied",());}}
fn fire(event,value,ctx,payload){ctx.emit(event,value);handled()}
fn dismiss(ctx,payload){
    ctx.set_state("panel",());ctx.set_state("reason",());ctx.set_state("other","");ctx.set_state("sent",false);
    ctx.set_state("panel_focused",false);
    handled()
}
fn close(ctx,payload){dismiss(ctx,());ctx.focus("feedback-trigger");handled()}
fn close_key(ctx,event){close(ctx,());}
fn panel_focus(ctx,focused){ctx.set_state("panel_focused",focused);}
fn vote(version,kind,ctx,payload){
    let votes=ctx.get_state("votes");let id=version.to_string();
    let next=if id in votes&&votes[id]==kind{()}else{kind};
    votes[id]=next;ctx.set_state("votes",votes);
    dismiss(ctx,());
    if next=="down"{ctx.set_state("panel",version);}
    ctx.emit("feedback",#{version:version,kind:next,reason:()});
    handled()
}
fn reason_click(spec,ctx,payload){ctx.set_state("reason",spec);handled()}
fn edit_reason(ctx,value){ctx.set_state("other",value);}
fn send_reason(version,ctx,payload){
    let spec=ctx.get_state("reason");
    if ctx.get_state("panel")!=version||ctx.get_state("sent")||spec==(){return handled();}
    let reason=if spec.custom{ctx.get_state("other")}else{spec.label};reason.trim();
    if reason==""{return handled();}
    ctx.emit("feedback",#{version:version,kind:"down",reason:reason});ctx.set_state("sent",true);
    handled()
}
fn expire_feedback(ctx,version){
    if ctx.get_state("panel")==version&&ctx.get_state("sent"){
        if ctx.get_state("panel_focused"){close(ctx,());}else{dismiss(ctx,());}
    }
}
fn glyph(ctx,id){
    let path=switch id{
        "copy"=>"M9 9h12v12H9zM5 16H3V3h13v2",
        "done"=>"m5 12 4 4L19 6",
        "regenerate"=>"M20 7V3m0 4h-4m4 0A9 9 0 1 0 21 14",
        "up"=>"M7 10 11 3c2 0 3 2 2 5h5a2 2 0 0 1 2 2l-2 9H7zm-4 0h4v9H3z",
        "down"=>"M7 14l4 7c2 0 3-2 2-5h5a2 2 0 0 0 2-2l-2-9H7zm-4 0h4V5H3z",
        "speak"=>"M4 10v4h4l5 4V6l-5 4zm12-2a5 5 0 0 1 0 8m3-11a9 9 0 0 1 0 14",
        "stop"=>"M7 7h10v10H7z",
        "prev"=>"m15 6-6 6 6 6",
        "next"=>"m9 6 6 6-6 6",
        _=>"m6 6 12 12m0-12L6 18"
    };
    svg(`<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("glyph",style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4))))
}
fn icon_action(ctx,id,icon,label,callback,pressed,busy){
    let graphic=if busy{button::loading_spinner(ctx)}else{glyph(ctx,icon)};
    let parts=if icon=="done"{["action","copied"]}else if pressed==true{["action","pressed"]}else{["action"]};
    let node=row([graphic]).with_key(id).accessibility_role("button").accessibility_label(label)
        .disabled(callback==()||busy).tab_stop(callback!=()&&!busy)
        .with_style(ctx.component_style(parts,style().width(theme_length("space.unit").times(7)).height(theme_length("space.unit").times(7)).items_center().justify_center()
            .flex_shrink(false).radius(theme_radius("sm")).text_color(theme_color("text_muted"))
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .paint_transition(160,cubic_bezier(0.22,1.0,0.36,1.0));
    if pressed!=(){node=node.accessibility_pressed(pressed);}
    if callback!=()&&!busy{node=node.on_click(callback);}
    node
}
fn feedback_panel(ctx,props,version){
    let reasons=optional(props,"reasons");if reasons==(){reasons=[ctx.t_or("message.not_accurate","Not accurate"),
        ctx.t_or("message.not_helpful","Not helpful"),ctx.t_or("message.incomplete","Incomplete")];}
    let picked=ctx.get_state("reason");let sent=ctx.get_state("sent");let chips=[];let seen=[];
    for label in reasons{
        if label==""||label in seen{continue;}seen.push(label);
        let data=#{label:label,custom:false};
        chips.push(button::Button(#{key:`reason-${chips.len}`,text:label,variant:if picked==data{"secondary"}else{"ghost"},size:"sm",
            disabled:sent,on_click:Fn("reason_click").curry(data)})
            .accessibility_pressed(picked==data)
            .with_style(ctx.component_style("reason",style().max_width(relative(1.0)))));
    }
    let other=#{label:ctx.t_or("message.other","Other"),custom:true};
    chips.push(button::Button(#{key:"other-reason",text:other.label,variant:if picked==other{"secondary"}else{"ghost"},size:"sm",
        disabled:sent,on_click:Fn("reason_click").curry(other)})
        .accessibility_pressed(picked==other)
        .with_style(ctx.component_style("reason",style())));
    let heading=row([text(ctx.t_or("message.why","What went wrong?")).with_style(ctx.component_style("heading",style().font_weight(500))),
        icon_action(ctx,"close","close",ctx.t_or("message.close_feedback","Close feedback"),Fn("close"),(),false)])
        .with_style(style().items_center().justify_between().gap(theme_length("space.spacing_unit").times(1.5)));
    let content=[heading,row(chips).with_style(ctx.component_style("reasons",style().flex_wrap().gap(theme_length("space.spacing_unit"))))];
    let draft=ctx.get_state("other");
    if picked==other{
        content.push(textarea::Textarea(#{key:"other",label:ctx.t_or("message.other_reason","Describe the issue"),value:draft,
            rows:1,max_length:500,disabled:sent,on_change:Fn("edit_reason")}).with_style(ctx.component_style("other",style().min_width(px(0)))));
    }
    draft.trim();
    let submit_label=if sent{ctx.t_or("message.thanks","Thanks for the feedback")}else{ctx.t_or("message.send","Send feedback")};
    let send=button::Button(#{key:"send",text:submit_label,variant:"primary",size:"sm",
        disabled:picked==()||sent||(picked==other&&draft==""),on_click:Fn("send_reason").curry(version)})
        .with_style(ctx.component_style(if sent{["send","thanks"]}else{["send"]},style()));
    content.push(row([send]).with_style(style().justify_end()));
    column(content).with_key("feedback-content").accessibility_role("group")
        .accessibility_label(ctx.t_or("message.feedback","Response feedback"))
        .with_style(ctx.component_style("feedback",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2.5)).gap(theme_length("space.spacing_unit").times(1.5))
            .background(theme_color("surface_raised")).radius(theme_radius("sm")).border(px(1)).border_color(theme_color("border"))))
        .on_focus_within_change(Fn("panel_focus"))
        .on_key_value("escape",Fn("close_key"),())
}
fn render_MessageActions(ctx,props){
    let external=optional(props,"version");
    let old=ctx.get_state("local_version");let previous=ctx.get_state("seen_versions");
    let initial=optional(props,"default_version");
    let local=min(max(if previous==0{if initial==(){props.versions-1}else{initial}}
        else if props.versions>previous||old==(){props.versions-1}else{old},0),props.versions-1);
    let version=if external==(){local}else{min(max(external,0),props.versions-1)};
    effect("sync_versions",[props.versions,local,version],Fn("sync_versions"),Fn("cleanup"));
    let copied=ctx.get_state("copied");
    if copied!=(){timeout("expire-copy",1600,false,Fn("expire_copy"),copied);}
    let panel=ctx.get_state("panel")==version&&"feedback" in props.actions&&enabled(props,"feedback");
    if panel&&ctx.get_state("sent"){timeout("expire-feedback",1500,false,Fn("expire_feedback"),version);}
    let votes=ctx.get_state("votes");let id=version.to_string();let selected=if id in votes{votes[id]}else{()};
    let children=[];let seen=[];
    for action in props.actions{
        if action in seen{continue;}seen.push(action);
        if action=="copy"&&optional(props,"text")!=(){
            let ok=copied!=()&&copied.version==version;
            children.push(icon_action(ctx,"copy",if ok{"done"}else{"copy"},if ok{ctx.t_or("message.copied","Copied")}else{ctx.t_or("message.copy","Copy response")},
                Fn("copy").curry(version,props.text),(),false));
        }else if action=="regenerate"&&enabled(props,"regenerate"){
            children.push(icon_action(ctx,"regenerate","regenerate",ctx.t_or("message.regenerate","Regenerate response"),
                Fn("fire").curry("regenerate",version),(),props.regenerating));
        }else if action=="feedback"&&enabled(props,"feedback"){
            children.push(icon_action(ctx,"up","up",ctx.t_or("message.up","Good response"),Fn("vote").curry(version,"up"),selected=="up",false));
            children.push(icon_action(ctx,"down","down",ctx.t_or("message.down","Bad response"),Fn("vote").curry(version,"down"),selected=="down",false)
                .with_ref(element_ref("feedback-trigger")).accessibility_expanded(panel));
        }else if action=="speak"&&enabled(props,"speak"){
            children.push(icon_action(ctx,"speak",if props.speaking{"stop"}else{"speak"},
                if props.speaking{ctx.t_or("message.stop","Stop reading")}else{ctx.t_or("message.speak","Read aloud")},
                Fn("fire").curry("speak",#{version:version,playing:!props.speaking}),props.speaking,false));
        }
    }
    if props.versions>1{
        let rtl=ctx.text_direction()=="rtl";
        let counter=row([text(ctx.format_number(version+1)).with_key("number").with_style(ctx.component_style("current",style().font_weight(500)))
                .motion(motion_transition("translate_y",5.0,0.0,#{duration_ms:240,intent:"feedback"})).motion_on_change(id),
            text("/"+ctx.format_number(props.versions)).with_style(ctx.component_style("total",style().text_color(theme_color("text_muted"))))])
            .with_key("version-count").test_id("message-version").with_style(ctx.component_style("counter",style().clip().typography("caption").font_feature("tnum",1)))
            .accessibility_role("status").accessibility_label(ctx.t_or("message.version","Version {current} of {total}",
                #{current:ctx.format_number(version+1),total:ctx.format_number(props.versions)}));
        let can_pick=external==()||enabled(props,"version_change");
        children.push(row([
            icon_action(ctx,"previous",if rtl{"next"}else{"prev"},ctx.t_or("message.previous","Previous version"),
                if can_pick&&version>0{Fn("choose").curry(version-1,external!=())}else{()},(),false),
            counter,
            icon_action(ctx,"next",if rtl{"prev"}else{"next"},ctx.t_or("message.next","Next version"),
                if can_pick&&version+1<props.versions{Fn("choose").curry(version+1,external!=())}else{()},(),false),
        ]).with_key("versions").with_style(style().items_center().flex_shrink(false)));
    }
    let title=optional(props,"label");if title==(){title=ctx.t_or("message.actions","Message actions");}
    let quiet=props.quiet&&!panel&&!props.speaking&&!props.regenerating&&copied==();
    let bar=row(children).with_key("actions")
        .with_style(ctx.component_style(if quiet{["bar","quiet"]}else{["bar"]},style().flex_wrap().items_center().gap(theme_length("space.spacing_unit").times(0.5))
            .opacity(if quiet{0.68}else{1.0}).hover(style().opacity(1.0)).group_focus(style().opacity(1.0))))
        .paint_transition(150,cubic_bezier(0.22,1.0,0.36,1.0));
    let nodes=[bar];
    if panel{nodes.push(accordion::disclosure_panel(ctx,"feedback-panel",true,(),feedback_panel(ctx,props,version)));}
    column(nodes).with_key(props.key).accessibility_role("group").accessibility_label(title)
        .tab_stop(false).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit"))))
}
