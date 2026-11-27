/* gpui-rhai
{"id":"patterns/comment_thread","export":"CommentThread","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/icon_button","components/input","components/thinking_loader"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","radius.lg","surface_hover","surface_raised","text.accent","text.success","text_muted","text_primary","typography.body","typography.caption","radius.pill","radius.sm","space.spacing_unit","space.unit","typography.title","typography.initials"],"environment":["density"]}
*/
// CommentThread shows accepted messages, optional assistant details and a controlled reply field.
// State: disclosure and scroll follow are local; replies, drafts and activity are application-owned.
// Example: CommentThread(#{key:"review",title:"Review comments",messages:[],draft:""})
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/input" as input;
import "components/thinking_loader" as thinking_loader;

define_component(#{
    metadata:#{id:"patterns/comment_thread","export":"CommentThread",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/icon_button","components/input","components/thinking_loader"],capabilities:#{},
        tokens:["border","focus_ring","radius.lg","surface_hover","surface_raised","text.accent","text.success","text_muted","text_primary","typography.body","typography.caption","radius.pill","radius.sm","space.spacing_unit","space.unit","typography.title","typography.initials"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable conversation identity; use a new key for a different conversation."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Visible and accessible heading."},
        messages:#{schema:#{type:"array",max_items:512,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Nonempty unique message ID. Reuse only for updates to this message."},
            author:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible author name."},
            avatar:#{schema:#{type:"optional",value:#{type:"handle",kind:"image"}},required:false,sensitive:false,doc:"Optional Host image; failed loading falls back to initials."},
            time:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Caller-formatted short timestamp; no timer or date parsing is performed."},
            text:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Plain message text, not Rhai, HTML or a command."},
            assistant:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Draws an assistant emblem instead of initials."},
            status:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Short result, shown as a disclosure when details are present."},
            details:#{schema:#{type:"array",max_items:24,items:#{type:"string"}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Plain detail lines for the status disclosure."},
        }}},required:true,sensitive:false,doc:"Accepted messages in chronological display order. At most 512 rows and 512 KiB of text per view."},
        draft:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted editor text. Clear only after the application accepts a reply."},
        placeholder:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional reply field placeholder."},
        thinking:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Shows an assistant activity row; does not run a reply task."},
        assistant_name:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Name shown while the assistant is thinking."},
        assistant_avatar:#{schema:#{type:"optional",value:#{type:"handle",kind:"image"}},required:false,sensitive:false,doc:"Optional Host avatar shown in the thinking row."},
        sending:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Blocks a second send while the Host response is pending."},
        message:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional Host rejection or confirmation."},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disables reply, close, and disclosure controls."},
        max_height:#{schema:#{type:"optional",value:#{type:"length"}},required:false,sensitive:false,doc:"Positive absolute length that caps only the transcript viewport."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the raw draft string."},
        on_send:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives trimmed nonblank reply text; no optimistic insertion."},
        on_close:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the thread key when close is requested."},
    },state:#{fields:#{
        open:#{schema:#{type:"ui_value"},"default":#{type:"map",value:#{}}},
        following:#{schema:#{type:"bool"},"default":#{type:"bool",value:true}},
    }},events:#{
        change:#{payload:#{type:"string"},doc:"Propose draft text."},
        send:#{payload:#{type:"string"},doc:"Propose a nonblank reply."},
        close:#{payload:#{type:"string"},doc:"Request closing this thread."},
    },slots:#{},parts:["root","header","title","count","close","card","list","entry","meta","avatar","assistant","author","time",
        "body","status","details","bullet","detail","composer","editor","send","message","empty","jump","icon"],effects:["prune"]},
    render:Fn("render_CommentThread")
});

