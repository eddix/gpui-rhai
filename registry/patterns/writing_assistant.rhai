/* gpui-rhai
{"id":"patterns/writing_assistant","export":"WritingAssistant","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/icon_button","components/textarea"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","on_accent","surface_hover","surface_raised","text.danger","text_muted","text_primary","typography.body","typography.caption","writing.glow_blue","writing.glow_pink","writing.glow_violet","space.spacing_unit","space.unit","typography.title"],"environment":["density"]}
*/
// WritingAssistant shows a controlled conversation, suggestion chips, an editor and optional dictation.
// State: native scroll following only. The application owns messages, drafts, microphone access and work.
// Example: WritingAssistant(#{key:"writer",conversation:"first",revision:1,messages:[],draft:""})
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/textarea" as textarea;

define_component(#{
    metadata:#{id:"patterns/writing_assistant","export":"WritingAssistant",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/icon_button","components/textarea"],capabilities:#{},
        tokens:["accent","border","focus_ring","on_accent","surface_hover","surface_raised","text.danger","text_muted","text_primary","typography.body","typography.caption","writing.glow_blue","writing.glow_pink","writing.glow_violet","space.spacing_unit","space.unit","typography.title"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of this assistant surface."},
        conversation:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of the accepted conversation; choose a new ID on reset."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Host revision echoed in every proposal."},
        name:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Assistant name."},
        tagline:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Detail underneath the name."},
        greeting:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Welcome heading."},
        placeholder:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Empty native editor hint."},
        messages:#{schema:#{type:"array",max_items:512,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable message ID; keep it unchanged when updating a streamed answer."},
            role:#{schema:#{type:"string",allowed:["user","assistant"]},required:true,sensitive:false,doc:"Who authored this message."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Current accepted text. Empty partial messages are not drawn."},
        }}},required:true,sensitive:false,doc:"Accepted messages in display order; at most 512 KiB of text."},
        suggestions:#{schema:#{type:"array",max_items:8,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique suggestion identity."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible and accessible chip caption."},
            prompt:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Text to propose, defaulting to label."},
        }}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Welcome actions, identified by stable caller keys."},
        draft:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Accepted editor value up to 4,000 graphemes. Do not clear it until you accept send."},
        phase:#{schema:#{type:"string",allowed:["idle","waiting","streaming","failed"]},"default":#{type:"string",value:"idle"},
            required:false,sensitive:false,doc:"Accepted request/answer phase. Waiting/streaming block a second send."},
        draft_locked:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,
            doc:"Make the draft read-only while the Host decides whether to accept its send. Otherwise the editor stays available during a response."},
        message:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Host-safe status or error shown without replacing the draft."},
        speech:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            state:#{schema:#{type:"string",allowed:["idle","starting","listening","stopping","failed"]},required:true,sensitive:false,doc:"Dictation state accepted by the Host."},
            lease:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted dictation session ID. Stop proposes this lease, not any earlier take."},
            preview:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Interim display only, never proposed for send."},
        }}},required:false,sensitive:false,doc:"Omit to hide dictation. The Host owns capture and authorization."},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Blocks new application actions."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives #{conversation,revision,text}."},
        on_send:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives #{conversation,revision,text,source,suggestion}, with an empty suggestion ID for draft sends."},
        on_reset:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives #{conversation,revision} and leaves reset to the application."},
        on_dictate:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives #{conversation,revision,action,lease}. Never opens the microphone itself."},
    },state:#{fields:#{follow:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},events:#{
        change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            conversation:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Conversation the edit targeted."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision of the edit source."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"New draft text, including newlines."},
        }},doc:"Propose a draft change, without submitting it."},
        send:#{payload:#{type:"object",allow_unknown:false,fields:#{
            conversation:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Conversation the send targeted."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision of the send source."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Trimmed message text."},
            source:#{schema:#{type:"string",allowed:["draft","suggestion"]},required:true,sensitive:false,doc:"Whether an editor or a suggestion initiated the send."},
            suggestion:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Selected suggestion ID, or empty for an editor send."},
        }},doc:"Propose typed text or one explicitly chosen suggestion."},
        reset:#{payload:#{type:"object",allow_unknown:false,fields:#{
            conversation:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Current conversation that may be reset."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Expected revision at reset."},
        }},doc:"Request a new conversation."},
        dictate:#{payload:#{type:"object",allow_unknown:false,fields:#{
            conversation:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Conversation that requested capture."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Expected revision of the request."},
            action:#{schema:#{type:"string",allowed:["start","stop"]},required:true,sensitive:false,doc:"Requested dictation action."},
            lease:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Last accepted capture lease, if any."},
        }},doc:"Request a Host dictation action; no speech data is emitted."},
    },slots:#{},parts:["root","header","name","tagline","reset","deck","body","stage","welcome","greeting","suggestions","suggestion",
        "transcript","row","user","assistant","typing","dot","jump","controls","field","editor","send","mic","mic_active","wave","bar","feedback",
        "glow_pink","glow_blue","glow_violet","glow_chat","icon"]},
    render:Fn("render_WritingAssistant")
});

