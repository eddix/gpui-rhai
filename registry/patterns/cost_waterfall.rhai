/* gpui-rhai
{"id":"patterns/cost_waterfall","export":"CostWaterfall","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/spinner"],"capabilities":{},"assets":[],"tokens":["accent","border","design.name","cost_waterfall.agent_bg","cost_waterfall.agent_edge","cost_waterfall.agent_out","cost_waterfall.agent_mark","cost_waterfall.llm_bg","cost_waterfall.llm_edge","cost_waterfall.llm_out","cost_waterfall.llm_mark","cost_waterfall.tool_bg","cost_waterfall.tool_edge","cost_waterfall.tool_out","cost_waterfall.tool_mark","cost_waterfall.error_bg","cost_waterfall.error_edge","cost_waterfall.error_out","cost_waterfall.muted","cost_waterfall.tray","focus_ring","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text.danger","text.success","text_muted","text_primary","typography.body","typography.caption","typography.code"],"environment":[]}
*/
// Show a run ledger, with exact monetary rollups and independently controlled view state.
// Example: CostWaterfall(#{key:"run",run:"run-1",steps:[#{id:"chat",name:"Review",kind:"llm",status:"done",cost_micros:42000,tokens_in:1600,tokens_out:370}]})
import "components/spinner" as spinner;

define_component(#{
    metadata:#{id:"patterns/cost_waterfall","export":"CostWaterfall",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/spinner"],capabilities:#{},assets:[],
        tokens:["accent","border","design.name","cost_waterfall.agent_bg","cost_waterfall.agent_edge","cost_waterfall.agent_out","cost_waterfall.agent_mark","cost_waterfall.llm_bg","cost_waterfall.llm_edge","cost_waterfall.llm_out","cost_waterfall.llm_mark","cost_waterfall.tool_bg","cost_waterfall.tool_edge","cost_waterfall.tool_out","cost_waterfall.tool_mark","cost_waterfall.error_bg","cost_waterfall.error_edge","cost_waterfall.error_out","cost_waterfall.muted","cost_waterfall.tray","focus_ring","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text.danger","text.success","text_muted","text_primary","typography.body","typography.caption","typography.code"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable card instance."},
        run:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Ledger identity. Changing it clears local view state."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Heading, defaults to Run cost."},
        task:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional caption next to the heading."},
        currency_prefix:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Monetary prefix for integer micro-units; defaults to $. No currency conversion is performed."},
        steps:#{schema:#{type:"array",max_items:192,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Nonempty stable step ID, unique across the run."},
            parent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Parent ID; omit for a root. Siblings retain their order."},
            name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Step label."},
            kind:#{schema:#{type:"string",allowed:["agent","llm","tool"]},required:true,sensitive:false,doc:"Step class used for color and details."},
            status:#{schema:#{type:"string",allowed:["running","done","failed"]},required:true,sensitive:false,doc:"Host-reported step state."},
            model:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Model name."},
            detail:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Secondary caption when there is no model."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative mark."},
            cost_micros:#{schema:#{type:"optional",value:#{type:"integer",min:-1000000000000,max:1000000000000}},required:false,sensitive:false,doc:"Own cost (negative for credit), in millionths of the displayed unit. Do not pre-sum descendants."},
            tokens_in:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Own input tokens, excluding descendants."},
            tokens_out:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Own output tokens, excluding descendants."},
            duration_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:604800000}},required:false,sensitive:false,doc:"Host-reported duration in milliseconds."},
            input:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional literal input excerpt."},
            output:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional literal output excerpt."},
            error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional literal failure explanation."}
        }}},required:true,sensitive:true,doc:"A bounded flat tree; each step contributes its own amounts exactly once."},
        status:#{schema:#{type:"optional",value:#{type:"string",allowed:["running","done","failed"]}},required:false,sensitive:false,doc:"Optional Host run state; otherwise derived from the steps."},
        mode:#{schema:#{type:"optional",value:#{type:"string",allowed:["cost","tokens"]}},required:false,sensitive:false,doc:"Controlled measure. Omit to let the view own it."},
        default_mode:#{schema:#{type:"string",allowed:["cost","tokens"]},required:false,sensitive:false,"default":#{type:"string",value:"cost"},doc:"Initial uncontrolled measure."},
        collapsed:#{schema:#{type:"optional",value:#{type:"array",max_items:192,items:#{type:"string"}}},required:false,sensitive:false,doc:"Controlled set of folded IDs. Omit for local folds."},
        default_collapsed:#{schema:#{type:"optional",value:#{type:"array",max_items:192,items:#{type:"string"}}},required:false,sensitive:false,doc:"Initial uncontrolled folded steps."},
        open_id:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Controlled detail ID. An empty string means closed; omit for local selection."},
        default_open_id:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initial uncontrolled detail ID."},
        max_height:#{schema:#{type:"optional",value:#{type:"integer",min:96,max:1800}},required:false,sensitive:false,doc:"Maximum height of the scrollable ledger, excluding its fixed headers."},
        on_mode_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a measure proposal."},
        on_fold_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the proposed folded IDs."},
        on_open_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the proposed detail selection."}
    },state:#{fields:#{local:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
    events:#{
        mode_change:#{payload:#{type:"object",allow_unknown:false,fields:#{run:#{schema:#{type:"string"},required:true,sensitive:false},
            mode:#{schema:#{type:"string",allowed:["cost","tokens"]},required:true,sensitive:false}}},doc:"Proposes the measure. A controlled card waits for Host props."},
        fold_change:#{payload:#{type:"object",allow_unknown:false,fields:#{run:#{schema:#{type:"string"},required:true,sensitive:false},
            collapsed:#{schema:#{type:"array",max_items:192,items:#{type:"string"}},required:true,sensitive:false}}},doc:"Proposes folding or unfolding."},
        open_change:#{payload:#{type:"object",allow_unknown:false,fields:#{run:#{schema:#{type:"string"},required:true,sensitive:false},
            id:#{schema:#{type:"string"},required:true,sensitive:false}}},doc:"Proposes the active detail, or an empty ID to close it."}
    },slots:#{},parts:["root","head","spark","heading","task","switch","switch_button","selected","status","status_pill","status_running","status_done","status_failed","live","stats","stat","stat_label",
        "amount","sub","top","panel","ledger_columns","header","step_column","metric","column_label","axis","tick","rule","viewport","row","row_open","indent","fold",
        "mark","name","model","flag","track","bar","credit","shimmer","detail","detail_header","excerpt","error","empty","glyph"],
    effects:["focus_selected"]},render:Fn("render_waterfall")
});