fn CommentThread(props){render_component("patterns/comment_thread",props)}
fn opt(map,key){if key in map{map[key]}else{()}}
fn or_value(map,key,fallback){let value=opt(map,key);if value==(){fallback}else{value}}
fn or_text(map,key,fallback){let value=opt(map,key);if value==()||value==""{fallback}else{value}}
fn can(props,event){!props.disabled&&opt(props,"on_"+event)!=()}
fn check_record(seen,record,bytes){
    let id_bytes=utf8_byte_count(record.id);let body_bytes=utf8_byte_count(record.text);
    if id_bytes==0||id_bytes>128||record.id in seen{throw "Comment IDs must be unique and 1–128 UTF-8 bytes";}
    if body_bytes>16000||utf8_byte_count(record.author)>512||utf8_byte_count(or_text(record,"time",""))>256{
        throw "Comment text, author or timestamp exceeds its size limit";
    }
    let total=bytes+body_bytes+id_bytes+utf8_byte_count(record.author)+utf8_byte_count(or_text(record,"time",""));
    if total>524288{throw "Comment content exceeds 512 KiB";}
    total
}
fn glyph(ctx,icon){
    let path=switch icon{
        "close"=>"m5 5 10 10M15 5 5 15",
        "send"=>"M10 16V4m-5 5 5-5 5 5",
        "sort"=>"M3 5h14M5.5 10h9M8 15h4",
        "check"=>"m4.5 10 3.5 3.5 7.5-8",
        "up"=>"m5 12 5-5 5 5",
        "down"=>"m5 8 5 5 5-5",
        _=>"M10 2v16M2 10h16M4.3 4.3l11.4 11.4m0-11.4L4.3 15.7",
    };
    svg(`<svg viewBox='0 0 20 20' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("icon",style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4)).flex_shrink(false)))
}
fn mark(ctx,name,assistant,avatar){
    let initials=text_initial(name);
    let mark=if avatar!=(){image(avatar).image_fallback_text(initials).with_style(style().width(relative(1.0)).height(relative(1.0)))}
        else if assistant{glyph(ctx,"assistant")}else{text(initials)};
    row([mark]).with_key("avatar").accessibility_hidden(true)
        .with_style(ctx.component_style(if assistant{["avatar","assistant"]}else{["avatar"]},style()
            .width(theme_length("space.unit").times(6)).height(theme_length("space.unit").times(6)).flex_shrink(false).items_center().justify_center().radius(theme_radius("pill")).clip()
            .background(theme_color("surface_hover")).text_color(theme_color("text.accent")).typography("initials").font_weight(600)))
}
fn meta(ctx,item,control){
    let cells=[mark(ctx,item.author,item.assistant,opt(item,"avatar")),text(item.author).with_key("author")
        .with_style(ctx.component_style("author",style().typography("body").font_weight(500).text_ellipsis().min_width(px(0)).flex_grow()))];
    if item.time!=""{
        cells.push(text(item.time).with_key("time").with_style(ctx.component_style("time",style()
            .typography("caption").text_color(theme_color("text_muted")).text_ellipsis().min_width(px(0)))));
    }
    if control!=(){cells.push(control);}
    // Initials, author, and time have deliberately distinct semantic roles.
    row(cells).with_key("meta").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("meta",style().min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(2))))
}
fn edit_text(message){
    let body=message;body.trim();if body.len<=5||body.sub_string(0,5).to_lower()!="/edit"{return ();}
    let tail=body.sub_string(5);
    if !tail.starts_with(" ")&&!tail.starts_with("\n")&&!tail.starts_with("\t"){return ();}
    tail.trim();if tail==""{()}else{tail}
}
fn message_body(ctx,message){
    text(message).selectable(true).with_key("body").with_style(ctx.component_style("body",style().min_width(px(0)).typography("body")))
}
fn empty_row(ctx,message){
    row([text(message).with_key("label").with_style(ctx.component_style("empty",style()
        .min_width(px(0)).typography("body").text_color(theme_color("text_muted"))))]).with_key("empty")
        .with_style(style().min_width(px(0)).justify_center().padding(theme_length("space.spacing_unit").times(4.5)))
}
fn draft_change(ctx,value){ctx.emit("change",value);}
fn propose_send(ctx,value){
    if utf8_byte_count(value)>16000{return handled();}
    value.trim();if value!=""{ctx.emit("send",value);}
    handled()
}
fn send_click(value,ctx,event){propose_send(ctx,value)}
fn request_close(key,ctx,event){ctx.emit("close",key);handled()}
fn header(ctx,props,title,count,close_label){
    let main=text(title).accessibility_role("heading").accessibility_level(3).with_key("title")
        .with_style(ctx.component_style("title",style().typography("title").font_weight(500).min_width(px(0)).line_clamp(2)));
    let labels=[main];
    if count!=""{
        labels.push(text(count).with_key("count").with_style(ctx.component_style("count",style()
            .min_width(px(0)).typography("caption").text_color(theme_color("text_muted")).font_feature("tnum",1))));
    }
    let children=[row(labels).with_style(style().min_width(px(0)).flex_grow().gap(theme_length("space.spacing_unit").times(2)).items_center().flex_wrap())];
    if opt(props,"on_close")!=(){
        children.push(icon_button::IconButton(#{key:"close",label:close_label,icon:glyph(ctx,"close"),size:"xs",
            disabled:props.disabled,on_click:Fn("request_close").curry(props.key)}).with_key("close")
            .with_style(ctx.component_style("close",style().flex_shrink(false))));
    }
    row(children).with_key("header").with_style(ctx.component_style("header",style().min_width(px(0))
        .padding(theme_length("space.spacing_unit").times(2)).gap(theme_length("space.spacing_unit")).items_center()))
}
fn composer(ctx,props,field,placeholder,send_label,author,avatar){
    let value=props.draft;let clean=value;clean.trim();
    let blocked=!can(props,"send")||props.sending||clean==""||utf8_byte_count(value)>16000;
    let editor=input::Input(#{key:"draft",label:field,placeholder:placeholder,value:value,appearance:"embedded",
        disabled:props.disabled,read_only:props.sending||!can(props,"change"),
        on_change:if can(props,"change")&&!props.sending{Fn("draft_change")}else{()},
        on_submit:if !blocked&&can(props,"change"){Fn("propose_send")}else{()}})
        .with_style(ctx.component_style("editor",style().min_width(px(0)).flex_grow().padding(px(0)).border(px(0)).shadows([])));
    let children=[];if author!=""||avatar!=(){children.push(mark(ctx,author,false,avatar));}
    children.push(editor);
    children.push(icon_button::IconButton(#{key:"send",label:send_label,size:"sm",variant:"primary",icon:glyph(ctx,"send"),
        disabled:blocked,loading:props.sending,on_click:Fn("send_click").curry(value)}).with_key("send")
        .with_style(ctx.component_style("send",style().flex_shrink(false))));
    row(children).with_key("composer").with_style(ctx.component_style("composer",style().min_width(px(0)).items_center()
        .gap(theme_length("space.spacing_unit").times(2)).padding(theme_length("space.spacing_unit").times(2)).focus_within(style().focus_ring(px(1),offset_px(-1),theme_color("focus_ring")))))
}
fn feedback(ctx,props){
    let message=or_text(props,"message","");
    if message==""{return ();}
    text(message).accessibility_role("status").with_key("message").with_style(ctx.component_style("message",style()
        .min_width(px(0)).padding_x(theme_length("space.spacing_unit").times(3)).padding_bottom(theme_length("space.spacing_unit").times(2)).typography("caption").text_color(theme_color("text_muted"))))
}
fn follow(ctx,value){ctx.set_state("following",value.following);}
fn jump(key,ctx,event){ctx.scroll_collection_to_end(key);handled()}
fn details_toggle(id,ctx,event){
    let open=ctx.get_state("open");
    if id in open{open.remove(id);}else{open[id]=true;}
    ctx.set_state("open",open);handled()
}
fn prune_details(ctx,seen){
    let current=ctx.get_state("open");let next=#{};
    for entry in current.keys(){if entry in seen{next[entry]=true;}}
    ctx.set_state("open",next);
}
fn unmount_details(ctx,seen){}
fn status_row(ctx,item){
    let has_details=item.details.len>0;
    let label=or_text(item,"status",ctx.t_or("comments.details","Details"));
    let cells=[glyph(ctx,"check"),text(label).with_style(style().min_width(px(0)).text_ellipsis())];
    if has_details{cells.push(glyph(ctx,if item.open{"up"}else{"down"}));}
    let row=row(cells).with_key("status").with_style(ctx.component_style("status",style().min_width(px(0)).self_start()
        .gap(theme_length("space.spacing_unit").times(1.25)).padding_x(theme_length("space.spacing_unit").times(1.5)).padding_y(theme_length("space.spacing_unit").times(0.75)).items_center().radius(theme_radius("sm"))
        .typography("caption").text_color(theme_color("text.success")).background(theme_color("surface_hover"))
        .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))));
    if !has_details{return row.accessibility_role("status").accessibility_label(label);}
    row.accessibility_role("button").accessibility_label(label).accessibility_expanded(item.open).disabled(item.disabled)
        .on_click(Fn("details_toggle").curry(item.id))
}
fn thread_row(ctx,payload){
    let item=payload.item;
    if item.kind=="empty"{return empty_row(ctx,item.text);}
    let children=[meta(ctx,item,())];
    if item.kind=="thinking"{
        children.push(thinking_loader::ThinkingLoader(#{key:"thinking",phase:0,variant:"plain",size:"sm"}));
    }else{
        if item.text!=""{children.push(message_body(ctx,item.text));}
        if item.details.len>0||opt(item,"status")!=(){children.push(status_row(ctx,item));}
        if item.open&&item.details.len>0{
            let details=[];
            for line in item.details{
                details.push(row([text("•").accessibility_hidden(true).with_style(ctx.component_style("bullet",style().flex_shrink(false))),
                    text(line).selectable(true).with_style(ctx.component_style("detail",style().min_width(px(0)).typography("caption")))])
                    .with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5))));
            }
            children.push(column(details).with_key("details").with_style(ctx.component_style("details",style()
                .min_width(px(0)).padding_start(theme_length("space.spacing_unit").times(2.5)).gap(theme_length("space.spacing_unit").times(0.75)).border_start(px(1)).border_color(theme_color("border"))))
                .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:ctx.motion_duration("fast"),easing:ctx.motion_easing("entrance"),intent:"feedback"})));
        }
    }
    column(children).with_key(payload.key).accessibility_role("group").accessibility_label(item.author)
        .with_style(ctx.component_style("entry",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2.5)).gap(theme_length("space.spacing_unit").times(1.25))))
}
fn viewport_config(data,label,max_height,follow_tail,follower){
    #{key:"transcript",label:label,data:data,estimated_height:82,max_height:max_height,follow_tail:follow_tail,on_follow_change:follower}
}
fn style_viewport(ctx,list,label){
    list
        .accessibility_role("list").accessibility_label(label).tab_stop(true)
        .with_style(ctx.component_style("list",style().min_width(px(0)).min_height(px(0))
            .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
}
fn render_CommentThread(ctx,props){
    let title=or_text(props,"title",ctx.t_or("comments.thread","Comment"));
    let author=or_text(props,"assistant_name",ctx.t_or("comments.assistant","Assistant"));
    let amount=#{count:ctx.format_number(props.messages.len)};
    let count=if props.messages.len==1{ctx.t_or("comments.count_one","{count} message",amount)}
        else{ctx.t_or("comments.count_many","{count} messages",amount)};
    let seen=#{};let bytes=0;let open=ctx.get_state("open");let data=[];
    for item in props.messages{
        bytes=check_record(seen,item,bytes);
        seen[item.id]=true;item.key="entry:"+item.id;item.kind="entry";
        item.open=item.id in open;item.disabled=props.disabled;item.time=or_text(item,"time","");
        item.assistant=or_value(item,"assistant",false);item.details=or_value(item,"details",[]);
        bytes+=utf8_byte_count(or_text(item,"status",""));
        for line in item.details{bytes+=utf8_byte_count(line);}
        if bytes>524288{throw "Comment content exceeds 512 KiB";}
        data.push(item);
    }
    effect("prune",seen,Fn("prune_details"),Fn("unmount_details"));
    if props.thinking{data.push(#{key:"activity",kind:"thinking",author:author,time:"",assistant:true,avatar:opt(props,"assistant_avatar")});}
    if data.len==0{data.push(#{key:"empty",kind:"empty",text:ctx.t_or("comments.empty","No comments yet.")});}
    let cap=opt(props,"max_height");if cap==(){cap=px(340);}
    let label=ctx.t_or("comments.transcript","Conversation");
    let list=virtual_collection(viewport_config(data,label,cap,true,Fn("follow")),Fn("thread_row"));
    let contents=[style_viewport(ctx,list,label)];
    if !ctx.get_state("following"){
        contents.push(button::Button(#{key:"latest",text:ctx.t_or("comments.latest","Jump to latest"),size:"xs",variant:"ghost",
            on_click:Fn("jump").curry("transcript")}).with_style(ctx.component_style("jump",style().self_center())));
    }
    let footer=[composer(ctx,props,ctx.t_or("comments.reply","Reply"),or_text(props,"placeholder",
        if opt(props,"assistant_name")==(){ctx.t_or("comments.add","Add a comment…")}
        else{ctx.t_or("comments.ask","Reply, or ask @{name}…",#{name:author})}),ctx.t_or("comments.send_reply","Send reply"),"",())];
    let message=feedback(ctx,props);if message!=(){footer.push(message);}
    let nodes=[header(ctx,props,title,count,ctx.t_or("comments.close_thread","Close comment")),
        column(contents).with_key("card").with_style(ctx.component_style("card",style().min_width(px(0))
            .background(theme_color("surface_raised")).radius(theme_radius("lg"))))];
    nodes+=footer;
    column(nodes).with_key(props.key).accessibility_role("region").accessibility_label(title)
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(420)).min_width(px(0))
            .padding(theme_length("space.spacing_unit")).radius(theme_radius("lg")).background(theme_color("surface_hover"))))
}
