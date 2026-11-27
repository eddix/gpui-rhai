/* gpui-rhai
{"id":"components/ai_answer","export":"AIAnswer","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/cited_answer","components/message_actions","components/thinking_loader"],"capabilities":{},"assets":[],"tokens":["accent","border","surface_raised","surface_hover","text_primary","text_muted","radius.lg","typography.body","typography.caption","space.spacing_unit"],"environment":["density"]}
*/
// AIAnswer composes a streamed reply, evidence, status, feedback and follow-ups.
// State: the Host owns the document, status, streaming and business requests.
// Example: AIAnswer(#{key:"reply",question:"What changed?",source:"The test now passes."})
import "components/button" as button;
import "components/cited_answer" as cited_answer;
import "components/message_actions" as message_actions;
import "components/thinking_loader" as thinking_loader;

define_component(#{
    metadata:#{id:"components/ai_answer","export":"AIAnswer",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/cited_answer","components/message_actions","components/thinking_loader"],capabilities:#{},assets:[],
        tokens:["accent","border","surface_raised","surface_hover","text_primary","text_muted","radius.lg","typography.body","typography.caption","space.spacing_unit"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable reply ID across Host document revisions."},
        question:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional question above the reply."},
        label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accessible response label."},
        source:#{schema:#{type:"optional",value:#{type:"one_of",variants:[#{type:"string"},#{type:"document"}]}},required:false,sensitive:false,
            doc:"Accepted Markdown text or immutable Host document. Pass updated revisions during writing. Links with gpui-cite:1, gpui-cite:2, etc. cite the corresponding source."},
        sources:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable evidence ID."},
            site:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Short source name."},
            title:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Source headline."},
            url:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Opaque destination; nothing is opened automatically."},
            date:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Ready-to-display date."},
            quote:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Evidence excerpt."},
            highlight:#{schema:#{type:"optional",value:#{type:"array",max_items:16,items:#{type:"string"}}},required:false,sensitive:false,
                doc:"Exact phrases emphasized in the excerpt."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative logo for the source."},
        }}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Evidence available for this reply."},
        status:#{schema:#{type:"string",allowed:["reading","writing","done","stopped"]},
            "default":#{type:"string",value:"done"},required:false,sensitive:false,doc:"Application-owned phase; stopped preserves the latest accepted content."},
        framed:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Use the standalone answer card instead of embedded conversation presentation."},
        follow_ups:#{schema:#{type:"array",max_items:12,items:#{type:"string"}},required:false,sensitive:false,
            "default":#{type:"array",value:[]},doc:"Suggested prompts shown after completion."},
        feedback_reasons:#{schema:#{type:"optional",value:#{type:"array",max_items:8,items:#{type:"string"}}},
            required:false,sensitive:false,doc:"Optional quick reasons supplied to MessageActions."},
        copy_text:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,
            doc:"Explicit clipboard text. Inline Markdown can copy its literal source by default; native documents remain opaque."},
        show_actions:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Show available copy, retry and feedback actions when idle."},
        on_stop:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Request stopping; the application changes status and cancels its producer."},
        on_regenerate:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Ask the application to regenerate."},
        on_feedback:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Feedback #{version,kind,reason}; version is zero for this reply."},
        on_copy:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Reports committed copy without including contents."},
        on_follow_up:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Proposes the selected prompt text."},
        on_open_source:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Explicit open-source request #{id,url}."},
        on_link:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Ordinary Markdown link activation #{href,text}."},
    },state:#{fields:#{}},events:#{
        stop:#{payload:#{type:"null"},doc:"User asked to stop."},
        regenerate:#{payload:#{type:"null"},doc:"User asked to regenerate."},
        copy:#{payload:#{type:"null"},doc:"Text copied as part of the input transaction."},
        follow_up:#{payload:#{type:"string"},doc:"The selected follow-up prompt."},
        feedback:#{payload:#{type:"object",allow_unknown:false,fields:#{
            version:#{schema:#{type:"integer"},required:true,sensitive:false},
            kind:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:false},
            reason:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:true},
        }},doc:"Vote and optional reason."},
        open_source:#{payload:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false},
            url:#{schema:#{type:"string"},required:true,sensitive:false},
        }},doc:"Explicit source activation."},
        link:#{payload:#{type:"object",allow_unknown:false,fields:#{
            href:#{schema:#{type:"string"},required:true,sensitive:false},
            text:#{schema:#{type:"string"},required:true,sensitive:false},
        }},doc:"Non-citation link activated."},
    },slots:#{},parts:["root","framed","card","question","status_row","status","indicator","stopped","answer","actions","follow_ups","follow_label","follow_button","stop"]},
    render:Fn("render_AIAnswer")
});

