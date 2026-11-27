/* gpui-rhai
{"id":"patterns/experiment_card","export":"ExperimentCard","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["patterns/project_card","patterns/agent_tree","components/button","components/toggle_group"],"capabilities":{},"assets":[],"tokens":["border","experiment_card.a","experiment_card.b","experiment_card.c","experiment_card.control","experiment_card.d","experiment_card.track","experiment_card.card","experiment_card.muted","experiment_card.tray","experiment_card.chip_red_ink","experiment_card.chip_red_wash","experiment_card.chip_amber_ink","experiment_card.chip_amber_wash","experiment_card.chip_green_ink","experiment_card.chip_green_wash","experiment_card.chip_blue_ink","experiment_card.chip_blue_wash","experiment_card.chip_violet_ink","experiment_card.chip_violet_wash","experiment_card.chip_gray_ink","experiment_card.chip_gray_wash","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],"environment":[]}
*/
// A compact experiment that unfolds into a controlled comparison.
// State: disclosure and preview attention; metric, candidate and confirmation belong to the Host.
// Example: ExperimentCard(#{key:"signup",experiment:"signup-v1",revision:0,title:"Sign-up test",metrics:[#{id:"rate",label:"Rate"}],variants:[]})
import "patterns/agent_tree" as ids;
import "patterns/project_card" as project;
import "components/button" as btn;
import "components/toggle_group" as tog;
define_component(#{
    metadata:#{id:"patterns/experiment_card","export":"ExperimentCard",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["patterns/project_card","patterns/agent_tree","components/button","components/toggle_group"],capabilities:#{},assets:[],tokens:["border","experiment_card.a","experiment_card.b","experiment_card.c","experiment_card.control","experiment_card.d","experiment_card.track","experiment_card.card","experiment_card.muted","experiment_card.tray","experiment_card.chip_red_ink","experiment_card.chip_red_wash","experiment_card.chip_amber_ink","experiment_card.chip_amber_wash","experiment_card.chip_green_ink","experiment_card.chip_green_wash","experiment_card.chip_blue_ink","experiment_card.chip_blue_wash","experiment_card.chip_violet_ink","experiment_card.chip_violet_wash","experiment_card.chip_gray_ink","experiment_card.chip_gray_wash","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],environment:[]},
    schema:#{
        props:#{
            key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable retained card key."},
            experiment:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable experiment ID."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Visible experiment title."},
            metrics:#{schema:#{type:"array",items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique metric ID."},label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible metric name."},lower_is_better:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Smaller readings are preferred."},decimals:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:6}},required:false,sensitive:false,doc:"Maximum displayed fraction digits."},suffix:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional unit suffix."}}},max_items:8},required:true,sensitive:false,doc:"Supported metrics. Order determines the default metric."},
            variants:#{schema:#{type:"array",items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable variant ID."},short:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Short badge, e.g. A."},name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Visible variant name."},control:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Designates the baseline; the first variant is the default."},values:#{schema:#{type:"map",values:#{type:"number",min:-1000000000000,max:1000000000000}},required:true,sensitive:true,doc:"Readings keyed by metric ID. Omitted readings remain missing."},confidence:#{schema:#{type:"optional",value:#{type:"map",values:#{type:"number",min:0,max:100}}},required:false,sensitive:false,doc:"Optional confidence percentages by metric."},visitors:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1000000000000}},required:false,sensitive:false,doc:"Accepted visitor count."},color:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:16777215}},required:false,sensitive:false,doc:"Optional RGB24 paint."}}},max_items:12},required:true,sensitive:true,doc:"Variants in their display and chart order."},
            metric:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted metric ID. Defaults to the first metric."},
            winner:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted candidate, absent until one is selected."},
            status:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Status words."},tone:#{schema:#{type:"optional",value:#{type:"string",allowed:["gray","violet","amber","blue","green","red"]}},required:false,sensitive:false,doc:"Status tint."}}}},required:false,sensitive:false,doc:"Optional status chip."},
            note:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional line beside the status."},
            open:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Optional controlled disclosure."},
            default_open:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Initial uncontrolled disclosure.","default":#{type:"bool",value:false}},
            trigger:#{schema:#{type:"string",allowed:["hover","click"]},required:false,sensitive:false,doc:"Pointer hover or explicit disclosure.","default":#{type:"string",value:"hover"}},
            busy:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Keep the current selection while the Host commits.","default":#{type:"bool",value:false}},
            hint:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Footer node before a variant is selected."},
            action_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Final action text. Defaults to Choose {name}."},
            on_open_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called for requested or local disclosure."},
            on_metric_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a versioned metric proposal."},
            on_winner_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a versioned candidate proposal."},
            on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a distinct request to act on the accepted candidate."},
        },
        state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        events:#{
            open_change:#{payload:#{type:"object",allow_unknown:false,fields:#{experiment:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable experiment ID."},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision."},open:#{schema:#{type:"bool"},required:true,sensitive:false,doc:"Requested disclosure."}}},doc:"Notify or propose disclosure."},
            metric_change:#{payload:#{type:"object",allow_unknown:false,fields:#{experiment:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable experiment ID."},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision."},metric:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Proposed metric."}}},doc:"Ask the Host to show a different metric."},
            winner_change:#{payload:#{type:"object",allow_unknown:false,fields:#{experiment:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable experiment ID."},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision."},winner:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:false,doc:"Proposed candidate, or null to deselect."}}},doc:"Nominate a candidate; no final action is performed."},
            action:#{payload:#{type:"object",allow_unknown:false,fields:#{experiment:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable experiment ID."},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision."},winner:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted candidate."},metric:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted metric."}}},doc:"Ask the Host to act on the accepted candidate in this snapshot."},
        },
        slots:#{},
        parts:["root","paper","card","head","head_narrow","chart","column","titles","title","sub","status","note","lead","lead_idle","body","switch","rows","row","row_hot","row_picked","badge","badge_picked","label","value","lift","track","fill","foot","caption","action","empty"],
    },
    render:Fn("render_experiment"),
});
fn ExperimentCard(p){render_component("patterns/experiment_card",p)}
fn get(m,k,d){project::get(m,k,d)}
fn u(n){project::u(n)}
fn sp(n){project::sp(n)}
fn ease(){cubic_bezier(0.32,0.72,0.24,1.0)}
fn line(ctx,s,k,style){project::line(ctx,s,k,style)}
fn decimal(v){if type_of(v)=="i64"{v.to_float()}else{v}}
fn send(name,t,ctx,e){ctx.emit(name,t);handled()}
fn cfg(ctx,p){
    let proxy=#{project:p.experiment,revision:p.revision,default_open:p.default_open,open:get(p,"open",())};
    let g=project::cfg_for(ctx,proxy);
    #{local:g,ticket:#{experiment:p.experiment,revision:p.revision},emit:get(p,"on_open_change",())!=(),open:g.open}
}
fn disclose(c,want,ctx,e){
    if project::update_local_disclosure(c.local,want,ctx)&&c.emit{
        ctx.emit("open_change",#{experiment:c.ticket.experiment,revision:c.ticket.revision,open:want});
    }
    handled()
}
fn later(c,ctx,inside){project::schedule(c.local,ctx,inside);}
fn delayed(ctx,p){disclose(p.cfg,p.open,ctx,());}
fn watch(c,field,id,ctx,inside){project::look(c.local,field,id,ctx,inside);}
fn fold(c,ctx,e){
    if c.open{ctx.focus("disclosure");disclose(c,false,ctx,e);}else{()}
}
fn choose(t,old,ctx,items){
    if items.len>0&&items[0]!=old{t.metric=items[0];ctx.emit("metric_change",t);}
}
fn selected(p,d,v){get(p,"winner","")==v.id}
fn propose(c,winner,ctx,e){let t=c.ticket;t.winner=winner;ctx.emit("winner_change",t);handled()}
fn data(ctx,p){
    if !ids::valid(p.experiment){throw "ExperimentCard requires a stable experiment ID";}
    let metrics=#{};
    for m in p.metrics{
        if !ids::valid(m.id)||m.id in metrics||m.label==""||utf8_byte_count(get(m,"suffix",""))>64 {throw "Experiment metrics need distinct IDs and bounded labels and units";}
        metrics[m.id]=m;
    }
    let metric_id=get(p,"metric",if p.metrics.len==0{""}else{p.metrics[0].id});
    if metric_id!=""&&!(metric_id in metrics){throw "The active experiment metric must exist";}
    let metric=get(metrics,metric_id,());let lower=if metric==(){false}else{get(metric,"lower_is_better",false)};
    let entries=#{};let values=#{};let control="";let shades=#{};let lo=0.0;let hi=0.0;let known=0;
    for v in p.variants{
        if !ids::valid(v.id)||v.id in entries||v.short==""||v.name==""{throw "Experiment variants need distinct IDs, labels and short names";}
        if get(v,"control",false){if control!=""{throw "An experiment can have only one baseline";}control=v.id;}
        entries[v.id]=v;
        for key in v.values.keys(){if !(key in metrics){throw "Every reading must name a declared metric";}}
        let certainty=get(v,"confidence",#{});
        for key in certainty.keys(){if !(key in metrics){throw "Every confidence must name a declared metric";}}
        if metric_id in v.values{
            let x=decimal(v.values[metric_id]);values[v.id]=x;
            if known==0{lo=x;hi=x;}else{lo=min(lo,x);hi=max(hi,x);}
            known+=1;
        }
    }
    if control==""&&p.variants.len>0{control=p.variants[0].id;}
    let winner=get(p,"winner","");
    if winner!=""&&!(winner in entries){throw "The accepted candidate must be in the experiment";}
    let base=get(values,control,0.0);let deltas=#{};let leader="";let lift=0.05;
    let idx=0;
    for v in p.variants{
        if v.id!=control{shades[v.id]=idx%4;idx+=1;}
        if v.id in values&&base!=0.0{
            let delta=(values[v.id]-base)/abs(base)*100.0;deltas[v.id]=delta;
            let improvement=if lower{-delta}else{delta};
            if v.id!=control&&improvement>lift{leader=v.id;lift=improvement;}
        }
    }
    #{entries:entries,metric:metric,metric_id:metric_id,lower:lower,values:values,lo:lo,hi:hi,
        control:control,leader:leader,winner:winner,deltas:deltas,shades:shades}
}
fn tint(d,v){
    if get(v,"color",())!=(){return rgb(v.color);}
    if v.id==d.control{return theme_color("experiment_card.control");}
    switch get(d.shades,v.id,0){
        0=>theme_color("experiment_card.a"),1=>theme_color("experiment_card.b"),
        2=>theme_color("experiment_card.c"),_=>theme_color("experiment_card.d"),
    }
}
fn sign(ctx,d){(if d>=0.0{"+"}else{"−"})+ctx.format_number(abs(d),#{min_fraction_digits:1,max_fraction_digits:1})+"%"}
fn size_of(d,v,head){
    if !(v.id in d.values){return if head{0.10}else{0.02};}
    let x=d.values[v.id];
    if d.hi==d.lo{return if head{0.70}else{if x==0.0{0.02}else{1.0}};}
    if head{return 0.38+0.62*(x-d.lo)/(d.hi-d.lo);}
    if d.lo<0.0{return max(0.02,(x-d.lo)/(d.hi-d.lo));}
    max(0.02,x/d.hi)
}
fn mini(ctx,p,d,c,hot){
    let cells=[];
    for v in p.variants{
        let f=size_of(d,v,true);let alt=hot!=""&&v.id!=hot;
        let visual=box([]).with_key("ink").with_style(ctx.component_style("column",style().width(relative(1.0)).height(relative(1.0))
            .radius(px(2)).background(tint(d,v)).opacity(if alt{0.45}else{1.0}).transform(#{origin_y:1.0,scale_y:f})))
            .motion(motion_transition("scale_y",f,f,#{duration_ms:620,easing:ease()})).paint_transition(160,"ease_out");
        let cell=column([visual]).with_key("column:"+v.id).accessibility_hidden(true)
            .with_style(style().flex_grow().flex_basis(px(0)).height(relative(1.0)).min_width(px(0)));
        if c.open{
            cell=cell.on_hover_change(Fn("watch").curry(c,"ring",v.id));
            if !p.busy&&get(p,"on_winner_change",())!=(){cell=cell.on_click(Fn("propose").curry(c,if d.winner==v.id{()}else{v.id}));}
        }else{cell=cell.on_click(Fn("disclose").curry(c,true)).tab_stop(false);}
        cells.push(cell);
    }
    row(cells).with_key("mini").with_style(ctx.component_style("chart",style().width(u(8.5)).height(u(8.5))
        .items_end().gap(sp(0.5)).padding(sp(1)).radius(theme_radius("sm"))
        .background(theme_color("experiment_card.track")).flex_shrink(false)))
}
fn chip_style(ctx,parts,tone,base){
    let source="experiment_card.chip_"+tone;
    ctx.component_style(parts,base.typography("caption").font_weight(500).items_center()
        .radius(theme_radius("sm")).background(theme_color(source+"_wash")).text_color(theme_color(source+"_ink")))
}
fn chip(ctx,s,part,tone){
    text(s).with_style(chip_style(ctx,part,tone,style().min_height(u(5)).padding_x(sp(1.5))))
}
fn chip_icon(clean,trend,down){
    let mark=if trend{
        if clean{"<svg viewBox='0 0 12 12' fill='none'><path d='M1.5 8.5l3-3 2 2 4-4M8 3.5h2.5V6' stroke='currentColor' stroke-width='1.2' stroke-linecap='round' stroke-linejoin='round'/></svg>"}
        else{"<svg viewBox='0 0 10 10' fill='none'><path d='M1.5 7L4 4.5l1.6 1.6L8.5 3M6.2 3h2.3v2.3' stroke='currentColor' stroke-width='1.4' stroke-linecap='round' stroke-linejoin='round'/></svg>"}
    }else if clean{"<svg viewBox='0 0 10 10' fill='currentColor'><circle cx='5' cy='5' r='4.5' opacity='.22'/><circle cx='5' cy='5' r='2.25'/></svg>"}
    else{"<svg viewBox='0 0 12 12'><circle cx='6' cy='6' r='4.25' stroke='currentColor' stroke-width='1.1' fill='none'/><circle cx='6' cy='6' r='1.5' fill='currentColor'/></svg>"};
    let edge=if clean==trend{12.0}else{10.0};
    let shape=style().width(px(edge)).height(px(edge)).flex_shrink(false);
    svg(mark).with_key("mark").accessibility_hidden(true)
        .with_style(if trend&&down{shape.transform(#{scale_y:-1.0})}else{shape})
}
fn chip_words(s){
    text(s).with_key("words").accessibility_hidden(true)
        .with_style(style().min_width(px(0)).whitespace_nowrap().text_ellipsis())
}
fn top(ctx,p,d,c,hot,narrow){
    let clean=ctx.theme_token("design.name")=="clean";
    let title=line(ctx,p.title,"title",style().typography("body").font_weight(600).text_color(theme_color("text_primary")))
        .with_key("toggle").with_ref(element_ref("disclosure")).accessibility_role("button").accessibility_label(p.title)
        .accessibility_expanded(c.open).tab_stop(true).with_style(style().focus_ring(px(2),offset_px(0),theme_color("focus_ring")))
        .on_click(Fn("disclose").curry(c,!c.open)).on_key_value("plain+enter",Fn("disclose").curry(c,!c.open),())
        .on_key_value("plain+space",Fn("disclose").curry(c,!c.open),());
    let sub=[];let state=get(p,"status",());
    if state!=(){
        let status_paint=chip_style(ctx,"status",get(state,"tone","gray"),style().min_height(u(5))
            .min_width(px(0)).gap(sp(1)).padding_x(sp(1.5)).flex_shrink(false));
        sub.push(row([chip_icon(clean,false,false),chip_words(state.label)]).with_key("chip")
            .accessibility_role("text").accessibility_label(state.label).with_style(status_paint));
    }
    if !narrow&&get(p,"note","")!=""{sub.push(line(ctx,p.note,"note",style().typography("caption").text_color(theme_color("text_muted"))));}
    let names=column([title,row(sub).with_style(ctx.component_style("sub",style().min_width(px(0)).gap(sp(1.5)).items_center()))])
        .with_key("titles").with_style(ctx.component_style("titles",style().min_width(px(0)).flex_grow()));
    let idle=d.leader=="";
    let lead_parts=if idle{["lead","lead_idle"]}else{["lead"]};
    let label=if idle{ctx.t_or("experiment_card.no_lift","No lift yet")}else{d.entries[d.leader].short+" "+sign(ctx,d.deltas[d.leader])};
    let words=[];
    if idle{words.push(chip_words(label));}else{
        words.push(text(d.entries[d.leader].short).with_key("variant").accessibility_hidden(true)
            .with_style(style().font_weight(if clean{500}else{600})));
        if !narrow{words.push(chip_icon(clean,true,d.deltas[d.leader]<0.0));}
        words.push(chip_words(sign(ctx,d.deltas[d.leader])));
    }
    let lead=row(words).accessibility_role("text").accessibility_label(label)
        .with_style(chip_style(ctx,lead_parts,if idle{"gray"}else{"green"},style().min_height(u(6))
            .padding_x(sp(2)).gap(sp(1)).flex_shrink(false)));
    row([mini(ctx,p,d,c,hot),names,lead.with_key("leader")]).with_key("head")
        .with_style(ctx.component_style(if narrow{["head","head_narrow"]}else{["head"]},
            style().min_width(px(0)).items_center().gap(sp(2.5)).padding(sp(3))))
}
fn result_row(ctx,p,d,c,v,hot){
    let picked=d.winner==v.id;let sample=get(d.values,v.id,());
    let decimals=if d.metric==(){1}else{get(d.metric,"decimals",1)};
    let suffix=if d.metric==(){""}else{get(d.metric,"suffix","")};
    let value=if sample==(){line(ctx,"—","value",style().typography("caption"))}else{
        ctx.number_text(sample,decimals,"",suffix,false)
            .motion(motion_transition("number_value",sample,sample,#{duration_ms:620,easing:ease()}))
            .with_style(ctx.component_style("value",style().typography("caption").font_feature("tnum",1).min_width(px(0)).whitespace_nowrap().text_ellipsis()))
    };
    let tone=tint(d,v);
    let marker=if picked{project::task_tick()}else{text(v.short)};
    let short=row([marker]).with_key("badge").accessibility_hidden(true)
        .with_style(ctx.component_style(if picked{["badge","badge_picked"]}else{["badge"]},
            style().min_width(u(5)).min_height(u(5)).padding_x(sp(1)).items_center().justify_center().radius(theme_radius("sm"))
                .typography("caption").font_weight(600).text_color(if picked{theme_color("surface_raised")}else{tone})
                .background(if picked{tone}else{alpha(tone,0.16)})));
    let change=get(d.deltas,v.id,());
    let effect=if change==(){0.0}else if d.lower{-change}else{change};
    let lift=if v.id==d.control{ctx.t_or("experiment_card.baseline","Base")}else if change==(){"—"}else{sign(ctx,change)};
    let lift_tone=if v.id==d.control||change==(){"gray"}else if effect>0.05{"green"}else if effect< -0.05{"red"}else{"gray"};
    let head=row([short,line(ctx,v.name,"label",style().typography("body").flex_grow()).with_key("name"),value.with_key("value"),
        chip(ctx,lift,"lift",lift_tone).with_key("lift")])
        .with_style(style().min_width(px(0)).items_center().gap(sp(2)));
    let f=size_of(d,v,false);
    let bar=box([]).with_key("fill").with_style(ctx.component_style("fill",style().height(relative(1.0))
        .width(relative(f)).radius(px(2)).background(tone)))
        .motion(motion_transition("width_fraction",f,f,#{duration_ms:620,easing:ease()}));
    let track=row([bar]).with_key("track").accessibility_hidden(true)
        .with_style(ctx.component_style("track",style().width(relative(1.0)).height(px(4)).radius(px(2))
            .min_width(px(0)).overflow_hidden().background(theme_color("experiment_card.track"))));
    let parts=["row"];if hot==v.id{parts.push("row_hot");}if picked{parts.push("row_picked");}
    let accessible=if sample==(){ctx.t_or("experiment_card.missing","No value")}else{ctx.format_number(sample,#{max_fraction_digits:decimals})+suffix};
    let node=column([head,track]).with_key("variant:"+v.id).accessibility_role("radio").accessibility_label(v.name)
        .accessibility_checked(picked).accessibility_description(accessible+" · "+lift).tab_stop(true)
        .on_hover_change(Fn("watch").curry(c,"row",v.id)).on_focus_within_change(Fn("watch").curry(c,"focus",v.id))
        .with_style(ctx.component_style(parts,style().padding(sp(1.5)).gap(sp(1.5)).min_width(px(0)).radius(theme_radius("sm"))
            .focus_ring(px(2),offset_px(-2),theme_color("focus_ring")).opacity(if hot==""||hot==v.id{1.0}else{0.50})))
        .paint_transition(160,"ease_out");
    if !p.busy&&get(p,"on_winner_change",())!=(){
        let next=if picked{()}else{v.id};let click=Fn("propose").curry(c,next);
        node=node.on_click(click).on_key_value("plain+space",click,()).on_key_value("plain+enter",click,());
    }
    node
}
fn comparison(ctx,p,d,c,hot){
    let nodes=[];
    if p.metrics.len>1{
        let opts=[];for m in p.metrics{opts.push(#{value:m.id,label:m.label});}
        nodes.push(tog::ToggleGroup(#{key:"metric",label:ctx.t_or("experiment_card.metric","Metric"),items:opts,
            values:[d.metric_id],allow_empty:false,size:"xs",disabled:p.busy||get(p,"on_metric_change",())==(),
            on_change:Fn("choose").curry(c.ticket,d.metric_id)})
            .with_style(ctx.component_style("switch",style().min_width(px(0)).max_width(relative(1.0)).overflow_x_scroll())));
    }
    let rows=[];for v in p.variants{rows.push(result_row(ctx,p,d,c,v,hot));}
    if rows.len==0{rows.push(line(ctx,ctx.t_or("experiment_card.empty","No variants"),"empty",style().typography("caption")));}
    nodes.push(column(rows).with_key("variants").accessibility_role("radiogroup").accessibility_label(ctx.t_or("experiment_card.candidate","Candidate"))
        .with_style(ctx.component_style("rows",style().min_width(px(0)).gap(sp(0.5)))));
    nodes
}
fn foot(ctx,p,d,c,hot){
    let content=[];let picked=get(d.entries,d.winner,());let custom=get(p,"hint",());
    let caption=if picked==(){ctx.t_or("experiment_card.pick","Pick a winner")}else{
        ctx.t_or("experiment_card.picked","{name} picked",#{name:picked.name})
    };
    if hot!=""{
        let a=d.entries[hot];caption=a.short;
        let visits=get(a,"visitors",());let confidence=get(get(a,"confidence",#{}),d.metric_id,());
        if visits!=(){caption+=" · "+ctx.t_or("experiment_card.visits","{n} visitors",#{n:ctx.format_number(visits)});}
        if hot==d.control{caption+=" · "+ctx.t_or("experiment_card.comparison_base","baseline");}
        else if confidence!=(){caption+=" · "+ctx.t_or("experiment_card.confidence","{n}% confidence",#{n:ctx.format_number(decimal(confidence))});}
    }
    if hot==""&&picked==()&&custom!=(){content.push(custom);}
    else{content.push(line(ctx,caption,"caption",style().typography("caption").text_color(theme_color("text_muted")).flex_grow()).with_key("cue").accessibility_role("status"));}
    if picked!=()&&get(p,"on_action",())!=(){
        let t=c.ticket;t.winner=picked.id;t.metric=d.metric_id;
        let label=get(p,"action_label",ctx.t_or("experiment_card.act","Choose {name}",#{name:picked.name}));
        content.push(btn::Button(#{key:"confirm",text:label,size:"sm",variant:"primary",disabled:p.busy,on_click:Fn("send").curry("action",t)})
            .with_style(ctx.component_style("action",style().max_width(relative(0.65)))));
    }
    row(content).with_key("footer").with_style(ctx.component_style("foot",style().min_width(px(0)).min_height(u(10)).padding_x(sp(2.5))
        .gap(sp(2)).items_center().justify_between()))
}
fn render_experiment(ctx,p){
    let d=data(ctx,p);let config=cfg(ctx,p);
    let s=project::local(ctx,p.experiment,p.default_open);let hot="";
    for a in [s.ring,s.row,s.focus]{if a in d.entries{hot=a;break;}}
    if s.hover!=()&&s.hover!=config.open{timeout("hover-disclosure",if s.hover{90}else{220},false,Fn("delayed"),#{cfg:config,open:s.hover});}
    let ref=element_ref("experiment");let narrow=ctx.element_width_class(ref,[301])!=1;
    let body=column(if config.open{comparison(ctx,p,d,config,hot)}else{[]}).with_key("body")
        .layout_size_motion(if config.open{"open"}else{"closed"},"height",520,ease())
        .with_style(ctx.component_style("body",if config.open{style().min_width(px(0)).padding_x(sp(3)).padding_bottom(sp(3)).overflow_hidden()}
            else{style().min_width(px(0)).overflow_hidden()}));
    let card=column([top(ctx,p,d,config,hot,narrow),body]).with_key("card")
        .with_style(ctx.component_style("card",style().min_width(px(0)).width(relative(1.0)).radius(theme_radius("lg"))
            .background(theme_color("surface_raised"))));
    let outer=[card];if config.open{outer.push(foot(ctx,p,d,config,hot));}
    // Style the same paper layer rather than reparenting focused content
    // when switching between a Modern tray and a Clean card.
    let paper=column(outer).with_key("paper").with_style(ctx.component_style("paper",
        style().min_width(px(0)).width(relative(1.0))));
    let node=column([paper]).with_key(p.key).with_ref(ref).accessibility_role("group").accessibility_label(p.title)
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(px(340))))
        .on_bubble("key:plain+escape",Fn("fold").curry(config));
    if p.trigger=="hover"{node=node.on_hover_change(Fn("later").curry(config));}
    node
}
