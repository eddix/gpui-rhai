/* gpui-rhai
{"id":"patterns/tool_call_card","export":"ToolCallCard","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/code_viewer","components/spinner","patterns/agent_tree"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text.danger","text.success","text_muted","text_primary","tool_call.code","tool_call.error","tool_call.error_border","tool_call.tile","typography.body","typography.caption","typography.code"],"environment":[]}
*/
// One read-only tool call with status, arguments, result and optional retry.
// Example: ToolCallCard(#{key:"render",session:"inspect",revision:0,entry:#{id:"call/a",name:"read_document",status:"success",result:#{pages:2}}})
import "components/button" as btn;
import "components/code_viewer" as code;
import "components/spinner" as spin;
import "patterns/agent_tree" as ids;

define_component(#{
    metadata:#{id:"patterns/tool_call_card","export":"ToolCallCard",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","components/code_viewer","components/spinner","patterns/agent_tree"],capabilities:#{},assets:[],tokens:["border","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text.danger","text.success","text_muted","text_primary","tool_call.code","tool_call.error","tool_call.error_border","tool_call.tile","typography.body","typography.caption","typography.code"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable visual instance."},
        session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Host session identity."},
        revision:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Accepted snapshot revision."},
        entry:#{schema:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable call identity within this session."},name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Tool name. Presentation only; the component never executes it."},summary:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Short optional summary."},icon:#{schema:#{type:"optional",value:#{type:"string",allowed:["tool","search","database","file","web","terminal"]}},required:false,sensitive:false,doc:"Built-in glyph name."},status:#{schema:#{type:"string",allowed:["running","success","error"]},required:true,sensitive:false,doc:"Accepted execution status."},arguments:#{schema:#{type:"optional",value:#{type:"ui_value"}},required:false,sensitive:true,doc:"Arguments as structured JSON values, JSON text, or plain text."},result:#{schema:#{type:"optional",value:#{type:"ui_value"}},required:false,sensitive:true,doc:"Result as structured JSON values, JSON text, or plain text."},error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Host-supplied failure message."},started_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Epoch time in milliseconds for a running duration."},duration_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Accepted final duration in milliseconds."}}},required:true,sensitive:true,doc:"Accepted call snapshot."},
        icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional custom glyph in place of the built-in icon."},
        open:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Controlled disclosure when supplied."},
        default_open:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Initial local disclosure policy.","default":#{type:"bool",value:false}},
        open_on_error:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Open a failed call until the caller or reader makes an explicit choice.","default":#{type:"bool",value:true}},
        pending:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Retry is awaiting Host acknowledgement.","default":#{type:"bool",value:false}},
        now_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9007199254740991}},required:false,sensitive:false,doc:"Optional deterministic current epoch milliseconds; otherwise use the view's clock."},
        max_code_height:#{schema:#{type:"integer",min:64,max:480},required:false,sensitive:false,doc:"Logical-pixel cap for the initial code viewport.","default":#{type:"integer",value:220}},
        on_open_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Disclosure request and local notification."},
        on_retry:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Versioned retry intent, only for failures."},
        on_copy:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives identity and section after a clipboard request, without raw content."},
      },
      state:#{fields:#{
          disclosure:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
          sections:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
          copied:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
          clock:#{schema:#{type:"ui_value"},"default":#{type:"null"}}
      }},
      events:#{
          open_change:#{payload:#{type:"bool"},doc:"The requested expanded state."},
          retry:#{payload:#{type:"object",allow_unknown:false,fields:#{session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Session ID."},revision:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Accepted revision."},id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business call ID."}}},doc:"Request another attempt; no execution takes place in the component."},
          copy:#{payload:#{type:"object",allow_unknown:false,fields:#{session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Session ID."},revision:#{schema:#{type:"integer",min:0,max:9007199254740991},required:true,sensitive:false,doc:"Accepted revision."},id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business call ID."},section:#{schema:#{type:"string",allowed:["arguments","result"]},required:true,sensitive:false,doc:"Copied section; never clipboard contents."}}},doc:"Clipboard request notification without arguments or result."}
      },
      slots:#{},parts:["root","header","header_open","icon","name","summary","status","failure","chevron","body","section","label",
         "code","copy","copy_done","more","error","error_text","retry","placeholder","announcement"],
      timers:["clock","clear_copy"]
    },render:Fn("render_call")
});
fn ToolCallCard(p){render_component("patterns/tool_call_card",p)}
fn get(p,k,d){if k in p&&p[k]!=(){p[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn u(n){theme_length("space.unit").times(n)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn tr(ctx,key,label){ctx.t_or("tool_call."+key,label)}
fn tint(ctx,part,value,base){text(value).with_style(ctx.component_style(part,base.min_width(px(0))))}
fn hint(){style().typography("caption").text_color(theme_color("text_muted")).whitespace_nowrap().text_ellipsis()}
fn glyph(kind){
    let path=switch kind{
        "search"=>"M10.5 10.5l3 3M12 7a5 5 0 1 1-10 0 5 5 0 0 1 10 0z",
        "database"=>"M2 4c0-3 12-3 12 0v8c0 3-12 3-12 0z M2 4c0 3 12 3 12 0 M2 8c0 3 12 3 12 0",
        "file"=>"M4 2h5l3 3v9H4z M9 2v4h3M6 9h4M6 11h4",
        "web"=>"M8 2a6 6 0 1 0 0 12 6 6 0 0 0 0-12z M2 8h12M8 2c-3 3-3 9 0 12M8 2c3 3 3 9 0 12",
        "terminal"=>"M3 4l4 4-4 4m6 0h4",
        "okay"=>"M3 8l3 3 7-7",
        "failed"=>"M8 2a6 6 0 1 0 0 12 6 6 0 0 0 0-12z M8 4.5v4.3m0 2.3h.01",
        "chevron"=>"M4 6l4 4 4-4",
        "copy"=>"M6 6h7v8H6z M10 6V2H3v8h3",
        "retry"=>"M3 7a5 5 0 1 1 1.8 5.1 M3 3v4h4",
        _=>"M10 2a3.5 3.5 0 0 0-4.4 4.4L2.5 10a1.7 1.7 0 0 0 2.4 2.4l3.6-3.1A3.5 3.5 0 0 0 13 5l-2 2-2-2Z"
    };
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.4' stroke-linecap='round' stroke-linejoin='round'><path d='"+path+"'/></svg>")
        .accessibility_hidden(true).with_style(style().width(u(3.5)).height(u(3.5)).flex_shrink(false))
}
fn validate_call(c){
    if !ids::valid(c.id)||c.name==""||utf8_byte_count(c.name)>200||utf8_byte_count(get(c,"summary",""))>2048||
        utf8_byte_count(get(c,"error",""))>8192{throw "Invalid tool call identity or text size";}
}
fn tick(ctx,scope){
    let old=ctx.get_state("clock");
    let now=ctx.now();
    let born=if old!=()&&old.scope==scope{old.born}else{now};
    ctx.set_state("clock",#{scope:scope,born:born,now:now});
}
fn tool_clock(ctx,scope,injected,active){
    if injected!=(){return #{scope:scope,born:injected,now:injected};}
    let prior=ctx.get_state("clock");
    if active{interval("clock",200,false,Fn("tick"),scope);}
    if prior!=()&&prior.scope==scope{return prior;}
    let now=ctx.now();#{scope:scope,born:now,now:now}
}
fn elapsed(entry,clock){
    if entry.status=="running"{return max(0,clock.now-get(entry,"started_ms",clock.born));}
    get(entry,"duration_ms",())
}
fn duration(ctx,ms){
    if ms==(){return "";}
    if ms<1000{return ctx.t_or("tool_call.ms","{n}ms",#{n:ctx.format_number(ms)});}
    if ms<60000{return ctx.t_or("tool_call.seconds","{n}s",#{n:ctx.format_number(ms.to_float()/1000.0,#{max_fraction_digits:1})});}
    ctx.format_duration(ms)
}
fn change_open(scope,controlled,notify,shown,ctx,event){
    if !controlled{ctx.set_state("disclosure",#{scope:scope,open:!shown});}
    if notify{ctx.emit("open_change",!shown);}
    handled()
}
fn button(node,cb){
    node.tab_stop(true).accessibility_role("button").on_click(cb)
        .on_key_value("plain+enter",cb,()).on_key_value("plain+space",cb,())
}
fn chevron(ctx,open){
    let angle=if open{180.0}else{0.0};
    glyph("chevron").with_key("arrow").with_style(ctx.component_style("chevron",
        style().text_color(theme_color("text_muted")).transform(#{rotate:angle})))
        .motion(motion_transition("rotate",angle,angle,#{duration_ms:220,easing:ease(),intent:"decorative"}))
}
fn state_label(ctx,phase){
    switch phase{"running"=>tr(ctx,"running","Running"),"success"=>tr(ctx,"finished","Finished"),_=>tr(ctx,"failed","Failed")}
}
fn header(ctx,p,expanded,age,cb){
    let c=p.entry;
    let decoration=if get(p,"icon",())==(){glyph(get(c,"icon","tool"))}else{p.icon};
    let icon=row([decoration]).with_key("icon").accessibility_hidden(true).with_style(ctx.component_style("icon",
            style().width(u(6.5)).height(u(6.5)).items_center().justify_center().flex_shrink(false)
                .background(theme_color("tool_call.tile")).text_color(theme_color("text_primary")).radius(theme_radius("sm"))));
    let words=[tint(ctx,"name",c.name,style().typography("code").font_weight(500).max_width(relative(0.72))
        .text_ellipsis().whitespace_nowrap()).with_key("name")];
    let summary=get(c,"summary","");
    if summary!=""{words.push(tint(ctx,"summary",summary,hint().flex_grow()).with_key("summary"));}
    let mark=if c.status=="running"{
        spin::Spinner(#{key:"live",label:tr(ctx,"running","Running"),size:"xs"}).accessibility_hidden(true)
    }else{glyph(if c.status=="success"{"okay"}else{"failed"})
        .with_style(style().text_color(theme_color(if c.status=="success"{"text.success"}else{"text.danger"})))};
    let state=state_label(ctx,c.status);
    let visible=if c.status=="error"{state}else{duration(ctx,age)};
    let line=[mark];
    if visible!=""{line.push(tint(ctx,if c.status=="error"{["status","failure"]}else{["status"]},visible,hint().font_feature("tnum",1)).with_key("elapsed"));}
    let status=row(line).with_key("status").with_style(ctx.component_style(if c.status=="error"{["status","failure"]}else{["status"]},
        style().items_center().gap(sp(1.25)).flex_shrink(false)));
    let core=row(words).with_key("main").with_style(style().min_width(px(0)).flex_grow().flex_basis(px(0)).items_center().gap(sp(1.5)));
    let named=row([icon,core,status,chevron(ctx,expanded)]).with_key("header").accessibility_label(c.name)
        .accessibility_description(state+(if summary!=""{". "+summary}else{""})+(if visible!=""&&c.status!="error"{". "+visible}else{""}))
        .accessibility_expanded(expanded)
        .with_style(ctx.component_style(if expanded{["header","header_open"]}else{["header"]},style().min_width(px(0)).width(relative(1.0))
            .items_center().gap(sp(2)).min_height(u(11)).padding_x(sp(2.5)).radius(theme_radius("lg"))
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-1),theme_color("focus_ring"))))
        .paint_transition(160,"ease_out");
    button(named,cb)
}
fn toggle_section(scope,key,expanded,ctx,event){
    let old=ctx.get_state("sections");
    let v=if old!=()&&old.scope==scope{old}else{#{scope:scope,arguments:false,result:false}};
    v[key]=!expanded;ctx.set_state("sections",v);handled()
}
fn copy_section(token,section,source,notify,scope,ctx,event){
    ctx.copy_text(source);
    ctx.set_state("copied",#{scope:scope,revision:token.revision,section:section});
    if notify{token.section=section;ctx.emit("copy",token);}
    handled()
}
fn clear_copy(ctx,state){if ctx.get_state("copied")==state{ctx.set_state("copied",());}}
fn retry_call(token,ctx,event){ctx.emit("retry",token);handled()}
fn panel_button(ctx,name,mark,part,cb){
    button(row([glyph(mark)]).with_style(ctx.component_style(part,
        style().width(u(7)).height(u(7)).items_center().justify_center().radius(theme_radius("sm"))
            .text_color(theme_color("text_muted")).hover(style().background(theme_color("surface_hover")))
            .focus_ring(px(2),offset_px(-1),theme_color("focus_ring"))))
        .accessibility_label(name),cb)
}
fn code_section(ctx,p,token,scope,section,value){
    let label=if section=="result"{tr(ctx,"result","Result")}else{tr(ctx,"arguments","Arguments")};
    let doc=diagnostic_document(value);
    let old=ctx.get_state("sections");
    let expanded=old!=()&&old.scope==scope&&old[section];
    let copied=ctx.get_state("copied");
    let did_copy=copied!=()&&copied.scope==scope&&copied.revision==token.revision&&copied.section==section;
    let heading=[tint(ctx,"label",label,hint().font_weight(500)).with_key("label")];
    if !doc.omitted{
        let name=if section=="result"{tr(ctx,"copy_result","Copy result")}else{tr(ctx,"copy_arguments","Copy arguments")};
        let act=panel_button(ctx,if did_copy{tr(ctx,"copied","Copied")}else{name},if did_copy{"okay"}else{"copy"},
            if did_copy{["copy","copy_done"]}else{["copy"]},Fn("copy_section").curry(token,section,doc.text,get(p,"on_copy",())!=(),scope));
        heading.push(act.with_key("copy"));
    }
    let kids=[row(heading).with_key("head").with_style(style().items_center().justify_between().gap(sp(2)).min_width(px(0)))];
    if doc.omitted{
        kids.push(tint(ctx,"placeholder",tr(ctx,"omitted","The document is too large to display."),hint()).with_key("limit"));
    }else{
        let reference=element_ref("code-"+section);
        let measured=ctx.element_size(reference);
        let capped=expanded||measured!=()&&measured.height>=p.max_code_height.to_float()-0.5;
        let view=code::CodeViewer(#{key:"data-"+section,label:label,source:doc.text,language:doc.language,
            show_line_numbers:false,show_gutter:false,wrap:"viewport",tab_size:2,height:"content",
            max_height:if expanded{640}else{p.max_code_height},scrollbar:if expanded{"auto"}else{"hidden"}})
            .with_key("data").with_ref(reference).with_style(style().border(px(0)).shadows([]).background(theme_color("tool_call.code")));
        let paint=style().min_width(px(0)).width(relative(1.0)).padding(sp(2)).radius(theme_radius("sm"))
            .background(theme_color("tool_call.code")).border(px(1)).border_color(theme_color("border")).overflow_hidden();
        if capped&&!expanded{paint=paint.bottom_fade(0.68,0.99);}
        kids.push(column([view]).with_key("clip").with_style(ctx.component_style("code",paint))
            .layout_size_motion(if expanded{"large"}else{"small"},"height",420,ease()));
        if capped{
            let words=if expanded{tr(ctx,"show_less","Show less")}else{tr(ctx,"show_all","Show all")};
            kids.push(btn::Button(#{key:"more-"+section,text:words,size:"xs",variant:"ghost",
                on_click:Fn("toggle_section").curry(scope,section,expanded)})
                .with_style(ctx.component_style("more",style().self_start())).accessibility_expanded(expanded));
        }
    }
    column(kids).with_key(section).with_style(ctx.component_style("section",style().min_width(px(0)).gap(sp(1))))
}
fn error_box(ctx,p,token){
    let error=get(p.entry,"error",tr(ctx,"fallback_error","The tool call failed."));
    let words=[glyph("failed").with_style(style().text_color(theme_color("text.danger"))),
        column([tint(ctx,"error_text",error,style().typography("caption").text_color(theme_color("text.danger")))])
            .with_style(style().min_width(px(0)).flex_grow().max_height(u(50)).overflow_y_scroll())];
    if get(p,"on_retry",())!=(){
        let cap=if p.pending{tr(ctx,"waiting_retry","Retry requested")}else{tr(ctx,"retry","Retry")};
        words.push(btn::Button(#{key:"retry",text:cap,prefix:glyph("retry"),size:"xs",variant:"secondary",disabled:p.pending,
            on_click:Fn("retry_call").curry(token)}).with_style(ctx.component_style("retry",style().flex_shrink(false))));
    }
    row(words).with_key("error").accessibility_role("group").accessibility_label(error)
        .with_style(ctx.component_style("error",style().min_width(px(0)).items_center().gap(sp(2)).padding(sp(2))
            .radius(theme_radius("sm")).background(theme_color("tool_call.error")).border(px(1))
            .border_color(theme_color("tool_call.error_border"))))
}
fn render_call(ctx,p){
    if !ids::valid(p.session){throw "ToolCallCard requires a stable session ID";}
    validate_call(p.entry);
    let scope=#{session:p.session,id:p.entry.id};
    let token=#{session:p.session,revision:p.revision,id:p.entry.id};
    let old=ctx.get_state("disclosure");let controlled=get(p,"open",())!=();
    let expanded=if controlled{p.open}else if old!=()&&old.scope==scope{old.open}
        else{p.default_open||p.open_on_error&&p.entry.status=="error"};
    let clock=tool_clock(ctx,scope,get(p,"now_ms",()),p.entry.status=="running");
    let age=elapsed(p.entry,clock);
    let contents=[header(ctx,p,expanded,age,Fn("change_open").curry(scope,controlled,get(p,"on_open_change",())!=(),expanded))];
    let summary=p.entry.name+" "+state_label(ctx,p.entry.status);
    contents.push(tint(ctx,"announcement",summary,style().absolute().width(px(1)).height(px(1)).overflow_hidden().opacity(0.0))
        .with_key("announcement").accessibility_role("status").accessibility_label(summary));
    if expanded{
        let parts=[];
        if p.entry.status=="error"{parts.push(error_box(ctx,p,token));}
        let args=get(p.entry,"arguments",());let result=get(p.entry,"result",());
        if args!=(){parts.push(code_section(ctx,p,token,scope,"arguments",args));}
        if result!=(){parts.push(code_section(ctx,p,token,scope,"result",result));}
        else if p.entry.status=="running"{
            parts.push(column([tint(ctx,"label",tr(ctx,"result","Result"),hint()),
                tint(ctx,"placeholder",tr(ctx,"waiting","Waiting for result…"),hint())]).with_key("pending"));
        }else if args==()&&p.entry.status=="success"{
            parts.push(tint(ctx,"placeholder",tr(ctx,"empty","No arguments or result."),hint()).with_key("empty"));
        }
        contents.push(column(parts).with_key("body").with_style(ctx.component_style("body",
            style().padding(sp(3)).gap(sp(2.5)).min_width(px(0)))));
        let copied=ctx.get_state("copied");if copied!=()&&copied.scope==scope&&copied.revision==p.revision{
            timeout("clear_copy",1300,false,Fn("clear_copy"),copied);
        }
    }
    column(contents).with_key(p.key).accessibility_role("group").accessibility_label(p.entry.name+" "+tr(ctx,"card","tool call"))
        .with_style(ctx.component_style("root",style().relative().width(relative(1.0)).max_width(px(760)).min_width(px(0))
            .background(theme_color("surface_raised")).radius(theme_radius("lg")).border(px(1)).border_color(theme_color("border"))))
        .layout_size_motion(if expanded{"open"}else{"closed"},"height",380,ease())
}
