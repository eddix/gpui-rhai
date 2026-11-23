/* gpui-rhai
{"id":"patterns/billing_schedule","export":"BillingSchedule","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/input","components/number_input","components/date_picker","components/combobox"],"capabilities":{},"assets":[],"tokens":["billing_schedule.card","billing_schedule.inset","billing_schedule.line","billing_schedule.tray","billing_schedule.ink","billing_schedule.text","billing_schedule.muted","billing_schedule.tint","border","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],"environment":[]}
*/
// Build a phase and preview the first invoice and phase value without executing billing.
// Example: BillingSchedule(#{key:"phase",schedule:"build",draft:#{phase:"Build-out",start:"2026-01-01",duration:"milestone",lines:[]}})
import "components/input" as input;
import "components/number_input" as number;
import "components/date_picker" as date;
import "components/combobox" as combo;

define_component(#{
    metadata:#{id:"patterns/billing_schedule","export":"BillingSchedule",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/input","components/number_input","components/date_picker","components/combobox"],capabilities:#{},assets:[],tokens:["billing_schedule.card","billing_schedule.inset","billing_schedule.line","billing_schedule.tray","billing_schedule.ink","billing_schedule.text","billing_schedule.muted","billing_schedule.tint","border","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable card instance."},
        schedule:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Phase identity. A new schedule clears the local draft."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted Host revision echoed in edits."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional visible heading."},
        currency:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Display prefix, defaults to $. Amounts always use two decimal places."},
        controlled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Only the caller's draft is authoritative; an edit merely emits a proposal."},
        draft:#{schema:#{type:"object",allow_unknown:false,fields:#{
            phase:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Editable phase name."},
            start:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Start as YYYY-MM-DD."},
            duration:#{schema:#{type:"string",allowed:["milestone","1m","3m","6m","12m"]},required:true,sensitive:false,doc:"Phase length or milestone."},
            lines:#{schema:#{type:"array",max_items:64,items:#{type:"one_of",variants:[
                #{type:"object",allow_unknown:false,fields:#{
                    kind:#{schema:#{type:"string",allowed:["product"]},required:true,sensitive:false,doc:"Product discriminant."},
                    id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable line ID."},name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Visible product name."},
                    repeat:#{schema:#{type:"string",allowed:["once","monthly","quarterly","yearly"]},required:true,sensitive:false,doc:"Product cadence."},
                    timing:#{schema:#{type:"string",allowed:["advance","arrears"]},required:true,sensitive:false,doc:"Only advance products enter the first invoice."},
                    price_minor:#{schema:#{type:"integer",min:0,max:1000000000},required:true,sensitive:true,doc:"Price per repeat, in integer minor units."}}},
                #{type:"object",allow_unknown:false,fields:#{
                    kind:#{schema:#{type:"string",allowed:["discount"]},required:true,sensitive:false,doc:"Discount discriminant."},
                    id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable line ID."},name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Visible discount name."},
                    discount_bps:#{schema:#{type:"integer",min:0,max:10000},required:true,sensitive:false,doc:"Discount in basis points; 10,000 is 100%."}}},
                #{type:"object",allow_unknown:false,fields:#{
                    kind:#{schema:#{type:"string",allowed:["minimum"]},required:true,sensitive:false,doc:"Minimum commitment discriminant."},
                    id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable line ID."},name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Visible minimum name."},
                    price_minor:#{schema:#{type:"integer",min:0,max:1000000000},required:true,sensitive:true,doc:"Phase total floor in integer minor units."}}}
            ]}},required:true,sensitive:true,doc:"Stable lines; at most one discount and one minimum."}
        }},required:true,sensitive:true,doc:"Starting local draft or current controlled snapshot."},
        catalog:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable catalog ID."},name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Product name."},
            repeat:#{schema:#{type:"string",allowed:["once","monthly","quarterly","yearly"]},required:true,sensitive:false,doc:"Initial cadence for a new line."},
            price_minor:#{schema:#{type:"integer",min:0,max:1000000000},required:true,sensitive:true,doc:"Initial price in integer minor units."}
        }}},"default":#{type:"array",value:[]},required:false,sensitive:true,doc:"Products available to add in the order supplied."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the candidate draft only after an actual edit, not on mount."}
    },
    state:#{fields:#{local:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
        approved:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
    events:#{change:#{payload:#{type:"object",allow_unknown:false,fields:#{
        schedule:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
        action:#{schema:#{type:"string"},required:true,sensitive:false},line_id:#{schema:#{type:"string"},required:true,sensitive:false},
        draft:#{schema:#{type:"ui_value"},required:true,sensitive:true}
    }},doc:"The proposed phase, start, duration and lines, with the affected line ID when applicable."}},
    slots:#{},parts:["root","heading","title","card","summary","phase","phase_root","phase_field","phase_dot","range","date","end","fields","fields_compact","field_caption",
        "field","caption","date_trigger","trigger","duration","inline","timing","action","trigger_icon","menu","option","head_row","line","compact",
        "name","line_kind","repeat_column","price_column","price","price_field","price_root","price_input","lines","remove","empty","actions","totals",
        "summary_label","summary_amount","min_tag"],effects:["sync_plan"]},render:Fn("render_schedule")
});

