/* gpui-rhai
{"id":"patterns/comments_panel","export":"CommentsPanel","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/icon_button","components/menu","components/tabs","patterns/comment_thread"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","radius.lg","surface_hover","surface_raised","text.danger","text.success","text_muted","text_primary","typography.body","typography.caption","radius.sm","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// CommentsPanel filters and sorts an accepted comment feed; edit and resolve actions are proposals.
// State: local filters and sort order; comments, drafts and edit results are application-owned.
// Example: CommentsPanel(#{key:"comments",comments:[],draft:""})
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/menu" as menu;
import "components/tabs" as tabs;
import "patterns/comment_thread" as conversation;

define_component(#{
    metadata:#{id:"patterns/comments_panel","export":"CommentsPanel",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/icon_button","components/menu","components/tabs","patterns/comment_thread"],capabilities:#{},
        tokens:["border","focus_ring","radius.lg","surface_hover","surface_raised","text.danger","text.success","text_muted","text_primary","typography.body","typography.caption","radius.sm","space.spacing_unit","space.unit"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable feed identity."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Visible and accessible heading."},
        comments:#{schema:#{type:"array",max_items:512,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique nonempty comment ID."},
            author:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible author name."},
            avatar:#{schema:#{type:"optional",value:#{type:"handle",kind:"image"}},required:false,sensitive:false,doc:"Optional Host image; failed loading falls back to initials."},
            text:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Plain comment text. An /edit prefix is presentation only."},
            time:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Caller-formatted short timestamp."},
            timestamp:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Caller-supplied timestamp for sorting. Ties preserve supplied order."},
            revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted item revision echoed in change requests."},
            kind:#{schema:#{type:"optional",value:#{type:"string",allowed:["note","question","edit"]}},required:false,sensitive:false,
                doc:"Override the display category. Omitted, /edit plus text is an edit and a terminal question mark is a question."},
            resolved:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Accepted resolution state; resolved records stay visible."},
            edit_state:#{schema:#{type:"optional",value:#{type:"string",allowed:["idle","running","applied","failed"]}},required:false,sensitive:false,
                doc:"Host-enabled edit state. Omit to show an edit without any actions; an /edit string alone never enables one."},
            edit_message:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Host-safe detail for an edit result."},
        }}},required:true,sensitive:false,doc:"Accepted comments. At most 512 rows and 512 KiB of text per view."},
        draft:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted editor text. Clear only after the application accepts a comment."},
        current_author:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional composer initials, not used to assign author identity."},
        current_avatar:#{schema:#{type:"optional",value:#{type:"handle",kind:"image"}},required:false,sensitive:false,doc:"Optional Host image for the composer."},
        placeholder:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional composer placeholder."},
        sending:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Blocks editing and repeated submission while the Host response is pending."},
        message:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional Host feedback after sending."},
        posted_id:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,
            doc:"ID of the user's newly accepted comment. When it appears in the feed, reveal it by returning to All and newest-first; unrelated incoming comments never trigger this."},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Blocks data mutations and close. Filtering and sorting remain available."},
        max_height:#{schema:#{type:"optional",value:#{type:"length"}},required:false,sensitive:false,doc:"Positive absolute cap for the list viewport."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the raw draft string."},
        on_send:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives trimmed nonblank plain text."},
        on_resolve:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives #{id,revision,resolved}."},
        on_edit:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives #{id,revision,action} (run or undo). The Host authorizes and performs it."},
        on_close:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the panel key when close is requested."},
    },state:#{fields:#{
        filter:#{schema:#{type:"string"},"default":#{type:"string",value:"all"}},
        order:#{schema:#{type:"string"},"default":#{type:"string",value:"newest"}},
        menu_open:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        menu_active:#{schema:#{type:"string"},"default":#{type:"string",value:"newest"}},
        revealed_post:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
    }},events:#{
        change:#{payload:#{type:"string"},doc:"Propose draft text."},
        send:#{payload:#{type:"string"},doc:"Propose a new comment."},
        close:#{payload:#{type:"string"},doc:"Request closing this panel."},
        resolve:#{payload:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            resolved:#{schema:#{type:"bool"},required:true,sensitive:false},
        }},doc:"Propose resolving or reopening an accepted comment."},
        edit:#{payload:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            action:#{schema:#{type:"string",allowed:["run","undo"]},required:true,sensitive:false},
        }},doc:"Propose a Host-owned edit action; no executable content is submitted."},
    },slots:#{},parts:["root","header","title","close","card","bar","tabs","sort","list","entry","first","resolved","meta","avatar","assistant",
        "author","time","body","resolve","edit_actions","run","undo","edit_note","edit_applied","empty","composer","editor","send","message","icon"],
        effects:["reveal_post"]},
    render:Fn("render_CommentsPanel")
});

fn CommentsPanel(props){render_component("patterns/comments_panel",props)}
fn comment_kind(item){
    let kind=conversation::opt(item,"kind");if kind!=(){return kind;}
    let body=item.text;body.trim();
    if conversation::edit_text(body)!=(){return "edit";}
    if body.ends_with("?")||body.ends_with("？")||body.ends_with("؟"){"question"}else{"note"}
}
fn entry_order(newest,a,b){
    if a.timestamp==b.timestamp{return a.index-b.index;}
    if a.timestamp>b.timestamp{if newest{-1}else{1}}else{if newest{1}else{-1}}
}
fn feed(ctx,props,filter,newest){
    let seen=#{};let bytes=0;let pending=0;let data=[];let index=0;
    let edit=conversation::can(props,"edit");let resolve=conversation::can(props,"resolve");
    for item in props.comments{
        bytes=conversation::check_record(seen,item,bytes)+utf8_byte_count(conversation::or_text(item,"edit_message",""));
        if bytes>524288{throw "Comment content exceeds 512 KiB";}
        seen[item.id]=true;item.key="entry:"+item.id;item.kind=comment_kind(item);item.index=index;index+=1;
        item.time=conversation::or_text(item,"time","");item.assistant=false;item.can_edit=edit;item.can_resolve=resolve;
        item.resolved=conversation::or_value(item,"resolved",false);
        item.revision=conversation::or_value(item,"revision",0);
        item.timestamp=conversation::or_value(item,"timestamp",0);
        item.edit_state=conversation::opt(item,"edit_state");
        if item.kind=="edit"&&!item.resolved&&item.edit_state!=()&&item.edit_state!="applied"{pending+=1;}
        if filter=="all"||item.kind==filter{data.push(item);}
    }
    data.sort(Fn("entry_order").curry(newest));
    if data.len==0{
        let empty=if filter=="edit"{ctx.t_or("comments.no_edits","No edits to run.")}
            else if filter=="question"{ctx.t_or("comments.no_questions","No questions yet.")}
            else{ctx.t_or("comments.empty","No comments yet.")};
        data.push(#{key:"empty",kind:"empty",text:empty});
    }
    let posted=conversation::opt(props,"posted_id");
    #{rows:data,pending:pending,posted:if posted!=()&&posted in seen{posted}else{()}}
}
fn set_filter(ctx,value){ctx.set_state("filter",value);}
fn reveal_post(ctx,request){
    if request==(){return;}
    if !request.ready{
        ctx.set_state("filter","all");ctx.set_state("order","newest");ctx.set_state("menu_open",false);return;
    }
    ctx.scroll_collection_to_key("transcript","entry:"+request.id);
    ctx.set_state("revealed_post",request.id);
}
fn cleanup_post(ctx,request){}
fn sort_open(ctx,value){
    if value{ctx.set_state("menu_active",ctx.get_state("order"));}
    ctx.set_state("menu_open",value);
}
fn sort_move(ctx,value){ctx.set_state("menu_active",value);}
fn sort_pick(ctx,value){ctx.set_state("order",value);ctx.set_state("menu_open",false);}
fn filters(ctx,projection){
    let label=ctx.t_or("comments.sort","Sort comments");
    let direction=ctx.get_state("order");
    let trigger=row([conversation::glyph(ctx,"sort")]).with_key("sort-trigger").accessibility_role("button").accessibility_label(label)
        .with_style(ctx.component_style("sort",style().min_width(px(28)).min_height(theme_length("space.unit").times(7)).items_center().justify_center()
            .flex_shrink(false).radius(theme_radius("sm")).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))
            .hover(style().background(theme_color("surface_hover")))));
    let orders=[#{kind:"item",value:"newest",label:ctx.t_or("comments.newest","Newest first"),checked:direction=="newest"},
        #{kind:"item",value:"oldest",label:ctx.t_or("comments.oldest","Oldest first"),checked:direction=="oldest"}];
    let order=menu::Menu(#{key:"sort",label:label,trigger:trigger,items:orders,open:ctx.get_state("menu_open"),
        active_value:ctx.get_state("menu_active"),on_open_change:Fn("sort_open"),on_active_change:Fn("sort_move"),on_action:Fn("sort_pick")})
        .with_style(style().flex_shrink(false));
    let strip=tabs::Tabs(#{key:"filter",label:ctx.t_or("comments.filter","Filter comments"),variant:"chip",size:"xs",panel:false,
        value:ctx.get_state("filter"),on_change:Fn("set_filter"),tabs:[#{value:"all",label:ctx.t_or("comments.all","All")},
        #{value:"question",label:ctx.t_or("comments.questions","Questions")},
        #{value:"edit",label:ctx.t_or("comments.edits","Edits"),count:if projection.pending>0{projection.pending}else{()}}]})
        .with_style(ctx.component_style("tabs",style().min_width(px(0)).flex_grow()));
    row([strip,order]).with_key("bar").with_style(ctx.component_style("bar",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).items_center()
        .padding(theme_length("space.spacing_unit").times(1.5)).border_bottom(px(1)).border_color(theme_color("border"))))
}
fn propose_edit(data,ctx,event){ctx.emit("edit",data);handled()}
fn propose_resolve(data,ctx,event){ctx.emit("resolve",data);handled()}
fn edit_actions(ctx,item){
    let state=item.edit_state;let nodes=[];
    if state=="applied"{
        let applied=ctx.t_or("comments.applied","Edit applied");
        nodes.push(row([conversation::glyph(ctx,"check"),text(applied)]).accessibility_role("status").accessibility_label(applied)
            .with_style(ctx.component_style("edit_applied",style().gap(theme_length("space.spacing_unit").times(1.25)).items_center().typography("caption").text_color(theme_color("text.success")))));
        if item.can_edit{
            nodes.push(button::Button(#{key:"undo",text:ctx.t_or("comments.undo","Undo"),size:"xs",variant:"ghost",
                on_click:Fn("propose_edit").curry(#{id:item.id,revision:item.revision,action:"undo"})})
                .with_style(ctx.component_style("undo",style())));
        }
    }else if item.can_edit||state=="running"{
        nodes.push(button::Button(#{key:"run",text:if state=="failed"{ctx.t_or("comments.retry_edit","Retry edit")}else{ctx.t_or("comments.run_edit","Run edit")},
            size:"xs",variant:"secondary",loading:state=="running",loading_text:ctx.t_or("comments.running_edit","Running…"),
            on_click:Fn("propose_edit").curry(#{id:item.id,revision:item.revision,action:"run"})})
            .with_style(ctx.component_style("run",style())));
    }
    let note=conversation::or_text(item,"edit_message","");
    if note!=""{
        nodes.push(text(note).accessibility_role("status").with_style(ctx.component_style("edit_note",style().min_width(px(0))
            .typography("caption").text_color(theme_color(if state=="failed"{"text.danger"}else{"text_muted"})))));
    }
    if nodes.len==0{return ();}
    row(nodes).with_key("edit-actions").with_style(ctx.component_style("edit_actions",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap()))
}
fn comment_row(ctx,payload){
    let item=payload.item;
    if item.kind=="empty"{return conversation::empty_row(ctx,item.text);}
    let resolve=();
    if item.can_resolve{
        let label=if item.resolved{ctx.t_or("comments.reopen","Reopen comment")}else{ctx.t_or("comments.resolve","Resolve comment")};
        resolve=icon_button::IconButton(#{key:"resolve",label:label,icon:conversation::glyph(ctx,"check"),size:"xs",selected:item.resolved,
            on_click:Fn("propose_resolve").curry(#{id:item.id,revision:item.revision,resolved:!item.resolved})})
            .with_style(ctx.component_style("resolve",style().flex_shrink(false)));
    }
    let body=conversation::message_body(ctx,item.text);
    if item.resolved{body=body.with_style(ctx.component_style("resolved",style().text_color(theme_color("text_muted"))));}
    let nodes=[conversation::meta(ctx,item,resolve),body];
    if item.kind=="edit"&&!item.resolved&&item.edit_state!=(){
        let actions=edit_actions(ctx,item);if actions!=(){nodes.push(actions);}
    }
    column(nodes).with_key(payload.key).accessibility_role("group").accessibility_label(item.author)
        .with_style(ctx.component_style(if payload.index==0{["entry","first"]}else{["entry"]},style().min_width(px(0))
            .padding(theme_length("space.spacing_unit").times(2.5)).gap(theme_length("space.spacing_unit").times(1.25)).border_top(px(1)).border_color(theme_color("border"))))
}
fn render_CommentsPanel(ctx,props){
    let title=conversation::or_text(props,"title",ctx.t_or("comments.title","Comments"));
    let filter=ctx.get_state("filter");let newest=ctx.get_state("order")=="newest";
    let projection=feed(ctx,props,filter,newest);
    let reveal=if projection.posted!=()&&projection.posted!=ctx.get_state("revealed_post"){
        #{id:projection.posted,ready:filter=="all"&&newest}
    }else{()};
    effect("reveal_post",reveal,Fn("reveal_post"),Fn("cleanup_post"));
    let cap=conversation::opt(props,"max_height");if cap==(){cap=px(360);}
    let label=ctx.t_or("comments.feed","Comment feed");
    let list=virtual_collection(conversation::viewport_config(projection.rows,label,cap,false,()),Fn("comment_row"));
    let card=column([filters(ctx,projection),conversation::style_viewport(ctx,list,label)]).with_key("card")
        .with_style(ctx.component_style("card",style().min_width(px(0)).background(theme_color("surface_raised")).radius(theme_radius("lg"))));
    let nodes=[conversation::header(ctx,props,title,"",ctx.t_or("comments.close","Close comments")),card,
        conversation::composer(ctx,props,ctx.t_or("comments.add_label","Add a comment"),conversation::or_text(props,"placeholder",ctx.t_or("comments.add_edit","Add a comment or /edit…")),
            ctx.t_or("comments.send_comment","Send comment"),conversation::or_text(props,"current_author",""),conversation::opt(props,"current_avatar"))];
    let message=conversation::feedback(ctx,props);if message!=(){nodes.push(message);}
    let surface=column(nodes).with_key(props.key).accessibility_role("region").accessibility_label(title)
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(400)).min_width(px(0))
            .padding(theme_length("space.spacing_unit")).radius(theme_radius("lg")).background(theme_color("surface_hover"))));
    // In the Modern tray the title and the composer initials are different
    // lanes, not two lines of a shared text column.
    if ctx.theme_variant()!=()&&ctx.theme_token("design.name")=="modern"{surface.audit_allow(["text-edge-misaligned"])}else{surface}
}
