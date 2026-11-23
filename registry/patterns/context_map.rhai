/* gpui-rhai
{"id":"patterns/context_map","export":"ContextMap","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/select","components/spinner","patterns/agent_tree"],"capabilities":{},"assets":[],"tokens":["accent","border","context_map.free_edge","context_map.model","context_map.muted","context_map.shine","context_map.free_hatch","context_map.system_bg","context_map.system_edge","context_map.system_ink","context_map.tools_bg","context_map.tools_edge","context_map.tools_ink","context_map.memory_bg","context_map.memory_edge","context_map.memory_ink","context_map.file_bg","context_map.file_edge","context_map.file_ink","context_map.chat_bg","context_map.chat_edge","context_map.chat_ink","context_map.result_bg","context_map.result_edge","context_map.result_ink","context_map.summary_bg","context_map.summary_edge","context_map.summary_ink","focus_ring","radius.lg","radius.sm","space.spacing_unit","surface_hover","surface_raised","text.danger","text_muted","text_primary","typography.body","typography.caption","space.unit"],"environment":[]}
*/
// Inspect the area occupied by ordered context blocks, including a prior compaction.
// Example: ContextMap(#{key:"map",context:"session",revision:0,limit:100000,blocks:[#{id:"chat",label:"Conversation",kind:"chat",tokens:26000}]})
import "components/button" as buttons;
import "components/select" as select;
import "components/spinner" as spinner;
import "patterns/agent_tree" as ids;

define_component(#{
    metadata:#{id:"patterns/context_map","export":"ContextMap",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/select","components/spinner","patterns/agent_tree"],capabilities:#{},assets:[],
        tokens:["accent","border","context_map.free_edge","context_map.model","context_map.muted","context_map.shine","context_map.free_hatch","context_map.system_bg","context_map.system_edge","context_map.system_ink","context_map.tools_bg","context_map.tools_edge","context_map.tools_ink","context_map.memory_bg","context_map.memory_edge","context_map.memory_ink","context_map.file_bg","context_map.file_edge","context_map.file_ink","context_map.chat_bg","context_map.chat_edge","context_map.chat_ink","context_map.result_bg","context_map.result_edge","context_map.result_ink","context_map.summary_bg","context_map.summary_edge","context_map.summary_ink","focus_ring","radius.lg","radius.sm","space.spacing_unit","surface_hover","surface_raised","text.danger","text_muted","text_primary","typography.body","typography.caption","space.unit"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable view instance."},
        context:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of the accepted context."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision for Host requests."},
        limit:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Token allowance when no agent is selected; zero is unknown."},
        blocks:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable nonempty business ID."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Human-readable name."},
            kind:#{schema:#{type:"string",allowed:["system","tools","memory","file","chat","result","summary"]},required:true,sensitive:false,doc:"Semantic palette category."},
            tokens:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Accepted nonnegative count."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Inspection caption."},
            items:#{schema:#{type:"optional",value:#{type:"array",max_items:64,items:#{type:"string"}}},required:false,sensitive:true,doc:"App-owned details, summarized after twelve entries."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative tile mark."}
        }}},required:true,sensitive:true,doc:"Ordered accepted blocks. Zero counts do not occupy space."},
        before:#{schema:#{type:"optional",value:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable nonempty business ID."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Human-readable name."},
            kind:#{schema:#{type:"string",allowed:["system","tools","memory","file","chat","result","summary"]},required:true,sensitive:false,doc:"Semantic palette category."},
            tokens:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Accepted nonnegative count."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Inspection caption."},
            items:#{schema:#{type:"optional",value:#{type:"array",max_items:64,items:#{type:"string"}}},required:false,sensitive:true,doc:"App-owned details."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative tile mark."}
        }}}},required:false,sensitive:true,doc:"Optional prior accepted snapshot, supplied with compaction."},
        compaction:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable compaction receipt identity."},
            summarized:#{schema:#{type:"optional",value:#{type:"array",max_items:64,items:#{type:"string"}}},required:false,sensitive:false,doc:"Before IDs folded into summary."},
            dropped:#{schema:#{type:"optional",value:#{type:"array",max_items:64,items:#{type:"string"}}},required:false,sensitive:false,doc:"Before IDs omitted."},
            summary:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted after block containing the summary."}
        }}},required:false,sensitive:false,doc:"Optional explanation of the previous compaction."},
        compare:#{schema:#{type:"optional",value:#{type:"string",allowed:["before","after"]}},required:false,sensitive:false,doc:"Controlled side. If omitted, comparison is local to the receipt."},
        show_comparison:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Reserve a status row even when no receipt exists.","default":#{type:"bool",value:false}},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Card title."},
        model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Model caption used without an active agent."},
        model_icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative model mark."},
        agents:#{schema:#{type:"optional",value:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Agent identity."},
            name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Display name."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Model caption."},
            limit:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Agent allowance; zero is unknown."}
        }}}},required:false,sensitive:true,doc:"Host-owned agents."},
        agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted agent whose snapshot is supplied."},
        status:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Short activity caption, such as Compacting."},
        on_agent_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Request a different agent; the Host owns the accepted data."},
        on_compare:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a comparison request."},
        on_select:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives an inspected block ID, or an empty string for unused capacity."}
    },state:#{fields:#{
        compared:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        chooser:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        hover:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        focus:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        pinned:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        dismissed:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        visual_history:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}
    }},events:#{
        agent_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            context:#{schema:#{type:"string"},required:true,sensitive:false},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            agent:#{schema:#{type:"string"},required:true,sensitive:false}
        }},doc:"Apply the agent, blocks and allowance together to accept."},
        compare:#{payload:#{type:"object",allow_unknown:false,fields:#{
            context:#{schema:#{type:"string"},required:true,sensitive:false},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            compaction:#{schema:#{type:"string"},required:true,sensitive:false},
            side:#{schema:#{type:"string",allowed:["before","after"]},required:true,sensitive:false}
        }},doc:"Requested side of an accepted compaction receipt."},
        select:#{payload:#{type:"object",allow_unknown:false,fields:#{
            context:#{schema:#{type:"string"},required:true,sensitive:false},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            side:#{schema:#{type:"string",allowed:["before","after"]},required:true,sensitive:false},
            block:#{schema:#{type:"string"},required:true,sensitive:false},
            pinned:#{schema:#{type:"bool"},required:true,sensitive:false}
        }},doc:"Inspection notice. Does not mutate business state."}
    },slots:#{},parts:["root","header","mark","title","model","status","summary","used","number","percent",
        "warn","full","compare","compare_current","field","tile","tile_pinned","tile_attention","free",
        "tile_label","tile_name","tile_count","tile_measures",
        "tooltip","tip_title","tip_meta","tip_detail","tip_fate","tip_items","tip_item","swatch","footer","fate","fate_empty","empty"],
        effects:["remember_visual_lanes"],timers:["discard_visual_exits"]
    },render:Fn("render_map")
});

