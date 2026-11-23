/* gpui-rhai
{"id":"patterns/tool_call_group","export":"ToolCallGroup","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["patterns/tool_call_card","patterns/agent_tree"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","radius.lg","radius.pill","space.spacing_unit","space.unit","surface_hover","text.danger","text_muted","text_primary","tool_call.tile","typography.body","typography.caption"],"environment":[]}
*/
// A compact summary and lazy disclosure for several read-only tool calls.
// Example: ToolCallGroup(#{key:"tools",session:"inspect",revision:0,calls:[#{id:"a",name:"read_document",status:"running"}]})
import "patterns/tool_call_card" as card;
import "patterns/agent_tree" as ids;

define_component(#{
    metadata:#{id:"patterns/tool_call_group","export":"ToolCallGroup",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/tool_call_card","patterns/agent_tree"],capabilities:#{},assets:[],tokens:["border","focus_ring","radius.lg","radius.pill","space.spacing_unit","space.unit","surface_hover","text.danger","text_muted","text_primary","tool_call.tile","typography.body","typography.caption"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable group identity."},
        session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Session ID carried by all contained call intents."},
        revision:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Accepted batch revision."},
        calls:#{schema:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable call identity within this session."},name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Tool name. Presentation only; the component never executes it."},summary:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short optional summary."},icon:#{schema:#{type:"optional",value:#{type:"string",allowed:["tool","search","database","file","web","terminal"]}},required:false,sensitive:false,doc:"Built-in glyph name."},status:#{schema:#{type:"string",allowed:["running","success","error"]},required:true,sensitive:false,doc:"Accepted execution status."},arguments:#{schema:#{type:"optional",value:#{type:"ui_value"}},required:false,sensitive:true,doc:"Arguments as structured JSON values, JSON text, or plain text."},result:#{schema:#{type:"optional",value:#{type:"ui_value"}},required:false,sensitive:true,doc:"Result as structured JSON values, JSON text, or plain text."},error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host-supplied failure message."},started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Epoch time in milliseconds for a running duration."},duration_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Accepted final duration in milliseconds."}}}},required:true,sensitive:true,doc:"Accepted calls in render order; IDs must be unique."},
        label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Alternative completed group caption."},
        running_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Alternative running caption."},
        duration_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Explicit accepted group duration; otherwise use the sum of known completed durations."},
        now_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Inject clock time for deterministic previews; otherwise the group runs one view-clock timer."},
        pending:#{schema:#{type:"array",max_items:16,items:#{type:"string"}},required:false,sensitive:false,"default":#{type:"array",value:[]},doc:"Accepted IDs whose retry is awaiting acknowledgement."},
        open:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Controlled disclosure when supplied."},
        default_open:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Initial local disclosure."},
        on_open_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Group disclosure request."},
        on_retry:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Forwards the child call's retry identity without executing it."},
        on_copy:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Forwards the copied section and IDs, never raw text."}
      },
      state:#{fields:#{disclosure:#{schema:#{type:"ui_value"},"default":#{type:"null"}},clock:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        children:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
      events:#{
          open_change:#{payload:#{type:"bool"},doc:"Group disclosure intent."},
          retry:#{payload:#{type:"object",allow_unknown:false,fields:#{
             session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Session ID."},
             revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision."},
             id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business call ID."}}},doc:"Versioned retry request from a child card."},
          copy:#{payload:#{type:"object",allow_unknown:false,fields:#{
             session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Session ID."},
             revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision."},
             id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business call ID."},
             section:#{schema:#{type:"string",allowed:["arguments","result"]},required:true,sensitive:false,doc:"Copied section without content."}}},doc:"Child clipboard request notification."}
      },slots:#{},parts:["root","header","icons","mark","title","failed","elapsed","arrow","list","announcement","empty"],
      timers:["clock"]
    },render:Fn("render_group")
});
fn ToolCallGroup(p){render_component("patterns/tool_call_group",p)}
fn get(p,k,d){if k in p&&p[k]!=(){p[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn unit(n){theme_length("space.unit").times(n)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn tr(ctx,key,caption){ctx.t_or("tool_call."+key,caption)}
fn pill(ctx,part,value,base){text(value).with_style(ctx.component_style(part,base.min_width(px(0))))}
fn hint(){style().typography("caption").text_color(theme_color("text_muted")).text_ellipsis().whitespace_nowrap()}
fn notify(kind,ctx,event){ctx.emit(kind,event);}
fn choices(ctx,scope){
    let v=ctx.get_state("children");
    if v!=()&&v.scope==scope{v.values}else{#{}}
}
fn child_choice(scope,live,id,ctx,open){
    let old=choices(ctx,scope);let next=#{};
    for key in live{if key in old{next[key]=old[key];}}
    next[id]=open;ctx.set_state("children",#{scope:scope,values:next});
}
fn toggle(scope,controlled,emit,shown,ctx,event){
    if !controlled{ctx.set_state("disclosure",#{scope:scope,open:!shown});}
    if emit{ctx.emit("open_change",!shown);}
    handled()
}
fn inspect_calls(p){
    let ids=#{};let working=false;let errors=0;let known=p.calls.len>0;let total=0;
    for c in p.calls{
        card::validate_call(c);
        if c.id in ids{throw "ToolCallGroup requires distinct call IDs";}
        ids[c.id]=true;
        if c.status=="running"{working=true;}else if c.status=="error"{errors+=1;}
        let duration=get(c,"duration_ms",());
        if duration==(){known=false;}else{total+=duration;}
    }
    #{working:working,errors:errors,total:total,known:known,ids:ids.keys()}
}
fn group_header(ctx,p,m,scope,expanded,controlled,elapsed){
    let count=if p.calls.len==1{
        tr(ctx,"used_one","Used 1 tool")
    }else{ctx.t_or("tool_call.used_many","Used {n} tools",#{n:ctx.format_number(p.calls.len)})};
    let title=if m.working{get(p,"running_label",tr(ctx,"using","Using tools…"))}else{get(p,"label",count)};
    let bits=[];
    for i in max(0,p.calls.len-3)..p.calls.len{
        let item=p.calls[i];
        let base=style().width(unit(6)).height(unit(6)).items_center().justify_center().flex_shrink(false)
            .radius(theme_radius("pill")).background(theme_color("tool_call.tile")).text_color(theme_color("text_primary"))
            .border(px(1)).border_color(theme_color("border"));
        if bits.len>0{base=base.margin_start(offset_px(-5));}
        bits.push(row([card::glyph(get(item,"icon","tool"))]).with_key(item.id).accessibility_hidden(true)
            .with_style(ctx.component_style("mark",base)));
    }
    let words=row(bits).with_key("badges").with_style(ctx.component_style("icons",style().items_center().flex_shrink(false)));
    let details=[words,pill(ctx,"title",title,hint().font_weight(500).flex_grow()).with_key("title")];
    let fail=if m.errors>0{ctx.t_or("tool_call.failed_count","{n} failed",#{n:ctx.format_number(m.errors)})}else{""};
    if !m.working&&fail!=""{details.push(pill(ctx,"failed","· "+fail,hint().text_color(theme_color("text.danger"))).with_key("failures"));}
    let time=card::duration(ctx,elapsed);
    if time!=""{details.push(pill(ctx,"elapsed","· "+time,hint().font_feature("tnum",1)).with_key("elapsed"));}
    let angle=if expanded{180.0}else{0.0};
    details.push(card::glyph("chevron").with_key("arrow").with_style(ctx.component_style("arrow",
        style().text_color(theme_color("text_muted")).transform(#{rotate:angle})))
        .motion(motion_transition("rotate",angle,angle,#{duration_ms:200,easing:ease(),intent:"decorative"})));
    let cb=Fn("toggle").curry(scope,controlled,get(p,"on_open_change",())!=(),expanded);
    row(details).with_key("header").tab_stop(true).accessibility_role("button").accessibility_label(title)
        .accessibility_expanded(expanded).accessibility_description(if fail!=""{fail}else{time})
        .on_click(cb).on_key_value("plain+enter",cb,()).on_key_value("plain+space",cb,())
        .with_style(ctx.component_style("header",style().width(relative(1.0)).min_width(px(0)).items_center().gap(sp(2))
            .min_height(unit(9)).padding_x(sp(2)).radius(theme_radius("lg"))
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-1),theme_color("focus_ring"))))
}
fn render_group(ctx,p){
    if !ids::valid(p.session){throw "ToolCallGroup requires a stable session ID";}
    let m=inspect_calls(p);let scope=#{session:p.session,key:p.key};
    let old=ctx.get_state("disclosure");let controlled=get(p,"open",())!=();
    let expanded=if controlled{p.open}else if old!=()&&old.scope==scope{old.open}else{p.default_open};
    let clock=card::tool_clock(ctx,scope,get(p,"now_ms",()),m.working);
    let elapsed=get(p,"duration_ms",());
    if m.working{
        elapsed=0;
        for c in p.calls{if c.status=="running"{elapsed=max(elapsed,card::elapsed(c,clock));}}
    }else if elapsed==()&&m.known{elapsed=m.total;}
    let contents=[group_header(ctx,p,m,scope,expanded,controlled,elapsed)];
    // The semantic status survives collapse without keeping any code viewer alive.
    let status=if m.working{get(p,"running_label",tr(ctx,"using","Using tools…"))}
        else{ctx.t_or("tool_call.used_many","Used {n} tools",#{n:ctx.format_number(p.calls.len)})};
    if m.errors>0{status+=" · "+ctx.t_or("tool_call.failed_count","{n} failed",#{n:ctx.format_number(m.errors)});}
    contents.push(pill(ctx,"announcement",status,style().absolute().width(px(1)).height(px(1)).opacity(0.0).overflow_hidden())
        .with_key("announcement").accessibility_role("status").accessibility_label(status));
    if expanded{
        let items=[];
        let child_state=choices(ctx,scope);
        for c in p.calls{
            items.push(card::ToolCallCard(#{key:"call:"+c.id,session:p.session,revision:p.revision,entry:c,
                now_ms:if c.status=="running"{clock.now}else{()},
                open:get(child_state,c.id,()),on_open_change:Fn("child_choice").curry(scope,m.ids,c.id),
                pending:c.id in p.pending,on_retry:if get(p,"on_retry",())!=(){Fn("notify").curry("retry")}else{()},
                on_copy:if get(p,"on_copy",())!=(){Fn("notify").curry("copy")}else{()}}));
        }
        if items.len==0{items.push(pill(ctx,"empty",tr(ctx,"no_tools","No tools were used."),hint()));}
        contents.push(column(items).with_key("list").with_style(ctx.component_style("list",style().min_width(px(0)).gap(sp(2)))));
    }
    column(contents).with_key(p.key).accessibility_role("group").accessibility_label(tr(ctx,"group","Tool calls"))
        .with_style(ctx.component_style("root",style().relative().width(relative(1.0)).max_width(px(760)).min_width(px(0))))
        .layout_size_motion(if expanded{"open"}else{"closed"},"height",380,ease())
}
