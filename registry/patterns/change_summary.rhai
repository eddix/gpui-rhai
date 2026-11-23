/* gpui-rhai
{"id":"patterns/change_summary","export":"ChangeSummary","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["patterns/agent_tree"],"capabilities":{},"assets":[],"tokens":["accent","border","change_summary.bad","change_summary.good","change_summary.icon","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.metadata"],"environment":[]}
*/
// Summarize accepted changes with separate direction and outcome.
// Example: ChangeSummary(#{key:"weekly",period:"This week",changes:[#{id:"latency",label:"Latency",change:"Down 14 ms to 62 ms",tone:"good",direction:"down"}]})
import "patterns/agent_tree" as ids;

define_component(#{
    metadata:#{id:"patterns/change_summary","export":"ChangeSummary",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_tree"],capabilities:#{},tokens:["accent","border","change_summary.bad","change_summary.good","change_summary.icon","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.metadata"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable summary key."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Summary heading; defaults to What's changed."},
        period:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional comparison period shown next to the heading."},
        icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative header icon in the fixed icon slot."},
        changes:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable row identity, independent of display label and rank."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Name of the metric in the caller's locale."},
            change:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Human-readable change. No numerical direction is inferred from its text."},
            tone:#{schema:#{type:"optional",value:#{type:"string",allowed:["good","bad","neutral"]}},required:false,sensitive:false,doc:"Outcome, independent of arrow direction; defaults to neutral."},
            direction:#{schema:#{type:"optional",value:#{type:"string",allowed:["up","down"]}},required:false,sensitive:false,doc:"Arrow direction. Defaults to up for good, down for bad; neutral always uses a dash."},
        }}},required:true,sensitive:true,doc:"A bounded ordered list of accepted metric changes. Empty renders only the heading."},
    },state:#{fields:#{}},events:#{},slots:#{},parts:[
        "root","head","head_narrow","heading","mark","title","period","list","row","row_divided","row_narrow",
        "label","change","text","trend","good","bad","neutral",
    ]},render:Fn("render_summary")
});

