/* gpui-rhai
{"id":"patterns/context_window","export":"ContextWindow","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/spinner","patterns/agent_tree","patterns/compact_breakdown"],"capabilities":{},"assets":[],"tokens":["border","context_window.c1","context_window.c2","context_window.c3","context_window.c4","context_window.c5","context_window.c6","context_window.free","context_window.shine","context_window.warn","context_window.warn_bg","context_window.full_bg","context_window.saved","context_window.muted","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text.danger","text_muted","text_primary","typography.body","typography.caption","typography.display"],"environment":[]}
*/
// A token allocation field with Host-controlled removal and compaction.
// Example: ContextWindow(#{key:"context",context:"chat",revision:0,limit:100000,parts:[#{id:"chat",label:"Conversation",tokens:23000}]})
import "components/button" as btn;
import "components/spinner" as spin;
import "patterns/agent_tree" as ids;
import "patterns/compact_breakdown" as breakdown;

define_component(#{
    metadata:#{id:"patterns/context_window","export":"ContextWindow",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/spinner","patterns/agent_tree","patterns/compact_breakdown"],capabilities:#{},assets:[],tokens:["border","context_window.c1","context_window.c2","context_window.c3","context_window.c4","context_window.c5","context_window.c6","context_window.free","context_window.shine","context_window.warn","context_window.warn_bg","context_window.full_bg","context_window.saved","context_window.muted","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text.danger","text_muted","text_primary","typography.body","typography.caption","typography.display"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable visual instance."},
        context:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable context identity."},
        revision:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Accepted capacity revision."},
        limit:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Accepted token allowance; zero means unavailable."},
        parts:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable part ID."},label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Part caption."},tokens:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Aggregate without items."},items:#{schema:#{type:"optional",value:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable item ID within its part."},label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Attached item label."},tokens:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Accepted item token count."}}}}},required:false,sensitive:false,doc:"Optional item breakdown. If supplied, its sum supersedes tokens."},ink:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional theme color token."},locked:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Prevent removal and compaction.","default":#{type:"bool",value:false}},compact:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Expose compaction if the caller handles it.","default":#{type:"bool",value:false}}}}},required:true,sensitive:true,doc:"Parts in the fill order, with distinct stable IDs."},
        model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional model tag."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional card heading."},
        warn_at:#{schema:#{type:"number",min:0.0,max:0.95},required:false,sensitive:false,doc:"Warning fraction of the allowance.","default":#{type:"float",value:0.8}},
        cells:#{schema:#{type:"integer",min:1,max:384},required:false,sensitive:false,doc:"Number of cells in the decorative field.","default":#{type:"integer",value:216}},
        columns:#{schema:#{type:"integer",min:8,max:48},required:false,sensitive:false,doc:"Wide layout columns. Narrow layouts use fewer.","default":#{type:"integer",value:36}},
        pending:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{kind:#{schema:#{type:"string",allowed:["remove","compact"]},required:true,sensitive:false,doc:"Operation awaiting acceptance."},part:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Target part."},item:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Target item for removal, omitted for compaction."}}}},required:false,sensitive:false,doc:"Authoritative acknowledgement state. Does not change the counts."},
        saved_tokens:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Optional accepted result of the previous compaction."},
        compacted:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{part:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Recently compacted part ID."},tokens:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Its accepted post-compaction size."}}}},required:false,sensitive:false,doc:"Show compacted until the accepted part grows again."},
        action_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional footer action label."},
        on_request:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Versioned remove or compact proposal, not a mutation."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Versioned footer action intent."},
      },state:#{fields:#{open_part:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
          attention:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
      events:#{
         request:#{payload:#{type:"object",allow_unknown:false,fields:#{context:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Context ID."},revision:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Accepted revision used for this request."},kind:#{schema:#{type:"string",allowed:["remove","compact"]},required:true,sensitive:false,doc:"Requested kind."},part:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Part ID."},item:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Item ID, or an empty string for compaction."}}},doc:"This component never mutates the accepted input."},
         action:#{payload:#{type:"object",allow_unknown:false,fields:#{context:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Context ID."},revision:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Accepted revision."}}},doc:"Footer action request."}
      },
      slots:#{},parts:["root","card","head","title","model","figure","big","of","pill","warn","full","field","tile","tile_active",
        "spark","parts","part","active","main","dot","name","amount","arrow","compact","done","items","item","item_name",
        "item_value","remove","empty","footer","note","saved","action","action_icon"]
    },render:Fn("render_context")
});
fn ContextWindow(p){render_component("patterns/context_window",p)}
fn get(p,k,d){if k in p&&p[k]!=(){p[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn unit(n){theme_length("space.unit").times(n)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn tx(ctx,k,v){ctx.t_or("context_window."+k,v)}
fn hint(){style().typography("caption").text_color(theme_color("text_muted")).whitespace_nowrap().text_ellipsis()}
fn ink(ctx,part,value,base){text(value).with_style(ctx.component_style(part,base.min_width(px(0))))}
fn icon(path){
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><path d='"+path+"'/></svg>")
        .accessibility_hidden(true).with_style(style().width(unit(3.5)).height(unit(3.5)).text_color(theme_color("text_muted")).flex_shrink(false))
}
fn label_valid(value){value!=""&&utf8_byte_count(value)<=512}
fn project(p){
    if !ids::valid(p.context){throw "ContextWindow requires a stable context ID";}
    let order=[];let items=#{};let amounts=[];let known_ids=#{};
    for source in p.parts{
        if !ids::valid(source.id)||!label_valid(source.label)||source.id in known_ids{throw "ContextWindow parts need unique IDs and bounded labels";}
        let p=source;let attached=get(p,"items",());let count=get(p,"tokens",0);
        p.locked=get(p,"locked",false);p.compact=get(p,"compact",false);
        if attached!=(){
            let local=#{};count=0;
            for item in attached{
                if !ids::valid(item.id)||!label_valid(item.label)||item.id in local{throw "ContextWindow items need unique IDs in a part";}
                local[item.id]=true;count+=item.tokens;
            }
            if count>9007199254740991{throw "ContextWindow part token count exceeds the supported range";}
        }
        let index=order.len;
        p.tokens=count;p.tone=get(p,"ink","context_window.c"+(1+index%6).to_string());
        known_ids[p.id]=index;items[p.id]=p;order.push(p.id);amounts.push(count);
    }
    let budget=context_capacity(amounts,p.limit,p.cells);
    #{order:order,parts:items,budget:budget}
}
fn current(ctx,p,m){
    let a=breakdown::attention(ctx,p.context);
    for id in [a.bar,a.row,a.focus]{if id in m.parts{return id;}}
    ""
}
fn alter(context,field,key,ctx,inside){breakdown::attend(context,field,key,ctx,inside);}
fn open_ref(index){"open:"+index.to_string()}
fn toggle(p,id,ctx,event){
    let old=ctx.get_state("open_part");let next=#{context:p,id:id};
    ctx.set_state("open_part",if old==next{()}else{next});
    handled()
}
fn focus_part(index,ctx,event){ctx.focus(open_ref(index));handled()}
fn request(ticket,target,ctx,event){
    // Keep a deterministic focus target if the Host removes this item.
    if target!=""{ctx.focus(target);}
    ctx.emit("request",ticket);handled()
}
fn action(ticket,ctx,event){ctx.emit("action",ticket);handled()}
fn node_button(node,cb){
    node.tab_stop(true).accessibility_role("button").on_click(cb)
       .on_key_value("plain+enter",cb,()).on_key_value("plain+space",cb,())
}
fn number_unit(value){
    if value>=999950000{#{suffix:"B",scale:1000000000.0}}
    else if value>=999950{#{suffix:"M",scale:1000000.0}}
    else if value>=1000{#{suffix:"K",scale:1000.0}}
    else{#{suffix:"",scale:1.0}}
}
fn short(ctx,value,always){
    let fmt=number_unit(value);
    let digits=if fmt.suffix==""{0}else{1};
    let shown=if always{digits}else{0};
    ctx.format_number(value.to_float()/fmt.scale,#{min_fraction_digits:shown,max_fraction_digits:digits})+fmt.suffix
}
fn reading_nodes(ctx,slot,part,value,suffix,base){
    let fmt=if suffix==""{number_unit(value)}else{#{suffix:suffix,scale:1.0}};
    let digits=if fmt.suffix in ["","%"]{0}else{1};
    let amount=value.to_float()/fmt.scale;
    // The sampled source and displayed unit change together at unit boundaries.
    let ref=element_ref("amount:"+slot+":"+if fmt.suffix=="%"{"percent"}else{fmt.suffix});
    let paint=box([]).with_key("sample:"+slot+":"+fmt.suffix).with_ref(ref).number_motion_source().accessibility_hidden(true)
        .with_style(style().absolute().width(px(0)).height(px(0)))
        .motion(motion_transition("number_value",amount,amount,#{duration_ms:620,easing:ease(),intent:"feedback"}));
    let number=ctx.number_text(amount,digits,"",fmt.suffix,false).with_key("number:"+slot).accessibility_hidden(true)
        .sample_numbers(motion_numbers([ref]),"amount",0).with_style(ctx.component_style(part,base.font_feature("tnum",1).flex_shrink(false)));
    [paint,number]
}
fn reading(ctx,slot,part,value,suffix,base){
    row(reading_nodes(ctx,slot,part,value,suffix,base)).with_key(slot).accessibility_hidden(true)
        .with_style(style().relative().min_width(px(0)).flex_shrink(false))
}
fn summary(ctx,p,m,hot){
    let b=m.budget;
    let fraction=if p.limit==0{0.0}else{b.used.to_float()/p.limit.to_float()};
    let percent=min(999.0,(100.0*fraction).round()).to_int();
    let shown=if hot in m.parts{m.parts[hot].tokens}else{b.used};
    let info=if hot in m.parts{m.parts[hot].label}else if p.limit==0{tx(ctx,"unknown","No token limit available")}
        else{ctx.t_or("context_window.of","of {limit} tokens",#{limit:short(ctx,p.limit,false)})};
    let info_key=if hot==""{"total"}else{"hot:"+hot};
    let pill_style=if p.limit>0&&fraction>=0.95{["pill","full"]}else if p.limit>0&&fraction>=p.warn_at{["pill","warn"]}else{["pill"]};
    let reads=reading_nodes(ctx,"figure","big",shown,"",style().typography("display").font_weight(500));
    let percentage=if p.limit==0{[ink(ctx,"pill","—",hint().flex_shrink(false)).with_key("number:percent").accessibility_hidden(true)]}
        else if fraction>9.99{[ink(ctx,pill_style,"999%+",hint().flex_shrink(false)).with_key("number:percent").accessibility_hidden(true)]}
        else{reading_nodes(ctx,"percent",pill_style,percent,"%",style().typography("caption").font_weight(500).padding_x(sp(1.5))
            .padding_y(sp(0.5)).radius(theme_radius("sm")).background(theme_color("surface_hover")))};
    let line=if p.limit==0{ctx.t_or("context_window.unknown_used","{used} tokens used; limit unavailable",#{used:ctx.format_number(b.used)})}
        else{ctx.t_or("context_window.used","{used} of {limit} tokens used",#{used:ctx.format_number(b.used),limit:ctx.format_number(p.limit)})};
    let words=if hot!=""&&hot in m.parts{m.parts[hot].label+": "+ctx.format_number(shown)+" "+tx(ctx,"tokens","tokens")}else{line};
    // All three visible texts are direct children of the baseline row. The
    // keyed numeric sources have no layout footprint or semantic duplicate.
    reads.push(ink(ctx,"of",info,hint().flex_grow()).with_key(info_key)
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:240,easing:ease(),intent:"decorative"})));
    let node=row(reads+percentage).with_key("summary").accessibility_label(tx(ctx,"usage","Token usage")).accessibility_description(words)
        .with_style(ctx.component_style("figure",style().relative().min_width(px(0)).gap(sp(1.5)).items_center()));
    if p.limit==0{node.accessibility_role("status")}
    else{node.accessibility_role("progressbar").accessibility_value(min(p.limit,b.used)).accessibility_value_min(0).accessibility_value_max(p.limit)}
}
fn field(ctx,p,m,hot,narrow){
    let grid=[];let pending=get(p,"pending",());
    let columns=if narrow{max(8,(p.columns*2+1)/3)}else{p.columns};
    columns=min(columns,p.cells);
    for i in 0..m.budget.owners.len{
        let index=m.budget.owners[i];let key=if index<0{""}else{m.order[index]};
        let shifting=pending!=()&&pending.kind=="compact"&&pending.part==key;
        let paint=style().width(relative(1.0)).aspect_ratio(1.0).min_width(px(0)).relative()
           .radius(theme_radius("sm")).background(theme_color(if index<0{"context_window.free"}else{m.parts[key].tone}))
           .opacity(if index<0||hot==""||key==hot{1.0}else{0.25});
        let light=[];
        if shifting{
            light.push(box([]).with_key("spark").with_style(ctx.component_style("spark",
                style().absolute().top(px(0)).inset_start(px(0)).width(relative(1.0)).height(relative(1.0))
                    .radius(theme_radius("sm")).background(theme_color("context_window.shine")).opacity(0.0)))
                .motion(motion_keyframes("opacity",[#{offset:0.0,value:0.0},#{offset:0.45,value:0.72},#{offset:1.0,value:0.0}],
                   #{duration_ms:900,delay_ms:(i%columns)*15,iterations:"infinite",intent:"decorative"})));
        }
        let shape=box(light).with_key(i.to_string()).accessibility_hidden(true)
            .with_style(ctx.component_style(if hot!=""&&key==hot{["tile","tile_active"]}else{["tile"]},paint))
            .paint_transition(380,"ease_out");
        grid.push(shape.on_hover_change(Fn("alter").curry(p.context,"bar",key)));
    }
    box(grid).with_key("field").accessibility_hidden(true).with_style(ctx.component_style("field",
        style().grid_cols(columns).width(relative(1.0)).min_width(px(0)).gap(sp(0.5))))
}
fn request_for(p,kind,part,item){#{context:p.context,revision:p.revision,kind:kind,part:part,item:item}}
fn items_view(ctx,p,part,ready,index){
    let children=[];
    for item in part.items{
        let nodes=[ink(ctx,"item_name",item.label,hint().flex_grow()),ink(ctx,"item_value",short(ctx,item.tokens,true),hint().font_feature("tnum",1).flex_shrink(false))];
        if !part.locked&&get(p,"on_request",())!=(){
            let label=ctx.t_or("context_window.remove","Remove {name}",#{name:item.label});
            let control=row([icon("M4 4l8 8m0-8-8 8")]).with_key("remove").tab_stop(true).accessibility_role("button")
                .accessibility_label(label).accessibility_description(if ready{""}else{tx(ctx,"waiting","Waiting for the Host")})
                .with_style(ctx.component_style("remove",style().width(unit(6)).height(unit(6)).flex_shrink(false)
                    .items_center().justify_center().radius(theme_radius("sm")).opacity(if ready{1.0}else{0.5})
                    .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-1),theme_color("focus_ring"))));
            if ready{control=node_button(control,Fn("request").curry(request_for(p,"remove",part.id,item.id),open_ref(index)));}
            nodes.push(control);
        }
        children.push(row(nodes).with_key(item.id).accessibility_role("listitem").accessibility_label(item.label)
            .accessibility_description(ctx.format_number(item.tokens)+" "+tx(ctx,"tokens","tokens"))
            .with_style(ctx.component_style("item",style().min_width(px(0)).min_height(unit(6)).items_center().gap(sp(1.5))))
            .exit_motion(motion_transition("opacity",1.0,0.0,#{duration_ms:240,easing:ease(),intent:"decorative"})));
    }
    if children.len==0{children.push(ink(ctx,"empty",tx(ctx,"empty","Nothing attached"),hint()));}
    column(children).with_key("items").accessibility_role("list").accessibility_label(part.label)
        .with_style(ctx.component_style("items",style().min_width(px(0)).padding_start(sp(4)).padding_bottom(sp(1)).gap(sp(0.5))))
        .layout_size_motion(part.items.len.to_string(),"height",420,ease())
}
fn compact_button(ctx,p,part,ready,narrow){
    let pending=get(p,"pending",());let compacted=get(p,"compacted",());
    let done=compacted!=()&&compacted.part==part.id&&part.tokens<=compacted.tokens;
    let busy=pending!=()&&pending.kind=="compact"&&pending.part==part.id;
    let available=ready&&part.tokens>0&&!done;
    let label=ctx.t_or("context_window.compact","Compact {name}",#{name:part.label});
    let word=if busy{tx(ctx,"compacting","Compacting")}else if done{tx(ctx,"compacted","Compacted")}else{tx(ctx,"compact_short","Compact")};
    let badge=if busy{spin::Spinner(#{key:"busy:"+part.id,label:word,size:"xs"}).accessibility_hidden(true)}
        else{icon(if done{"M3 8l3 3 7-7"}else{"M8 2v5m-3-3 3 3 3-3M8 14V9m-3 3 3-3 3 3"})};
    let bits=[badge];if !narrow||busy{bits.push(ink(ctx,"amount",word,hint()));}
    let node=row(bits).with_key("compact").accessibility_role("button").accessibility_label(label)
        .accessibility_description(word).tab_stop(true).with_style(ctx.component_style(if done{["compact","done"]}else{["compact"]},
           style().height(unit(6)).gap(sp(0.75)).items_center().padding_x(sp(1.5)).radius(theme_radius("sm")).flex_shrink(false)
              .opacity(if available||busy||done{1.0}else{0.6}).hover(style().background(theme_color("surface_hover")))
              .focus_ring(px(2),offset_px(-1),theme_color("focus_ring"))));
    if available{node_button(node,Fn("request").curry(request_for(p,"compact",part.id,""),""))}else{node}
}
fn list(ctx,p,m,hot,narrow){
    let rows=[];let open=ctx.get_state("open_part");
    let ready=get(p,"pending",())==()&&get(p,"on_request",())!=();
    for i in 0..m.order.len{
        let id=m.order[i];let part=m.parts[id];let expandable=get(part,"items",())!=();
        let shown=expandable&&open!=()&&open.context==p.context&&open.id==id;
        let mark=box([]).accessibility_hidden(true).with_style(ctx.component_style("dot",
            style().width(unit(2)).height(unit(2)).flex_shrink(false).radius(theme_radius("sm"))
                .background(theme_color(part.tone))));
        let bits=[mark,ink(ctx,"name",part.label,hint().text_color(theme_color("text_primary")).flex_grow())];
        if expandable{
            let deg=if shown{180.0}else{0.0};
            bits.push(icon("M4 6l4 4 4-4").with_key("arrow")
                .with_style(ctx.component_style("arrow",style().transform(#{rotate:deg}))))
        }
        let trigger=row(bits).with_key("head").with_ref(element_ref(open_ref(i))).tab_stop(true)
            .accessibility_role(if expandable{"button"}else{"group"}).accessibility_label(part.label)
            .accessibility_description(ctx.format_number(part.tokens)+" "+tx(ctx,"tokens","tokens"))
            .with_style(ctx.component_style("main",style().flex_grow().flex_basis(px(0)).min_width(px(0)).gap(sp(1.5))
                .items_center().radius(theme_radius("sm")).min_height(unit(8)).focus_ring(px(2),offset_px(-1),theme_color("focus_ring"))))
            .on_key_value("plain+up",Fn("focus_part").curry(max(0,i-1)),())
            .on_key_value("plain+down",Fn("focus_part").curry(min(m.order.len-1,i+1)),());
        if expandable{trigger=node_button(trigger.accessibility_expanded(shown),Fn("toggle").curry(p.context,id));}
        let row_items=[trigger];
        if part.compact&&!part.locked&&get(p,"on_request",())!=(){row_items.push(compact_button(ctx,p,part,ready,narrow));}
        row_items.push(reading(ctx,"part:"+i.to_string(),"amount",part.tokens,"",hint().typography("body").text_end()));
        let contents=[row(row_items).with_key("row").with_style(style().min_width(px(0)).items_center().gap(sp(1.5)))];
        if shown{contents.push(items_view(ctx,p,part,ready,i));}
        rows.push(column(contents).with_key(id).accessibility_role("listitem")
            .on_hover_change(Fn("alter").curry(p.context,"row",id)).on_focus_within_change(Fn("alter").curry(p.context,"focus",id))
            .with_style(ctx.component_style(if hot==id{["part","active"]}else{["part"]},
                style().min_width(px(0)).padding_x(sp(1)).radius(theme_radius("sm"))
                    .opacity(if hot==""||hot==id{1.0}else{0.52})))
            .layout_size_motion(if shown{"open"}else{"closed"},"height",380,ease()).paint_transition(160,"ease_out"));
    }
    column(rows).with_key("parts").accessibility_role("list").accessibility_label(tx(ctx,"parts","Context parts"))
        .with_style(ctx.component_style("parts",style().min_width(px(0))))
}
fn footer(ctx,p,m){
    let b=m.budget;let saved=get(p,"saved_tokens",0);let pending=get(p,"pending",());
    let note=if pending!=(){if pending.kind=="compact"{tx(ctx,"compacting","Compacting")}else{tx(ctx,"waiting","Waiting for the Host")}}
      else if saved>0{ctx.t_or("context_window.saved","Saved {n} tokens",#{n:short(ctx,saved,false)})}
      else if p.limit==0{tx(ctx,"unknown","No token limit available")}
      else if b.overflow>0{ctx.t_or("context_window.over","{n} tokens over the limit",#{n:short(ctx,b.overflow,false)})}
      else if b.remaining==0{tx(ctx,"at_limit","Window is full")}
      else{ctx.t_or("context_window.left","{n} tokens left",#{n:short(ctx,b.remaining,false)})};
    let caption=hint().font_feature("tnum",1).text_color(theme_color("context_window.muted")).flex_grow();
    let pieces=[ink(ctx,if saved>0&&pending==(){["note","saved"]}else{["note"]},note,caption).with_key("note")
        .accessibility_role("status").accessibility_label(note)];
    if get(p,"action_label","")!=""&&get(p,"on_action",())!=(){
        let path=if ctx.text_direction()=="rtl"{"M9.5 6h-7M5.5 3l-3 3 3 3"}else{"M2.5 6h7M6.5 3l3 3-3 3"};
        let arrow=svg("<svg viewBox='0 0 12 12' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><path d='"+path+"'/></svg>")
            .with_key("arrow").accessibility_hidden(true)
            .with_style(ctx.component_style("action_icon",style().width(unit(3)).height(unit(3)).flex_shrink(false)));
        pieces.push(btn::Button(#{key:"action",text:p.action_label,size:"xs",variant:"ghost",suffix:arrow,
            on_click:Fn("action").curry(#{context:p.context,revision:p.revision})})
            .with_style(ctx.component_style("action",style().min_width(px(0)).max_width(relative(0.5)))));
    }
    row(pieces).with_key("footer").with_style(ctx.component_style("footer",style().min_width(px(0)).min_height(unit(10))
         .padding_x(sp(3)).items_center().gap(sp(1.5))))
}
fn render_context(ctx,p){
    let m=project(p);let hot=current(ctx,p,m);
    let reference=element_ref("context");
    let narrow=ctx.element_width_class(reference,[296])==0;
    let title=get(p,"title",tx(ctx,"title","Context window"));
    let heading=[ink(ctx,"title",title,hint().font_weight(500).text_color(theme_color("text_primary")).flex_grow())
        .accessibility_role("heading").accessibility_level(2)];
    let model=get(p,"model","");
    if model!=""{heading.push(ink(ctx,"model",model,hint().padding_x(sp(1.5)).max_width(relative(0.62))
         .background(theme_color("surface_hover")).radius(theme_radius("sm"))));}
    let head=row(heading).with_key("head").with_style(ctx.component_style("head",style().min_width(px(0)).items_center().gap(sp(2))));
    let content=column([head,summary(ctx,p,m,hot),field(ctx,p,m,hot,narrow),list(ctx,p,m,hot,narrow)])
        .with_key("card").with_style(ctx.component_style("card",style().padding(sp(3.5)).gap(sp(2))
           .min_width(px(0)).width(relative(1.0)).background(theme_color("surface_raised")).radius(theme_radius("lg"))));
    column([content,footer(ctx,p,m)]).with_key(p.key).with_ref(reference).accessibility_role("group").accessibility_label(title)
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(360))
            .radius(theme_radius("lg")).background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border"))))
}
