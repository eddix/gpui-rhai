/* gpui-rhai
{"id":"patterns/tips_panel","export":"TipsPanel","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","patterns/agent_tree"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","tips_panel.done","typography.body","typography.caption"],"environment":[]}
*/
// Present bounded onboarding tips and propose actions to the application.
// Example: TipsPanel(#{key:"intro",series:"account",revision:0,tips:[#{id:"search",title:"Find a command",description:"Open the command menu.",action:"Try search"}]})
import "components/button" as btn;
import "patterns/agent_tree" as ids;

define_component(#{
    metadata:#{id:"patterns/tips_panel","export":"TipsPanel",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/button","patterns/agent_tree"],capabilities:#{},tokens:["accent","border","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","tips_panel.done","typography.body","typography.caption"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable panel key."},
        series:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of this onboarding checklist."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision of the accepted tip states."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Panel title; defaults to Tips."},
        tips:#{schema:#{type:"array",max_items:48,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable tip identity."},
            title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Short heading of the tip."},
            description:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Plain language description, also accessible when detail replaces its display."},
            detail:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional formatted description node, e.g. a row with a keyboard keycap."},
            action:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Button label for the requested action."},
            primary:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Use a strong primary appearance for the most important tip."},
            done:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Accepted completion, published by the Host after an action succeeds."},
            done_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Replacement label after the Host marks the tip done."},
        }}},required:true,sensitive:true,doc:"At most 48 tips in application order. The done counter includes only present tips."},
        pending:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"The ID of an in-flight tip request. Prevents duplicate requests while the Host decides."},
        max_height:#{schema:#{type:"optional",value:#{type:"integer",min:120,max:800}},required:false,sensitive:false,doc:"Optional viewport height cap in logical pixels; normally the theme supplies 420."},
        close_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Footer action label. No footer action is shown without on_close."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a versioned tip action request. Without it the checklist is read-only."},
        on_close:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a close request. Removing or retaining the panel is the Host's decision."},
    },state:#{fields:#{}},events:#{
        action:#{payload:#{type:"object",allow_unknown:false,fields:#{
            series:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted onboarding series."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision the user acted on."},
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable ID of the requested tip."},
        }},doc:"A proposal to perform the tip's action. Advance the snapshot with done=true only after acceptance."},
        close:#{payload:#{type:"object",allow_unknown:false,fields:#{
            series:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted onboarding series."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Revision at the close request."},
        }},doc:"A proposal to close the panel, not an automatic dismissal."},
    },slots:#{},parts:[
        "root","root_no_footer","head","title","count","body","grid","tip","tip_done",
        "tip_title","description","action","action_primary","action_done","mark","empty","footer","close",
    ]},render:Fn("render_tips")
});