fn AIAnswer(props){render_component("components/ai_answer",props)}
fn opt(props,key){if key in props{props[key]}else{()}}
fn has_callback(props,key){opt(props,"on_"+key)!=()}
fn forwarded(props,event){if has_callback(props,event){Fn("pass").curry(event)}else{()}}
fn pass(event,ctx,value){ctx.emit(event,value);handled()}
fn request(event,ctx,payload){ctx.emit(event,());handled()}
fn follow(prompt,ctx,payload){ctx.emit("follow_up",prompt);handled()}
fn phase_view(ctx,props){
    let stopped=props.status=="stopped";
    let description=if props.status=="reading"{ctx.t_or("answer.reading","Reading sources…")}
        else if stopped{ctx.t_or("answer.stopped","Stopped")}else{ctx.t_or("answer.writing","Writing answer…")};
    let children=[thinking_loader::ThinkingLoader(#{key:"activity",phases:[description],phase:0,
        stopped:stopped,variant:"plain",size:"sm",part_styles:#{
            icon:ctx.component_style("indicator",style()),
            label:ctx.component_style(if stopped{["status","stopped"]}else{["status"]},style().typography("caption")
                .text_color(theme_color("text_muted"))),
        }})];
    if !stopped&&has_callback(props,"stop"){
        children.push(button::Button(#{key:"stop",text:ctx.t_or("answer.stop","Stop answer"),variant:"ghost",
            size:"sm",on_click:Fn("request").curry("stop")}).with_style(ctx.component_style("stop",style())));
    }
    row(children).with_key("phase-row").with_style(ctx.component_style("status_row",style().gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap()))
}
fn render_AIAnswer(ctx,props){
    let label=opt(props,"label");if label==(){label=ctx.t_or("answer.title","Answer");}
    let pending=props.status=="reading"||props.status=="writing";
    let question=opt(props,"question");let source=opt(props,"source");
    let children=[];
    if question!=()&&question!=""{children.push(text(question).with_key("question").selectable(true)
        .with_style(ctx.component_style("question",style().typography("body").font_weight(600))));}
    if pending||props.status=="stopped"{children.push(phase_view(ctx,props));}
    if source!=()||props.sources.len>0{
        children.push(cited_answer::CitedAnswer(#{key:"reply-body",label:label,source:source,sources:props.sources,
            show_header:false,show_tray:false,on_open_source:forwarded(props,"open_source"),on_link:forwarded(props,"link"),
            part_styles:#{root:ctx.component_style("answer",style())}}));
    }
    if !pending&&props.show_actions{
        let copy=opt(props,"copy_text");
        if copy==()&&type_of(source)=="string"&&source!=""{copy=source;}
        let can_retry=has_callback(props,"regenerate");
        let can_vote=has_callback(props,"feedback");
        if copy!=()||can_retry||can_vote{
            children.push(message_actions::MessageActions(#{key:"reply-actions",text:copy,
                reasons:opt(props,"feedback_reasons"),actions:["copy","regenerate","feedback"],
                on_copy:if has_callback(props,"copy"){Fn("request").curry("copy")}else{()},
                on_regenerate:if can_retry{Fn("request").curry("regenerate")}else{()},
                on_feedback:forwarded(props,"feedback")})
                .with_style(ctx.component_style("actions",style())));
        }
    }
    if props.status=="done"&&has_callback(props,"follow_up")&&props.follow_ups.len>0{
        let follow_ups=[];let seen=[];
        for prompt in props.follow_ups{
            if prompt==""||prompt in seen{continue;}seen.push(prompt);
            follow_ups.push(button::Button(#{key:`follow-${follow_ups.len}`,text:prompt,variant:"ghost",size:"sm",
                on_click:Fn("follow").curry(prompt)})
                .with_style(ctx.component_style("follow_button",style().max_width(relative(1.0)))));
        }
        children.push(column([text(ctx.t_or("answer.next","Follow-up questions")).with_style(ctx.component_style("follow_label",style()
            .typography("caption").text_color(theme_color("text_muted")))),
            column(follow_ups).with_style(style().gap(theme_length("space.spacing_unit").times(0.5)).min_width(px(0)))]).with_key("follow-ups")
            .with_style(ctx.component_style("follow_ups",style().min_width(px(0)).gap(theme_length("space.spacing_unit")))));
    }
    let appearance=if props.framed{["root","framed"]}else{["root"]};
    let presentation=if props.framed{[column(children).with_key("card").with_style(ctx.component_style("card",
        style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2))))]}else{children};
    column(presentation).with_key(props.key).accessibility_role("region").accessibility_label(label).tab_stop(false)
        .with_style(ctx.component_style(appearance,style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).text_color(theme_color("text_primary"))))
}
