// Native anchor integration fixture: one accepted document, two real consumers.
import "components/button" as button;

define_component(#{
    metadata:#{id:"gallery/document_anchors_demo","export":"DocumentAnchorsDemo",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/button"],capabilities:#{}},
    schema:#{props:#{key:#{schema:#{type:"string"},required:true,sensitive:false}},
        state:#{fields:#{sample:#{schema:#{type:"ui_value"},"default":#{type:"null"}}}},
        slots:#{},events:#{},parts:["root"]},render:Fn("demo")
});
fn DocumentAnchorsDemo(props){render_component("gallery/document_anchors_demo",props)}
fn model(ctx){
    let sample=ctx.get_state("sample");
    if sample==(){#{revision:1,accepted:false,narrow:false,turned:false,forward:false,comment:false,hidden:false}}else{sample}
}
fn accept(ctx,e){
    let s=model(ctx);if !s.accepted{s.accepted=true;s.revision+=1;ctx.set_state("sample",s);}
}
fn resize(ctx,e){let s=model(ctx);s.narrow=!s.narrow;ctx.set_state("sample",s);}
fn rotate(ctx,e){let s=model(ctx);s.turned=!s.turned;ctx.set_state("sample",s);}
fn move_cursor(ctx,e){let s=model(ctx);s.forward=!s.forward;ctx.set_state("sample",s);}
fn open(ctx,e){let s=model(ctx);s.comment=!s.comment;ctx.set_state("sample",s);}
fn hide(ctx,e){let s=model(ctx);s.hidden=!s.hidden;ctx.set_state("sample",s);}
fn pan(ctx,e){ctx.scroll_to("viewport",0.0,235.0);}
fn unpan(ctx,e){ctx.scroll_to("viewport",0.0,0.0);}
fn control(key,label,f,disabled){button::Button(#{key:key,text:label,size:"sm",variant:"ghost",disabled:disabled,on_click:f})}

fn doc(){
    #{prefix:"The team finished the ",before:"release notes",after:"release readiness checklist",
        suffix:" before activating the preview. Read 中文, العربية and the 👩‍👧 note in the same paragraph, then compare the spacing on a narrower surface. The cursor follows the drawn text, not a font-width estimate."}
}
fn ready(ref,model,body){
    let content=body.prefix+(if model.accepted{body.after}else{body.before})+body.suffix;
    let start=grapheme_count(body.prefix);
    let end=start+grapheme_count(if model.accepted{body.after}else{body.before});
    let plan=text_anchor_plan(content,model.revision,[
        #{id:"cursor",start:if model.forward{end+20}else{start}},
        #{id:"selection",start:start,end:end},
        #{id:"comment",start:end+2},
    ]);
    let parts=[span(body.prefix),span(if model.accepted{body.after}else{body.before}).bold(),span(body.suffix)];
    let affine=if model.turned{#{rotate:8.0,scale_x:0.94,scale_y:0.9,origin_x:0.0,origin_y:0.0}}else{#{}};
    let paragraph=text(parts).with_key("paragraph").with_ref(ref).capture_text_anchors(plan)
        .accessibility_label("Collaborative paragraph")
        .with_style(style().typography("body").width(relative(1.0)).min_width(px(0)).transform(affine));
    #{node:paragraph,plan:plan}
}
fn xy(p,origin){[p.x-origin.x,p.y-origin.y]}
fn polygon(q,origin){
    let a=xy(q[0],origin);let b=xy(q[1],origin);let c=xy(q[2],origin);let d=xy(q[3],origin);
    [path_move(a[0],a[1]),path_line(b[0],b[1]),path_line(c[0],c[1]),path_line(d[0],d[1]),path_close()]
}
fn in_view(anchor){
    anchor.rects.len>0&&anchor.rects[0].clipped!=()&&anchor.caret!=()
}
fn anchored(items,stage,s){
    let commands=[];let layers=[];
    let anchor=if "cursor" in items{items.cursor}else{()};
    if "selection" in items {
        let i=0;
        for rect in items.selection.rects{
            if rect.clipped!=()&&rect.quad!=(){
                commands.push(canvas_fill_path("selection:"+i.to_string(),polygon(rect.quad,stage),alpha(theme_color("accent"),0.17)));
                i+=1;
            }
        }
    }
    if anchor!=()&&in_view(anchor){
        let r=anchor.rects[0].quad;let start=xy(r[0],stage);let end=xy(r[3],stage);
        commands.push(canvas_stroke_path("caret",[path_move(start[0],start[1]),path_line(end[0],end[1])],2,theme_color("accent")));
        let at=xy(anchor.caret,stage);
        layers.push(row([text("Review agent").with_style(theme_typography("caption"))]).with_key("flag")
            .accessibility_role("status").accessibility_label("Live document cursor")
            .with_style(style().absolute().left(px(max(0.0,min(stage.width-115.0,at[0]+6.0)))).top(px(max(0.0,at[1]-24.0)))
                .min_height(px(23)).items_center().padding_x(theme_spacing("xs")).radius(theme_radius("sm"))
                .background(theme_color("accent")).text_color(theme_color("on_accent"))));
    }
    if commands.len>0 {
        layers.insert(0,canvas(canvas_scene(commands)).with_key("ink").accessibility_hidden(true)
            .with_style(style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0))));
    }
    if "comment" in items&&in_view(items.comment){
        let at=xy(items.comment.caret,stage);
        let left=max(0.0,min(stage.width-106.0,at[0]-16.0));
        let top=max(0.0,min(stage.height-32.0,at[1]+29.0));
        layers.push(button::Button(#{key:"comment-pin",text:"Comment",size:"xs",variant:"secondary",on_click:Fn("open")})
            .accessibility_label("Open anchored comment")
            .with_style(style().absolute().left(px(left)).top(px(top))));
        if s.comment{
            layers.push(column([text("Anchored comment").accessibility_role("heading"),
                text("Check the release wording at this position.").with_style(theme_typography("caption"))])
                .with_key("comment-card").with_style(style().absolute().left(px(max(0.0,min(stage.width-225.0,left))))
                    .top(px(max(0.0,min(stage.height-68.0,top+35.0)))).max_width(px(220)).padding(theme_spacing("md"))
                    .radius(theme_radius("md")).background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border"))));
        }
    }
    layers
}
fn demo(ctx,props){
    let s=model(ctx);let stage_ref=element_ref("stage");let text_ref=element_ref("paragraph");
    let target=ready(text_ref,s,doc());
    let stage=ctx.element_bounds(stage_ref);
    let anchors=ctx.text_anchors(text_ref,target.plan);
    let paragraphs=[text("Release preview").with_style(theme_typography("heading"))];
    if !s.hidden{paragraphs.push(target.node);}
    paragraphs.push(column([text("Further context").with_style(theme_typography("label")),
        text("Scroll this pane to move the rendered paragraph and its anchors together.").with_style(theme_typography("caption"))])
        .with_style(style().min_height(px(420)).padding_y(theme_spacing("lg"))));
    let viewport=column(paragraphs).with_key("viewport").with_ref(element_ref("viewport"))
        .accessibility_role("group").accessibility_label("Document viewport")
        .with_style(style().width(relative(1.0)).height(relative(1.0)).padding(theme_spacing("lg"))
            .gap(theme_spacing("md")).overflow_y_scroll().background(theme_color("surface_raised")));
    let content=[viewport];
    if !s.hidden&&stage!=()&&anchors!=(){content+=anchored(anchors,stage.visual,s);}
    let sheet=stack(content).with_key("stage").with_ref(stage_ref)
        .with_style(style().relative().width(px(if s.narrow{340}else{610})).max_width(relative(1.0)).height(px(240))
            .border(px(1)).border_color(theme_color("border")).radius(theme_radius("md")).clip());
    let label="Document revision "+s.revision.to_string();
    column([row([text(label).with_key("revision").accessibility_role("status"),
            text(if anchors==(){"Awaiting text anchors"}else{"Text anchors ready"}).with_key("publication").accessibility_role("status")
                .with_style(theme_typography("caption"))]).with_style(style().gap(theme_spacing("md")).items_center()),sheet,
        row([control("accept","Accept text suggestion",Fn("accept"),s.accepted),
            control("width",if s.narrow{"Widen document"}else{"Narrow document"},Fn("resize"),false),
            control("rotation",if s.turned{"Reset tilt"}else{"Tilt document text"},Fn("rotate"),false),
            control("cursor","Move cursor without changing revision",Fn("move_cursor"),false),
            control("scroll","Scroll document",Fn("pan"),false),control("restore","Restore document scroll",Fn("unpan"),false),
            control("visibility",if s.hidden{"Restore text source"}else{"Remove text source"},Fn("hide"),false)])
            .with_key("tools").with_style(style().min_width(px(0)).flex_wrap().gap(theme_spacing("xs")))
    ]).with_key(props.key).with_style(ctx.component_style("root",style().min_width(px(0)).gap(theme_spacing("md"))))
}