fn TipsPanel(props){render_component("patterns/tips_panel",props)}
fn get(m,k,f){if k in m&&m[k]!=(){m[k]}else{f}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn unit(n){theme_length("space.unit").times(n)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn request(kind,ticket,ctx,event){ctx.emit(kind,ticket);handled()}
fn done_icon(ctx){
    svg("<svg viewBox='0 0 16 16'><circle cx='8' cy='8' r='7' fill='currentColor'/><path d='m5 8.2 2 2 4-4.2' fill='none' stroke='#ffffff' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'/></svg>")
        .accessibility_hidden(true).with_style(ctx.component_style("mark",style().width(unit(4)).height(unit(4))
            .flex_shrink(false).text_color(theme_color("tips_panel.done"))))
}
fn tip_action(ctx,p,t,primary,done,index){
    let accepted=get(t,"done_label",ctx.t_or("tips_panel.done","Done"));
    let waiting=get(p,"pending","")==t.id;
    let actionable=get(p,"on_action",())!=()&&get(p,"pending","")==""&&!done;
    let label=if done{accepted}else{t.action};
    let plan=#{key:"command:"+t.id,variant:if done{"ghost"}else if primary{"primary"}else{"secondary"},text:label,size:"sm",
        disabled:!actionable,loading:waiting,loading_text:ctx.t_or("tips_panel.working","Working…")};
    if done{plan.prefix=done_icon(ctx);}
    if actionable{plan.on_click=Fn("request").curry("action",#{series:p.series,revision:p.revision,id:t.id});}
    let parts=["action"];if done{parts.push("action_done");}else if primary{parts.push("action_primary");}
    let node=btn::Button(plan).with_key(plan.key).with_ref(element_ref("tip-action:"+index.to_string()))
        .tab_stop(!done&&get(p,"on_action",())!=())
        .with_style(ctx.component_style(parts,style().max_width(relative(1.0)).min_width(px(0)).self_start()));
    if done{
        node=node.accessibility_role("status").accessibility_label(accepted).accessibility_description(t.title);
    }
    node
}
fn tip(ctx,p,t,index){
    let done=get(t,"done",false);let strong=get(t,"primary",false);
    let headline=text(t.title).with_key("heading").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("tip_title",style().typography("body").font_weight(500).text_color(theme_color("text_primary")).min_width(px(0)).text_start()));
    let prose=get(t,"detail",text(t.description));
    let detail=column([prose]).with_key("description").with_style(ctx.component_style("description",style().min_width(px(0))
        .width(relative(1.0)).flex_grow().margin_top(sp(1.5)).margin_bottom(sp(3.5)).typography("caption").text_color(theme_color("text_muted"))));
    let delay=80+min(index,12)*50;
    column([headline,detail,tip_action(ctx,p,t,strong,done,index)]).with_key("tip:"+t.id)
        .accessibility_role("listitem").accessibility_label(t.title).accessibility_description(t.description)
        .with_style(ctx.component_style(if done{["tip","tip_done"]}else{["tip"]},
            style().min_width(px(0)).padding(sp(4)).radius(theme_radius("lg")).border(px(1)).border_color(theme_color("border"))))
        .paint_transition(240,"ease_out")
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:480,delay_ms:delay,easing:ease(),intent:"decorative"}))
        .motion(motion_transition("translate_y",4.0,0.0,#{duration_ms:480,delay_ms:delay,easing:ease(),intent:"decorative"}))
}
fn render_tips(ctx,p){
    if !ids::valid(p.series){throw "TipsPanel requires a valid series ID";}
    // The grid's own width is the body content box, already excluding a
    // skin's border. The thresholds do not need per-theme border guesses.
    let grid_width=element_ref("grid-width");
    let width=ctx.element_width_class(grid_width,[380,560]);
    let cols=if width==2{3}else if width==1{2}else{1};
    let complete=0;let seen=#{};let content=[];
    for i in 0..p.tips.len{
        let t=p.tips[i];
        if !ids::valid(t.id)||t.id in seen||t.title==""||t.action==""||utf8_byte_count(t.title)>512||utf8_byte_count(t.description)>4096||utf8_byte_count(t.action)>256 {
            throw "TipsPanel needs distinct IDs, nonempty action labels and bounded text";
        }
        seen[t.id]=get(t,"done",false);
        if seen[t.id]{complete+=1;}
        content.push(tip(ctx,p,t,i));
    }
    let pending=get(p,"pending","");
    if pending!=""&&(!(pending in seen)||seen[pending]){throw "The pending tip must exist and not be done";}
    let title=get(p,"title",ctx.t_or("tips_panel.title","Tips"));
    let count=ctx.t_or("tips_panel.count","{done}/{total} done",#{done:ctx.format_number(complete),total:ctx.format_number(p.tips.len)});
    let header=row([
        text(title).with_key("title").accessibility_role("heading").accessibility_level(2).with_style(ctx.component_style("title",
            style().typography("body").font_weight(600).text_color(theme_color("text_primary")).min_width(px(0)).flex_grow())),
        text(count).with_key("count").accessibility_role("status").with_style(ctx.component_style("count",
            style().typography("caption").font_feature("tnum",1).flex_shrink(false).text_color(theme_color("text_muted")))),
    ]).with_key("head").with_style(ctx.component_style("head",style().min_width(px(0)).justify_between().items_center().gap(sp(3)).padding(sp(3))));
    if content.len==0{
        content.push(text(ctx.t_or("tips_panel.empty","No tips to explore")).with_key("empty")
            .with_style(ctx.component_style("empty",style().typography("caption").text_color(theme_color("text_muted")))));
    }
    let grid=box(content).with_key("grid").accessibility_role("list").with_style(ctx.component_style("grid",style()
        .grid().grid_cols(cols).min_width(px(0)).width(relative(1.0)).padding(sp(3.5)).gap(sp(3))))
        .with_ref(grid_width);
    let body_style=ctx.component_style("body",style().min_width(px(0)).width(relative(1.0)).max_height(unit(105))
        .overflow_y_scroll().background(theme_color("surface_raised")));
    let cap=get(p,"max_height",());
    if cap!=(){body_style=body_style.max_height(px(cap));}
    let body=column([grid]).with_key("body").with_ref(element_ref("tip-scroll")).accessibility_role("group")
        .accessibility_label(ctx.t_or("tips_panel.tip_list","Tip list")).with_style(body_style.relative())
        .scrollbars("hidden","auto");
    let out=[header,body];let close=get(p,"on_close",())!=();
    if close{
        let b=btn::Button(#{key:"close",variant:"secondary",size:"sm",text:get(p,"close_label",ctx.t_or("tips_panel.close","Close")),
            on_click:Fn("request").curry("close",#{series:p.series,revision:p.revision})})
            .with_style(ctx.component_style("close",style().max_width(relative(1.0))));
        out.push(row([b]).with_key("footer").with_style(ctx.component_style("footer",style().min_width(px(0)).padding(sp(2)))));
    }
    column(out).with_key(p.key).accessibility_role("group").accessibility_label(title)
        .with_style(ctx.component_style(if close{["root"]}else{["root","root_no_footer"]},
            style().width(relative(1.0)).max_width(px(760)).min_width(px(0)).overflow_hidden()))
}