fn BillingSchedule(p){render_component("patterns/billing_schedule",p)}
fn get(m,k,d){if k in m&&m[k]!=(){m[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn u(n){theme_length("space.unit").times(n)}
fn tx(ctx,key,fallback){ctx.t_or("billing_schedule."+key,fallback)}
fn color(k){let token="billing_schedule."+k;theme_color(token)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn subtle(){style().typography("caption").text_color(color("muted")).min_width(px(0))}
fn label(ctx,part,words,base){text(words).with_style(ctx.component_style(part,base))}
fn icon(ctx,d,edge){
    svg("<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.65' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
        .with_style(ctx.component_style("trigger_icon",style().width(px(edge)).height(px(edge)).flex_shrink(false))).accessibility_hidden(true)
}
fn money(ctx,prefix,n){
    prefix+ctx.format_number(n.to_float()/100.0,#{min_fraction_digits:2,max_fraction_digits:2})
}
fn duration_title(ctx,k){switch k{
    "1m"=>tx(ctx,"month","1 month"),"3m"=>tx(ctx,"quarter","3 months"),"6m"=>tx(ctx,"half","6 months"),
    "12m"=>tx(ctx,"year","12 months"),_=>tx(ctx,"milestone","Milestone")
}}
fn repeat_title(ctx,k){switch k{
    "monthly"=>tx(ctx,"monthly","Monthly"),"quarterly"=>tx(ctx,"quarterly","Quarterly"),
    "yearly"=>tx(ctx,"yearly","Yearly"),_=>tx(ctx,"once","Once")
}}
fn timing_title(ctx,k){if k=="arrears"{tx(ctx,"arrears","Billed in arrears")}else{tx(ctx,"advance","Billed in advance")}}
fn options(ctx,k){
    let out=[];
    if k=="duration"{for d in ["milestone","1m","3m","6m","12m"]{out.push(#{value:d,label:duration_title(ctx,d)});}}
    else if k=="repeat"{for d in ["once","monthly","quarterly","yearly"]{out.push(#{value:d,label:repeat_title(ctx,d)});}}
    else{for d in ["advance","arrears"]{out.push(#{value:d,label:timing_title(ctx,d)});}}
    out
}
fn local(ctx,id){
    let state=ctx.get_state("local");
    if state!=()&&state.id==id{state}else{#{id:id,draft:(),menu:"",next:0}}
}
fn sync_plan(ctx,data){
    let old=ctx.get_state("local");
    if old!=()&&old.id!=data.id{ctx.set_state("local",local(ctx,data.id));}
    ctx.set_state("approved",data);
}
fn idle(ctx,data){}
fn current(state,ticket,source){if ticket.controlled||state.draft==(){source}else{state.draft}}
fn edit_current(ctx,state,ticket){
    let a=ctx.get_state("approved");
    if a==()||a.id!=ticket.id||a.revision!=ticket.revision{return ();}
    current(state,ticket,a.draft)
}
fn select_menu(id,key,ctx,open){
    let s=local(ctx,id);
    if open{s.menu=key;}else if s.menu==key{s.menu="";}else{return;}
    ctx.set_state("local",s);
}
fn commit(ticket,state,next,action,line_id,ctx){
    billing_schedule_projection(next.start,next.duration,next.lines);
    if !ticket.controlled{state.draft=next;}
    ctx.set_state("local",state);
    ctx.emit("change",#{schedule:ticket.id,revision:ticket.revision,action:action,line_id:line_id,draft:next});
}
fn patch(ticket,operation,id,ctx,value){
    let state=local(ctx,ticket.id);let d=edit_current(ctx,state,ticket);
    if d==()||value==(){return;}
    if operation in ["phase","start","duration"]{
        if d[operation]==value{return;}
        if operation=="phase"&&utf8_byte_count(value)>512{return;}
        d[operation]=value;
        commit(ticket,state,d,operation,"",ctx);return;
    }
    let at=-1;
    for i in 0..d.lines.len{if d.lines[i].id==id{at=i;break;}}
    if at<0{return;}
    if operation=="remove"{d.lines.remove(at);}
    else{
        let n=value;
        if operation in ["price_minor","discount_bps"]{
            let scaled=value.to_float()*100.0;
            let units=scaled.round();
            // Reject finer precision instead of silently quoting another amount.
            if abs(scaled-units)>0.000001{return;}
            n=units.to_int();
        }
        if d.lines[at][operation]==n{return;}
        d.lines[at][operation]=n;
    }
    commit(ticket,state,d,operation,id,ctx);
}
fn choose(ticket,operation,id,ctx,values){
    if values.len==1{patch(ticket,operation,id,ctx,values[0]);}
}
fn add(ticket,entry,ctx){
    let s=local(ctx,ticket.id);let d=edit_current(ctx,s,ticket);
    if d==()||d.lines.len>=64{return;}
    if entry.kind!="product"{for line in d.lines{if line.kind==entry.kind{return;}}}
    let id="";let ids=[];for old in d.lines{ids.push(old.id);}
    while id==""||id in ids{s.next+=1;id="local:"+s.next.to_string();}
    entry.id=id;
    if entry.kind=="product"{
        let at=d.lines.len;
        for i in 0..d.lines.len{if d.lines[i].kind!="product"{at=i;break;}}
        d.lines.insert(at,entry);
    }else{d.lines.push(entry);}
    s.menu="";
    commit(ticket,s,d,"add_"+entry.kind,id,ctx);
}
fn pick_product(ticket,lookup,ctx,values){
    if values.len==1&&values[0] in lookup{
        let e=lookup[values[0]];
        add(ticket,#{kind:"product",name:e.name,repeat:e.repeat,timing:"advance",price_minor:e.price_minor},ctx);
    }
}
fn pick_adjustment(ticket,entry,ctx,e){add(ticket,entry,ctx);handled()}
fn remove(ticket,id,ctx,e){patch(ticket,"remove",id,ctx,true);handled()}
fn trigger_recipe(ctx,part){
    let base=style().min_width(px(0)).items_center().gap_x(sp(1)).border(px(0))
        .typography("caption").radius(theme_radius("sm")).cursor_pointer()
        .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-1),theme_color("focus_ring"));
    if part=="duration"{base=base.width(relative(1.0)).justify_between().min_height(u(10)).padding_x(sp(3)).typography("body");}
    else{base=base.min_height(u(6)).padding_x(sp(1.5));}
    if part=="inline"||part=="timing"{
        base=base.background(rgba(0x00000000)).shadows([]).focus(style().shadows([]));
    }
    ctx.component_style(["trigger",part],base)
}
fn base_trigger(ctx,words,glyph,with_chevron,recipe){
    let children=[];
    if glyph!=""{children.push(icon(ctx,glyph,14.0));}
    children.push(text(words).with_style(style().min_width(px(0)).whitespace_nowrap().text_ellipsis()));
    if with_chevron{children.push(icon(ctx,"m6 9 6 6 6-6",12.0));}
    row(children).with_style(recipe).paint_transition(160,"ease_out")
}
fn menu(ctx,m,key,name,part,words,glyph,selected,choices,action,above,empty){
    let recipe=trigger_recipe(ctx,part);
    let trigger=base_trigger(ctx,words,glyph,part!="action",recipe);
    let border=color("line");
    combo::Combobox(#{key:key,label:name,trigger:trigger,options:choices,selected:selected,query:"",open:m.local.menu==key,
        width:if part=="action"{px(270)}else{px(190)},placement:if above{"top"}else{"bottom"},max_visible:8,
        disabled:empty,on_open_change:Fn("select_menu").curry(m.ticket.id,key),on_change:action,
        part_styles:#{root:style().min_width(px(0)),trigger:recipe,panel:ctx.component_style("menu",style().background(color("card")).border_color(border)),
            option:ctx.component_style("option",style().min_width(px(0)))}})
}
fn edit_name(ctx,m){
    let ref_style=ctx.component_style("phase",style().font_weight(500).min_width(px(0)).typography("body"));
    input::Input(#{key:"phase-name",label:tx(ctx,"phase","Phase name"),value:m.draft.phase,
        appearance:"embedded",size:"xs",intrinsic_width:true,
        on_change:Fn("patch").curry(m.ticket,"phase",""),part_styles:#{
            root:ctx.component_style(["phase_root","phase_field"],style().max_width(relative(1.0)).min_width(px(0)).flex_shrink(true)
                .min_height(u(6)).padding_x(sp(1)).radius(theme_radius("sm")).border(px(0))
                .hover(style().background(theme_color("surface_hover")))),input:ref_style}})
}
fn header(ctx,m,q){
    let start=ctx.format_date(m.draft.start,"medium");
    let end=if m.draft.duration=="milestone"{duration_title(ctx,"milestone")}
        else if q.end_date==(){tx(ctx,"no_end","—")}else{ctx.format_date(q.end_date,"medium")};
    let arrow=if ctx.text_direction()=="rtl"{"M19 12H5m0 0 5-5m-5 5 5 5"}else{"M5 12h14m0 0-5-5m5 5-5 5"};
    let dates=[];
    if m.clean{dates.push(label(ctx,"phase_dot","·",subtle()).with_key("separator").accessibility_hidden(true));}
    dates.push(label(ctx,"date",start,subtle()).with_key("start"));
    dates.push(icon(ctx,arrow,if m.clean{14.0}else{15.0}));
    dates.push(label(ctx,"end",end,subtle()).with_key(end).motion(motion_transition("opacity",0.0,1.0,#{duration_ms:260,intent:"decorative"})));
    let range=row(dates).with_key("range").with_style(ctx.component_style("range",style()
        .items_center().gap(sp(1.5)).min_width(px(0)).max_width(relative(1.0)).flex_shrink(false).flex_wrap()));
    let geometry=style().min_width(px(0)).gap_x(sp(2)).gap_y(sp(0.5)).items_center();
    // The line itself decides when the date group needs a second row. Keep
    // the same native Input and selected range at either card width.
    row([edit_name(ctx,m),range]).with_key("summary").with_style(ctx.component_style("summary",
        if m.compact{geometry.flex_wrap()}else{geometry.flex_nowrap()}))
}
fn dates(ctx,m){
    let cap=tx(ctx,"start","Start date");let dur=tx(ctx,"duration","Duration");
    let date=date::DatePicker(#{key:"start-date",label:cap,value:m.draft.start,width:relative(1.0),size:"sm",
        display_style:"medium",on_change:Fn("patch").curry(m.ticket,"start",""),
        part_styles:#{root:style().min_width(px(0)).width(relative(1.0)),trigger:ctx.component_style("date_trigger",style().width(relative(1.0)))}});
    let duration=menu(ctx,m,"duration",dur,"duration",duration_title(ctx,m.draft.duration),"",[m.draft.duration],options(ctx,"duration"),
        Fn("choose").curry(m.ticket,"duration",""),false,false);
    let base=ctx.component_style("field",style().min_width(px(0)).flex_grow().flex_basis(px(0)).gap_y(sp(1.5)));
    let fields=[column([label(ctx,["caption","field_caption"],cap,subtle()),date]).with_key("start").with_style(base),
        column([label(ctx,["caption","field_caption"],dur,subtle()),duration]).with_key("duration").with_style(base)];
    let frame=if m.compact{column(fields)}else{row(fields)};
    frame.with_key("fields").with_style(ctx.component_style(if m.compact{["fields","fields_compact"]}else{["fields"]},
        style().min_width(px(0)).gap(sp(2.5)).padding(sp(2.5)).background(color("inset")).radius(theme_radius("lg"))))
}
fn tap(n,fnptr,enabled){
    n=n.accessibility_role("button").disabled(!enabled).tab_stop(enabled);
    if enabled{n=n.on_click(fnptr).on_key_value("plain+enter",fnptr,()).on_key_value("plain+space",fnptr,());}
    n
}
fn number_cell(ctx,m,l){
    let percent=l.kind=="discount";
    let n=if percent{l.discount_bps}else{l.price_minor};
    let name=if percent{tx(ctx,"discount_percent","Discount percent")}else if l.kind=="minimum"{tx(ctx,"minimum","Minimum commitment")}
        else{l.name+" "+tx(ctx,"price","price")};
    let amount=number::NumberInput(#{key:"number:"+l.id,label:name,value:n.to_float()/100.0,min:0,max:if percent{100}else{10000000},
        step:0.01,prefix:if percent{()}else{text(m.currency)},unit:if percent{"%"}else{()},size:"xs",
        fraction_digits:2,minimum_fraction_digits:if percent{0}else{2},on_change:Fn("patch").curry(m.ticket,if percent{"discount_bps"}else{"price_minor"},l.id),
        part_styles:#{label:style().hidden(),steps:style().hidden(),root:ctx.component_style("price_root",style().gap(px(0)).min_width(px(0))),
            field:ctx.component_style("price_field",style().min_height(u(6)).padding_x(sp(1)).border(px(0)).background(color("card")).shadows([])
                .hover(style().background(theme_color("surface_hover")))),
            input:ctx.component_style("price_input",style().typography("caption").font_weight(500))}});
    let unit=if percent{tx(ctx,"off","off")}else if l.kind=="minimum"{tx(ctx,"floor","minimum")}
        else{"/ "+switch l.repeat{"monthly"=>tx(ctx,"unit_month","month"),"quarterly"=>tx(ctx,"unit_quarter","quarter"),
            "yearly"=>tx(ctx,"unit_year","year"),_=>tx(ctx,"unit_once","once")}};
    let remove=tap(row([icon(ctx,"m7 7 10 10M17 7 7 17",14.0)]).with_key("remove").accessibility_label(tx(ctx,"remove","Remove")+" "+l.name)
        .with_style(ctx.component_style("remove",style().width(u(6.5)).height(u(6.5)).items_center().justify_center().flex_shrink(false)
            .cursor_pointer().radius(theme_radius("sm")).hover(style().background(theme_color("surface_hover")))
            .focus_ring(px(2),offset_px(-1),theme_color("focus_ring")))),Fn("remove").curry(m.ticket,l.id),true);
    let line=row([amount,label(ctx,"caption",unit,subtle().whitespace_nowrap().text_ellipsis()),remove])
        .with_style(ctx.component_style("price",(if m.compact{style().width(relative(1.0)).justify_start()}else{style().justify_end()})
            .min_width(px(0)).items_center().gap_x(sp(1))));
    let children=[line];
    if l.kind=="product"{
        children.push(menu(ctx,m,"timing:"+l.id,l.name+" "+tx(ctx,"billing_timing","billing timing"),"timing",timing_title(ctx,l.timing),"",
            [l.timing],options(ctx,"timing"),Fn("choose").curry(m.ticket,"timing",l.id),false,false));
    }
    let base=(if m.compact{style().width(relative(1.0)).items_start()}else{style().width(u(51)).items_end()}).min_width(px(0)).flex_shrink(false);
    column(children).with_key("price").with_style(ctx.component_style("price_column",base))
}
fn schedule_row(ctx,m,l){
    let name=[label(ctx,"name",l.name,style().typography("body").min_width(px(0)).whitespace_nowrap().text_ellipsis().font_weight(500))];
    if l.kind!="product"{
        let path=if l.kind=="discount"{"M7 17 17 7M8 8h.01M16 16h.01"}else{"M5 19h14M8 15h8M10 11h4"};
        name.insert(0,row([icon(ctx,path,14.0)]).with_key("kind").with_style(ctx.component_style("line_kind",style().width(u(5)).height(u(5))
            .radius(theme_radius("sm")).items_center().justify_center().background(color("inset")))));
    }
    let leading=row(name).with_key("name").with_style(style().min_width(px(0)).flex_grow().flex_basis(px(0)).items_center().gap_x(sp(1.5)));
    let repeats=if l.kind=="product"{
        menu(ctx,m,"repeat:"+l.id,l.name+" "+tx(ctx,"repeats","repeats"),"inline",repeat_title(ctx,l.repeat),
            "M17 4l3 3-3 3M20 7H8a4 4 0 0 0-4 4M7 20l-3-3 3-3M4 17h12a4 4 0 0 0 4-4",
            [l.repeat],options(ctx,"repeat"),Fn("choose").curry(m.ticket,"repeat",l.id),false,false)
    }else{label(ctx,"caption",if l.kind=="discount"{tx(ctx,"whole","Whole phase")}else{tx(ctx,"per","Per phase")},subtle())};
    let repeat_column=row([repeats]).with_key("cadence").with_style(ctx.component_style("repeat_column",style().min_width(px(0)).width(u(26)).flex_shrink(false)));
    let rail=style().width(relative(1.0)).min_width(px(0)).items_center().gap_x(sp(2.5)).gap_y(sp(0.5)).min_height(u(13))
        .border_bottom(px(1)).border_color(color("line"));
    if m.compact{rail=rail.grid_cols(24).gap_x(px(0));}
    else{rail=rail.flex_row();}
    let row_style=ctx.component_style(if m.compact{["line","compact"]}else{["line"]},rail);
    let price=number_cell(ctx,m,l);
    let contents=if m.compact{[
        leading.with_style(style().grid_cell(1,1).col_span(16)),
        repeat_column.with_style(style().grid_cell(17,1).col_span(8).width(relative(1.0))),
        price.with_style(style().grid_cell(1,2).col_span(24))]}else{[leading,repeat_column,price]};
    // Editors and overlays are not paint snapshots. The exit lease keeps
    // only read-only text, never a detached native input or callback.
    if ctx.motion_preference()=="normal"{
        let digits=if l.kind=="discount"{ctx.format_number(l.discount_bps.to_float()/100.0,#{max_fraction_digits:2})+"%"}
            else{money(ctx,m.currency,l.price_minor)};
        let recurrence=if l.kind=="product"{repeat_title(ctx,l.repeat)}else{""};
        contents.push(row([label(ctx,"name",l.name,style().typography("body").min_width(px(0)).flex_grow().text_ellipsis().whitespace_nowrap()),
            label(ctx,"caption",recurrence,subtle()),label(ctx,"summary_amount",digits,style().typography("caption"))]).with_key("exit-paint")
            .accessibility_hidden(true).with_style(row_style.absolute().top(px(0)).inset_start(px(0)).width(relative(1.0))
                .height(relative(1.0)).background(color("card")).opacity(0.0))
            .exit_motion(motion_transition("opacity",1.0,0.0,#{duration_ms:240,easing:ease(),intent:"decorative"})));
    }
    box(contents).with_key("line:"+l.id).accessibility_role("row").accessibility_label(l.name)
        .with_style(row_style.relative())
        .enter_motion(motion_transition("opacity",0.0,1.0,#{duration_ms:320,easing:ease(),intent:"decorative"}))
}
fn actions(ctx,p,m){
    let list=[];let lookup=#{};
    for c in p.catalog{
        if c.id==""||utf8_byte_count(c.id)>96||c.id in lookup||c.name==""||utf8_byte_count(c.name)>512{throw "BillingSchedule catalog needs bounded unique IDs and names";}
        lookup[c.id]=c;
        list.push(#{value:c.id,label:c.name+" · "+money(ctx,m.currency,c.price_minor)+" · "+repeat_title(ctx,c.repeat)});
    }
    let full=m.draft.lines.len>=64;
    let available=[];
    for l in m.draft.lines{available.push(l.kind);}
    let product=tx(ctx,"add_product","Add product");
    let opts=[menu(ctx,m,"product",product,"action",product,"M12 5v14M5 12h14",[],list,
        Fn("pick_product").curry(m.ticket,lookup),true,full||list.len==0)];
    for entry in [#{kind:"discount",name:tx(ctx,"discount","Discount"),discount_bps:1000},
        #{kind:"minimum",name:tx(ctx,"minimum","Minimum commitment"),price_minor:1000000}]{
        let cap=if entry.kind=="discount"{tx(ctx,"add_discount","Add discount")}else{tx(ctx,"add_minimum","Add minimum")};
        let glyph=if entry.kind=="discount"{"M7 17 17 7M8 8h.01M16 16h.01"}else{"M5 19h14M8 15h8M10 11h4"};
        opts.push(tap(base_trigger(ctx,cap,glyph,false,trigger_recipe(ctx,"action")).with_key(entry.kind).accessibility_label(cap),
            Fn("pick_adjustment").curry(m.ticket,entry),!full&&!(entry.kind in available)));
    }
    row(opts).with_key("actions").with_style(ctx.component_style("actions",style().width(relative(1.0)).min_width(px(0)).justify_between().gap(sp(1.5)).flex_wrap()))
}
fn measure(ctx,m,part,words,n,minimum){
    let amount=money(ctx,m.currency,n);
    let children=[label(ctx,"summary_label",words,subtle()),label(ctx,"summary_amount",amount,style().typography("body").font_weight(500).font_feature("tnum",1))];
    if minimum{children.push(label(ctx,"min_tag",tx(ctx,"applies","minimum applies"),subtle().text_color(color("tint")).radius(theme_radius("sm")).padding_x(sp(1.25))));}
    row(children).with_key(part).accessibility_role("status").accessibility_label(words+" "+amount+(if minimum{" · "+tx(ctx,"applies","minimum applies")}else{""}))
        .with_style(style().items_center().gap(sp(1.5)).min_width(px(0)).flex_wrap())
}
fn finish(ctx,m,q){
    let cap=tx(ctx,"phase_total","Phase total")+(if m.draft.duration=="milestone"{""}else{" ("+duration_title(ctx,m.draft.duration)+")"});
    let a=[measure(ctx,m,"invoice",tx(ctx,"invoice","First invoice"),q.first_minor,false),measure(ctx,m,"total",cap,q.total_minor,q.minimum_applies)];
    let frame=if m.compact{column(a)}else{row(a)};
    let base=(if m.compact{style().flex_col().items_start()}else{style().flex_row().justify_between().flex_wrap()})
        .min_width(px(0)).gap(sp(2));
    frame.with_key("totals").with_style(ctx.component_style("totals",base))
}
// Measure the minimum comfortable wide rail using the same design lengths as
// its fixed columns. A clipped zero-size shell contributes no layout or scroll
// extent; no controls, text, or focus targets are duplicated.
fn wide_rail_measure(reference){
    let budget=box([]).with_key("wide-rail-width").with_ref(reference)
        .with_style(style().width(u(125)).height(px(0)).flex_shrink(false));
    box([budget]).with_key("wide-rail-measure").accessibility_hidden(true)
        .with_style(style().absolute().inset_start(px(0)).top(px(0)).width(px(0)).height(px(0))
            .overflow_hidden().invisible());
}
fn render_schedule(ctx,p){
    if p.schedule==""||utf8_byte_count(p.schedule)>96||utf8_byte_count(get(p,"currency","$"))>12{throw "BillingSchedule requires a bounded identity and currency prefix";}
    let root=element_ref("billing-schedule");
    let local=local(ctx,p.schedule);
    let ticket=#{id:p.schedule,revision:p.revision,controlled:p.controlled};
    let d=current(local,ticket,p.draft);
    if utf8_byte_count(d.phase)>512{throw "BillingSchedule phase name is too long";}
    let q=billing_schedule_projection(d.start,d.duration,d.lines);
    let wide_ref=element_ref("wide-rail-width");
    let measured=ctx.element_size(wide_ref);
    let wide=measured!=() && measured.width>0.0 && ctx.element_width_class(root,[measured.width])==1;
    let theme=ctx.theme_variant();
    let m=#{draft:d,ticket:ticket,local:local,compact:!wide,clean:theme!=()&&theme.family=="Clean",currency:get(p,"currency","$")};
    effect("sync_plan",#{id:p.schedule,revision:p.revision,draft:p.draft},Fn("sync_plan"),Fn("idle"));
    let rows=[];for l in d.lines{rows.push(schedule_row(ctx,m,l));}
    if rows.len==0{rows.push(label(ctx,"empty",tx(ctx,"empty","No products yet – add one below."),subtle().padding(sp(3))));}
    let table=[];
    if !m.compact{
        table.push(row([label(ctx,"caption",tx(ctx,"name","Name"),subtle().flex_grow()),label(ctx,"caption",tx(ctx,"repeats","Repeats"),subtle().width(u(26))),
            label(ctx,"caption",tx(ctx,"price_head","Price"),subtle().width(u(51)).text_end().padding_end(sp(6)))])
            .with_key("columns").with_style(ctx.component_style("head_row",style().min_height(u(10)).items_center().gap_x(sp(2.5))
                .border_bottom(px(1)).border_color(color("line")))));
    }
    table.push(column(rows).with_key("lines").with_style(ctx.component_style("lines",style().min_width(px(0)).overflow_hidden()))
        .layout_size_motion(d.lines.len.to_string(),"height",320,ease()));
    let title=get(p,"title",tx(ctx,"title","Create a billing schedule"));
    let heading=column([label(ctx,"title",title,style().typography("body").font_weight(500))]).with_key("title")
        .with_style(ctx.component_style("heading",style().min_width(px(0))));
    let sections=column([header(ctx,m,q),dates(ctx,m),column(table).with_key("table").accessibility_role("table").accessibility_label(tx(ctx,"lines","Billing lines")),
        actions(ctx,p,m)]).with_key("details").with_style(ctx.component_style("card",style().min_width(px(0)).background(color("card"))
            .radius(theme_radius("lg")).gap_y(sp(1.5)).padding(sp(3))));
    column([heading,sections,finish(ctx,m,q),wide_rail_measure(wide_ref)]).with_key("billing-schedule").with_ref(root)
        .with_style(ctx.component_style("root",style().relative().width(relative(1.0)).max_width(px(560)).min_width(px(0)).radius(theme_radius("lg"))
            .padding(sp(1.5)).background(color("tray"))))
}