fn ContextMap(p){render_component("patterns/context_map",p)}
fn get(p,k,d){if p!=()&&k in p&&p[k]!=(){p[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn tx(ctx,k,s){ctx.t_or("context_map."+k,s)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn token(kind,suffix){
    switch kind+":"+suffix{
        "system:bg"=>theme_color("context_map.system_bg"),
        "system:edge"=>theme_color("context_map.system_edge"),
        "system:ink"=>theme_color("context_map.system_ink"),
        "tools:bg"=>theme_color("context_map.tools_bg"),
        "tools:edge"=>theme_color("context_map.tools_edge"),
        "tools:ink"=>theme_color("context_map.tools_ink"),
        "memory:bg"=>theme_color("context_map.memory_bg"),
        "memory:edge"=>theme_color("context_map.memory_edge"),
        "memory:ink"=>theme_color("context_map.memory_ink"),
        "file:bg"=>theme_color("context_map.file_bg"),
        "file:edge"=>theme_color("context_map.file_edge"),
        "file:ink"=>theme_color("context_map.file_ink"),
        "chat:bg"=>theme_color("context_map.chat_bg"),
        "chat:edge"=>theme_color("context_map.chat_edge"),
        "chat:ink"=>theme_color("context_map.chat_ink"),
        "result:bg"=>theme_color("context_map.result_bg"),
        "result:edge"=>theme_color("context_map.result_edge"),
        "result:ink"=>theme_color("context_map.result_ink"),
        "summary:bg"=>theme_color("context_map.summary_bg"),
        "summary:edge"=>theme_color("context_map.summary_edge"),
        _=>theme_color("context_map.summary_ink")
    }
}
fn lines(){style().min_width(px(0)).whitespace_nowrap().text_ellipsis()}
fn caption(){lines().typography("caption").text_color(theme_color("text_muted"))}
fn ink(ctx,part,s,style){text(s).with_style(ctx.component_style(part,style))}
fn bounded(s,cap){s!=""&&utf8_byte_count(s)<=cap}
fn compact(n,ctx){
    let d=if n>=999500000{#{n:1000000000.0,s:"B"}}else if n>=999500{#{n:1000000.0,s:"M"}}
        else if n>=1000{#{n:1000.0,s:"k"}}else{#{n:1.0,s:""}};
    ctx.format_number(n.to_float()/d.n,#{max_fraction_digits:if d.n==1.0||n>=100000&&n<999500{0}else{1}})+d.s
}
fn kind_name(ctx,kind){
    switch kind{
        "system"=>tx(ctx,"system","System prompt"),"tools"=>tx(ctx,"tools","Tools"),
        "memory"=>tx(ctx,"memory","Memory"),"file"=>tx(ctx,"file","File"),
        "chat"=>tx(ctx,"chat","Conversation"),"result"=>tx(ctx,"result","Tool result"),
        _=>tx(ctx,"summary_kind","Summary")
    }
}
fn blocks(items){
    let by=#{};let weights=[];
    for b in items{
        if !ids::valid(b.id)||!bounded(b.label,512)||b.id in by||
            utf8_byte_count(get(b,"detail",""))>2048{throw "ContextMap needs unique IDs and bounded block labels and details";}
        for name in get(b,"items",[]){if !bounded(name,512){throw "ContextMap item labels must be nonempty and at most 512 bytes";}}
        by[b.id]=b;weights.push(b.tokens);
    }
    #{ordered:items,by:by,weights:weights}
}
fn receipts(p,after,before){
    let c=get(p,"compaction",());
    if (c==())!=(get(p,"before",())==()){throw "ContextMap before and compaction must be supplied together";}
    if c==(){return #{};}
    if !ids::valid(c.id){throw "ContextMap requires a stable compaction ID";}
    let fates=#{};
    for fate in ["summarized","dropped"]{
        for id in get(c,fate,[]){
            if !(id in before.by)||id in fates{throw "ContextMap compaction outcomes must refer to distinct before blocks";}
            fates[id]=fate;
        }
    }
    let summary=get(c,"summary","");
    if summary!=""&&(!(summary in after.by)||after.by[summary].kind!="summary"){
        throw "ContextMap summary must reference an accepted summary block";
    }
    fates
}
fn agent_view(p){
    let roster=#{};let options=[];
    for a in get(p,"agents",[]){
        if !ids::valid(a.id)||a.id in roster||!bounded(a.name,512)||utf8_byte_count(get(a,"model",""))>512{
            throw "ContextMap agents require unique stable IDs and bounded labels";
        }
        roster[a.id]=a;options.push(#{value:a.id,label:a.name});
    }
    let a=get(p,"agent","");
    if a!=""&&!(a in roster){throw "ContextMap agent must refer to the accepted roster";}
    #{options:options,agent:if a==""{()}else{roster[a]},key:a}
}
fn compare_scope(p,c,a){#{context:p.context,compaction:get(c,"id",""),agent:a.key}}
fn ticket(p,m){#{context:p.context,revision:p.revision,side:m.side}}
fn key_scope(p,m){
    utf8_byte_count(p.context).to_string()+":"+p.context+":"+utf8_byte_count(m.agent.key).to_string()+":"+m.agent.key+":"
}
fn view_model(ctx,p){
    if !ids::valid(p.context){throw "ContextMap requires a stable context ID";}
    let a=agent_view(p);
    let after=blocks(p.blocks);
    let before=blocks(get(p,"before",[]));
    let changes=receipts(p,after,before);
    let c=get(p,"compaction",());
    let scope=compare_scope(p,c,a);
    let saved=ctx.get_state("compared");
    let side=if c==(){"after"}else{get(p,"compare",if get(saved,"scope",())==scope{saved.side}else{"after"})};
    let shown=if side=="before"{before}else{after};
    let limit=get(a.agent,"limit",p.limit);
    #{agent:a,scope:scope,side:side,shown:shown,before:before,after:after,changes:changes,receipt:c,limit:limit,
      rtl:ctx.text_direction()=="rtl",
      stamp:#{scope:scope,side:side}}
}
fn compare(scope,revision,side,local,ctx,event){
    if local{ctx.set_state("compared",#{scope:scope,side:side});}
    ctx.emit("compare",#{context:scope.context,revision:revision,compaction:scope.compaction,side:side});
    handled()
}
fn open_chooser(context,ctx,is_open){ctx.set_state("chooser",if is_open{context}else{()});}
fn pick_agent(context,revision,ctx,value){ctx.emit("agent_change",#{context:context,revision:revision,agent:value});}
fn attention(stamp,field,id,ctx,enter){
    let key=#{stamp:stamp,id:id};
    let old=ctx.get_state(field);
    if enter{
        if old!=key{ctx.set_state(field,key);ctx.set_state("dismissed",());}
    }else if old==key{ctx.set_state(field,());}
}
fn target(ctx,field,m){
    let value=ctx.get_state(field);
    if value!=()&&value.stamp==m.stamp&&value.id in m.shown.by&&m.shown.by[value.id].tokens>0{value.id}else{""}
}
fn point_to(ctx,m){
    let pin=target(ctx,"pinned",m);
    if pin!=""{return pin;}
    let hover=target(ctx,"hover",m);
    let active=if hover!=""{hover}else{target(ctx,"focus",m)};
    if ctx.get_state("dismissed")==#{stamp:m.stamp,id:active}{""}else{active}
}
fn inspect(stamp,info,id,ctx,event){
    let key=#{stamp:stamp,id:id};
    let pinned=id!=""&&ctx.get_state("pinned")!=key;
    ctx.set_state("pinned",if pinned{key}else{()});
    ctx.set_state("dismissed",if pinned{()}else{key});
    info.block=id;info.pinned=pinned;ctx.emit("select",info);
    handled()
}
fn dismiss(stamp,id,ctx,event){
    ctx.set_state("pinned",());ctx.set_state("dismissed",#{stamp:stamp,id:id});handled()
}
fn svg_mark(ctx,d,color,w){
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.3' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
       .accessibility_hidden(true).with_style(ctx.component_style("mark",style().width(sp(w)).height(sp(w))
            .flex_shrink(false).text_color(color)))
}
fn pill(ctx,p,m,plan){
    let level=if m.limit>0{plan.used.to_float()/m.limit.to_float()}else{0.0};
    let percent=if m.limit==0{"—"}else if level>9.99{"999%+"}else{ctx.format_number((level*100.0).round().to_int())+"%"};
    let amount=compact(plan.used,ctx);
    let amount_paint=caption().text_color(theme_color("text_primary")).font_weight(600).font_feature("tnum",1)
        .glyph_ink_sweep(theme_color("context_map.shine"),0.36,2.0,0.0);
    let amount=ink(ctx,"used",amount,amount_paint).with_key("used-amount")
        .motion(motion_transition("glyph_sweep",0.0,1.0,
            #{duration_ms:1100,easing:"linear",iterations:"infinite",intent:"decorative"}));
    let denominator=if m.limit>0{"/ "+compact(m.limit,ctx)}else{tx(ctx,"unknown","Limit unavailable")};
    let digits=row([amount,ink(ctx,"number",denominator,caption().font_feature("tnum",1))])
        .with_style(style().gap(sp(1)).items_center());
    let parts=if level>=0.95{["percent","full"]}else if level>=0.8{["percent","warn"]}else{["percent"]};
    let readout=ink(ctx,parts,percent,caption().font_weight(600).font_feature("tnum",1).padding_x(sp(2)))
        .with_key("capacity-readout").paint_transition(300,cubic_bezier(0.25,0.1,0.25,1.0));
    let controls=[];
    if m.receipt!=(){
        let group=[];
        for s in ["before","after"]{
            let name=if s=="before"{tx(ctx,"before","Before")}else{tx(ctx,"after","After")};
            group.push(buttons::Button(#{key:s,text:name,variant:if m.side==s{"secondary"}else{"ghost"},size:"xs",
                on_click:Fn("compare").curry(m.scope,p.revision,s,get(p,"compare",())==())})
                .accessibility_pressed(m.side==s));
        }
        controls.push(row(group).with_key("compare").accessibility_role("group").accessibility_label(tx(ctx,"comparison","Compaction comparison"))
            .with_style(ctx.component_style("compare",style().gap(sp(0.5)).items_center())));
    }
    controls.push(row([digits,readout]).with_key("totals").accessibility_role("status").accessibility_label(tx(ctx,"usage","Context usage"))
        .accessibility_description(if m.limit>0{ctx.format_number(plan.used)+" / "+ctx.format_number(m.limit)+" · "+percent}else{ctx.format_number(plan.used)+" · "+denominator})
        .with_style(ctx.component_style("summary",style().gap(sp(2)).items_center().flex_shrink(false))));
    row(controls).with_key("summary").with_style(style().min_width(px(0)).flex_wrap().gap(sp(2)).items_center())
}
fn header(ctx,p,m,plan){
    let who=m.agent.agent;
    let head=[svg_mark(ctx,"M2 2h12v12H2zM8 2v12M2 8h6M8 10h6",theme_color("accent"),4),
        ink(ctx,"title",get(p,"title",tx(ctx,"title","Context")),lines().typography("body").font_weight(600).text_color(theme_color("text_primary")))];
    if m.agent.options.len>0{
        head.push(select::Select(#{key:"agent",label:tx(ctx,"agent","Context agent"),options:m.agent.options,value:m.agent.key,query:"",
            open:ctx.get_state("chooser")==p.context&&get(p,"on_agent_change",())!=(),size:"sm",disabled:get(p,"on_agent_change",())==(),
            width:px(172),on_change:Fn("pick_agent").curry(p.context,p.revision),on_open_change:Fn("open_chooser").curry(p.context)}));
        let model=get(who,"model","");
        if model!=""{head.push(ink(ctx,"model",model,caption().typography("body")));}
    }else{
        let model=get(p,"model","");
        let icon=get(p,"model_icon",());
        if icon!=(){head.push(icon.accessibility_hidden(true));}
        if model!=""{head.push(ink(ctx,"model",model,caption().typography("body")));}
    }
    let status=get(p,"status","");
    if status!=""{
        head.push(row([spinner::Spinner(#{key:"work",size:"xs",label:status}).accessibility_hidden(true),
            ink(ctx,"status",status,caption().text_color(token("summary","ink")))])
            .with_key("work").accessibility_role("status").accessibility_label(status)
            .with_style(ctx.component_style("status",style().gap(sp(1)).items_center().padding_x(sp(1.5)).padding_y(sp(0.5))
                .background(token("summary","bg")).radius(theme_radius("sm"))))
            .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:160,easing:ease(),intent:"feedback"})));
    }
    row([row(head).with_key("title-row").with_style(style().gap(sp(1.5)).min_width(px(0)).flex_wrap().items_center()),pill(ctx,p,m,plan)])
        .with_key("header").with_style(ctx.component_style("header",style().min_width(px(0)).padding_x(sp(2))
            .padding_y(sp(1.75)).gap_x(sp(3)).gap_y(sp(1)).items_center().justify_between().flex_wrap()))
}
fn fate(m,b){
    if m.receipt==(){return "";}
    if m.side=="before"{get(m.changes,b.id,"kept")}
    else if get(m.receipt,"summary","")==b.id{"summary"}else{""}
}
fn fate_name(ctx,f){
    switch f{
        "summarized"=>tx(ctx,"summarized","Folded into the summary"),
        "dropped"=>tx(ctx,"dropped","Dropped when it compacted"),
        "kept"=>tx(ctx,"kept","Kept as it was"),
        _=>tx(ctx,"merged","Summary of earlier blocks")
    }
}
fn free_hatch(){
    let clear=rgba(0x00000000);
    let stripe=theme_color("context_map.free_hatch");
    let edge=6.0/7.0;
    // Keep a true seven-pixel repeat in the moving tile's own paper;
    // neither the viewport width nor the number of stripes drives it.
    layered_gradient([#{angle:135,repeat_period:7,origin:"border",clip:"border",stops:[
        #{offset:0,color:clear},#{offset:edge,color:clear},
        #{offset:edge,color:stripe},#{offset:1,color:stripe}
    ]}])
}
fn tile_label(node,style,g,min_w,min_h){
    let style=if g.width>=min_w&&g.height>=min_h{style}else{style.invisible()};
    node.with_style(style).sample_partition_visibility(g.group,g.lane,g.dims.width,g.dims.height,3.0,min_w,min_h)
}
fn tile(ctx,p,m,plan,s,content,dims,active,pin,b,live,group,lane){
    let is_free=b==();
    let id=get(b,"id","");
    let name=get(b,"label",tx(ctx,"free","Free"));
    let count=get(b,"tokens",plan.free);
    let kind=get(b,"kind","");
    let tone=if is_free{theme_color("context_map.muted")}else{token(kind,"ink")};
    let f=if is_free{""}else{fate(m,b)};
    let w=s.width*dims.width;
    let h=s.height*dims.height;
    let gap=min(3.0,min(w,h)*0.33);
    let text_w=content.width*dims.width;
    let text_h=content.height*dims.height;
    let label_gap=min(3.0,min(text_w,text_h)*0.33);
    let g=#{group:group,lane:lane,dims:dims,width:text_w-label_gap,height:text_h-label_gap};
    let title=[];
    let icon=get(b,"icon",());
    if icon!=(){title.push(icon.accessibility_hidden(true));}
    // Shape the whole line and feather its clipping edge, rather than
    // replacing its final glyphs with an ellipsis.
    let tile_line=style().min_width(px(0)).typography("caption").whitespace_nowrap().text_color(tone);
    title.push(ink(ctx,"tile_name",name,tile_line.font_weight(600)));
    let label=[row(title).with_key("title").with_style(style().gap(sp(1)).min_width(px(0)).items_center())];
    let count_style=tile_line.font_feature("tnum",1);
    let measures=[ink(ctx,"tile_count",compact(count,ctx),count_style.flex_shrink(false))];
    if !is_free{
        let suffix=" · "+if f==""{kind_name(ctx,kind)}else{tx(ctx,"fate_"+f,f)};
        measures.push(tile_label(row([ink(ctx,"tile_count",suffix,count_style)]).with_key("detail"),
            style().min_width(px(0)),g,120.0,44.0));
    }
    let measures_style=ctx.component_style("tile_measures",style().min_width(px(0)).items_center().opacity(0.7));
    label.push(tile_label(row(measures).with_key("counts"),measures_style,g,54.0,44.0));
    // Keep the last 4px behind the ramp transparent at every reading scale.
    let edge_tail=theme_length("space.unit").without_reading_scale();
    let label_clip=style().min_width(px(0)).margin_start(sp(2.5)).margin_end(edge_tail).margin_top(sp(2)).overflow_hidden()
        .edge_fade("x",if m.rtl{18.0}else{0.0},if m.rtl{0.0}else{18.0});
    let nodes=[tile_label(column(label).accessibility_hidden(true),ctx.component_style("tile_label",label_clip),g,54.0,28.0)];
    let t=style().absolute().inset_start(relative(s.left+gap/(2.0*dims.width))).top(relative(s.top+gap/(2.0*dims.height)))
        .width(relative((w-gap)/dims.width)).height(relative((h-gap)/dims.height)).min_width(px(0)).min_height(px(0))
        .overflow_hidden().radius(px(min(9.0,min(text_w,text_h)*0.25)));
    let parts=["tile"];
    if is_free{
        parts.push("free");
        t=t.layered_gradient(free_hatch()).border(px(1))
            .border_color(theme_color("context_map.free_edge")).border_dashed();
    }else{
        t=t.background(token(kind,"bg")).border(px(1)).border_color(token(kind,"edge"));
        if live{t=t.cursor_pointer();}
        if f=="summarized"{t=t.border_dashed();}
        if f=="dropped"{t=t.opacity(0.55);}
        if active!=""&&active!=id{t=t.opacity(if f=="dropped"{0.38}else{0.58});}
        if active==id{parts.push("tile_attention");}
        if pin==id{parts.push("tile_pinned");t=t.border_solid().border_color(tone).opacity(1.0);}
    }
    let node=column(nodes).with_key(if is_free{"free:"+key_scope(p,m)}else{"block:"+key_scope(p,m)+id}).with_style(ctx.component_style(parts,t));
    if !live{return node.accessibility_hidden(true);}
    let click=Fn("inspect").curry(m.stamp,ticket(p,m),id);
    let description=ctx.format_number(count)+" "+tx(ctx,"tokens","tokens")+
        (if is_free{""}else{" · "+kind_name(ctx,kind)})+(if f!=""{" · "+fate_name(ctx,f)}else{""});
    let node=node.accessibility_role("button").accessibility_label(name)
        .accessibility_description(description).tab_stop(true)
        .on_click(click).on_key_value("plain+enter",click,()).on_key_value("plain+space",click,())
        .on_key_value("plain+escape",Fn("dismiss").curry(m.stamp,id),());
    if is_free{node}else{
        node.accessibility_pressed(pin==id)
            .on_hover_change(Fn("attention").curry(m.stamp,"hover",id))
            .on_focus_within_change(Fn("attention").curry(m.stamp,"focus",id))
    }
}
fn tooltip(ctx,m,b,dims,r){
    let f=fate(m,b);
    let share=if m.limit>0{" · "+ctx.format_number(b.tokens.to_float()/m.limit.to_float()*100.0,#{max_fraction_digits:1})+"%"}else{""};
    let desc=kind_name(ctx,b.kind)+" · "+ctx.format_number(b.tokens)+" "+tx(ctx,"tokens","tokens")+share;
    let icon=get(b,"icon",());
    let icon=if icon!=(){icon.accessibility_hidden(true)}else{box([]).accessibility_hidden(true)
        .with_style(ctx.component_style("swatch",style().width(sp(2.5)).height(sp(2.5)).flex_shrink(false)
           .radius(theme_radius("sm")).background(token(b.kind,"bg")).border(px(1)).border_color(token(b.kind,"edge"))))};
    let out=[row([icon,ink(ctx,"tip_title",b.label,caption().font_weight(600).text_color(theme_color("text_primary")))])
        .with_style(style().min_width(px(0)).gap(sp(1.5)).items_center()),
        ink(ctx,"tip_meta",desc,caption().font_feature("tnum",1))];
    let detail=get(b,"detail","");
    if detail!=""{out.push(ink(ctx,"tip_detail",detail,style().typography("caption").line_clamp(3).text_color(theme_color("text_primary"))));}
    if f!=""{out.push(ink(ctx,"tip_fate",fate_name(ctx,f),caption().text_color(token("summary","ink"))));}
    let items=get(b,"items",[]);
    if items.len>0{
        let rows=[];
        for i in 0..min(items.len,if items.len>12{11}else{12}){
            rows.push(ink(ctx,"tip_item",items[i],caption()).with_key("item:"+i.to_string()));
        }
        if items.len>12{rows.push(ink(ctx,"tip_item","+"+(items.len-11).to_string()+" "+tx(ctx,"more","more"),caption()));}
        out.push(box(rows).with_style(ctx.component_style("tip_items",style().grid_cols(if dims.width>=420.0{2}else{1})
            .gap_x(sp(2)).gap_y(sp(0.5)).min_width(px(0)).padding_top(sp(1)))));
    }
    let tip=r.tooltip;
    column(out).with_key("tip:"+b.id).accessibility_role("tooltip").accessibility_label(b.label)
        .accessibility_description(desc)
        .with_style(ctx.component_style("tooltip",style().absolute().inset_start(px(tip.left)).top(px(tip.top)).width(px(tip.width))
            .min_width(px(0)).max_height(px(tip.max_height)).overflow_hidden()
            .padding_x(sp(3)).padding_y(sp(2)).gap(sp(0.5)).radius(theme_radius("lg"))
            .background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border"))))
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:120,easing:ease(),intent:"feedback"}))
}
fn indexed_layout(plan,snapshot){
    let tiles=#{};
    let free=();
    for s in plan.tiles{
        if s.index==(){free=s;}else{tiles[snapshot.ordered[s.index].id]=s;}
    }
    #{tiles:tiles,free:free}
}
fn source(key,slot,from,to){
    // The node owns motion continuity and uses the exact business key.
    // Refs only locate current lanes; their syntax is deliberately ASCII.
    let ref=element_ref("sample:"+slot);
    let node=box([]).with_key(key).with_ref(ref).number_motion_source().accessibility_hidden(true)
        .with_style(style().absolute().width(px(0)).height(px(0)))
        .motion(motion_transition("number_value",from.to_float(),to.to_float(),#{duration_ms:420,easing:ease(),intent:"feedback"}));
    #{ref:ref,node:node}
}
fn remember_visual_lanes(ctx,value){
    if ctx.get_state("visual_history")!=value{ctx.set_state("visual_history",value);}
}
fn noop(ctx,value){}
fn discard_visual_exits(ctx,ticket){
    let history=ctx.get_state("visual_history");
    if history==()||history.scope!=ticket.scope||history.generation!=ticket.generation{return;}
    let active=[];
    for b in history.order{if b.id in history.current{active.push(b);}}
    history.order=active;ctx.set_state("visual_history",history);
}
fn lane_order(ctx,p,m){
    let current=#{};
    let order=[];
    // One receipt preserves the union while either side is active.
    for snapshot in [m.before,m.after]{
        for b in snapshot.ordered{
            if b.id in current{continue;}
            current[b.id]=get(m.shown.by,b.id,b);order.push(b.id);
        }
    }
    // A regular update has no receipt. Carry only old paint data while its
    // keyed track converges to zero, separate from accepted data and events.
    let scope=#{context:p.context,agent:m.agent.key};
    let previous=ctx.get_state("visual_history");
    let generation=get(previous,"generation",0);
    let same=previous!=()&&previous.scope==scope;
    let tails=#{"":[]};
    let seen=#{};
    let exiting=0;
    let restarted=false;
    if same&&ctx.motion_preference()=="normal"{
        let anchor="";
        for b in previous.order{
            if b.id in current{anchor=b.id;}
            else if !(b.id in seen)&&order.len+exiting<128{
                let pending=get(tails,anchor,[]);
                pending.push(b);tails[anchor]=pending;
                seen[b.id]=true;exiting+=1;
                if b.id in previous.current{restarted=true;}
            }
        }
    }
    let lanes=tails[""];
    for id in order{
        lanes.push(current[id]);
        for b in get(tails,id,[]){lanes.push(b);}
    }
    let paint=[];
    let active=#{};
    for b in lanes{
        paint.push(#{id:b.id,label:b.label,kind:b.kind,tokens:b.tokens});
        if b.id in current{active[b.id]=true;}
    }
    let generation=generation+if !same||restarted{1}else{0};
    effect("remember_visual_lanes",#{scope:scope,generation:generation,order:paint,current:active},
        Fn("remember_visual_lanes"),Fn("noop"));
    if exiting>0{
        timeout("discard_visual_exits",480,false,Fn("discard_visual_exits"),#{scope:scope,generation:generation});
    }
    lanes
}
fn field(ctx,p,m,ref,dims,plan){
    let active=point_to(ctx,m);
    let pin=target(ctx,"pinned",m);
    let out=[];
    let lanes=lane_order(ctx,p,m);let refs=[];
    for i in 0..lanes.len{
        let b=lanes[i];
        let accepted=get(m.shown.by,b.id,());
        let s=source("sample:"+key_scope(p,m)+b.id,i.to_string(),0,get(accepted,"tokens",0));
        refs.push(s.ref);out.push(s.node);
    }
    if refs.len==0{
        let empty=source("sample-empty","empty",0,0);
        refs.push(empty.ref);out.push(empty.node);
    }
    let capacity=source("sample-capacity:"+key_scope(p,m),"capacity",m.limit,m.limit);
    out.push(capacity.node);
    let group=motion_numbers(refs).with_capacity(capacity.ref);
    let accepted=indexed_layout(plan,m.shown);
    let other=if m.side=="before"{m.after}else{m.before};
    let absent=if m.receipt==(){#{tiles:#{},free:()}}
        else{indexed_layout(context_partition(other.weights,m.limit,dims.width,dims.height),other)};
    let zero=#{left:0.0,top:0.0,width:0.0,height:0.0};
    let read=();
    for i in 0..lanes.len{
        let b=lanes[i];
        let s=get(accepted.tiles,b.id,());
        let live=s!=();
        let content=get(accepted.tiles,b.id,get(absent.tiles,b.id,zero));
        let node=tile(ctx,p,m,plan,if live{s}else{zero},content,dims,active,pin,b,live,group,i);
        out.push(node.sample_partition(group,i,dims.width,dims.height,3.0,false));
        if live&&b.id==active{read=tooltip(ctx,m,b,dims,s).sample_partition(group,i,dims.width,dims.height,0.0,true);}
    }
    let content=get(accepted,"free",get(absent,"free",#{left:0.0,top:0.0,width:1.0,height:1.0}));
    let node=tile(ctx,p,m,plan,get(accepted,"free",zero),content,dims,active,pin,(),accepted.free!=(),group,-1);
    out.push(node.sample_partition(group,-1,dims.width,dims.height,3.0,false));
    if plan.tiles.len==0{
        out.push(row([ink(ctx,"empty",tx(ctx,"empty","No context blocks"),caption())]).with_key("empty")
            .with_style(style().absolute().width(relative(1.0)).height(relative(1.0)).justify_center().items_center()));
    }
    if read!=(){out.push(read);}
    box(out).with_key("field").with_ref(ref).accessibility_role("group").accessibility_label(tx(ctx,"field","Context allocation map"))
        .with_style(ctx.component_style("field",style().relative().min_width(px(0)).width(relative(1.0)).flex_shrink(false)
            .height(px(if dims.width<480.0{240}else{280})).radius(theme_radius("lg")).background(theme_color("surface_raised"))))
}
fn footer(ctx,p,m){
    if m.receipt==(){
        return row([ink(ctx,["fate","fate_empty"],tx(ctx,"no_compaction","No compaction yet"),caption())]).with_key("fates")
            .accessibility_role("status").accessibility_label(tx(ctx,"no_compaction","No compaction yet"))
            .with_style(ctx.component_style("footer",style().min_width(px(0)).padding_x(sp(2)).padding_y(sp(1.5))));
    }
    let totals=#{kept:0,summarized:0,dropped:0};
    for b in m.before.ordered{let f=get(m.changes,b.id,"kept");totals[f]+=b.tokens;}
    let lines=[];
    for f in ["kept","summarized","dropped"]{
        lines.push(row([svg_mark(ctx,if f=="kept"{"m3 8 3 3 7-7"}else if f=="dropped"{"m4 4 8 8m0-8-8 8"}else{"m5 3 3 4 3-4M5 13l3-4 3 4M3 8h10"},
            theme_color("context_map.muted"),2.5),ink(ctx,"fate",compact(totals[f],ctx)+" "+tx(ctx,"fate_"+f,f),caption().font_feature("tnum",1))])
            .with_key(f).with_style(style().gap(sp(1)).items_center()));
    }
    row(lines).with_key("fates").accessibility_role("group").accessibility_label(tx(ctx,"fates","Compaction outcomes"))
        .with_style(ctx.component_style("footer",style().min_width(px(0)).padding_x(sp(2)).padding_y(sp(1.5)).gap(sp(3)).flex_wrap()))
}
fn render_map(ctx,p){
    let m=view_model(ctx,p);
    let ref=element_ref("field");
    let measured=ctx.element_size(ref);
    let dims=if measured==(){#{width:550.0,height:280.0}}else{#{width:max(1.0,measured.width),height:max(1.0,measured.height)}};
    let plan=context_partition(m.shown.weights,m.limit,dims.width,dims.height);
    let contents=[header(ctx,p,m,plan),field(ctx,p,m,ref,dims,plan)];
    if p.show_comparison||m.receipt!=(){contents.push(footer(ctx,p,m));}
    column(contents).with_key("context-map").accessibility_role("group").accessibility_label(get(p,"title",tx(ctx,"title","Context")))
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).padding(sp(1))
            .radius(theme_radius("lg")).background(theme_color("surface_hover"))))
}
