/* gpui-rhai
{"id":"patterns/fallback_chain","export":"FallbackChain","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/hover_card"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.initials","typography.metadata","fallback_chain.tray","fallback_chain.area","fallback_chain.card","fallback_chain.grid","fallback_chain.fill","fallback_chain.ink","fallback_chain.text","fallback_chain.idle","fallback_chain.wait","fallback_chain.muted","fallback_chain.quiet","fallback_chain.rail","fallback_chain.active","fallback_chain.active_edge","fallback_chain.active_fill","fallback_chain.ok","fallback_chain.ok_edge","fallback_chain.ok_fill","fallback_chain.error","fallback_chain.error_edge","fallback_chain.error_fill","fallback_chain.popover"],"environment":[]}
*/
// Inspect attempts in a host-owned fallback chain, without making requests.
// Example: FallbackChain(#{key:"route",run:"sample",now_ms:5200,attempts:[#{id:"mini",model:"Atlas Mini",status:"trying",started_ms:0}]})
import "components/hover_card" as hover;

define_component(#{
    metadata:#{id:"patterns/fallback_chain","export":"FallbackChain",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/hover_card"],capabilities:#{},assets:[],tokens:["border","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.initials","typography.metadata","fallback_chain.tray","fallback_chain.area","fallback_chain.card","fallback_chain.grid","fallback_chain.fill","fallback_chain.ink","fallback_chain.text","fallback_chain.idle","fallback_chain.wait","fallback_chain.muted","fallback_chain.quiet","fallback_chain.rail","fallback_chain.active","fallback_chain.active_edge","fallback_chain.active_fill","fallback_chain.ok","fallback_chain.ok_edge","fallback_chain.ok_fill","fallback_chain.error","fallback_chain.error_edge","fallback_chain.error_fill","fallback_chain.popover"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable instance key."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business identity of the chain."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted Host snapshot revision."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Text above the route; defaults to Request."},
        route:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Route name on the request node."},
        request:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            name:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Endpoint name."},
            logo:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative endpoint mark."},
            text:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Preview of the request text."},
            details:#{schema:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Detail field label."},
                value:#{schema:#{type:"one_of",variants:[#{type:"string"},#{type:"node"}]},required:true,sensitive:true,doc:"Plain text or a rich SDK node."}
            }}},"default":#{type:"array",value:[]},required:false,sensitive:true,doc:"Optional preview details."}
        }}},required:false,sensitive:true,doc:"Optional request details."},
        response:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            name:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Endpoint name."},
            logo:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative endpoint mark."},
            text:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Preview of the response text."},
            details:#{schema:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Detail field label."},
                value:#{schema:#{type:"one_of",variants:[#{type:"string"},#{type:"node"}]},required:true,sensitive:true,doc:"Plain text or a rich SDK node."}
            }}},"default":#{type:"array",value:[]},required:false,sensitive:true,doc:"Optional preview details."}
        }}},required:false,sensitive:true,doc:"Optional response details."},
        now_ms:#{schema:#{type:"integer",min:0,max:1000000000000},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Host-owned elapsed time. No timer or status transition is inferred from the wall clock."},
        currency:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Currency prefix, defaults to $."},
        attempts:#{schema:#{type:"array",max_items:24,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable, unique attempt ID."},
            model:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Model or route name."},
            provider:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Provider shown on wider cards and in details."},
            region:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional region for the preview."},
            logo:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative mark, otherwise initials."},
            provider_logo:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative mark for the provider preview."},
            status:#{schema:#{type:"string",allowed:["waiting","trying","failed","answered","skipped"]},"default":#{type:"string",value:"waiting"},required:false,sensitive:false,doc:"Host-owned state. At most one current attempt or answer."},
            started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Trying attempt start on the same clock as now_ms."},
            duration_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Recorded time for an answered or failed attempt."},
            timeout_ms:#{schema:#{type:"integer",min:1,max:1000000000000},"default":#{type:"integer",value:10000},required:false,sensitive:false,doc:"Positive timeout scale. Reaching it fills the bar but does not fail a task."},
            cost_micros:#{schema:#{type:"optional",value:#{type:"integer",min:-1000000000000,max:1000000000000}},required:false,sensitive:true,doc:"Signed accepted charge in currency micro-units."},
            reason:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short failure reason."},
            note:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short note on the answered card."},
            details:#{schema:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Detail field label."},
                value:#{schema:#{type:"one_of",variants:[#{type:"string"},#{type:"node"}]},required:true,sensitive:true,doc:"Plain text or a rich SDK node."}
            }}},"default":#{type:"array",value:[]},required:false,sensitive:true,doc:"Optional preview details."}
        }}},required:true,sensitive:true,doc:"Attempts in the order in which they appear."},
        on_retry:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Enables a Retry proposal on a terminal all-failed chain."}
    },state:#{fields:#{}},events:#{retry:#{payload:#{type:"object",allow_unknown:false,fields:#{
        run:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false}
    }},doc:"Requests a retry of the accepted failed chain without changing its status."}},slots:#{},
    parts:["root","head","title","totals","dot","sum","state","retry","flow","tree","grid","wires","pulse","endpoint","end_wait",
        "end_failed","end_answered","endpoint_icon","node_title","node_sub","route","attempts","attempts_tree","attempt","attempt_trying",
        "attempt_answered","attempt_skipped","logo","number","model","provider","attempt_status","timing","cost","blocks","block","popup",
        "popup_title","popup_sub","badge","quote","detail_rows","detail_label","detail_value","empty"]},render:Fn("render_chain")
});