fn CostWaterfall(p){render_component("patterns/cost_waterfall",p)}
fn get(p,k,d){if p!=()&&k in p&&p[k]!=(){p[k]}else{d}}
fn u(x){theme_length("space.unit").times(x)}
fn sp(x){theme_length("space.spacing_unit").times(x)}
fn tx(ctx,k,s){ctx.t_or("cost_waterfall."+k,s)}
fn curve(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn line(){style().typography("body").min_width(px(0)).whitespace_nowrap().text_ellipsis().text_color(theme_color("text_primary"))}
fn quiet(){line().typography("caption").text_color(theme_color("text_muted"))}
fn label(ctx,part,t,base){text(t).with_style(ctx.component_style(part,base))}
fn paint(kind,part){let key="cost_waterfall."+kind+"_"+part;theme_color(key)}
fn glyph(ctx,path){svg("<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><path d='"+path+"'/></svg>")
    .accessibility_hidden(true).with_style(ctx.component_style("glyph",style().width(u(3.5)).height(u(3.5)).flex_shrink(false)))}
fn money(ctx,p,n){
    let sign=if n<0{"−"}else{""};let m=abs(n);
    let prefix=get(p,"currency_prefix","$");
    if m==0{return prefix+"0";}
    if m<1000{return sign+"<"+prefix+"0.001";}
    let digits=if m<10000{3}else if m<10_000_000{2}else{0};
    sign+prefix+ctx.format_number(m.to_float()/1_000_000.0,#{min_fraction_digits:digits,max_fraction_digits:digits})
}
fn axis_money(ctx,p,n){
    (if n<0{"−"}else{""})+get(p,"currency_prefix","$")+
        ctx.format_number(abs(n).to_float()/1_000_000.0,#{max_fraction_digits:6})
}
fn compact(ctx,n){
    if n<1000{return ctx.format_number(n);}
    let million=n>=999500;
    ctx.format_number(n.to_float()/if million{1_000_000.0}else{1000.0},#{max_fraction_digits:if million||n<100000{1}else{0}})+if million{"M"}else{"k"}
}
fn time(ctx,n){
    if n==(){return "–";}
    if n<1000{return ctx.format_number(n)+" ms";}
    if n<60000{return ctx.format_number(n.to_float()/1000.0,#{max_fraction_digits:if n<10000{1}else{0}})+" s";}
    let minutes=n/60000;
    if minutes<60{return minutes.to_string()+" m "+((n/1000)%60).to_string()+" s";}
    if minutes<1440{return (minutes/60).to_string()+" h "+(minutes%60).to_string()+" m";}
    (minutes/1440).to_string()+" d "+((minutes/60)%24).to_string()+" h"
}
fn defaults(p){#{run:p.run,mode:p.default_mode,collapsed:get(p,"default_collapsed",[]),open:get(p,"default_open_id",""),active:"",focus:""}}
fn local(ctx,base){let s=ctx.get_state("local");if s!=()&&s.run==base.run{s}else{base}}
fn request_mode(base,controlled,mode,ctx,event){
    let s=local(ctx,base);if !controlled{s.mode=mode;}s.focus="";
    ctx.set_state("local",s);ctx.emit("mode_change",#{run:base.run,mode:mode});handled()
}
fn request_open(base,controlled,id,ctx,event){
    let s=local(ctx,base);if !controlled{s.open=id;}s.focus="";
    ctx.set_state("local",s);ctx.emit("open_change",#{run:base.run,id:id});handled()
}
fn fold(base,controlled,collapsed,id,ctx,event){
    let next=[];
    for v in collapsed{if v!=id{next.push(v);}}
    if !(id in collapsed){next.push(id);}
    let s=local(ctx,base);if !controlled{s.collapsed=next;}s.active=id;s.focus="";
    ctx.set_state("local",s);ctx.emit("fold_change",#{run:base.run,collapsed:next});handled()
}
fn inspect_top(base,cs,os,collapsed,ancestors,id,ctx,event){
    let requested=[];
    for old in collapsed{if !(old in ancestors){requested.push(old);}}
    let s=local(ctx,base);if !cs{s.collapsed=requested;}if !os{s.open=id;}
    s.active=id;s.focus=id;ctx.set_state("local",s);
    if requested!=collapsed{ctx.emit("fold_change",#{run:base.run,collapsed:requested});}
    ctx.emit("open_change",#{run:base.run,id:id});handled()
}
fn focused(base,id,ctx,inside){
    if inside{let s=local(ctx,base);if s.active!=id{s.active=id;ctx.set_state("local",s);}}
}
fn move_focus(target,ctx,event){ctx.focus(target);ctx.scroll_into_view(target);handled()}
fn focus_selected(ctx,data){
    let s=local(ctx,data.base);
    if s.focus!=data.id{return;}
    s.focus="";ctx.set_state("local",s);
    ctx.focus(data.ref);ctx.scroll_into_view(data.ref);
}
fn idle(ctx,data){}
fn refkey(index){"step:"+index.to_string()}
fn moment(ctx,p,root){
    if p.run==""||utf8_byte_count(p.run)>96||utf8_byte_count(get(p,"currency_prefix","$"))>12{throw "CostWaterfall requires a bounded run identity and currency prefix";}
    let base=defaults(p);let s=local(ctx,base);
    let cfg=#{base:base,local:s,mode:get(p,"mode",s.mode),collapsed:get(p,"collapsed",s.collapsed),open:get(p,"open_id",s.open),
        cm:get(p,"mode",())!=(),cf:get(p,"collapsed",())!=(),co:get(p,"open_id",())!=(),width:ctx.element_width_class(root,[520,720])};
    let by=#{};
    let input=[];let running=false;let failed=false;
    for i in 0..p.steps.len{
        let q=p.steps[i];by[q.id]=i;
        if q.name==""||utf8_byte_count(q.name)>512||utf8_byte_count(get(q,"input",""))>16000||
            utf8_byte_count(get(q,"output",""))>16000||utf8_byte_count(get(q,"error",""))>4096{throw "CostWaterfall step captions and excerpts must be bounded";}
        input.push(#{id:q.id,parent:get(q,"parent",()),kind:q.kind,cost_micros:get(q,"cost_micros",()),
            tokens_in:get(q,"tokens_in",0),tokens_out:get(q,"tokens_out",0)});
        running=running||q.status=="running";failed=failed||q.status=="failed";
    }
    let model=cost_waterfall_projection(input,cfg.mode,cfg.collapsed);
    let visible=[];let shown=#{};
    let row_by=#{};
    for r in model.rows{
        let id=p.steps[r.index].id;row_by[id]=r;
        if !r.hidden{shown[id]=visible.len;visible.push(r);}
    }
    cfg.status=get(p,"status",if running{"running"}else if failed{"failed"}else{"done"});
    let target=s.active;
    while target!=""&&target in by&&!(target in shown){target=get(p.steps[by[target]],"parent","");}
    cfg.active=if target in shown{target}else if visible.len>0{p.steps[visible[0].index].id}else{""};
    if s.focus!=""&&cfg.open==s.focus&&s.focus in shown{
        effect("focus_selected",#{base:base,id:s.focus,ref:refkey(by[s.focus])},Fn("focus_selected"),Fn("idle"));
    }
    let hatch=();
    for r in visible{
        if r.credit{
            let size=ctx.element_size(root);
            hatch=credit_hatch(if size==(){760.0}else{max(1.0,min(4096.0,size.width))});
            break;
        }
    }
    #{cfg:cfg,data:model,visible:visible,shown:shown,rows:row_by,by:by,hatch:hatch}
}
fn button(ctx,key,part,content,name,callback){
    row(content).with_key(key).accessibility_role("button").accessibility_label(name).tab_stop(true)
        .on_click(callback).on_key_value("plain+enter",callback,()).on_key_value("plain+space",callback,())
        .with_style(ctx.component_style(part,style().min_width(px(0)).min_height(u(6.5)).items_center().gap(sp(1)).padding_x(sp(1.5))
            .radius(theme_radius("sm")).cursor_pointer().hover(style().background(theme_color("surface_hover")))
            .focus_ring(px(2),offset_px(-2),theme_color("focus_ring")))).paint_transition(150,"ease_out")
}
fn status_text(ctx,status){
    switch status{"running"=>tx(ctx,"live","Live"),"failed"=>tx(ctx,"failed","Failed"),_=>tx(ctx,"done","Done")}
}
fn head(ctx,p,m){
    let c=m.cfg;let mode=c.mode;let title=get(p,"title",tx(ctx,"title","Run cost"));
    let knobs=[];
    for option in ["cost","tokens"]{
        let cap=if option=="cost"{tx(ctx,"cost","Cost")}else{tx(ctx,"tokens","Tokens")};
        let selected=mode==option;let slots=if selected{["switch_button","selected"]}else{["switch_button"]};
        knobs.push(button(ctx,option,slots,[label(ctx,"sub",cap,quiet())],cap,Fn("request_mode").curry(c.base,c.cm,option)).accessibility_pressed(selected));
    }
    let title_row=[glyph(ctx,"M12 2l2.5 7.5L22 12l-7.5 2.5L12 22l-2.5-7.5L2 12l7.5-2.5z"),
        label(ctx,"heading",title,line().font_weight(600)).accessibility_role("heading")
            .accessibility_level(3).accessibility_label(title)];
    let task=get(p,"task","");
    if task!=""{title_row.push(label(ctx,"task",task,quiet().text_color(theme_color("cost_waterfall.muted"))).with_key("task"));}
    let words=status_text(ctx,c.status);let status=[];
    let status_ink=if c.status=="running"{theme_color("accent")}else if c.status=="done"{theme_color("text.success")}else{theme_color("text.danger")};
    if c.status=="running"{
        status.push(box([]).with_key("pulse").accessibility_hidden(true)
            .with_style(ctx.component_style("live",style().width(u(1.5)).height(u(1.5)).radius(u(0.75)).background(current_color())))
            .motion(motion_transition("opacity",0.32,1.0,#{duration_ms:680,iterations:"infinite",autoreverse:true,intent:"decorative"})));
    }
    status.push(label(ctx,"status",words,quiet().font_weight(500).inherit_text_color()).with_key("label"));
    row([row(title_row).with_key("title-group").with_style(style().min_width(px(0)).gap(sp(1.5)).items_center()),
        row(knobs).with_key("switch").accessibility_role("group").accessibility_label(tx(ctx,"measure","Measure by"))
            .with_style(ctx.component_style("switch",style().gap(sp(0.5)).padding(sp(0.5)).radius(theme_radius("sm")).background(theme_color("surface_hover")))),
        row(status).with_key("run-state").accessibility_role("status").accessibility_label(words)
            .with_style(ctx.component_style(["status_pill","status_"+c.status],style().min_height(u(5)).padding_x(sp(1))
                .gap(sp(1)).items_center().flex_shrink(false).radius(theme_radius("pill")).text_color(status_ink)))])
        .with_key("header").with_style(ctx.component_style("head",style().min_width(px(0)).items_center().justify_between().flex_wrap().gap(sp(2)).padding(sp(2))))
}
fn stat(ctx,key,name,words,weight){
    let content=[label(ctx,"stat_label",name,quiet())];for w in words{content.push(w);}
    column(content).with_key(key).with_style(ctx.component_style("stat",style().min_width(px(0))
        .flex_basis(px(0)).flex_grow(weight).gap(sp(0.5))))
}
fn stats(ctx,p,m){
    let c=m.cfg;let data=m.data;let t=data.total;
    let spend=if t.priced{money(ctx,p,t.micros)}else{"–"};
    let counts=compact(ctx,t.input+t.output);
    let top=[];
    if data.top==(){top.push(label(ctx,"sub","–",quiet()));}
    else{
        let q=p.steps[data.top];let ancestors=[];let parent=get(q,"parent","");
        while parent!=""&&parent in m.by{ancestors.push(parent);parent=get(p.steps[m.by[parent]],"parent","");}
        top.push(button(ctx,"expensive","top",[label(ctx,"amount",q.name,line().font_weight(500)),label(ctx,"sub",money(ctx,p,q.cost_micros),quiet())],
            tx(ctx,"top","Most expensive")+": "+q.name,Fn("inspect_top").curry(c.base,c.cf,c.co,c.collapsed,ancestors,q.id)));
    }
    let cost_total=stat(ctx,"summary-cost",tx(ctx,"total_cost","Total cost"),[label(ctx,"amount",spend,line().font_weight(600).font_feature("tnum",1)).accessibility_role("status").accessibility_label("Total cost "+spend)],1.0);
    let token_values=[label(ctx,"amount",counts,line().font_weight(600).font_feature("tnum",1))];
    if c.width!=0{token_values.push(label(ctx,"sub",compact(ctx,t.input)+" "+tx(ctx,"input_short","in")+" · "+compact(ctx,t.output)+" "+tx(ctx,"output_short","out"),quiet()));}
    let token_total=stat(ctx,"summary-tokens",tx(ctx,"total_tokens","Total tokens"),token_values,1.0);
    let top=stat(ctx,"summary-top",tx(ctx,"top","Most expensive"),top,1.4);
    let edge=style().border_start(px(1)).border_color(theme_color("border"));
    let token_total=token_total.with_style(edge);
    let frame=ctx.component_style("stats",style().min_width(px(0)).padding_y(sp(3.5)));
    let summary=if c.width==0{
        column([row([cost_total.with_style(style().padding_start(sp(4))),token_total.with_style(style().padding_x(sp(3)))])
            .with_style(style().min_width(px(0))),
            row([top.with_style(style().padding_start(sp(4)))]).with_style(style().min_width(px(0)))])
    }else{row([cost_total.with_style(style().padding_start(sp(5.5))),token_total,top.with_style(edge)])};
    summary.with_key("stats").with_style(frame)
        .with_style(if c.width==0{style().gap_y(sp(2.5)).padding_y(sp(2.5))}else{style()})
}
// One row scaffold is shared by the axis, the bars and the grid behind them.
fn keys(c){if c.width==2{["in","out","cost","time"]}else if c.mode=="cost"{["cost","time"]}else{["tokens","time"]}}
fn ledger_columns(ctx,c){
    ctx.component_style("ledger_columns",style().width(relative(1.0)).min_width(px(0))
        .padding_start(sp(2.5)).padding_end(sp(if c.width==0{3}else{4})).gap(sp(3)).items_center())
}
fn step_rail(ctx,c,children){
    let base=style().min_width(px(0)).items_center().overflow_hidden();
    let base=if c.width==0{base.flex_grow().flex_basis(px(0)).height(u(7.5))}
        else{base.width(u(if c.width==2{57}else{50})).flex_shrink(false).height(u(8))};
    row(children).with_style(ctx.component_style("step_column",base))
}
fn metric_width(key){u(if key=="cost"||key=="tokens"{13}else if key=="time"{12.5}else{11})}
fn metric(ctx,key,value){
    metric_part(ctx,key,value,"metric")
}
fn metric_part(ctx,key,value,part){
    label(ctx,part,value,quiet().width(metric_width(key)).flex_shrink(false).text_end().font_feature("tnum",1))
}
fn scaffold(c,title,plot,metrics,style){
    if c.width==0{
        let front=[title];for m in metrics{front.push(m);}
        column([row(front).with_style(style().min_width(px(0)).width(relative(1.0)).height(u(7.5)).gap(sp(3)).items_center()),
            row([plot]).with_style(style().min_width(px(0)).width(relative(1.0)).height(u(3)))])
            .with_style(style).with_style(style().gap_y(px(0)).items_start())
    }else{
        let parts=[title,plot];for m in metrics{parts.push(m);}
        row(parts).with_style(style)
    }
}
fn fraction(node,width,base){
    node.with_style(base.width(relative(width))).motion(motion_transition("width_fraction",width,width,
        #{duration_ms:560,easing:curve(),intent:"decorative"}))
}
fn grid(ctx,m){
    let ticks=[];
    for q in m.data.ticks{
        if q.at>=0.999{continue;}
        let rule=box([]).with_style(ctx.component_style("rule",style().width(px(1)).height(relative(1.0)).flex_shrink(false)
            .background(alpha(theme_color("border"),if q.value==0.0{1.0}else{0.6}))));
        let start=fraction(box([]).with_key("offset"),q.at,style().height(px(0)).flex_shrink(false));
        ticks.push(row([start,rule]).with_key("grid:"+m.cfg.mode+":"+q.value.to_string())
            .with_style(style().absolute().width(relative(1.0)).height(relative(1.0)).top(px(0)).inset_start(px(0))));
    }
    box(ticks).with_key("rules").accessibility_hidden(true)
        .with_style(style().absolute().width(relative(1.0)).height(relative(1.0)).top(px(0)).inset_start(px(0)))
}
fn grid_plane(ctx,m){
    let c=m.cfg;
    let track=box([grid(ctx,m)]).with_style(style().relative().height(relative(1.0)).flex_grow().min_width(px(0)));
    let cells=[];
    if c.width!=0{cells.push(step_rail(ctx,c,[]));}
    cells.push(track);
    if c.width!=0{
        for key in keys(c){cells.push(box([]).with_style(style().width(metric_width(key)).flex_shrink(false)));}
    }
    row(cells).with_key("ledger-grid").accessibility_hidden(true)
        .with_style(ledger_columns(ctx,c).absolute().top(px(0)).inset_start(px(0))
            .height(if c.width==0||m.visible.len==0{px(30)}else{relative(1.0)}))
}
fn axis(ctx,p,m){
    let clean=ctx.theme_variant()!=()&&ctx.theme_token("design.name")=="clean";
    let ticks=[];
    for q in m.data.ticks{
        if q.at>=0.999{continue;}
        let n=q.value.round().to_int();
        let name=if m.cfg.mode=="cost"{axis_money(ctx,p,n)}else{compact(ctx,abs(n))};
        let start=fraction(box([]).with_key("offset"),q.at,style().height(px(0)).flex_shrink(false));
        ticks.push(row([start,label(ctx,"tick",name,quiet().font_feature("tnum",1).margin_start(sp(1.5)).flex_shrink(false))]).with_key("tick:"+m.cfg.mode+":"+n.to_string())
            .with_style(style().absolute().top(px(0)).inset_start(px(0)).width(relative(1.0)).items_center()));
    }
    let track=box(ticks).with_key("axis").with_style(ctx.component_style("axis",style().relative().flex_grow()
        .min_width(px(0)).height(u(7.5)).overflow_hidden()));
    let metrics=[];for k in keys(m.cfg){let title=switch k{
        "in"=>tx(ctx,"in_col","In"),"out"=>tx(ctx,"out_col","Out"),"time"=>tx(ctx,"time","Time"),"cost"=>tx(ctx,"cost","Cost"),_=>tx(ctx,"tokens","Tokens")};
        metrics.push(metric_part(ctx,k,if clean{title.to_upper()}else{title},["metric","column_label"])
            .accessibility_role("columnheader"));}
    let c=m.cfg;
    let layout=ctx.component_style("header",ledger_columns(ctx,c).height(u(7.5)));
    let title=tx(ctx,"step","Step");
    let head=if c.width==0{row([track]).with_style(layout)}
        else{scaffold(c,step_rail(ctx,c,[label(ctx,["sub","column_label"],if clean{title.to_upper()}else{title},
            quiet().padding_start(sp(1.5))).accessibility_role("columnheader")]),track,metrics,layout)};
    head.with_key("columns").accessibility_hidden(true)
}
fn credit_hatch(width){
    let marks=[];let height=116.0;let stride=8.5;
    for i in 1..500{
        let s=i.to_float()*stride;
        if s>=width+height{break;}
        let x=max(0.0,s-height);let y=max(0.0,s-width);
        marks.push(canvas_line("credit:"+i.to_string(),x,s-x,s-y,y,3,current_color()));
    }
    canvas_scene(marks)
}
fn bar(ctx,p,m,r,s){
    let palette=if s.status=="failed"{"error"}else{s.kind};
    let c=paint(palette,"bg");let pieces=[];
    if m.cfg.mode=="tokens"&&r.split>0.0{
        pieces.push(fraction(box([]).with_key("input"),1.0-r.split,style().height(relative(1.0)).flex_shrink(false).background(c)));
        pieces.push(fraction(box([]).with_key("output"),r.split,style().height(relative(1.0)).flex_shrink(false).background(paint(palette,"out"))));
    }
    if r.credit&&m.hatch!=(){
        pieces.push(canvas(m.hatch).with_key("credit-stripes").accessibility_hidden(true)
            .with_style(style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0)).text_color(c)));
    }
    if s.status=="running"&&r.size>0.0{
        pieces.push(box([]).with_key("sheen").with_style(ctx.component_style("shimmer",style().absolute().top(px(0)).inset_start(px(0))
            .width(relative(0.32)).height(relative(1.0)).background(alpha(theme_color("surface_raised"),0.3))))
            .motion(motion_transition("translate_x_fraction",-1.0,3.5,#{duration_ms:1000,iterations:"infinite",intent:"decorative"})));
    }
    let parts=if r.credit{["bar","credit"]}else{["bar"]};
    let fill=ctx.component_style(parts,style().relative().height(u(4)).flex_shrink(false).clip().radius(theme_radius("sm"))
        .border(px(if r.size>0.0{0.75}else{0.0})).border_color(paint(palette,"edge"))
        .background(if r.credit||m.cfg.mode=="tokens"&&r.split>0.0{rgba(0x00000000)}else{c}));
    let fill=if m.cfg.width==0{fill.height(px(10)).radius(px(3))}else{fill};
    let measure=fraction(row(pieces).with_key("measure"),r.size,fill);
    let start=fraction(box([]).with_key("start"),r.left,style().height(px(0)).flex_shrink(false));
    row([start,measure]).with_key("lane").accessibility_hidden(true)
        .with_style(ctx.component_style("track",style().relative().height(px(if m.cfg.width==0{12}else{32}))
            .flex_grow().min_width(px(0)).items_center().overflow_hidden()))
}
fn describe(ctx,p,r,s){
    let n=r.total;
    status_text(ctx,s.status)+" · "+(if n.priced{money(ctx,p,n.micros)}else{"–"})+" · "+ctx.format_number(n.input+n.output)+" "+tx(ctx,"tokens","Tokens")
}
fn detail(ctx,p,s,narrow){
    let fields=if s.kind=="llm"{["Prompt","Response"]}else if s.kind=="tool"{["Input","Output"]}else{["Task","Result"]};
    let descriptions=[get(s,"input","–"),get(s,"output",if s.status=="running"{tx(ctx,"waiting","Still working…")}else{"–"})];
    let blocks=[];
    for i in 0..2{
        let base=style().min_width(px(0)).whitespace_normal().typography(if s.kind=="tool"{"code"}else{"body"});
        let bind=column([label(ctx,"excerpt",descriptions[i],base)]).with_key("excerpt")
            .with_style(style().min_width(px(0)).max_height(u(25)).overflow_y_scroll()).scrollbars("hidden","auto").tab_stop(true)
            .accessibility_role("group").accessibility_label(fields[i]);
        blocks.push(column([label(ctx,"detail_header",fields[i],quiet()),bind]).with_key("io:"+i.to_string())
            .with_style(style().flex_grow().flex_basis(px(0)).min_width(px(0)).gap(sp(1))));
    }
    let out=[];
    let error=get(s,"error","");
    if error!=""{out.push(label(ctx,"error",error,quiet().text_color(theme_color("text.danger"))));}
    out.push(if narrow{column(blocks).with_style(style().min_width(px(0)).gap(sp(2)))}
        else{row(blocks).with_style(style().min_width(px(0)).gap(sp(2)))});
    column(out).with_key("detail").accessibility_role("region").accessibility_label(s.name+" "+tx(ctx,"details","details"))
        .with_style(ctx.component_style("detail",style().min_width(px(0)).padding(sp(2)).gap(sp(2))))
}
fn entry(ctx,p,m,i){
    let r=m.visible[i];let s=p.steps[r.index];let c=m.cfg;let open=c.open==s.id;
    let contents=[];
    if r.depth>0{
        contents.push(box([]).with_style(ctx.component_style("indent",style().width(u(min(r.depth,4).to_float()*3.5)).height(relative(1.0))
            .flex_shrink(false).border_end(px(1)).border_color(theme_color("border")))).accessibility_hidden(true));
    }
    let cue=if r.folded{"m9 6 6 6-6 6"}else{"m6 9 6 6 6-6"};
    let fold_call=Fn("fold").curry(c.base,c.cf,c.collapsed,s.id);
    let fold_box=style().width(u(4)).flex_shrink(false).margin_end(sp(1));
    if r.children{
        contents.push(button(ctx,"fold","fold",[glyph(ctx,cue)],if r.folded{"Expand "+s.name}else{"Collapse "+s.name},fold_call)
            .tab_stop(false).with_style(fold_box));
    }else{contents.push(box([]).with_style(fold_box));}
    let logo=get(s,"icon",());
    let mark=if logo!=(){logo}else{box([]).with_style(ctx.component_style("mark",style().width(u(2)).height(u(2))
        .radius(px(2.5)).flex_shrink(false).background(paint(s.kind,"mark"))))};
    contents.push(row([mark]).accessibility_hidden(true).with_style(style().width(u(5)).height(u(5))
        .flex_shrink(false).items_center().justify_center().margin_end(sp(2))));
    let words=[label(ctx,"name",s.name,line().font_weight(if r.children{500}else{400})
        .max_width(relative(1.0)).flex_shrink(false))];
    let context=get(s,"model",get(s,"detail",""));if context!=""&&c.width!=0{words.push(label(ctx,"model","· "+context,quiet()));}
    contents.push(row(words).with_style(style().min_width(px(0)).flex_grow().items_baseline().gap(sp(1.5)).overflow_hidden()));
    if m.data.top==r.index&&c.width>0{contents.push(label(ctx,"flag",tx(ctx,"flag","Top"),quiet()));}
    if s.status=="running"{contents.push(spinner::Spinner(#{key:"running",size:"xs",label:"Running"}).accessibility_hidden(true));}
    if s.status=="failed"{contents.push(label(ctx,"error","!",quiet().text_color(theme_color("text.danger"))));}
    let metrics=[];let n=r.total;
    for k in keys(c){
        metrics.push(metric(ctx,k,switch k{
            "in"=>if n.input>0{compact(ctx,n.input)}else{"–"}, "out"=>if n.output>0{compact(ctx,n.output)}else{"–"},
            "cost"=>if n.priced{money(ctx,p,n.micros)}else{"–"}, "tokens"=>compact(ctx,n.input+n.output),
            _=>time(ctx,get(s,"duration_ms",()))
        }));
    }
    let parts=if open{["row","row_open"]}else{["row"]};
    let click=Fn("request_open").curry(c.base,c.co,if open{""}else{s.id});
    let node=scaffold(c,step_rail(ctx,c,contents),bar(ctx,p,m,r,s),metrics,
        ctx.component_style(parts,ledger_columns(ctx,c).height(px(if c.width==0{46}else{32}))
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .with_key("step").with_ref(element_ref(refkey(r.index))).accessibility_role("treeitem").accessibility_label(s.name)
        .accessibility_description(describe(ctx,p,r,s)).accessibility_level(r.depth+1).accessibility_selected(open)
        .tab_stop(c.active==s.id).on_focus_within_change(Fn("focused").curry(c.base,s.id))
        .on_click(click).on_key_value("plain+enter",click,()).on_key_value("plain+space",click,())
        .on_key_value("plain+escape",Fn("request_open").curry(c.base,c.co,""),())
        .on_key_value("plain+up",Fn("move_focus").curry(refkey(m.visible[max(0,i-1)].index)),())
        .on_key_value("plain+down",Fn("move_focus").curry(refkey(m.visible[min(m.visible.len-1,i+1)].index)),())
        .on_key_value("plain+home",Fn("move_focus").curry(refkey(m.visible[0].index)),())
        .on_key_value("plain+end",Fn("move_focus").curry(refkey(m.visible[m.visible.len-1].index)),())
        .paint_transition(160,"ease_out");
    let forward="plain+right";
    let back="plain+left";
    if r.children{
        node=node.accessibility_expanded(!r.folded);
        if r.folded{node=node.on_key_value(forward,fold_call,());}
        else{
            node=node.on_key_value(back,fold_call,()).on_key_value(forward,Fn("move_focus").curry(refkey(m.visible[i+1].index)),());
        }
    }
    let parent=get(s,"parent","");
    if (!r.children||r.folded)&&parent!=""&&parent in m.by{node=node.on_key_value(back,Fn("move_focus").curry(refkey(m.by[parent])),());}
    let nodes=[node];
    if open{nodes.push(detail(ctx,p,s,c.width==0));}
    column(nodes).with_key("step:"+s.id).with_style(style().min_width(px(0)))
        .layout_size_motion(if open{"open"}else{"closed"},"height",300,curve())
}
fn render_waterfall(ctx,p){
    let ref=element_ref("waterfall");
    let m=moment(ctx,p,ref);
    let entries=[];
    for i in 0..m.visible.len{entries.push(entry(ctx,p,m,i));}
    if entries.len==0{entries.push(label(ctx,"empty",tx(ctx,"empty","No steps in this run"),quiet().padding(sp(2))));}
    let rows=column(entries).with_key("steps").accessibility_role("tree").accessibility_label(tx(ctx,"ledger","Run steps"))
        .with_style(ctx.component_style("viewport",style().min_width(px(0)).max_height(px(get(p,"max_height",420))).overflow_y_scroll()))
        .scrollbars("hidden","auto");
    let ledger=column([grid_plane(ctx,m),axis(ctx,p,m),rows]).with_key("ledger")
        .with_style(style().relative().min_width(px(0)));
    let body=column([stats(ctx,p,m),ledger]).with_key("panel").with_style(ctx.component_style("panel",
        style().background(theme_color("surface_raised")).min_width(px(0)).radius(theme_radius("lg"))));
    column([head(ctx,p,m),body]).with_key("waterfall").with_ref(ref).accessibility_role("group").accessibility_label(tx(ctx,"card","Cost waterfall"))
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).padding(sp(1))
            .background(theme_color("cost_waterfall.tray")).radius(theme_radius("lg"))))
}
