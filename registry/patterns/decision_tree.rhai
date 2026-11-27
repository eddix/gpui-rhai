/* gpui-rhai
{"id":"patterns/decision_tree","export":"DecisionTree","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/hover_card","components/icon_button","components/textarea","patterns/approval_card","patterns/decision_card"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","selection","surface_raised","surface_hover","text.danger","text.success","text.warning","text_muted","text_primary","radius.lg","spacing.md","typography.body","typography.caption","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// A geometry-backed tree of versioned choices. Only a Host receipt resolves a request.
// State: visible request, focused/pointed branch, expanded branch and custom draft.
// Example: DecisionTree(#{key:"plan",decisions:[#{id:"plan",revision:0,agent:"Planner",question:"Continue?",options:[#{id:"go",label:"Proceed"}]}]})
import "components/button" as button;
import "components/hover_card" as hover_card;
import "components/icon_button" as icon_button;
import "components/textarea" as textarea;
import "patterns/approval_card" as review;
import "patterns/decision_card" as cards;

define_component(#{
    metadata:#{id:"patterns/decision_tree","export":"DecisionTree",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/hover_card","components/icon_button","components/textarea","patterns/approval_card","patterns/decision_card"],
        capabilities:#{},tokens:["accent","border","focus_ring","selection","surface_raised","surface_hover","text.danger","text.success","text.warning","text_muted","text_primary","radius.lg","spacing.md","typography.body","typography.caption","space.spacing_unit","space.unit"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable tree instance key."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional heading above the pending queue."},
        allow_other:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Offer the user a custom instruction branch."},
        decisions:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique request ID."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Version on which the user's answer is based."},
            question:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Question at the root of this decision."},
            context:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Context below the question."},
            agent:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Who is asking."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional model name."},
            risk:#{schema:#{type:"optional",value:#{type:"string",allowed:["low","medium","high"]}},required:false,sensitive:false,doc:"Optional assessment of this decision's risk."},
            options:#{schema:#{type:"ui_value"},required:true,sensitive:true,doc:"One to eight unique #{id,label,effects?,recommended?,children?} options. Each effect is plain text or an array of text and #{text,note,source?} spans. A branch may contain one level of choices; option IDs are unique across the request."},
            busy:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Host is considering a proposal; choices are not available."},
            notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Non-final Host feedback, such as a declined proposal."},
            receipt:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
                option:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Chosen leaf option ID, or empty if a custom instruction was accepted."},
                message:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Accepted result returned by the Host."},
            }}},required:false,sensitive:true,doc:"Only an accepted Host receipt locks the choice and shows the final outcome."},
        }}},required:true,sensitive:true,doc:"Requests in Host presentation order; remove a finished request when its receipt no longer needs to be shown."},
        on_choose:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives one versioned proposal when a leaf or custom instruction is submitted."},
    },state:#{fields:#{
        page:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        hover:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        focus:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        focus_plan:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        panel:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        draft:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
    }},events:#{
        choose:#{payload:#{type:"object",allow_unknown:false,fields:#{
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"The decision being answered."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"The version presented to the user."},
            option:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Chosen leaf ID; empty for a custom instruction."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Trimmed custom instruction, or empty for a known choice."},
        }},doc:"Versioned answer proposal. A proposal does not change accepted facts."},
    },slots:#{},parts:["root","head","title","pager","count","board","question","avatar","agent","model","text","context",
        "branches","branch","choice","active","muted","number","label","recommended","leaf","effect","flag","flag_label","flag_tip","source",
        "subchoices","editor","receipt","notice","empty","connector","icon","risk","risk_low","risk_medium","risk_high"],
        effects:["focus_control"]},
    render:Fn("render_DecisionTree")
});

fn DecisionTree(props){render_component("patterns/decision_tree",props)}
fn bounded(value,limit){type_of(value)=="string"&&utf8_byte_count(value)<=limit}
fn token(item){#{request:item.id,revision:item.revision}}
fn value(ctx,key,t,fallback){
    let state=ctx.get_state(key);if state!=()&&state.token==t{state.value}else{fallback}
}
fn tagged(ctx,key,t,v){ctx.set_state(key,#{token:t,value:v});}
fn nonempty_id(id){bounded(id,128)&&id!=""}
fn spans(value){
    if bounded(value,4000){return [value];}
    if type_of(value)!="array"||value.len>32{throw "Decision evidence must be bounded text or a list of text and flagged spans";}
    let bytes=0;
    for span in value{
        if bounded(span,4000){bytes+=utf8_byte_count(span);}
        else{
            if type_of(span)!="map"{throw "Invalid decision evidence span";}
            for field in span.keys(){if !(field in ["text","note","source"]){throw "Unknown decision evidence field";}}
            if !("text" in span)||!bounded(span.text,4000)||!("note" in span)||!bounded(span.note,4000)||span.note==""||
                ("source" in span&&!bounded(span.source,1600)){throw "Flagged evidence needs bounded text, a note, and an optional source label";}
            bytes+=utf8_byte_count(span.text)+utf8_byte_count(span.note)+utf8_byte_count(review::value_or(span,"source",""));
        }
        if bytes>16000{throw "Decision evidence exceeds 16,000 UTF-8 bytes";}
    }
    value
}
// Shared with decision/review patterns. This is display evidence, not markup or a link.
fn plain_evidence(value){
    let plain="";for span in spans(value){plain+=if type_of(span)=="string"{span}else{span.text};} plain
}
fn evidence_line(ctx,key,value){
    let parts=spans(value);
    if parts.len==1&&type_of(parts[0])=="string"{
        return text(parts[0]).with_key(key).with_style(ctx.component_style("effect",style().min_width(px(0)).typography("caption").text_color(theme_color("text_muted"))));
    }
    let nodes=[];for i in 0..parts.len{
        let part=parts[i];let id="span-"+i.to_string();
        if type_of(part)=="string"{nodes.push(text(part).with_key(id).with_style(style().min_width(px(0)).max_width(relative(1.0))));continue;}
        let label=ctx.t_or("decision_tree.uncertainty","Uncertain evidence");
        let source=review::value_or(part,"source","");
        let description=if source==""{part.note}else{part.note+" · "+source};
        let mark=row([text(part.text).with_key("span-text").with_style(style().min_width(px(0)))])
            .with_key("flag-mark").tab_stop(true).accessibility_role("group").accessibility_label(part.text).accessibility_description(label+": "+description)
            .with_style(ctx.component_style("flag",style().min_width(px(0)).max_width(relative(1.0)).padding_x(theme_length("space.spacing_unit").times(0.5))
                .background(theme_color("surface_hover")).text_color(theme_color("text.warning"))
                .focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))));
        let detail=[text(label).with_key("heading").with_style(ctx.component_style("flag_label",style().font_weight(600))),
            text(part.note).with_key("note")];
        if source!=""{detail.push(text(source).with_key("source").with_style(ctx.component_style("source",style().font_weight(500))));}
        nodes.push(hover_card::HoverCard(#{key:id,label:description,trigger:mark,content:column(detail)
            .with_key("evidence").with_style(ctx.component_style("flag_tip",style().min_width(px(0)).gap(theme_length("space.spacing_unit")))),
            placement:"bottom",show_delay_ms:250}));
    }
    row(nodes).with_key(key).audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("effect",style().min_width(px(0)).flex_wrap().items_center().typography("caption").text_color(theme_color("text_muted"))))
}
fn checked_option(opt,nested){
    if type_of(opt)!="map"||!("id" in opt)||!nonempty_id(opt.id)||!("label" in opt)||!bounded(opt.label,600)||opt.label==""{
        throw "Decision branches need a bounded stable ID and a nonempty label";
    }
    for field in opt.keys(){if !(field in ["id","label","effects","recommended","children"]){throw "Unknown decision branch field";}}
    if "recommended" in opt&&type_of(opt.recommended)!="bool"{throw "Decision branch recommended must be a boolean";}
    let effects=review::value_or(opt,"effects",[]);
    if type_of(effects)!="array"||effects.len>6{throw "A decision branch supports up to six effects";}
    for effect in effects{spans(effect);}
    if "children" in opt{
        if nested||type_of(opt.children)!="array"||opt.children.len==0||opt.children.len>8{
            throw "Decision branches support one level of one to eight leaf choices";
        }
    }
}
fn active_page(ctx,items){
    let ids=#{}; let page=ctx.get_state("page");let index=if page==(){0}else{min(page.index,items.len-1)};
    for i in 0..items.len{
        let id=items[i].id;
        if !nonempty_id(id)||id in ids{throw "Decision request IDs must be bounded and unique";}
        ids[id]=true;
        if page!=()&&id==page.id{index=i;}
    }
    index
}
fn options_for(item){
    let choices=item.options;let ids=#{};let selected=review::opt(item,"receipt");let found=selected==()||selected.option=="";
    if type_of(choices)!="array"||choices.len==0||choices.len>8{throw "DecisionTree expects one to eight branches";}
    for opt in choices{
        checked_option(opt,false);
        if opt.id in ids{throw "Decision option ID reused";}
        ids[opt.id]=true;
        if "children" in opt{
            for leaf in opt.children{
                checked_option(leaf,true);
                if leaf.id in ids{throw "Decision option ID reused";}
                ids[leaf.id]=true;
                if selected!=()&&selected.option==leaf.id{found=true;}
            }
        }else if selected!=()&&selected.option==opt.id{found=true;}
    }
    if !found{throw "Decision receipt must identify an offered leaf";}
    choices
}
fn page_to(item,index,ctx,event){ctx.set_state("page",#{id:item,index:index});handled()}
fn point(t,index,ctx,entered){
    if entered{tagged(ctx,"hover",t,index);}
    else if value(ctx,"hover",t,-1)==index{tagged(ctx,"hover",t,-1);}
}
fn focus_branch(t,index,ctx,entered){
    if entered{tagged(ctx,"focus",t,index);}
    else if value(ctx,"focus",t,-1)==index{tagged(ctx,"focus",t,-1);}
}
fn first_target(index){"branch-"+index.to_string()}
fn leaf_target(index){"sub-"+index.to_string()}
fn open_panel(t,index,ctx,event){
    tagged(ctx,"panel",t,index);
    tagged(ctx,"focus_plan",t,index);handled()
}
fn focus_after_commit(ctx,plan){
    if plan.target==()||value(ctx,"focus_plan",plan.token,())!=plan.target{return;}
    let target=if plan.target<0{"editor"}else{leaf_target(0)};
    ctx.set_state("focus_plan",());
    ctx.focus(target);
}
fn forget_panel(ctx,plan){}
fn close_panel(t,focus,ctx,event){
    tagged(ctx,"panel",t,());ctx.focus(focus);handled()
}
fn choose(t,id,ctx,event){
    ctx.emit("choose",#{request:t.request,revision:t.revision,option:id,text:""});handled()
}
fn draft(t,ctx,v){if utf8_byte_count(v)<=16000{tagged(ctx,"draft",t,v);}}
fn submit(t,ctx,v){
    let value=v; value.trim();
    if value!=""&&utf8_byte_count(value)<=16000{
        ctx.emit("choose",#{request:t.request,revision:t.revision,option:"",text:value});
    }
    handled()
}
fn send_draft(t,ctx,event){submit(t,ctx,value(ctx,"draft",t,""))}
fn risk_color(risk){theme_color(if risk=="high"{"text.danger"}else if risk=="medium"{"text.warning"}else{"accent"})}
fn question_card(ctx,item,target){
    let identity=[text(item.agent).with_key("agent").with_style(ctx.component_style("agent",style().typography("caption").font_weight(600)))];
    let model=review::value_or(item,"model","");
    if model!=""{identity.push(review::heading_note(ctx,model,"model").with_key("model"));}
    let who=[row([text(if item.agent==""{"AI"}else{item.agent.sub_string(0,1).to_upper()})]).with_key("avatar").accessibility_hidden(true)
        .with_style(ctx.component_style("avatar",style().width(theme_length("space.unit").times(6.5)).height(theme_length("space.unit").times(6.5)).flex_shrink(false)
        .items_center().justify_center().background(theme_color("surface_hover")).radius(px(6)).typography("caption"))),
        column(identity).with_key("identity").with_style(style().min_width(px(0)).flex_grow())];
    let risk=review::value_or(item,"risk","low");
    if risk!="low"{who.push(review::risk_badge(ctx,risk,review::risk_label(ctx,risk)));}
    let body=[row(who).with_key("who").audit_allow(["mixed-type-in-row"]).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).flex_wrap().items_center()),
        text(item.question).with_key("question").accessibility_role("heading").accessibility_level(3)
            .with_style(ctx.component_style("text",style().min_width(px(0)).typography("body").font_weight(600)))];
    let description=review::value_or(item,"context","");
    if description!=""{body.push(review::heading_note(ctx,description,"context").with_key("context"));}
    column(body).with_key("root-question").with_ref(target).with_style(ctx.component_style("question",
        style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(3)).gap(theme_length("space.spacing_unit").times(2)).radius(theme_radius("lg")).background(theme_color("surface_raised"))))
}
fn numbered_label(ctx,n,label,recommended){
    let cells=[row([text(ctx.format_number(n))]).with_key("number").accessibility_hidden(true)
        .with_style(ctx.component_style("number",style().min_width(px(20)).height(theme_length("space.unit").times(5)).flex_shrink(false).typography("caption")
            .items_center().justify_center().radius(px(5)).background(theme_color("surface_hover")))),
        text(label).with_key("label").with_style(ctx.component_style("label",style().flex_grow().min_width(px(0)).typography("body").font_weight(500)))];
    if recommended{cells.push(review::heading_note(ctx,ctx.t_or("decision_tree.pick","Agent's pick"),"recommended").with_key("recommended"));}
    row(cells).with_key("label-row").audit_allow(["mixed-type-in-row"]).with_style(style().gap(theme_length("space.spacing_unit").times(1.5)).items_center().min_width(px(0)).flex_wrap())
}
fn effects(ctx,key,list){
    let rows=[];for i in 0..list.len{rows.push(evidence_line(ctx,"effect-"+i.to_string(),list[i]));}
    column(rows).with_key(key).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(0.75)))
}
fn choice_surface(ctx,part,active,muted){
    let parts=[part];if active{parts.push("active");}else if muted{parts.push("muted");}
    ctx.component_style(parts,style().min_width(px(0)).min_height(theme_length("space.unit").times(8.5)).gap(theme_length("space.spacing_unit").times(1.25)).padding_x(theme_length("space.spacing_unit").times(2.5)).padding_y(theme_length("space.spacing_unit").times(1.5))
        .radius(theme_radius("lg")).background(theme_color(if active{"selection"}else{"surface_raised"}))
        .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring")))
}
fn outcome(ctx,receipt){
    review::heading_note(ctx,receipt.message,"receipt").with_key("receipt").accessibility_role("status")
        .motion(motion_transition("opacity",0.45,1.0,#{duration_ms:200,intent:"feedback"}))
}
fn action_for(opt,t,i){if "children" in opt{Fn("open_panel").curry(t,i)}else{Fn("choose").curry(t,opt.id)}}
fn first_keys(node,t,i,choices,other){
    let count=choices.len+if other{1}else{0};
    node=node.on_key_value("plain+up",Fn("move_focus").curry(first_target(max(0,i-1))),())
        .on_key_value("plain+down",Fn("move_focus").curry(first_target(min(count-1,i+1))),())
        .on_key_value("plain+home",Fn("move_focus").curry(first_target(0)),())
        .on_key_value("plain+end",Fn("move_focus").curry(first_target(count-1)),());
    for j in 0..choices.len{
        node=node.on_key_value("plain+"+(j+1).to_string(),action_for(choices[j],t,j),());
    }
    if other{node=node.on_key_value("plain+"+count.to_string(),Fn("open_panel").curry(t,-1),());}
    node
}
fn move_focus(key,ctx,event){ctx.focus(key);handled()}
fn branch(ctx,cfg,choices,i,target){
    let opt=choices[i];let active=i==cfg.lit;let muted=cfg.accepted&&cfg.lit>=0&&!active;
    let choice=column([numbered_label(ctx,i+1,opt.label,review::value_or(opt,"recommended",false))]).with_key("button")
        .with_ref(target).accessibility_role("button").accessibility_label(opt.label)
        .accessibility_key_shortcuts((i+1).to_string()).tab_stop(cfg.enabled)
        .disabled(!cfg.enabled).with_style(choice_surface(ctx,"choice",active,muted));
    if "children" in opt{choice=choice.accessibility_expanded(cfg.panel==i);}
    if cfg.accepted&&cfg.chosen==opt.id{choice=choice.accessibility_current("true");}
    if cfg.enabled{
        choice=first_keys(choice.on_click(action_for(opt,cfg.token,i)).on_hover_change(Fn("point").curry(cfg.token,i)),cfg.token,i,choices,cfg.other);
    }
    let evidence=review::value_or(opt,"effects",[]);
    let detail=[];if evidence.len>0{detail.push(effects(ctx,"effects",evidence));}
    let accepted_leaf="children" in opt&&cfg.accepted&&active;
    if "children" in opt&&(cfg.panel==i||accepted_leaf){
        choice=choice.accessibility_expanded(true);
        let leaves=[];for j in 0..opt.children.len{
            let leaf=opt.children[j];
            let tile=column([numbered_label(ctx,j+1,leaf.label,review::value_or(leaf,"recommended",false))])
                .with_key("subchoice-"+j.to_string()).with_ref(element_ref(leaf_target(j)))
                .accessibility_role("button").accessibility_label(leaf.label).tab_stop(cfg.enabled).disabled(!cfg.enabled)
                .with_style(choice_surface(ctx,"choice",cfg.chosen==leaf.id,false));
            if cfg.accepted&&cfg.chosen==leaf.id{tile=tile.accessibility_current("true");}
            if cfg.enabled{
                tile=tile.on_click(Fn("choose").curry(cfg.token,leaf.id))
                    .on_key_value("plain+up",Fn("move_focus").curry(leaf_target(max(0,j-1))),())
                    .on_key_value("plain+down",Fn("move_focus").curry(leaf_target(min(opt.children.len-1,j+1))),())
                    .on_key_value("plain+escape",Fn("close_panel").curry(cfg.token,first_target(i)),());
            }
            let consequences=review::value_or(leaf,"effects",[]);
            leaves.push(column(if consequences.len>0{[tile,effects(ctx,"effects",consequences)]}else{[tile]})
                .with_key(leaf.id).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit"))));
        }
        detail.push(column(leaves).with_key("subchoices").with_style(ctx.component_style("subchoices",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.75)))));
    }
    [choice,detail]
}
fn other(ctx,cfg,choices,i,target){
    let label=ctx.t_or("decision_tree.other","Other…");
    let choice=column([numbered_label(ctx,i+1,label,false)]).with_key("button")
        .with_ref(target).accessibility_role("button").accessibility_label(label)
        .accessibility_key_shortcuts((i+1).to_string()).accessibility_expanded(cfg.panel==-1)
        .tab_stop(cfg.enabled).disabled(!cfg.enabled).with_style(choice_surface(ctx,"choice",i==cfg.lit,false));
    if cfg.enabled{
        choice=first_keys(choice.on_click(Fn("open_panel").curry(cfg.token,-1))
            .on_hover_change(Fn("point").curry(cfg.token,i)),cfg.token,i,choices,true);
    }
    if cfg.accepted&&cfg.chosen==""{choice=choice.accessibility_current("true");}
    let detail=[];
    if cfg.panel==-1&&!cfg.accepted{
        let draft=value(ctx,"draft",cfg.token,"");
        detail.push(textarea::Textarea(#{key:"custom",label:ctx.t_or("decision_tree.instruction","Custom instruction"),value:draft,
            editor_ref:element_ref("editor"),rows:2,max_length:4000,submit_key:"enter",disabled:!cfg.enabled,
            placeholder:ctx.t_or("decision_tree.placeholder","Tell the agent what to do"),
            on_change:Fn("draft").curry(cfg.token),on_submit:Fn("submit").curry(cfg.token)})
            .with_style(ctx.component_style("editor",style().min_width(px(0)).width(relative(1.0))))
            .on_bubble("key:plain+escape",Fn("close_panel").curry(cfg.token,first_target(i))));
        detail.push(button::Button(#{key:"send",text:ctx.t_or("decision_tree.send","Send instruction"),size:"sm",
            disabled:!cfg.enabled||draft=="",on_click:Fn("send_draft").curry(cfg.token)}).with_style(style().self_start()));
    }
    [choice,detail]
}
fn paired(ctx,pair,cfg,key,i,target){
    let nodes=[pair[0].with_style(style().min_width(px(0)).width(if cfg.compact{relative(1.0)}else{relative(0.43)}))];
    if pair[1].len>0{
        nodes.push(column(pair[1]).with_key("leaf").with_ref(target)
            .with_style(ctx.component_style("leaf",style().min_width(px(0)).gap(theme_length("space.spacing_unit")).flex_grow().padding(theme_length("space.spacing_unit").times(1.75))
                .radius(theme_radius("lg")).background(theme_color("surface_hover")))));
    }
    let branch=if cfg.compact{column(nodes).with_style(style().padding_start(theme_length("space.spacing_unit").times(5.5)).gap(theme_length("space.spacing_unit").times(1.5)))}else{row(nodes).with_style(style().items_center().gap(theme_length("space.spacing_unit").times(6)))};
    branch.with_key(key).audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("branch",style().min_width(px(0))))
        .on_focus_within_change(Fn("focus_branch").curry(cfg.token,i))
}
fn mid_y(b){b.y+b.height*0.5}
fn edge(b,right){if right{b.x+b.width}else{b.x}}
fn curve(key,x1,y1,x2,y2,color,hot){
    let dx=(x2-x1)*0.55;
    canvas_stroke_path(key,[path_move(x1,y1),path_cubic(x2,y2,x1+dx,y1,x2-dx,y2)],if hot{1.6}else{1.0},color).stroke_cap("round")
}
fn connectors(ctx,cfg,roots,first_refs,leaf_refs){
    let rb=ctx.element_bounds(roots.board);let qb=ctx.element_bounds(roots.question);
    if rb==()||qb==(){return ();}
    let frame=rb.layout;let q=qb.layout;let layers=[];
    if frame.width<=0.0||frame.height<=0.0{return ();}
    for i in 0..first_refs.len{
        let measured=ctx.element_bounds(first_refs[i]);if measured==(){continue;}
        let b=measured.layout;let lit=i==cfg.lit;
        let ink=if lit{risk_color(cfg.risk)}else{theme_color("border")};
        let id=i.to_string();let by=mid_y(b)-frame.y;
        if cfg.compact{
            let start_right=b.x<=q.x+1.0;
            let x2=edge(b,start_right)-frame.x;
            let rail=x2+if start_right{12.0}else{-12.0};
            let start_y=q.y+q.height-frame.y;
            // The full-height rail lies in the branches' start padding, not behind their text.
            layers.push(canvas_stroke_path("branch-"+id,
                [path_move(rail,start_y),path_line(rail,by),path_line(x2,by)],if lit{1.6}else{1.0},ink)
                .stroke_join("round").stroke_cap("round"));
        }else{
            let right=b.x>=q.x;let x2=edge(b,!right)-frame.x;
            layers.push(curve("branch-"+id,edge(q,right)-frame.x,mid_y(q)-frame.y,x2,by,ink,lit));
            let leaf=if leaf_refs[i]==(){()}else{ctx.element_bounds(leaf_refs[i])};
            if leaf!=(){let l=leaf.layout;layers.push(curve("leaf-"+id,edge(b,right)-frame.x,by,edge(l,!right)-frame.x,mid_y(l)-frame.y,ink,lit));}
        }
    }
    if layers.len==0{return ();}
    canvas(canvas_scene(layers).view_box(0,0,frame.width,frame.height)).with_key("lines").accessibility_hidden(true)
        .with_style(ctx.component_style("connector",style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0))))
}
fn render_DecisionTree(ctx,props){
    let items=props.decisions;let title=review::value_or(props,"title",ctx.t_or("decision_tree.title","Decision tree"));
    let header=[text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
        .with_style(ctx.component_style("title",style().typography("body").font_weight(600).flex_grow().min_width(px(0))))];
    if items.len==0{
        return column([header[0],review::heading_note(ctx,ctx.t_or("decision_tree.empty","No decisions are waiting"),"empty").with_key("empty")])
            .with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(3))));
    }
    let index=active_page(ctx,items);let item=items[index];let selected=review::opt(item,"receipt");let t=token(item);
    let choices=options_for(item);
    let count=ctx.format_number(index+1)+" / "+ctx.format_number(items.len);
    let prev=items[max(0,index-1)].id;let next=items[min(items.len-1,index+1)].id;
    if items.len>1{
        header.push(row([
            icon_button::IconButton(#{key:"prev",label:ctx.t_or("decision.previous","Previous decision"),icon:cards::icon(ctx,if ctx.text_direction()=="rtl"{"forward"}else{"back"}),
                size:"sm",disabled:index==0,on_click:Fn("page_to").curry(prev,max(0,index-1))}),
            review::heading_note(ctx,count,"count").with_key("count").accessibility_role("status"),
            icon_button::IconButton(#{key:"next",label:ctx.t_or("decision.next","Next decision"),icon:cards::icon(ctx,if ctx.text_direction()=="rtl"{"back"}else{"forward"}),
                size:"sm",disabled:index+1==items.len,on_click:Fn("page_to").curry(next,min(items.len-1,index+1))}),
        ]).with_key("pager").with_style(ctx.component_style("pager",style().gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_shrink(false))));
    }
    let roots=#{board:element_ref("board"),question:element_ref("question")};
    let width=ctx.element_size(roots.board);let compact=width!=()&&width.width<620.0;
    let allowed=props.allow_other;let panel=value(ctx,"panel",t,());let lit=value(ctx,"hover",t,-1);
    let chosen=if selected==(){""}else{selected.option};
    if selected!=(){
        lit=if chosen==""&&allowed{choices.len}else{-1};
        for i in 0..choices.len{
            let choice=choices[i]; if choice.id==chosen{lit=i;}
            if "children" in choice{for leaf in choice.children{if leaf.id==chosen{lit=i;}}}
        }
    }else if lit<0{
        lit=value(ctx,"focus",t,-1);
        if lit<0&&panel!=(){lit=if panel<0{choices.len}else{panel};}
    }
    let cfg=#{token:t,enabled:selected==()&&!review::value_or(item,"busy",false)&&review::opt(props,"on_choose")!=(),
        accepted:selected!=(),chosen:chosen,panel:panel,lit:lit,risk:review::value_or(item,"risk","low"),compact:compact,other:allowed};
    effect("focus_control",#{token:t,target:if cfg.enabled{value(ctx,"focus_plan",t,())}else{()}},
        Fn("focus_after_commit"),Fn("forget_panel"));
    let branches=[];let first_refs=[];let leaf_refs=[];
    for i in 0..(choices.len+if allowed{1}else{0}){
        let target=element_ref(first_target(i));
        let pair=if i==choices.len{other(ctx,cfg,choices,i,target)}else{branch(ctx,cfg,choices,i,target)};
        if selected!=()&&i==lit{
            let facts=pair[1];facts.push(outcome(ctx,selected));pair[1]=facts;
        }
        let leaf=if pair[1].len>0{element_ref("leaf-"+i.to_string())}else{()};
        let key=if i==choices.len{"custom-choice"}else{"choice-"+choices[i].id};
        branches.push(paired(ctx,pair,cfg,key,i,leaf));first_refs.push(target);leaf_refs.push(leaf);
    }
    let question=question_card(ctx,item,roots.question).with_style(style().width(if compact{relative(1.0)}else{relative(0.26)}));
    let decisions=column(branches).with_key("branches").with_style(ctx.component_style("branches",style().gap(theme_spacing("md")).min_width(px(0)).flex_grow()));
    let arrangement=if compact{column([question,decisions])}else{row([question,decisions]).with_style(style().items_center())};
    let nodes=[];let lines=connectors(ctx,cfg,roots,first_refs,leaf_refs);if lines!=(){nodes.push(lines);}
    nodes.push(arrangement.with_key("layout").with_style(ctx.component_style("board",style().width(relative(1.0)).gap(theme_length("space.spacing_unit").times(7)).min_width(px(0)))));
    let result=[row(header).with_key("head").audit_allow(["mixed-type-in-row"])
        .with_style(ctx.component_style("head",style().gap(theme_length("space.spacing_unit").times(2)).min_width(px(0)).items_center())),
        stack(nodes).with_key("decision-board").with_ref(roots.board).with_style(style().relative().min_width(px(0)).width(relative(1.0)))];
    if selected!=(){if lit<0{result.push(outcome(ctx,selected));}}
    else{
        let notice=review::value_or(item,"notice",if review::value_or(item,"busy",false){ctx.t_or("decision_tree.pending","Waiting for confirmation…")}else{""});
        if notice!=""{result.push(review::heading_note(ctx,notice,"notice").with_key("notice").accessibility_role("status"));}
    }
    column(result).with_key(props.key).accessibility_role("group").accessibility_label(title)
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(960)).gap(theme_length("space.spacing_unit").times(3))))
}