fn ChangeSummary(props){render_component("patterns/change_summary",props)}
fn get(m,k,f){if k in m&&m[k]!=(){m[k]}else{f}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn unit(n){theme_length("space.unit").times(n)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn path(d,stroke,extent){
    svg("<svg viewBox='0 0 "+extent+" "+extent+"' fill='none' stroke='currentColor' stroke-width='"+stroke+
        "' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
        .accessibility_hidden(true).with_style(style().width(relative(1.0)).height(relative(1.0)).flex_shrink(false))
}
fn head(ctx,p,title,narrow){
    let icon=get(p,"icon",path("M2.5 4h4M2.5 8h8M6.5 12h7M9.5 4h1","1.4","16"));
    let mark=row([icon]).with_key("mark").accessibility_hidden(true)
        .with_style(ctx.component_style("mark",style().width(unit(4)).height(unit(4)).overflow_hidden().items_center()
            .justify_center().flex_shrink(false).text_color(theme_color("change_summary.icon"))));
    let name=text(title).with_key("title").accessibility_role("heading").accessibility_level(2)
        .with_style(ctx.component_style("title",style().min_width(px(0)).typography("body").text_color(theme_color("text_primary"))
            .whitespace_normal().font_weight(500)));
    let parts=[row([mark,name]).with_key("title-group").with_style(ctx.component_style("heading",style().items_center()
        .gap(sp(2)).min_width(px(0)).flex_grow()))];
    let period=get(p,"period","");
    if period!=""{
        parts.push(text(period).with_key("period").with_style(ctx.component_style("period",style().min_width(px(0))
            .max_width(relative(0.5)).whitespace_nowrap().text_ellipsis().typography("metadata").text_color(theme_color("text_muted")))));
    }
    row(parts).with_key("head").with_style(ctx.component_style(if narrow{["head","head_narrow"]}else{["head"]},
        style().min_width(px(0)).items_center().justify_between().gap(sp(3)).min_height(unit(11)).padding_x(sp(4))))
}
fn meaning(ctx,tone,direction){
    if tone=="neutral"{return ctx.t_or("change_summary.neutral","Neutral");}
    if tone=="good"{
        if direction=="down"{return ctx.t_or("change_summary.good_down","Favourable decrease");}
        return ctx.t_or("change_summary.good_up","Favourable increase");
    }
    if direction=="down"{ctx.t_or("change_summary.bad_down","Unfavourable decrease")}else{ctx.t_or("change_summary.bad_up","Unfavourable increase")}
}
fn metric(ctx,c,narrow,index){
    let tone=get(c,"tone","neutral");let direction=get(c,"direction",if tone=="bad"{"down"}else{"up"});
    let shape=if tone=="neutral"{"M3 6h6"}else if direction=="down"{"M6 2.5v7M3 6.5l3 3 3-3"}else{"M6 9.5v-7M3 5.5l3-3 3 3"};
    let tint=if tone=="good"{theme_color("change_summary.good")}else if tone=="bad"{theme_color("change_summary.bad")}else{theme_color("change_summary.icon")};
    let hint=meaning(ctx,tone,direction);
    let glyph=path(shape,"1.5","12").with_style(style().width(px(12)).height(px(12)));
    let trend=row([glyph]).with_key("trend").accessibility_hidden(true)
        .with_style(ctx.component_style(["trend",tone],style().width(unit(5)).height(unit(5)).radius(theme_radius("sm"))
            .flex_shrink(false).items_center().justify_center().text_color(tint)));
    let a=style().typography("body").font_weight(500).text_color(theme_color("text_primary")).min_width(px(0)).text_start();
    if !narrow{a=a.flex_grow(0.8).flex_basis(px(0)).whitespace_nowrap().text_ellipsis();}
    let label=text(c.label).with_key("label").with_style(ctx.component_style("label",a));
    let b=style().items_center().gap(sp(2)).min_width(px(0));
    if narrow {b=b.items_start();} else {b=b.flex_grow(1.2).flex_basis(px(0));}
    let change=row([trend,text(c.change).with_key("detail").with_style(ctx.component_style("text",style()
        .min_width(px(0)).flex_grow().text_start().line_clamp(3).typography("body").text_color(theme_color("text_muted"))))])
        .with_key("detail").with_style(ctx.component_style("change",b));
    let parts=["row"];if index>0{parts.push("row_divided");}if narrow{parts.push("row_narrow");}
    let base=style().min_width(px(0)).min_height(unit(12)).padding_x(sp(4)).gap(sp(4))
        .hover(style().background(theme_color("surface_hover"))).items_center();
    let n=if narrow{column([label,change])}else{row([label,change])};
    let delay=120+min(index,12)*70;
    n.with_key("metric:"+c.id).accessibility_role("listitem").accessibility_label(c.label)
        .accessibility_description(c.change+" · "+hint).with_style(ctx.component_style(parts,base))
        .paint_transition(140,"ease_out")
        .motion(motion_transition("opacity",0.0,1.0,#{duration_ms:520,delay_ms:delay,easing:ease(),intent:"decorative"}))
        .motion(motion_transition("translate_y",4.0,0.0,#{duration_ms:520,delay_ms:delay,easing:ease(),intent:"decorative"}))
}
fn render_summary(ctx,p){
    let title=get(p,"title",ctx.t_or("change_summary.title","What's changed"));
    // CSS-style container breakpoints use the usable box, not the outer
    // tray. Keep the content frame stable as rows switch direction.
    let content=element_ref("summary-content");
    let narrow=ctx.element_width_class(content,[400])!=1;
    let seen=#{};let rows=[];
    for i in 0..p.changes.len{
        let c=p.changes[i];
        if !ids::valid(c.id)||c.id in seen||c.label==""||utf8_byte_count(c.label)>512||utf8_byte_count(c.change)>2048{
            throw "ChangeSummary requires distinct IDs, nonempty labels and bounded text";
        }
        seen[c.id]=true;rows.push(metric(ctx,c,narrow,i));
    }
    let out=[head(ctx,p,title,narrow)];
    if rows.len>0{
        out.push(column(rows).with_key("list").accessibility_role("list").with_style(ctx.component_style("list",
            style().width(relative(1.0)).min_width(px(0)).background(theme_color("surface_raised")).overflow_hidden())));
    }
    let frame=column(out).with_key("content").with_ref(content)
        .with_style(style().width(relative(1.0)).min_width(px(0)));
    column([frame]).with_key(p.key).with_ref(element_ref("summary")).accessibility_role("group").accessibility_label(title)
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(480)).min_width(px(0))))
}