fn FallbackChain(p){render_component("patterns/fallback_chain",p)}
fn get(m,k,d){if m!=()&&k in m&&m[k]!=(){m[k]}else{d}}
fn u(n){theme_length("space.unit").times(n)}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn tx(ctx,k,t){ctx.t_or("fallback_chain."+k,t)}
fn ink(k){let path="fallback_chain."+k;theme_color(path)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn subtle(){style().typography("caption").text_color(ink("muted")).min_width(px(0))}
fn caption(){subtle().whitespace_nowrap().text_ellipsis()}
fn label(ctx,p,t,s){text(t).with_style(ctx.component_style(p,s))}
fn readout(ctx,part,key,value,paint,sweep){
    if sweep&&value!=""{paint=paint.glyph_ink_sweep(ink("quiet"),0.3,2.0,0.0);}
    let result=label(ctx,part,value,paint.font_feature("tnum",1)).with_key(key);
    if !sweep||value==""{return result;}
    result.motion(motion_keyframes("glyph_sweep",[
        #{offset:0.0,value:0.0},#{offset:0.55,value:1.0,easing:"ease_in_out"},#{offset:1.0,value:1.0}
    ],#{duration_ms:2600,iterations:"infinite",intent:"decorative"}))
}
fn two(n){if n<10{"0"+n.to_string()}else{n.to_string()}}
fn duration(ctx,n){
    if n==(){return "—";}
    if n<1000{return ctx.format_number((n+5)/10*10)+"ms";}
    if n<60000{return ctx.format_number(n.to_float()/1000.0,#{min_fraction_digits:1,max_fraction_digits:1})+"s";}
    let s=(n+500)/1000;
    if s<3600{return (s/60).to_string()+"m "+two(s%60)+"s";}
    let mins=(s+30)/60;
    ctx.format_number(mins/60)+"h "+two(mins%60)+"m"
}
fn money(ctx,p,n){
    if n==(){return "—";}
    let prefix=get(p,"currency","$");let val=abs(n);let sign=if n<0{"−"}else{""};
    if val==0{return prefix+"0";}
    if val<100{return sign+"<"+prefix+"0.0001";}
    let digits=if val<10000{4}else if val<1000000{3}else{2};
    sign+prefix+ctx.format_number(val.to_float()/1000000.0,#{min_fraction_digits:digits,max_fraction_digits:digits})
}
fn state_label(ctx,s){switch s{
    "trying"=>tx(ctx,"trying","Trying"),"answered"=>tx(ctx,"answered","Answered"),"failed"=>tx(ctx,"failed","Failed"),
    "skipped"=>tx(ctx,"skipped","Not needed"),_=>tx(ctx,"waiting","Waiting")
}}
fn tone(s){switch s{"trying"=>"active","answered"=>"ok","failed"=>"error",_=>"quiet"}}
fn icon(ctx,which){
    let d=if which=="request"{"M4 4h6a2 2 0 0 1 0 4H5a2 2 0 0 0 0 4h1M4 4h-1m9 4h1m-5 4h2"}
        else{"M3 4h10v7H8l-4 3v-3H3Z M5.5 6.5h5m-5 2h3.5"};
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.3' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
        .with_style(style().width(u(4)).height(u(4)).flex_shrink(false)).accessibility_hidden(true)
}
fn initials(words){
    let result="";let n=0;
    for word in words.split(" "){
        if word!=""&&n<2{result+=grapheme_slice(word,0,1).to_string().to_upper();n+=1;}
    }
    result
}
fn request_retry(ticket,ctx,e){ctx.emit("retry",ticket);handled()}
fn head(ctx,p,m){
    let state=m.data.state;
    let message=if state=="trying"{ctx.t_or("fallback_chain.attempt_of","Attempt {step} of {total}",#{step:m.data.trying+1,total:p.attempts.len})}
        else if state=="failed"{tx(ctx,"no_answer","No model answered")}else{state_label(ctx,state)};
    let total_t=duration(ctx,m.data.time);let total_c=money(ctx,p,m.data.cost);
    let dot=box([]).with_key("dot").accessibility_hidden(true).with_style(ctx.component_style("dot",
        style().width(u(1.5)).height(u(1.5)).radius(u(5)).flex_shrink(false).background(ink(tone(state)))));
    if state=="trying"{dot=dot.motion(motion_transition("opacity",0.4,1.0,#{duration_ms:1200,iterations:"infinite",autoreverse:true,intent:"decorative"}));}
    let state_ink=if state=="answered"||state=="failed"{tone(state)}else{"muted"};
    let stats=row([dot,label(ctx,"state",message,caption().text_color(ink(state_ink))).with_key("state"),
        readout(ctx,"sum","time-total",total_t,caption().text_color(ink("ink")),state=="trying"),
        readout(ctx,"sum","cost-total",total_c,caption().text_color(ink("ink")),state=="trying")])
        .with_key("summary").accessibility_role("status").accessibility_label(message+" · "+total_t+" · "+total_c)
        .with_style(ctx.component_style("totals",style().min_width(px(0)).items_center().gap_x(sp(3)).flex_wrap()));
    let title=label(ctx,"title",get(p,"title",tx(ctx,"title","Request")),style().typography("body").font_weight(600).min_width(px(0))
        .whitespace_nowrap().text_ellipsis().flex_grow());
    if m.narrow{title=title.with_style(style().width(relative(1.0)));}
    let parts=[title,stats];
    if state=="failed"&&get(p,"on_retry",())!=(){
        let name=tx(ctx,"retry","Retry");
        let callback=Fn("request_retry").curry(#{run:p.run,revision:p.revision});
        parts.push(row([text(name)]).with_key("retry").accessibility_role("button").accessibility_label(name).tab_stop(true)
            .on_click(callback).on_key_value("plain+enter",callback,()).on_key_value("plain+space",callback,())
            .with_style(ctx.component_style("retry",style().min_height(u(7)).items_center().padding_x(sp(3)).cursor_pointer()
                .radius(theme_radius("sm")).hover(style().background(theme_color("surface_hover")))
                .focus_ring(px(2),offset_px(-2),theme_color("focus_ring")))));
    }
    row(parts).with_key("head").with_style(ctx.component_style("head",style().min_width(px(0)).items_center().gap_x(sp(3))
        .gap_y(sp(1)).padding_x(sp(3)).flex_wrap()))
}
fn preview(ctx,title,sub,mark,state,badge,quote,fields){
    let words=[label(ctx,"popup_title",title,caption().font_weight(600).text_color(ink("ink")))];
    if sub!=""{words.push(label(ctx,"popup_sub",sub,caption()));}
    let top=[box([mark]).accessibility_hidden(true).with_style(style().width(u(7)).height(u(7)).flex_shrink(false)
        .flex_col().items_center().justify_center().radius(theme_radius("sm")).background(ink("card")).text_color(ink("text"))),
        column(words).with_style(style().min_width(px(0)).flex_basis(px(0)).flex_grow())];
    if badge!=""{
        let color=tone(state);
        top.push(label(ctx,"badge",badge,caption().text_color(ink(color))
            .background(if color=="quiet"{ink("idle")}else{ink(color+"_fill")})
            .padding_x(sp(2)).radius(theme_radius("sm")).font_weight(500).flex_shrink(false)));
    }
    let body=[row(top).with_style(style().min_width(px(0)).items_center().gap_x(sp(2)))];
    if quote!=""{body.push(label(ctx,"quote",quote,subtle().line_clamp(3).padding(sp(2)).background(ink("idle")).radius(theme_radius("sm"))));}
    let rows=[];
    for field in fields{
        let value=if type_of(field.value)=="string"{label(ctx,"detail_value",field.value,subtle().line_clamp(3).text_end()
            .text_color(if get(field,"error",false){ink("error")}else{ink("ink")}))}
            else{field.value};
        rows.push(row([label(ctx,"detail_label",field.label,caption().flex_shrink(false)),
            column([value]).with_style(style().min_width(px(0)).flex_grow().items_end())]).with_style(style().min_width(px(0)).gap_x(sp(2)).items_start()));
    }
    if rows.len>0{body.push(column(rows).with_style(ctx.component_style("detail_rows",style().gap_y(sp(1)).padding_top(sp(2))
        .border_top(px(1)).border_color(theme_color("border")).min_width(px(0)))));}
    column(body).with_style(style().min_width(px(0)).gap_y(sp(2)))
}
fn floating(ctx,p,id,caption,trigger,content,placement,align,base,reference){
    hover::HoverCard(#{key:id,label:caption,trigger:trigger,content:content,width:px(280),placement:placement,align:align,show_delay_ms:140,hide_delay_ms:180,
        part_styles:#{root:base,content:ctx.component_style("popup",style().background(ink("popover")).padding(sp(3)))}})
        .with_ref(reference)
}
fn endpoint(ctx,p,m,which){
    let req=which=="request";let state=if req{"trying"}else{m.data.state};let winner=m.data.winner;
    let name=if req{tx(ctx,"request","Request")}else if state=="failed"{tx(ctx,"no_response","No answer")}else{tx(ctx,"response","Response")};
    let route=get(p,"route","");let sub=if req{route}
        else if state=="answered"{let a=p.attempts[winner];get(a,"note",a.model)}
        else if state=="failed"{ctx.format_number(m.data.failed)+" "+tx(ctx,"failed_count","failed")}else{tx(ctx,"pending","Waiting…")};
    let spec=get(p,which,#{});
    let mark=icon(ctx,which);
    let badge=if req{""}else if state=="answered"{tx(ctx,"delivered","Delivered")}else if state=="failed"{tx(ctx,"failed","Failed")}else{tx(ctx,"pending_badge","Pending")};
    let fields=if req{get(spec,"details",[])}else{[
        #{label:tx(ctx,"total_time","Total time"),value:duration(ctx,m.data.time)},
        #{label:tx(ctx,"total_cost","Total cost"),value:money(ctx,p,m.data.cost)}]};
    fields.push(#{label:tx(ctx,"tried","Tried so far"),value:ctx.format_number(m.data.tried)+" / "+ctx.format_number(p.attempts.len)});
    if !req&&state=="answered"{fields+=get(spec,"details",[]);}
    let show_detail=req||state=="answered";
    let card=preview(ctx,get(spec,"name",name),if req{route}else if state=="answered"{p.attempts[winner].model}else{sub},
        get(spec,"logo",mark),state,badge,if show_detail{get(spec,"text","")}else{""},fields);
    let base=if req{["endpoint"]}else if state=="answered"{["endpoint","end_answered"]}
        else if state=="failed"{["endpoint","end_failed"]}else{["endpoint","end_wait"]};
    let pic=box([mark]).accessibility_hidden(true).with_style(ctx.component_style("endpoint_icon",style().width(u(6)).height(u(6)).flex_shrink(false)
        .flex_col().items_center().justify_center().radius(theme_radius("sm"))
        .text_color(ink(tone(state))).background(ink(if tone(state)=="quiet"{"idle"}else{tone(state)+"_fill"}))));
    let ids=[label(ctx,"node_title",name,caption().font_weight(600).text_color(ink("ink")))];
    if sub!=""{
        let detail=if !req&&state in ["answered","failed"]{caption().text_color(ink(tone(state)))}else{caption()};
        ids.push(label(ctx,if req{"route"}else{"node_sub"},sub,detail));
    }
    let view=row([pic,column(ids).with_style(style().min_width(px(0)).flex_grow())]).with_key(which+"-body")
        .accessibility_role("group").accessibility_label(name).accessibility_description(sub).tab_stop(true)
        .with_style(ctx.component_style(base,style().height(u(13)).min_width(px(0)).items_center().gap_x(sp(2)).padding_x(sp(2))
            .radius(theme_radius("lg")).background(ink("card")).focus_ring(px(2),offset_px(-2),theme_color("focus_ring")))).paint_transition(400,"ease_out");
    let container=style().width(u(if m.narrow{40}else{35})).min_width(px(0)).max_width(relative(1.0)).flex_shrink(false);
    floating(ctx,p,which,name+" details",view,card,"bottom",if req{"start"}else{"end"},container,element_ref(which))
}
fn short_status(ctx,a,r,m,i){
    if r.status=="trying"{return tx(ctx,"trying_line","Trying…");}
    if r.status=="failed"{return get(a,"reason",tx(ctx,"failed","Failed"));}
    if r.status=="answered"{return tx(ctx,"answered","Answered")+(if get(a,"note","")==""{""}else{" · "+a.note});}
    if r.status=="waiting"&&i==m.next_wait{return tx(ctx,"up_next","Up next");}
    state_label(ctx,r.status)
}
fn block_style(ctx){
    ctx.component_style("block",style().width(u(1.25)).height(u(1.25)).flex_shrink(false).radius(u(0.3125)).background(ink("fill")))
}
fn strip_style(ctx){
    ctx.component_style("blocks",style().min_width(px(0)).height(u(1.25))
        .width(relative(1.0)).gap_x(u(0.5)).justify_between().overflow_hidden())
}
fn block_metrics(ctx){
    let unit=ctx.element_size("block-unit");let pair=ctx.element_size("block-pair");
    if unit==()||pair==()||unit.width<=0.0||pair.width<2.0*unit.width{return #{unit:5.0,gap:2.0};}
    #{unit:unit.width,gap:pair.width-2.0*unit.width}
}
fn measure_blocks(ctx){
    let chip=block_style(ctx);
    row([box([]).with_key("block-unit").with_style(chip).with_ref(element_ref("block-unit")),box([]).with_style(chip)])
        .with_key("block-metrics").with_ref(element_ref("block-pair")).accessibility_hidden(true)
        .with_style(strip_style(ctx).absolute().top(px(0)).left(px(0)).width(auto()).padding(px(0)).border(px(0)).justify_start().invisible())
}
fn blocks(ctx,r,m,i){
    // A position belongs to the diagram, not to the Host's unrestricted ID.
    let ref=element_ref("blocks:"+i.to_string());let measured=ctx.element_size(ref);
    let width=if measured==(){196.0}else{measured.width};
    // The 96-cell cap bounds script and paint work; within it neither an
    // oversized final cell nor a row using another card's width is needed.
    let n=max(0,min(96,((width+m.metrics.gap)/(m.metrics.unit+m.metrics.gap)+0.000001).floor().to_int()));
    let t=if r.elapsed_ms==(){0}else{r.elapsed_ms};
    let filled=min(n,(t.to_float()/r.timeout_ms.to_float()*n.to_float()).round().to_int());
    let off=block_style(ctx).opacity(if r.status in ["waiting","skipped"]{0.55}else{1.0});
    let fill=off.background(ink(if r.status=="trying"{"active"}else if r.status=="answered"{"ok_edge"}else{"error_edge"}))
        .opacity(if r.status=="trying"{0.7}else{1.0});
    let parts=[];
    for k in 0..n{
        let box=box([]).with_key(k.to_string()).with_style(if k<filled{fill}else{off});
        if r.status=="trying"{box=box.paint_transition(220,"ease_out");}
        if r.status=="trying"&&k==filled-1{
            box=box.motion(motion_transition("opacity",0.4,1.0,#{duration_ms:700,iterations:"infinite",autoreverse:true,intent:"decorative"}));
        }
        parts.push(box);
    }
    row(parts).with_key("blocks").with_ref(ref).accessibility_hidden(true).with_style(strip_style(ctx))
}
fn model(ctx,p,m,i){
    let data=p.attempts[i];let r=m.data.rows[i];let name=data.model;let provider=get(data,"provider","");
    let supplied=get(data,"logo",());
    let mark=if supplied==(){text(initials(name))}else{supplied};
    let quiet=tone(r.status)=="quiet";
    let reverse=if quiet{ink("muted")}else{ink("card")};
    let cnt=row([text(ctx.format_number(i+1))]).with_key("number").with_style(ctx.component_style("number",caption()
        .text_color(reverse).font_feature("tnum",1).absolute().inset_end(offset_px(-5)).bottom(offset_px(-4))
        .min_width(u(3.25)).height(u(3.25)).items_center().justify_center().padding_x(u(0.75)).radius(u(1.75))
        .background(if quiet{ink("card")}else{ink(tone(r.status))}))).paint_transition(300,"ease_out");
    let logo=row([mark,cnt]).with_key("logo").accessibility_hidden(true).with_style(ctx.component_style("logo",
        style().relative().width(u(6.5)).height(u(6.5)).items_center().justify_center().flex_shrink(false)
            .radius(theme_radius("sm")).background(if supplied==(){ink("fill")}else{ink("card")})
            .text_color(ink("text")).typography(if supplied==(){"initials"}else{"caption"})
            .isolated_opacity(true).opacity(if r.status=="waiting"{0.6}else{1.0})))
        .paint_transition(400,"ease_out",#{group_effects:true});
    let who=[label(ctx,"model",name,caption().text_color(ink("ink")).font_weight(600)
        .isolated_opacity(true).opacity(if r.status=="waiting"{0.6}else{1.0})).with_key("model")
        .paint_transition(400,"ease_out",#{group_effects:true})];
    if m.show_provider&&provider!=""{who.push(label(ctx,"provider",provider,caption().typography("metadata")
        .text_color(ink("quiet")).flex_basis(px(0)).flex_grow()).with_key("provider"));}
    let line=short_status(ctx,data,r,m,i);
    let people=column([row(who).with_style(style().min_width(px(0)).gap_x(sp(1.5)).items_baseline()),
        label(ctx,"attempt_status",line,caption().text_color(ink(if quiet{"muted"}else{tone(r.status)}))).with_key("attempt-status")])
        .with_style(style().min_width(px(0)).flex_basis(px(0)).flex_grow());
    let elapsed=duration(ctx,r.elapsed_ms);let charge=if r.cost_micros==(){""}else{money(ctx,p,r.cost_micros)};
    let timing=caption().typography("metadata");
    let bills=column([readout(ctx,"timing","timing",elapsed,timing.text_color(ink(if quiet{"quiet"}else{"ink"})),r.status=="trying"),
        readout(ctx,"cost","cost",charge,timing,r.status=="trying")]).with_style(style().items_end().flex_shrink(false));
    let body=column([row([people,bills]).with_style(style().min_width(px(0)).gap_x(sp(2.5)).items_center()),blocks(ctx,r,m,i)])
        .with_style(style().min_width(px(0)).flex_grow().flex_basis(px(0)).gap_y(u(1.75)));
    let content_style=style().min_width(px(0)).width(relative(1.0)).items_center().gap_x(sp(2.5)).isolated_opacity(true);
    let content=if r.status=="skipped"{ctx.component_style("attempt_skipped",content_style.opacity(0.5))}else{content_style.opacity(1.0)};
    let body=row([logo,body]).with_key("attempt-content").with_style(content).paint_transition(400,"ease_out",#{group_effects:true});
    let show=["attempt"];if r.status in ["trying","answered"]{show.push("attempt_"+r.status);}
    let frame=row([body]).with_key("attempt-body").accessibility_role("group").accessibility_label(name)
        .accessibility_description(line+" · "+elapsed+(if charge==""{""}else{" · "+charge})).tab_stop(true)
        .with_style(ctx.component_style(show,style().height(u(15)).width(relative(1.0)).min_width(px(0)).items_center()
            .padding_start(sp(2.5)).padding_end(sp(3)).radius(theme_radius("lg")).background(ink("card"))
            .focus_ring(px(2),offset_px(-2),theme_color("focus_ring")))).paint_transition(400,"ease_out");
    let fields=[#{label:tx(ctx,"model","Model"),value:name},
        #{label:tx(ctx,"time","Time"),value:elapsed+" / "+duration(ctx,r.timeout_ms)}];
    if charge!=""{fields.push(#{label:tx(ctx,"cost","Cost"),value:charge});}
    fields+=get(data,"details",[]);
    if r.status=="failed"{fields.push(#{label:tx(ctx,"why","Why"),value:line,error:true});}
    let title=if provider==""{name}else{provider};
    let tip=preview(ctx,title,get(data,"region",""),get(data,"provider_logo",mark),r.status,state_label(ctx,r.status),"",fields);
    floating(ctx,p,"attempt:"+data.id,name+" details",frame,tip,if i*2<p.attempts.len{"bottom"}else{"top"},"center",
        style().width(relative(1.0)).min_width(px(0)),element_ref("model:"+i.to_string()))
}
fn layer(ctx,part){
    ctx.component_style(part,style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)))
}
fn grid(ctx,b){
    if b.width<1.0||b.width>800.0||b.height<1.0||b.height>2200.0{return ();}
    canvas(fallback_dot_grid(b.width,b.height,ink("grid"))).with_key("dots").with_style(layer(ctx,"grid")).accessibility_hidden(true)
}
fn position(ctx,key,root){
    let p=ctx.element_bounds(key);if p==(){return ();}
    let b=p.layout;
    #{x:b.x-root.x,y:b.y-root.y,w:b.width,h:b.height,cy:b.y-root.y+b.height/2.0}
}
fn curve(a,b,y,z){
    let d=b-a;
    [path_move(a,y),path_cubic(b,z,a+d*0.55,y,a+d*0.45,z)]
}
fn wire(key,segs,status){
    let color=switch status{"trying"=>ink("active_edge"),"answered"=>ink("ok"),"failed"=>ink("error_edge"),
        "skipped"=>alpha(ink("rail"),0.5),_=>ink("rail")};
    let path=if status=="waiting"||status=="skipped"{fallback_dash_route(segs,0.012,0.030)}
        else if status=="failed"{fallback_dash_route(segs,0.035,0.035)}else{segs};
    canvas_stroke_path(key,path,if status=="answered"{1.5}else{1.25},color).stroke_cap("round")
}
fn paths(ctx,p,m,b){
    let request=position(ctx,"request",b);let response=position(ctx,"response",b);
    if request==()||response==(){return [];}
    let rtl=ctx.text_direction()=="rtl";let dir=if rtl{-1.0}else{1.0};
    let incoming=[];let outgoing=[];let heat=();
    for i in 0..p.attempts.len{
        let row=position(ctx,"model:"+i.to_string(),b);if row==(){continue;}
        let key=i.to_string();let r=m.data.rows[i];
        let near=if rtl{row.x+row.w}else{row.x};let far=if rtl{row.x}else{row.x+row.w};
        let start=if rtl{request.x}else{request.x+request.w};
        let finish=if rtl{response.x+response.w}else{response.x};
        let segs=curve(start,near,request.cy,row.cy);
        if m.narrow{
            let spine=near-dir*20.0;
            let y0=request.y+request.h;
            let bend=max(y0,row.cy-10.0);
            segs=[path_move(spine,y0),path_line(spine,bend),path_quadratic(spine+dir*10.0,row.cy,spine,row.cy),path_line(near,row.cy)];
        }
        incoming.push(wire("in:"+key,segs,r.status));
        if r.status=="trying"{heat=segs;}
        if !m.narrow{outgoing.push(wire("out:"+key,curve(far,finish,row.cy,response.cy),if m.data.winner==i{"answered"}else{"waiting"}));}
    }
    let out=[canvas(canvas_scene(incoming+outgoing).view_box(0,0,b.width,b.height)).with_key("wires")
        .with_style(layer(ctx,"wires")).accessibility_hidden(true)];
    if heat!=()&&ctx.motion_preference()=="normal"{
        let route=canvas_stroke_path("attempt-light",heat,1.0,rgba(0x00000000));
        let scene=canvas_scene([route]).view_box(0,0,b.width,b.height).path_pulse("attempt-light",2.6,ink("active"),false,false);
        out.push(canvas(scene).with_key("pulse:"+m.data.trying.to_string()).with_style(layer(ctx,"pulse")).accessibility_hidden(true)
            .motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:1400,easing:ease(),iterations:"infinite",intent:"decorative"})));
    }
    out
}
fn render_chain(ctx,p){
    if p.run==""||utf8_byte_count(p.run)>96||utf8_byte_count(get(p,"currency","$"))>12{throw "FallbackChain needs bounded identity and currency prefix";}
    for a in p.attempts{
        if a.model==""||utf8_byte_count(a.model)>512||utf8_byte_count(get(a,"reason",""))>4096||utf8_byte_count(get(a,"note",""))>4096{throw "FallbackChain text must be bounded";}
    }
    let shell=element_ref("fallback-chain");let stage=element_ref("flow");
    let data=fallback_chain_summary(p.attempts,p.now_ms);
    let width_class=ctx.element_width_class(shell,[520,680]);
    let next=-1;
    if data.trying!=(){for i in 0..p.attempts.len{if i>data.trying&&data.rows[i].status=="waiting"{next=i;break;}}}
    let m=#{data:data,narrow:width_class==0,show_provider:width_class==2,metrics:block_metrics(ctx),next_wait:next};
    let attempts=[];for i in 0..p.attempts.len{attempts.push(model(ctx,p,m,i));}
    if attempts.len==0{attempts.push(label(ctx,"empty",tx(ctx,"empty","No models in this chain"),subtle().padding(sp(3))));}
    let ls=style().min_width(px(0)).gap_y(sp(2.5));
    if m.narrow{ls=ls.width(relative(1.0)).padding_start(u(10));}else{ls=ls.flex_grow().flex_basis(px(0));}
    let list=column(attempts).with_key("models").accessibility_role("list").accessibility_label(tx(ctx,"models","Models in fallback order"))
        .with_style(ctx.component_style(if m.narrow{["attempts","attempts_tree"]}else{["attempts"]},ls));
    let children=[];
    if p.attempts.len>0{children.push(measure_blocks(ctx));}
    let b=ctx.element_bounds(stage);
    if b!=()&&b.layout.width>0.0&&b.layout.height>0.0{
        let dots=grid(ctx,b.layout);if dots!=(){children.push(dots);}
        children+=paths(ctx,p,m,b.layout);
    }
    children+=[endpoint(ctx,p,m,"request"),list,endpoint(ctx,p,m,"response")];
    let base=if m.narrow{style().flex_col().items_start().gap_y(sp(3))}else{style().flex_row().items_center().gap_x(u(12))};
    let flow=box(children).with_key("flow").with_ref(stage).accessibility_role("group").accessibility_label(tx(ctx,"diagram","Fallback route"))
        .with_style(ctx.component_style(if m.narrow{["flow","tree"]}else{["flow"]},base.relative().width(relative(1.0)).min_width(px(0))
            .padding(sp(4)).radius(theme_radius("lg")).background(ink("area"))));
    column([head(ctx,p,m),flow]).with_key("fallback-chain").with_ref(shell)
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(u(187)).min_width(px(0)).gap_y(sp(1)).padding(sp(1))
            .background(ink("tray")).radius(theme_radius("lg"))))
}