fn WritingAssistant(props){render_component("patterns/writing_assistant",props)}
fn opt(map,key){if key in map{map[key]}else{()}}
fn caption(map,key,fallback){let value=opt(map,key);if value==()||value==""{fallback}else{value}}
fn can(props,event){!props.disabled&&opt(props,"on_"+event)!=()}
fn target(props){#{conversation:props.conversation,revision:props.revision}}
fn field_text(value){let out=value;out.trim();out}
fn valid_send(value){value!=""&&utf8_byte_count(value)<=16000}
fn request(event,value,ctx,payload){ctx.emit(event,value);handled()}
fn changed(token,ctx,value){token.text=value;ctx.emit("change",token);}
fn submit(token,source,suggestion,ctx,value){
    let clean=field_text(value);
    if valid_send(clean){token.text=clean;token.source=source;token.suggestion=suggestion;ctx.emit("send",token);}
    handled()
}
fn send_click(token,raw,ctx,event){submit(token,"draft","",ctx,raw)}
fn choose(token,id,raw,ctx,event){submit(token,"suggestion",id,ctx,raw)}
fn track_follow(conversation,ctx,value){ctx.set_state("follow",#{id:conversation,following:value.following});}
fn jump(key,ctx,event){ctx.scroll_collection_to_end(key);handled()}
fn glyph(ctx,id,side){
    let path=if id=="send"{"M12 18.5v-13M6.5 11 12 5.5l5.5 5.5"}
        else if id=="reset"{"M12 5.5v13M5.5 12h13"}
        else{"M12 3.5a3 3 0 0 1 3 3v5a3 3 0 0 1-6 0v-5a3 3 0 0 1 3-3ZM6 11.5a6 6 0 0 0 12 0M12 17.5v3"};
    svg(`<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("icon",style().width(px(side)).height(px(side)).flex_shrink(false)))
}
fn cycle_motion(property,start,mid,finish,duration,intent){
    motion_keyframes(property,[#{offset:0.0,value:start},#{offset:0.5,value:mid,easing:"ease_in_out"},
        #{offset:1.0,value:finish,easing:"ease_in_out"}],#{duration_ms:duration,iterations:"infinite",intent:intent})
}
fn glow(ctx,part,tint,chat,pose,width,height,sigma,delta,duration){
    let parts=[part];if chat{parts.push("glow_chat");}
    // This shape does not depend on destination pixels. Cache its transparent
    // Gaussian source as SVG artwork and move it with native motion. Live
    // backdrop filtering remains a separate operation on the controls.
    let halo=sigma*4;let w=width+halo*2;let h=height+halo*2;let x=halo+width*0.5;let y=halo+height*0.5;
    let shape=svg(`<svg viewBox='0 0 ${w} ${h}' width='${w}' height='${h}'><defs><filter id='blur' filterUnits='userSpaceOnUse' x='0' y='0' width='${w}' height='${h}'><feGaussianBlur stdDeviation='${sigma}'/></filter></defs><ellipse cx='${x}' cy='${y}' rx='${width*0.5}' ry='${height*0.5}' fill='currentColor' filter='url(#blur)'/></svg>`)
        .with_style(style().absolute().left(offset_relative(-halo/width)).right(offset_relative(-halo/width))
            .top(offset_relative(-halo/height)).bottom(offset_relative(-halo/height)));
    let light=row([shape]).with_key(part).accessibility_hidden(true).with_style(ctx.component_style(parts,pose.absolute()
        .text_color(theme_color(tint)).opacity(if chat{0.17}else{0.28})));
    light.motion(cycle_motion("translate_x_fraction",0.0,delta,0.0,duration,"decorative"))
        .motion(cycle_motion("translate_y_fraction",0.0,-0.2,0.0,duration,"decorative"))
}
fn indicator(ctx,bars){
    let nodes=[];
    for i in 0..3{
        let levels=[0.42,0.7,1.0];let level=levels[i];
        let node=row([]).with_key(i.to_string()).with_style(ctx.component_style(if bars{"bar"}else{"dot"},
            style().width(px(if bars{3}else{6})).height(px(if bars{5+i*3}else{6})).radius(px(3))
                .background(theme_color(if bars{"on_accent"}else{"text_muted"}))));
        let light=cycle_motion("opacity",level,levels[(i+1)%3],level,960,"feedback");
        if bars{
            node=node.motion(cycle_motion("translate_y_fraction",0.0,if i%2==0{-0.26}else{0.23},0.0,1040,"feedback"));
        }
        nodes.push(node.motion(light));
    }
    row(nodes).accessibility_hidden(true).with_style(ctx.component_style(if bars{"wave"}else{"typing"},style().items_center().gap(theme_length("space.spacing_unit").times(0.75))))
}
fn welcome(ctx,props,blocked){
    let chips=[];let seen=#{};
    for item in props.suggestions{
        if item.id==""||utf8_byte_count(item.id)>128||item.id in seen{throw "WritingAssistant requires unique nonempty suggestion IDs up to 128 bytes";}
        let prompt=caption(item,"prompt",item.label);
        if utf8_byte_count(item.label)>512||!valid_send(field_text(prompt)){throw "WritingAssistant requires a bounded, nonempty suggestion";}
        seen[item.id]=true;
        chips.push(button::Button(#{key:item.id,text:item.label,variant:"ghost",size:"sm",disabled:blocked||!can(props,"send"),
            on_click:Fn("choose").curry(target(props),item.id,prompt)}).with_key(item.id)
            .with_style(ctx.component_style("suggestion",style().min_width(px(0)).max_width(relative(1.0)))));
    }
    let title=caption(props,"greeting",ctx.t_or("writer.greeting","What should we write?"));
    column([text(title).accessibility_role("heading").accessibility_level(4).with_key("greeting")
        .with_style(ctx.component_style("greeting",style().min_width(px(0)).typography("title"))),
        row(chips).with_style(ctx.component_style("suggestions",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap()))])
        .with_key("welcome").with_style(ctx.component_style("welcome",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(3))))
}
fn messages(props){
    let seen=#{};let data=[];let bytes=0;
    for item in props.messages{
        if item.id==""||utf8_byte_count(item.id)>128||item.id in seen{throw "WritingAssistant requires unique nonempty message IDs up to 128 bytes";}
        seen[item.id]=true;bytes+=utf8_byte_count(item.text);
        if bytes>524288{throw "WritingAssistant message collection exceeds 512 KiB";}
        if item.text!=""{item.key="message:"+item.id;data.push(item);}
    }
    if props.phase=="waiting"{data.push(#{key:"pending",role:"pending",text:""});}
    data
}
fn chat_row(ctx,payload){
    let item=payload.item;let user=item.role=="user";
    let bubble=if item.role=="pending"{
        row([indicator(ctx,false)]).accessibility_role("status").accessibility_label(ctx.t_or("writer.writing","Writing…"))
    }else{text(item.text).selectable(true)};
    let base=style().min_width(px(0)).max_width(relative(0.88))
        .padding_x(theme_length("space.spacing_unit").times(2.5)).padding_y(theme_length("space.spacing_unit").times(1.5)).typography("body")
        .radius(theme_length("space.unit").times(4)).background(theme_color(if user{"surface_hover"}else{"surface_raised"}));
    base=if user{base.self_end()}else{base.self_start()};
    if ctx.text_direction()=="rtl"{base=if user{base.radius_bottom_left(px(6))}else{base.radius_bottom_right(px(6))};}
    else{base=if user{base.radius_bottom_right(px(6))}else{base.radius_bottom_left(px(6))};}
    column([bubble.with_key("bubble").with_style(ctx.component_style(if user{"user"}else{"assistant"},base))])
        .with_key(payload.key).with_style(ctx.component_style("row",style().min_width(px(0)).padding_bottom(theme_length("space.spacing_unit").times(2))))
}
fn transcript(ctx,props,data){
    let key="writing:"+props.conversation;let label=ctx.t_or("writer.transcript","Writing conversation");
    let list=virtual_collection(#{key:key,label:label,data:data,estimated_height:48,fill_height:true,alignment:"bottom",follow_tail:true,
        on_follow_change:Fn("track_follow").curry(props.conversation)},Fn("chat_row"))
        .accessibility_role("list").accessibility_label(label).tab_stop(true)
        .with_style(ctx.component_style("transcript",style().min_width(px(0)).min_height(px(0)).flex_grow()
            .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))));
    let children=[list];let follow=ctx.get_state("follow");
    if follow!=()&&follow.id==props.conversation&&!follow.following{
        children.push(button::Button(#{key:"jump",text:ctx.t_or("writer.latest","Jump to latest"),size:"xs",variant:"ghost",
            on_click:Fn("jump").curry(key)}).with_style(ctx.component_style("jump",style().self_center())));
    }
    column(children).with_style(style().min_height(px(0)).min_width(px(0)).flex_grow())
}
fn controls(ctx,props,speech,read_only,blocked,busy){
    let listening=speech!=()&&(speech.state=="listening"||speech.state=="stopping");
    let editor_label=ctx.t_or("writer.message","Message");
    let editor_style=ctx.component_style("editor",style().min_width(px(0)).padding(px(0)).border(px(0))
        .shadows([]).background(rgba(0x00000000)));
    let token=target(props);
    let editor=textarea::Textarea(#{key:"draft:"+props.conversation,label:editor_label,value:props.draft,
        placeholder:caption(props,"placeholder",ctx.t_or("writer.placeholder","Ask anything…")),min_rows:1,max_rows:4,max_length:4000,
        disabled:props.disabled,read_only:read_only||!can(props,"change"),
        on_change:if can(props,"change")&&!read_only{Fn("changed").curry(token)}else{()},
        submit_key:if can(props,"send")&&!blocked{ "enter" }else{()},
        on_submit:if can(props,"send")&&!blocked{Fn("submit").curry(token,"draft","")}else{()},
        part_styles:#{editor:editor_style}}).retained_page(!listening)
        .with_key("draft").with_style(style().flex_grow().min_width(px(0)));
    let children=[editor];
    if listening{
        children.push(text(caption(speech,"preview",ctx.t_or("writer.listening","Listening…"))).with_key("dictation-preview")
            .test_id("writing-dictation-preview").accessibility_role("status")
            .with_style(ctx.component_style("editor",style().min_width(px(0)).line_clamp(4).flex_grow().typography("body"))));
    }
    if !listening&&props.draft!=""{
        children.push(icon_button::IconButton(#{key:"send",label:ctx.t_or("writer.send","Send"),size:"xs",variant:"primary",
            icon:glyph(ctx,"send",16),disabled:blocked||!can(props,"send")||!valid_send(field_text(props.draft)),
            on_click:Fn("send_click").curry(token,props.draft)}).with_key("send")
            .with_style(ctx.component_style("send",style().self_end().flex_shrink(false))));
    }
    let row_nodes=[row(children).with_key("field").with_style(ctx.component_style("field",style().min_width(px(0)).flex_grow()
        .items_center().gap(theme_length("space.spacing_unit").times(1.25)).padding(theme_length("space.spacing_unit").times(2.25)).background(theme_color("surface_raised")).radius(theme_length("space.unit").times(4))
        .focus_within(style().focus_ring(px(1),offset_px(-1),theme_color("focus_ring")))))];
    if speech!=()&&opt(props,"on_dictate")!=(){
        let verb=if listening{"stop"}else{"start"};
        let label=if listening{ctx.t_or("writer.stop_dictation","Stop dictation")}else{ctx.t_or("writer.dictate","Dictate")};
        let request=target(props);request.action=verb;request.lease=speech.lease;
        let icon=if listening{indicator(ctx,true)}else{glyph(ctx,"mic",22)};
        let parts=if listening{["mic","mic_active"]}else{["mic"]};
        row_nodes.push(icon_button::IconButton(#{key:"mic",label:label,icon:icon,selected:listening,
            disabled:props.disabled||speech.state=="starting"||speech.state=="stopping"||(busy&&!listening),
            on_click:Fn("request").curry("dictate",request)}).with_key("mic")
            .with_style(ctx.component_style(parts,style().flex_shrink(false).width(theme_length("space.unit").times(13)).height(theme_length("space.unit").times(13)).radius(theme_length("space.unit").times(249.75))
                .background(theme_color(if listening{"accent"}else{"surface_raised"})))));
    }
    row(row_nodes).with_key("controls").with_style(ctx.component_style("controls",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).items_end()))
}
fn render_WritingAssistant(ctx,props){
    if props.conversation==""||utf8_byte_count(props.conversation)>128{throw "WritingAssistant conversation ID must be 1–128 bytes";}
    let name=caption(props,"name",ctx.t_or("writer.name","Quill"));
    let tagline=caption(props,"tagline",ctx.t_or("writer.tagline","Your writing companion."));
    let speech=opt(props,"speech");
    let active_speech=speech!=()&&(speech.state=="starting"||speech.state=="listening"||speech.state=="stopping");
    if active_speech&&(speech.lease==""||utf8_byte_count(speech.lease)>128){throw "WritingAssistant active dictation needs a bounded lease ID";}
    let busy=props.phase=="waiting"||props.phase=="streaming";
    let read_only=props.disabled||active_speech||props.draft_locked;
    let locked=read_only||busy;
    let data=messages(props);let chat=data.len>0;
    let header=[column([
        text(name).accessibility_role("heading").accessibility_level(3).with_style(ctx.component_style("name",style().min_width(px(0)).typography("title"))),
        text(tagline).with_style(ctx.component_style("tagline",style().min_width(px(0)).typography("caption").text_color(theme_color("text_muted"))))
    ]).with_style(style().min_width(px(0)).flex_grow())];
    if (chat||active_speech||props.phase=="failed")&&opt(props,"on_reset")!=(){
        header.push(icon_button::IconButton(#{key:"reset",label:ctx.t_or("writer.reset","New chat"),icon:glyph(ctx,"reset",18),size:"sm",
            disabled:props.disabled,on_click:Fn("request").curry("reset",target(props))})
            .with_style(ctx.component_style("reset",style().flex_shrink(false))));
    }
    let stage=if chat{transcript(ctx,props,data)}else{welcome(ctx,props,locked)};
    let contents=[stage,controls(ctx,props,speech,read_only,locked,busy)];
    let feedback=caption(props,"message","");
    if feedback!=""{
        contents.push(text(feedback).accessibility_role("status").with_key("feedback")
            .with_style(ctx.component_style("feedback",style().min_width(px(0)).typography("caption")
                .text_color(theme_color(if props.phase=="failed"{"text.danger"}else{"text_muted"})))));
    }
    let deck_ref=element_ref("deck");
    let deck_size=ctx.element_size(deck_ref);
    let width=if deck_size==(){360.0}else{max(1.0,min(deck_size.width,720.0))};
    let height=if deck_size==(){340.0}else{max(1.0,min(deck_size.height,720.0))};
    let variant=ctx.theme_variant();let clean=variant!=()&&variant.family=="Clean";
    let sigma=if clean{36.0}else{34.0};
    let pink_w=if clean{0.58}else{0.60};let pink_h=if clean{0.40}else{0.42};
    let violet_w=if clean{0.54}else{0.56};let violet_h=if clean{0.42}else{0.44};
    let blue_w=if clean{0.62}else{0.64};let blue_h=pink_h;
    let paint=[
        glow(ctx,"glow_pink","writing.glow_pink",chat,style().left(offset_relative(-0.18)).top(relative(if clean{0.54}else{0.46}))
            .width(relative(pink_w)).height(relative(pink_h)), max(16.0,(width*pink_w).round()),max(16.0,(height*pink_h).round()),sigma,0.55,6000),
        glow(ctx,"glow_violet","writing.glow_violet",chat,style().right(offset_relative(-0.16)).top(relative(if clean{0.46}else{0.36}))
            .width(relative(violet_w)).height(relative(violet_h)),max(16.0,(width*violet_w).round()),max(16.0,(height*violet_h).round()),sigma,-0.55,7000),
        glow(ctx,"glow_blue","writing.glow_blue",chat,style().left(relative(0.22)).bottom(offset_relative(if clean{-0.22}else{-0.16}))
            .width(relative(blue_w)).height(relative(blue_h)),max(16.0,(width*blue_w).round()),max(16.0,(height*blue_h).round()),sigma,-0.45,5500),
        // The padded composer and greeting have independent text lanes.
        column(contents).with_key("body").audit_allow(["text-edge-misaligned"]).with_style(ctx.component_style("body",
            (if chat{style().height(theme_length("space.unit").times(85))}else{style().min_height(theme_length("space.unit").times(85))})
                .min_width(px(0)).min_height(if chat{px(0)}else{px(340)}).padding(theme_length("space.spacing_unit").times(3)).gap(theme_length("space.spacing_unit").times(2.5)).justify_end()))
    ];
    let deck=stack(paint).with_key("deck").with_ref(deck_ref).with_style(ctx.component_style("deck",style().min_width(px(0)).relative().clip()
        .background(theme_color("surface_raised")).radius(theme_length("space.unit").times(5.5))));
    column([row(header).with_key("head").with_style(ctx.component_style("header",style().min_width(px(0))
        .items_start().gap(theme_length("space.spacing_unit").times(2)).padding(theme_length("space.spacing_unit").times(3)))),deck]).with_key(props.key).accessibility_role("region").accessibility_label(name)
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(380)).min_width(px(0))
            .padding(theme_length("space.spacing_unit").times(1.5)).radius(theme_length("space.unit").times(6.5)).background(theme_color("surface_raised"))))
}
