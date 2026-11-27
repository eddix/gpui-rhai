/* gpui-rhai
{"id":"patterns/decision_inbox","export":"DecisionInbox","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","patterns/approval_card"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","surface_raised","surface_hover","text.success","text.warning","text_primary","text_muted","typography.body","typography.caption","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// DecisionInbox is a queue of decisions with two quick responses and accepted receipts.
// State: local disclosure only; the Host owns the phase, answer and receipt lifetime.
// Example: DecisionInbox(#{key:"incoming",decisions:[#{id:"reply",revision:0,title:"Reply to the request",primary:"Send",secondary:"Wait"}]})
import "components/button" as button;
import "patterns/approval_card" as review;

define_component(#{
    metadata:#{id:"patterns/decision_inbox","export":"DecisionInbox",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","patterns/approval_card"],capabilities:#{},
        tokens:["border","focus_ring","surface_raised","surface_hover","text.success","text.warning","text_primary","text_muted","typography.body","typography.caption","space.spacing_unit","space.unit"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable presentation identity."},
        decisions:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique request ID."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision echoed in a proposal."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Short title."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Context revealed with the actions."},
            primary:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Primary quick-answer caption."},
            secondary:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Secondary quick-answer caption."},
            collapsed:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Default disclosure for a new item revision."},
            phase:#{schema:#{type:"optional",value:#{type:"string",allowed:["pending","busy","resolved"]}},required:false,sensitive:false,
                doc:"Accepted phase; pending by default."},
            receipt:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted result shown while the resolved row is retained."},
            notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional Host rejection or clarification."},
        }}},required:true,sensitive:true,doc:"Ordered open and briefly retained resolved decisions. Host removal shortens the queue."},
        auto_open_next:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,
            doc:"Open the next folded pending item after an accepted receipt arrives."},
        max_height:#{schema:#{type:"optional",value:#{type:"length"}},required:false,sensitive:false,doc:"Positive absolute bound for the native viewport, default 440px."},
        on_answer:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a versioned primary or secondary proposal."},
    },state:#{fields:#{
        opened:#{schema:#{type:"ui_value"},"default":#{type:"map",value:#{}}},
        previous:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        answer:#{payload:#{type:"object",allow_unknown:false,fields:#{
            request:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Request being answered."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision when answering."},
            choice:#{schema:#{type:"string",allowed:["primary","secondary"]},required:true,sensitive:false,doc:"A quick-answer lane, not a displayed label."},
        }},doc:"Host-owned answer proposal. The Inbox never resolves optimistically."},
    },slots:#{},parts:["root","head","badge","count","card","row","first","title","closed","detail","actions","primary","secondary","receipt","notice","empty","icon"],
        effects:["rows"]},
    render:Fn("render_DecisionInbox")
});

fn DecisionInbox(props){render_component("patterns/decision_inbox",props)}
fn token(item){review::target(#{request:item.id,revision:item.revision})}
fn choice(item,value){let request=token(item);request.choice=value;request}
fn emit_answer(spec,ctx,event){review::send("answer",spec,ctx,event)}
fn open_item(item,ctx,event){
    let open=ctx.get_state("opened");open[item.request]=item.revision;ctx.set_state("opened",open);handled()
}
fn is_open(state,item){(item.id in state&&state[item.id]==item.revision)||!item.collapsed||item.phase=="resolved"}
fn rows(ctx,projection){
    let old=ctx.get_state("previous");let entries=ctx.get_state("opened");let kept=#{};
    let received=false;
    for item in projection.items{
        if item.id in entries&&entries[item.id]==item.revision{kept[item.id]=item.revision;}
        if old!=()&&item.phase=="resolved"&&
                (!(item.id in old)||old[item.id].revision!=item.revision||old[item.id].phase!="resolved"){received=true;}
    }
    if projection.auto&&received{
        for item in projection.items{
            if item.phase=="pending"&&!is_open(kept,item){kept[item.id]=item.revision;break;}
        }
    }
    let next=#{};
    for item in projection.items{next[item.id]=#{revision:item.revision,phase:item.phase};}
    if next!=old{ctx.set_state("previous",next);}
    if kept!=entries{ctx.set_state("opened",kept);}
}
fn cleanup(ctx,projection){}
fn render_row(ctx,payload){
    let item=payload.item;let resolved=item.phase=="resolved";
    if item.empty{
        return row([review::heading_note(ctx,ctx.t_or("decision.inbox_empty","Nothing waiting on you"),"empty")])
            .with_key(payload.key).with_style(style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2)));
    }
    let base=style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2)).gap(theme_length("space.spacing_unit").times(1.25));
    let parts=["row"];if payload.index==0{parts.push("first");}
    if !item.open{
        let narrow=row([text(item.title).with_style(ctx.component_style("title",style().min_width(px(0)).flex_grow()
            .typography("body").text_ellipsis().whitespace_nowrap())),
            review::glyph(ctx,"down",13)])
            .with_key(payload.key).accessibility_role("button").accessibility_label(item.title).accessibility_expanded(false)
            .on_click(Fn("open_item").curry(token(item)))
            .with_style(ctx.component_style(parts+["closed"],base.items_center().radius(theme_length("space.unit").times(2)).text_color(theme_color("text_muted"))
                .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))));
        return narrow;
    }
    let name=text(item.title).with_key("title").accessibility_role("heading").accessibility_level(4)
        .with_style(ctx.component_style("title",style().min_width(px(0)).typography("body").font_weight(500)));
    let content=[name];
    let context=review::opt(item,"detail");
    if context!=(){content.push(review::heading_note(ctx,context,"detail").with_key("detail"));}
    if resolved{
        let label=review::value_or(item,"receipt",ctx.t_or("decision.recorded","Answer recorded"));
        content.push(row([review::glyph(ctx,"check",14),text(label)]).with_key("receipt")
            .accessibility_role("status").accessibility_label(label)
            .with_style(ctx.component_style("receipt",style().min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(1.25))
                .typography("caption").text_color(theme_color("text.success"))))
            .motion(motion_transition("opacity",0.25,1.0,#{duration_ms:210,intent:"feedback"})));
    }else{
        let enabled=item.interactive&&item.phase=="pending";
        content.push(row([
            button::Button(#{key:"primary",text:item.primary,size:"sm",disabled:!enabled,loading:item.phase=="busy",
                loading_text:ctx.t_or("review.processing","Processing…"),on_click:Fn("emit_answer").curry(choice(item,"primary"))})
                .with_style(ctx.component_style("primary",style().min_width(px(0)))),
            button::Button(#{key:"secondary",text:item.secondary,size:"sm",variant:"secondary",disabled:!enabled,
                on_click:Fn("emit_answer").curry(choice(item,"secondary"))}).with_style(ctx.component_style("secondary",style().min_width(px(0)))),
        ]).with_key("actions").with_style(ctx.component_style("actions",style().min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap())));
    }
    let notice=review::opt(item,"notice");
    if notice!=(){content.push(review::heading_note(ctx,notice,"notice").accessibility_role("status").with_key("notice"));}
    column(content).with_key(payload.key).accessibility_role("group").accessibility_label(item.title).accessibility_expanded(true)
        .with_style(ctx.component_style(parts,base));
}
fn render_DecisionInbox(ctx,props){
    let records=[];let compact=[];let seen=#{};let pending=0;let bytes=0;
    let opened=ctx.get_state("opened");let input=review::opt(props,"on_answer")!=();
    for item in props.decisions{
        let id=token(item);
        if id.request in seen{throw "Decision inbox request IDs must be unique";}
        seen[id.request]=true;
        item.phase=review::value_or(item,"phase","pending");item.collapsed=review::value_or(item,"collapsed",false);
        compact.push(#{id:item.id,revision:item.revision,phase:item.phase,collapsed:item.collapsed});
        item.open=is_open(opened,item);item.key="decision:"+item.id;item.interactive=input;item.empty=false;
        bytes+=utf8_byte_count(item.title)+utf8_byte_count(review::value_or(item,"detail",""));
        if bytes>262144{throw "Decision inbox exceeds 256 KiB";}
        if item.phase!="resolved"{pending+=1;}
        records.push(item);
    }
    effect("rows",#{items:compact,auto:props.auto_open_next},Fn("rows"),Fn("cleanup"));
    if records.len==0{records.push(#{key:"empty",empty:true,phase:"pending"});}
    let label=if pending==0{ctx.t_or("decision.caught_up","All caught up")}
        else if pending==1{ctx.t_or("decision.needs_one","1 needs your input")}
        else{ctx.t_or("decision.need_many","{count} need your input",#{count:ctx.format_number(pending)})};
    let stage=ctx.t_or("decision.inbox","Decision inbox");
    let viewport=virtual_collection(#{key:"decisions",label:stage,data:records,estimated_height:76,
        max_height:review::value_or(props,"max_height",px(440))},Fn("render_row"))
        .accessibility_role("list").accessibility_label(stage).tab_stop(true)
        .with_style(ctx.component_style("card",style().min_width(px(0)).radius(theme_length("space.unit").times(3))
            .background(theme_color("surface_raised")).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))));
    let header=row([review::glyph(ctx,if pending==0{"check"}else{"warning"},16),text(label).with_style(ctx.component_style("count",style()
        .min_width(px(0)).typography("body").font_weight(500)))])
        .with_key("head").accessibility_role("status").accessibility_label(label)
        .with_style(ctx.component_style("head",style().min_width(px(0)).items_center().gap(theme_length("space.spacing_unit").times(1.5)).padding(theme_length("space.spacing_unit").times(1.75))
            .text_color(theme_color(if pending==0{"text.success"}else{"text.warning"}))));
    column([header,viewport]).with_key(props.key).accessibility_role("region").accessibility_label(stage)
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(500))
            .padding(theme_length("space.spacing_unit")).radius(theme_length("space.unit").times(4.25)).background(theme_color("surface_hover"))))
}
