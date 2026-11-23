/* gpui-rhai
{"id":"patterns/decision_card","export":"DecisionCard","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/icon_button","components/input","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","selection","surface_raised","surface_hover","text_primary","text_muted","text.warning","typography.body","typography.caption","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// DecisionCard pages through Host-owned choices, visual picks and approvals.
// State: current page, collapse, per-revision draft and native option focus.
// Example: DecisionCard(#{key:"review",decisions:[#{id:"release",revision:0,question:"Continue?"}]})
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/input" as input;
import "patterns/approval_card" as review;

define_component(#{
    metadata:#{id:"patterns/decision_card","export":"DecisionCard",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/icon_button","components/input","patterns/approval_card"],capabilities:#{},
        tokens:["border","focus_ring","selection","surface_raised","surface_hover","text_primary","text_muted","text.warning","typography.body","typography.caption","space.spacing_unit","space.unit"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable presentation identity."},
        decisions:#{schema:#{type:"array",max_items:48,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Request ID; unique among open decisions."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Expected revision when proposing an answer."},
            kind:#{schema:#{type:"optional",value:#{type:"string",allowed:["decision","pick","approval"]}},required:false,sensitive:false,doc:"Question layout; defaults to decision rows."},
            question:#{schema:#{type:"string"},required:true,sensitive:true,doc:"The question shown in the decision card."},
            task:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Task label in the header."},
            options:#{schema:#{type:"optional",value:#{type:"array",max_items:5,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable option ID."},
                title:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible and accessible option name."},
                summary:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short description, also exposed on the option."},
                recommended:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Marks the initially focused option without answering it."},
                preview:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative content in a pick tile; never owns a second set of controls."},
                blocks:#{schema:#{type:"ui_value"},required:false,sensitive:false,
                    doc:"Up to 16 wireframe blocks: #{x,y,width,height,tone?} in 120×80 integer coordinates; tone is ink, mid or soft."},
            }}}},required:false,sensitive:true,doc:"Options for decision and pick; approval has built-in Approve and Decline."},
            facts:#{schema:#{type:"optional",value:#{type:"array",max_items:8,items:#{type:"object",allow_unknown:false,fields:#{
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Evidence label."},
                value:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Evidence value displayed without taking action."},
            }}}},required:false,sensitive:true,doc:"Read-only facts about the decision."},
            busy:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"This item is awaiting an accepted answer; navigation stays enabled."},
            notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Acknowledge or explain a rejected proposal."},
        }}},required:true,sensitive:true,doc:"Open decisions in presentation order. The Host removes an item after accepting an answer."},
        initial_folded:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Initially show only the waiting summary."},
        reveal:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Changing this Host signal unfolds the card and focuses the currently offered options."},
        on_answer:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a versioned answer proposal."},
        on_later:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the request hidden by Later; not an answer."},
    },state:#{fields:#{
        page:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        folded:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        drafts:#{schema:#{type:"ui_value"},"default":#{type:"map",value:#{}}},
        cursor:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        focus_plan:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        focused:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        previous:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        reveal_seen:#{schema:#{type:"integer"},"default":#{type:"integer",value:-1}},
    }},events:#{
        answer:#{payload:#{type:"object",allow_unknown:false,fields:#{
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"The decision being answered."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"The revision accepted when the user answered."},
            kind:#{schema:#{type:"string",allowed:["option","custom","approve","decline"]},required:true,sensitive:false,doc:"Which answer control was used."},
            option:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Chosen option ID, or empty for another answer kind."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Trimmed custom answer, or empty for another answer kind."},
        }},doc:"Versioned proposal; rejection does not remove the item or clear its draft."},
        later:#{payload:#{type:"object",allow_unknown:false,fields:#{
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"The deferred decision."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Its accepted revision."},
        }},doc:"Presentation preference; does not resolve a decision."},
    },slots:#{},parts:["root","head","badge","task","pager","page","count","later","show","card","question","facts","fact","fact_label","fact_value",
        "options","option","tile","preview","number","option_title","summary","recommended","own","editor","send","notice","empty","icon"],effects:["present"]},
    render:Fn("render_DecisionCard")
});

fn DecisionCard(props){render_component("patterns/decision_card",props)}
fn item_token(item){review::target(#{request:item.id,revision:item.revision})}
fn trim_text(value){let clean=value;clean.trim();clean}
fn active_page(ctx,items){
    if items.len==0{return 0;}
    let page=ctx.get_state("page");if page==(){return 0;}
    for i in 0..items.len{if page.id==items[i].id{return i;}}
    min(page.index,items.len-1)
}
fn page_to(id,index,ctx,payload){ctx.set_state("page",#{id:id,index:index});ctx.set_state("focus_plan","pager");handled()}
fn report_focus(ctx,value){ctx.set_state("focused",value);}
fn defer(token,ctx,payload){
    ctx.emit("later",token);ctx.set_state("folded",true);ctx.set_state("focus_plan","show");handled()
}
fn show(ctx,payload){ctx.set_state("folded",false);ctx.set_state("focus_plan","active");handled()}
fn data_set(ctx,cfg){
    let drafts=ctx.get_state("drafts");let kept=#{};
    for id in drafts.keys(){if id in cfg.seen&&drafts[id].revision==cfg.seen[id]{kept[id]=drafts[id];}}
    if kept!=drafts{ctx.set_state("drafts",kept);}
    let old=ctx.get_state("previous");ctx.set_state("previous",cfg.current);
    let signal=ctx.get_state("reveal_seen");
    if signal!=cfg.reveal{
        ctx.set_state("reveal_seen",cfg.reveal);
        if signal>=0&&cfg.current!=(){ctx.set_state("folded",false);ctx.set_state("focus_plan","active");return;}
    }
    if cfg.plan!=""{
        ctx.set_state("focus_plan","");
        if cfg.plan=="show"&&cfg.folded{ctx.focus("show");}
        else if cfg.plan=="active"&&cfg.focus!=""{ctx.focus(cfg.focus);}
    }else if old!=()&&old!=cfg.current&&cfg.focus!=""&&ctx.get_state("focused"){
        ctx.focus(cfg.focus);
    }
}
fn clear_data(ctx,cfg){}
fn edit_draft(token,ctx,value){
    if utf8_byte_count(value)>16000{return;}
    let drafts=ctx.get_state("drafts");
    drafts[token.request]=#{revision:token.revision,text:value};
    ctx.set_state("drafts",drafts);
}
fn draft_for(ctx,token){
    let drafts=ctx.get_state("drafts");
    if token.request in drafts&&drafts[token.request].revision==token.revision{drafts[token.request].text}else{""}
}
fn answer(token,kind,option,text){#{request:token.request,revision:token.revision,kind:kind,option:option,text:text}}
fn submit(token,ctx,value){
    let value=trim_text(value);
    if value!=""&&utf8_byte_count(value)<=16000{ctx.emit("answer",answer(token,"custom","",value));}
    handled()
}
fn submit_button(token,ctx,value){submit(token,ctx,value)}
fn move_focus(key,ctx,event){ctx.focus(key);handled()}
fn option_focused(token,index,ctx,value){
    if value{ctx.set_state("cursor",#{token:token,index:index});}
}
fn answers(ctx,item,kind){
    if kind=="approval"{return [
        #{id:"approve",title:ctx.t_or("review.approve","Approve"),answer:"approve"},
        #{id:"decline",title:ctx.t_or("decision.decline","Decline"),answer:"decline"},
    ];}
    let options=review::value_or(item,"options",[]);let ids=#{};
    for option in options{
        if option.id==""||utf8_byte_count(option.id)>128||option.id in ids{throw "Decision option IDs must be unique and bounded";}
        ids[option.id]=true;
    }
    options
}
fn recommended(options){
    for i in 0..options.len{if review::value_or(options[i],"recommended",false){return i;}}
    0
}
fn opt_ref(i){"choice-"+i.to_string()}
fn icon(ctx,kind){
    let path=if kind=="back"{"m11 5-5 5 5 5"}else if kind=="forward"{"m9 5 5 5-5 5"}else{"M10 16V4m-4 4 4-4 4 4"};
    svg(`<svg viewBox='0 0 20 20' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("icon",style().width(px(14)).height(px(14))))
}
fn preview_blocks(option){
    let blocks=review::value_or(option,"blocks",[]);
    if type_of(blocks)!="array"||blocks.len>16{throw "Decision preview requires an array of at most 16 blocks";}
    for b in blocks{
        if type_of(b)!="map"{throw "Decision preview blocks must be maps";}
        for key in b.keys(){if !(key in ["x","y","width","height","tone"]){throw "Unknown decision preview block field";}}
        for key in ["x","y","width","height"]{
            if !(key in b)||type_of(b[key])!="i64"{throw "Decision block coordinates must be integers";}
        }
        if b.x<0||b.x>120||b.y<0||b.y>80||b.width<1||b.width>120||b.height<1||b.height>80{
            throw "Decision preview block coordinates outside supported bounds";
        }
        if !(review::value_or(b,"tone","soft") in ["ink","mid","soft"]){throw "Invalid decision preview block tone";}
    }
    blocks
}
fn preview(ctx,option,compact){
    let image=review::opt(option,"preview");
    if image==(){
        let shapes=[canvas_rect("paper",1,1,118,78,theme_color("surface_raised"))];
        let blocks=preview_blocks(option);
        for i in 0..blocks.len{
            let b=blocks[i];let tone=review::value_or(b,"tone","soft");
            let w=min(120-b.x,b.width);let h=min(80-b.y,b.height);
            if w>0&&h>0{shapes.push(canvas_rect("b"+i.to_string(),b.x,b.y,w,h,alpha(theme_color("text_primary"),
                if tone=="ink"{0.42}else if tone=="mid"{0.16}else{0.07})));}
        }
        image=canvas(canvas_scene(shapes).view_box(0,0,120,80));
    }
    row([image.with_style(style().width(relative(1.0)).height(relative(1.0)))]).disabled(true).accessibility_hidden(true).with_key("preview")
        .with_style(ctx.component_style("preview",style().min_width(px(0)).width(if compact{px(56)}else{relative(1.0)})
            .height(if compact{px(36)}else{px(82)}).justify_center().items_center().background(theme_color("surface_hover"))
            .radius(px(7)).clip().flex_shrink(false)))
}
fn number(ctx,n){
    row([text(ctx.format_number(n))]).accessibility_hidden(true).with_style(ctx.component_style("number",style().width(theme_length("space.unit").times(5.5))
        .height(theme_length("space.unit").times(5.5)).radius(px(5)).flex_shrink(false).items_center().justify_center()
        .typography("caption").background(theme_color("surface_hover")).text_color(theme_color("text_muted"))))
}
fn row_for(ctx,row_config,options,i){
    let option=options[i];let token=row_config.token;let current=i==row_config.cursor;
    let name=option.title;let reco=review::value_or(option,"recommended",false);
    let note=review::value_or(option,"summary","");let tiles=row_config.kind=="pick";
    let labels=[text(name).with_style(ctx.component_style("option_title",style().typography("body").font_weight(500).min_width(px(0)).line_clamp(2)))];
    if note!=""&&!tiles{labels.push(review::heading_note(ctx,note,"summary").with_style(style().min_width(px(0)).text_ellipsis().whitespace_nowrap()));}
    let cells=[number(ctx,i+1),column(labels).with_style(style().min_width(px(0)).flex_grow())];
    if reco{
        cells.push(review::heading_note(ctx,ctx.t_or("decision.recommended","Recommended"),"recommended").with_style(style().min_width(px(0))));
    }
    let content=row(cells).with_style(style().min_width(px(0)).flex_grow().gap(theme_length("space.spacing_unit").times(1.5)).items_center())
        .audit_allow(["mixed-type-in-row"]);
    let surface=if tiles&&!row_config.compact{column([preview(ctx,option,false),content])}
        else if tiles{row([content,preview(ctx,option,true)]).with_style(style().items_center())}else{row([content]).with_style(style().items_center())};
    let base=style().min_width(px(0)).min_height(theme_length("space.unit").times(8.75)).padding(theme_length("space.spacing_unit").times(1.25)).gap(theme_length("space.spacing_unit").times(1.25)).radius(px(7))
        .hover(style().background(theme_color("surface_hover"))).focus(style().background(theme_color("selection")))
        .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"));
    let node=surface.with_key(option.id).with_ref(element_ref(opt_ref(i))).tab_stop(current&&row_config.enabled)
        .disabled(!row_config.enabled).accessibility_role("option").accessibility_label(name)
        .accessibility_description(if reco{ctx.t_or("decision.recommended","Recommended")+" · "+note}else{note})
        .accessibility_key_shortcuts((i+1).to_string())
        .with_style(ctx.component_style(if tiles{["option","tile"]}else{["option"]},base));
    if !row_config.enabled{return node;}
    let kind=review::value_or(option,"answer","option");
    node=node.on_focus_within_change(Fn("option_focused").curry(token,i))
        .on_click(Fn("send_option").curry(answer(token,kind,if kind=="option"{option.id}else{""},"")))
        .on_key_value("plain+up",Fn("move_focus").curry(opt_ref(max(0,i-1))),())
        .on_key_value("plain+down",Fn("move_focus").curry(if i+1<options.len{opt_ref(i+1)}else{"custom"}),())
        .on_key_value("plain+home",Fn("move_focus").curry(opt_ref(0)),())
        .on_key_value("plain+end",Fn("move_focus").curry(opt_ref(options.len-1)),());
    if tiles{
        node=node.on_key_value("plain+left",Fn("move_focus").curry(opt_ref(max(0,i-1))),())
            .on_key_value("plain+right",Fn("move_focus").curry(opt_ref(min(options.len-1,i+1))),());
    }
    for j in 0..options.len{
        let other=options[j];let kind=review::value_or(other,"answer","option");
        node=node.on_key_value("plain+"+(j+1).to_string(),Fn("send_option")
            .curry(answer(token,kind,if kind=="option"{other.id}else{""},"")),());
    }
    node.on_key_value("plain+"+(options.len+1).to_string(),Fn("move_focus").curry("custom"),())
}
fn send_option(spec,ctx,event){review::send("answer",spec,ctx,event)}
fn decision_body(ctx,item,token,kind,options,cursor,enabled,width){
    let children=[text(item.question).with_key("question").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("question",style().min_width(px(0)).padding_x(theme_length("space.spacing_unit").times(1.25)).typography("body").font_weight(500)))];
    let facts=review::value_or(item,"facts",[]);
    if facts.len>0{
        let lines=[];
        for i in 0..facts.len{
            let f=facts[i];
            lines.push(row([review::heading_note(ctx,f.label,"fact_label").with_style(style().width(theme_length("space.unit").times(20.5)).flex_shrink(false).text_ellipsis()),
                text(f.value).with_style(ctx.component_style("fact_value",style().min_width(px(0)).flex_grow().typography("body").line_clamp(2)))])
                .with_key(i.to_string()).audit_allow(["mixed-type-in-row"])
                .with_style(ctx.component_style("fact",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.75)))));
        }
        children.push(column(lines).with_key("facts").with_style(ctx.component_style("facts",style().min_width(px(0)).padding_x(theme_length("space.spacing_unit").times(1.25)).gap(theme_length("space.spacing_unit").times(0.5)))));
    }
    let compact=width<420.0;let grid=kind=="pick"&&!compact;let rows=[];
    let cols=if width<560.0{3}else{max(3,options.len)};
    for i in 0..options.len{
        let option=row_for(ctx,#{token:token,cursor:cursor,kind:kind,compact:compact,enabled:enabled},options,i);
        if grid{option=option.with_style(style().grid_cell(i%cols+1,i/cols+1));}
        rows.push(option);
    }
    if rows.len>0{
        let list=if grid{box(rows).with_style(style().grid().grid_cols(cols))}else{column(rows)};
        list=list.with_key("options").accessibility_role("listbox").accessibility_label(item.question)
            .with_style(ctx.component_style("options",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(0.75))));
        children.push(list);
    }
    let draft=draft_for(ctx,token);let placeholder=ctx.t_or("decision.custom","Something else…");
    let field=input::Input(#{key:token.request+":"+token.revision.to_string(),label:ctx.t_or("decision.answer_label","Your answer"),
        value:draft,placeholder:placeholder,appearance:"embedded",read_only:!enabled,
        on_change:if enabled{Fn("edit_draft").curry(token)}else{()},
        on_submit:if enabled{Fn("submit").curry(token)}else{()}})
        .with_ref(element_ref("custom")).with_style(ctx.component_style("editor",style().min_width(px(0)).flex_grow()
            .padding(px(0)).border(px(0)).shadows([])));
    if enabled&&options.len>0&&draft==""{field=field.on_bubble("key:plain+up",Fn("move_focus").curry(opt_ref(options.len-1)));}
    let own=[number(ctx,options.len+1),field];
    if trim_text(draft)!=""{
        own.push(icon_button::IconButton(#{key:"send",label:ctx.t_or("decision.send","Send answer"),icon:icon(ctx,"send"),
            variant:"primary",size:"xs",disabled:!enabled,on_click:Fn("submit_button").curry(token,draft)})
            .with_style(ctx.component_style("send",style().flex_shrink(false))));
    }
    children.push(row(own).with_key("own").with_style(ctx.component_style("own",style().min_width(px(0)).items_center()
        .gap(theme_length("space.spacing_unit").times(1.5)).padding(theme_length("space.spacing_unit").times(1.25)).radius(px(7)).focus_within(style().focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))))));
    let status=review::opt(item,"notice");
    if status!=(){children.push(review::heading_note(ctx,status,"notice").accessibility_role("status").with_key("notice"));}
    column(children).with_key("body").audit_allow(["text-edge-misaligned"]).with_style(ctx.component_style("card",style()
        .min_width(px(0)).padding(theme_length("space.spacing_unit").times(1.5)).gap(theme_length("space.spacing_unit")).radius(theme_length("space.unit").times(3)).background(theme_color("surface_raised"))))
}
fn render_DecisionCard(ctx,props){
    let items=props.decisions;let seen=#{};
    for current in items{
        let token=item_token(current);
        if token.request in seen{throw "Open decision IDs must be unique";}
        seen[token.request]=token.revision;
    }
    let state=ctx.get_state("folded");let folded=if state==(){props.initial_folded}else{state};
    let index=active_page(ctx,items);let scene=[];let focus="";let current=();
    let surface_ref=element_ref("surface");let size=ctx.element_size(surface_ref);let width=if size==(){550.0}else{size.width};
    if items.len==0{
        scene.push(review::heading_note(ctx,ctx.t_or("decision.empty","Nothing else waits"),"empty").accessibility_role("status"));
    }else{
        let item=items[index];current=item_token(item);
        let tag=text(ctx.t_or("decision.needs","Needs you")).with_style(ctx.component_style("badge",style().typography("caption")
            .padding_x(theme_length("space.spacing_unit").times(1.5)).padding_y(theme_length("space.spacing_unit").times(0.5)).radius(px(5)).background(theme_color("surface_hover")).text_color(theme_color("text.warning"))));
        let head=[tag];
        if folded{
            head.push(review::heading_note(ctx,ctx.t_or("decision.waiting","{count} waiting",#{count:ctx.format_number(items.len)}),"task")
                .with_style(style().min_width(px(0)).flex_grow()));
            head.push(button::Button(#{key:"show",text:ctx.t_or("decision.show","Show decisions"),size:"xs",variant:"ghost",on_click:Fn("show")})
                .with_key("show").with_ref(element_ref("show")).with_style(ctx.component_style("show",style().flex_shrink(false))));
        }else{
            let task=review::value_or(item,"task","");
            if task!=""{head.push(review::heading_note(ctx,task,"task").with_style(style().min_width(px(0)).flex_grow().text_ellipsis()));}
            if items.len>1{
                let prev=(index+items.len-1)%items.len;let next=(index+1)%items.len;
                head.push(row([
                    icon_button::IconButton(#{key:"prev",label:ctx.t_or("decision.previous","Previous decision"),icon:icon(ctx,if ctx.text_direction()=="rtl"{"forward"}else{"back"}),
                        size:"xs",on_click:Fn("page_to").curry(items[prev].id,prev)}).with_style(ctx.component_style("page",style())),
                    review::heading_note(ctx,ctx.t_or("decision.of","{index} of {count}",#{index:ctx.format_number(index+1),count:ctx.format_number(items.len)}),"count"),
                    icon_button::IconButton(#{key:"next",label:ctx.t_or("decision.next","Next decision"),icon:icon(ctx,if ctx.text_direction()=="rtl"{"back"}else{"forward"}),
                        size:"xs",on_click:Fn("page_to").curry(items[next].id,next)}).with_style(ctx.component_style("page",style())),
                ]).with_key("pager").with_style(ctx.component_style("pager",style().gap(theme_length("space.spacing_unit").times(0.75)).items_center().flex_shrink(false))));
            }
            head.push(button::Button(#{key:"later",text:ctx.t_or("decision.later","Later"),size:"xs",variant:"ghost",
                on_click:Fn("defer").curry(current)}).with_style(ctx.component_style("later",style().flex_shrink(false))));
        }
        scene.push(row(head).with_key("head").with_style(ctx.component_style("head",style().min_width(px(0))
            .padding(theme_length("space.spacing_unit")).gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap())).audit_allow(["mixed-type-in-row"]));
        if !folded{
            let kind=review::value_or(item,"kind","decision");let opts=answers(ctx,item,kind);
            let stored=ctx.get_state("cursor");let cursor=if stored!=()&&stored.token==current&&stored.index<opts.len{stored.index}else{recommended(opts)};
            let enabled=!review::value_or(item,"busy",false)&&review::opt(props,"on_answer")!=();
            if enabled{focus=if opts.len==0{"custom"}else{opt_ref(cursor)};}
            scene.push(decision_body(ctx,item,current,kind,opts,cursor,enabled,width));
        }
    }
    effect("present",#{seen:seen,current:current,reveal:props.reveal,folded:folded,focus:focus,plan:ctx.get_state("focus_plan")},Fn("data_set"),Fn("clear_data"));
    let root=column(scene).with_ref(surface_ref).with_key(props.key).accessibility_role("region")
        .accessibility_label(ctx.t_or("decision.region","Decision card")).on_focus_within_change(Fn("report_focus"))
        .audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(640))
            .padding(theme_length("space.spacing_unit")).gap(theme_length("space.spacing_unit").times(0.75)).radius(theme_length("space.unit").times(4.5)).background(theme_color("surface_hover"))));
    if current==(){
        return root.with_style(style().height(px(0)).min_height(px(0)).padding(px(0)).clip()
            .background(rgba(0x00000000)).shadows([]));
    }
    if !folded{root=root.on_bubble("key:plain+escape",Fn("defer").curry(current));}
    root
}
