/* gpui-rhai
{"id":"components/cited_answer","export":"CitedAnswer","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/accordion","components/button","components/markdown_viewer"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","selection","surface_hover","surface_raised","text_muted","text_primary","radius.sm","radius.lg","typography.caption","typography.body","space.spacing_unit","space.unit","text.accent","typography.control"],"environment":["density"]}
*/
// CitedAnswer presents linked evidence, inline citations and a source preview.
// State: Host-owned content; preview is local unless preview is supplied.
// Example: CitedAnswer(#{key:"evidence",segments:[#{text:"The build passed.",cite:["log"]}],sources:[#{id:"log",site:"Build",title:"Job log"}]})
import "components/accordion" as accordion;
import "components/button" as button;
import "components/markdown_viewer" as markdown;

define_component(#{
    metadata:#{id:"components/cited_answer","export":"CitedAnswer",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/accordion","components/button","components/markdown_viewer"],capabilities:#{},assets:[],
        tokens:["accent","border","focus_ring","selection","surface_hover","surface_raised","text_muted","text_primary","radius.sm","radius.lg","typography.caption","typography.body","space.spacing_unit","space.unit","text.accent","typography.control"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable evidence identity."},
        label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accessible name of the answer."},
        source:#{schema:#{type:"optional",value:#{type:"one_of",variants:[#{type:"string"},#{type:"document"}]}},required:false,sensitive:false,
            doc:"Optional Markdown string or Host document. Links with destinations gpui-cite:1, gpui-cite:2, etc. cite the source at that 1-based position. Use this or segments."},
        segments:#{schema:#{type:"array",max_items:128,items:#{type:"object",allow_unknown:false,fields:#{
            text:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Literal sentence or phrase; leading and trailing spaces are preserved."},
            paragraph:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Start a new paragraph before this segment."},
            cite:#{schema:#{type:"array",max_items:8,items:#{type:"string"}},required:false,sensitive:false,doc:"Cited source IDs ordered as presented. Duplicate or unknown IDs are ignored."},
        }}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Text and evidence IDs when not passing a Markdown source."},
        sources:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable nonempty source ID."},
            site:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Short source name for its chip."},
            title:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Heading or document name."},
            url:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Opaque destination passed to the application, never opened by the component."},
            date:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Ready-to-display date."},
            quote:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Evidence excerpt displayed and copied as literal text."},
            highlight:#{schema:#{type:"optional",value:#{type:"array",max_items:16,items:#{type:"string"}}},required:false,sensitive:false,
                doc:"Exact phrases to mark in the excerpt. Omit to mark words shared with structured citing statements."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative icon. Defaults to the site's first character."},
        }}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Evidence in presentation order; IDs must be unique."},
        pages_read:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Optional count supplied by the application; does not fetch pages."},
        default_open:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initial source ID to preview in local mode."},
        preview:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted source ID; use \"\" for closed, omit for local preview."},
        show_header:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Show the evidence summary above the answer."},
        show_tray:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Show the evidence caption below the preview."},
        action_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional tray action label, shown with on_action."},
        on_preview_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Source ID proposal or empty string to close."},
        on_open_source:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called only when Open source is activated; receives #{id,url}."},
        on_link:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Ordinary Markdown link activation #{href,text}."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Activate the optional tray button."},
    },state:#{fields:#{
        open:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        hot:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        copied:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        serial:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    }},events:#{
        preview_change:#{payload:#{type:"string"},doc:"Proposed source ID; empty closes."},
        open_source:#{payload:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false},
            url:#{schema:#{type:"string"},required:true,sensitive:false},
        }},doc:"Explicit request to open an opaque source destination."},
        link:#{payload:#{type:"object",allow_unknown:false,fields:#{
            href:#{schema:#{type:"string"},required:true,sensitive:false},
            text:#{schema:#{type:"string"},required:true,sensitive:false},
        }},doc:"Ordinary Markdown link activated."},
        action:#{payload:#{type:"null"},doc:"Tray action activated."},
    },slots:#{},parts:["root","embedded","card","header","title","summary","body","paragraph","sources","sources_label","chip","chip_active",
        "number","glyph","chip_label","tag","tag_active","annotation","preview","preview_header","preview_title","meta",
        "quote","quote_annotation","preview_actions","tray","caption","action"],effects:["retain_preview"]},
    render:Fn("render_CitedAnswer")
});

fn CitedAnswer(props){render_component("components/cited_answer",props)}
fn opt(props,key){if key in props{props[key]}else{()}}
fn allowed(props,key){opt(props,"on_"+key)!=()}
fn caption_number(ctx,n){ctx.format_number(n)}
fn source_uri(n){"gpui-cite:"+n.to_string()}
fn chip_ref(index){"source-row:"+index.to_string()}
fn index_sources(items){
    let index=#{};let links=#{};let ids=[];
    for item in items{
        if item.id==""||item.id in index{throw "CitedAnswer source IDs must be unique and nonempty";}
        ids.push(item.id);
        let uri=source_uri(ids.len);
        index[item.id]=#{number:ids.len,uri:uri};
        links[uri]=item.id;
    }
    #{index:index,links:links,ids:ids}
}
fn rich_body(ctx,segments,index,active){
    let markdown="";let plain=0;let ranges=[];let total=0;let cited=0;let counts=#{};
    let ends_line=false;
    for seg in segments{
        if seg.text==""{continue;}
        if utf8_byte_count(seg.text)>12000{throw "CitedAnswer segment exceeds 12000 bytes";}
        if seg.paragraph==true&&markdown!=""{markdown+="\n\n";plain+=if ends_line{1}else{2};}
        let text=seg.text;text.replace("\u0000","�");
        let bytes=utf8_byte_count(text);
        if plain+bytes>60000{throw "CitedAnswer visible text exceeds 60000 bytes";}
        let trimmed=text;trimmed.trim();
        if trimmed==""{
            markdown+=markdown_literal(text);plain+=bytes;ends_line=text.ends_with("\n");
            continue;
        }
        let from=text.index_of(trimmed);
        let leading=text.sub_string(0,from);
        let trailing=text.sub_string(from+trimmed.len);
        let start=plain+utf8_byte_count(leading);
        let ids=opt(seg,"cite");if ids==(){ids=[];}
        let seen=[];
        let marker="";
        for id in ids{
            if !(id in index)||id in seen{continue;}
            seen.push(id);counts[id]=if id in counts{counts[id]+1}else{1};
            let n=caption_number(ctx,index[id].number);
            let label="["+n+"]";
            marker+=`&#160;[${markdown_literal(label)}](${index[id].uri})`;
            plain+=2+utf8_byte_count(label);
        }
        total+=1;if seen.len>0{cited+=1;}
        if active!=""&&active in seen{ranges.push(#{start:start,end:start+utf8_byte_count(trimmed)});}
        markdown+=markdown_literal(leading+trimmed)+marker+markdown_literal(trailing);
        plain+=bytes;ends_line=text.ends_with("\n");
        if plain>60000{throw "CitedAnswer visible text exceeds 60000 bytes";}
    }
    #{source:markdown,ranges:ranges,cited:cited,total:total,counts:counts}
}
fn update_preview(ctx,spec){
    if ctx.get_state("open")==(){ctx.set_state("open",spec.open);}
    else if !spec.controlled&&ctx.get_state("open")!=spec.open{ctx.set_state("open",spec.open);}
    if ctx.get_state("hot")!=""&&!(ctx.get_state("hot") in spec.ids){ctx.set_state("hot","");}
}
fn cleanup(ctx,spec){}
fn show(config,ctx,id){
    if !(id in config.ids)||!config.enabled{return handled();}
    let next=if config.open==id{""}else{id};
    if !config.controlled{ctx.set_state("open",next);}
    ctx.emit("preview_change",next);handled()
}
fn close(config,ctx,payload){
    if config.open==""||!config.enabled{return propagate();}
    if !config.controlled{ctx.set_state("open","");}
    ctx.emit("preview_change","");
    ctx.focus(chip_ref(config.ids.index_of(config.open)));handled()
}
fn target(config,ctx,payload){
    if payload.href in config.links{show(config,ctx,config.links[payload.href]);}
    else if !payload.href.starts_with("gpui-cite:"){ctx.emit("link",payload);}
}
fn open_source(id,url,ctx,payload){ctx.emit("open_source",#{id:id,url:url});handled()}
fn tray_action(ctx,payload){ctx.emit("action",());handled()}
fn hover_source(ctx,event){
    if event.hovered{ctx.set_state("hot",event.value);}
    else if ctx.get_state("hot")==event.value{ctx.set_state("hot","");}
}
fn focus_source(ctx,event){
    if event.focused{ctx.set_state("hot",event.value);}
    else if ctx.get_state("hot")==event.value{ctx.set_state("hot","");}
}
fn move_chip(ids,index,delta,ctx,payload){
    let next=if delta==32{ids.len-1}else if delta== -32{0}else{min(max(index+delta,0),ids.len-1)};
    ctx.focus(chip_ref(next));handled()
}
fn document_body_style(){style().background(rgba(0x00000000)).border(px(0)).radius(px(0)).padding(px(0)).shadows([])}
fn copy_quote(id,text,ctx,payload){
    ctx.copy_text(text);
    let serial=ctx.get_state("serial")+1;ctx.set_state("serial",serial);
    ctx.set_state("copied",#{id:id,serial:serial});handled()
}
fn expire(ctx,value){if ctx.get_state("copied")==value{ctx.set_state("copied",());}}
fn logo(ctx,source){
    let icon=opt(source,"icon");
    let leaf=if icon==(){text(source.site.sub_string(0,1)).with_style(style().typography("caption").font_weight(600))}
        else{icon.with_style(style().width(px(14)).height(px(14)))};
    row([leaf]).with_key("icon").accessibility_hidden(true).with_style(ctx.component_style("glyph",style()
        .width(theme_length("space.unit").times(4.5)).height(theme_length("space.unit").times(4.5)).items_center().justify_center().flex_shrink(false)
        .radius(theme_radius("sm")).background(theme_color("surface_hover")).text_color(theme_color("text_primary"))))
}
fn source_chip(ctx,source,index,config){
    let is_open=config.open==source.id;
    let n=caption_number(ctx,index+1);
    let group=row([logo(ctx,source),text(source.site).with_key("name").with_style(ctx.component_style("chip_label",style()
        .whitespace_nowrap().text_ellipsis().min_width(px(0)))),text(n).with_key("number").with_style(ctx.component_style("number",style()
        .text_color(theme_color("text_muted")).flex_shrink(false)))])
        .with_key(source.id).with_ref(element_ref(chip_ref(index)))
        .accessibility_role("button").accessibility_label(source.site)
        .accessibility_expanded(is_open).accessibility_description(source.title)
        .disabled(!config.enabled).tab_stop(config.enabled)
        .with_style(ctx.component_style(if is_open{["chip","chip_active"]}else{["chip"]},style()
            .items_center().gap(theme_length("space.spacing_unit").times(1.5)).min_height(theme_length("space.unit").times(7)).min_width(px(0)).max_width(relative(1.0)).padding_x(theme_length("space.spacing_unit").times(2))
            .radius(theme_radius("sm")).border(px(1)).border_color(theme_color("border")).typography("caption")
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .paint_transition(175,cubic_bezier(0.22,1.0,0.36,1.0));
    if !config.enabled{return group;}
    let step=if ctx.text_direction()=="rtl"{-1}else{1};
    group.on_click_value(Fn("show").curry(config),source.id)
        .on_hover_value(Fn("hover_source"),source.id).on_focus_within_value(Fn("focus_source"),source.id)
        .on_key_value("left",Fn("move_chip").curry(config.ids,index,-step),())
        .on_key_value("right",Fn("move_chip").curry(config.ids,index,step),())
        .on_key_value("home",Fn("move_chip").curry(config.ids,index,-32),())
        .on_key_value("end",Fn("move_chip").curry(config.ids,index,32),())
}
fn preview_view(ctx,props,source,config,copied){
    let quote=opt(source,"quote");let date=opt(source,"date");let url=opt(source,"url");
    let details=source.site+(if date==(){""}else{" · "+date});
    let body=[
        row([logo(ctx,source),column([
            text(source.title).with_key("title").with_style(ctx.component_style("preview_title",style().font_weight(600).typography("control"))),
            text(details).with_key("details").with_style(ctx.component_style("meta",style().typography("caption").text_color(theme_color("text_muted")))),
        ]).with_style(style().min_width(px(0)).flex_grow()),
        button::Button(#{key:"close",text:ctx.t_or("cite.close","Close preview"),variant:"ghost",size:"sm",
            on_click:Fn("close").curry(config),disabled:!config.enabled})]).with_style(ctx.component_style("preview_header",style().gap(theme_length("space.spacing_unit").times(2)).items_center())),
    ];
    if quote!=(){
        let literal=quote;literal.replace("\u0000","�");
        let terms=opt(source,"highlight");let match_words=terms==();
        if match_words{
            terms=[];let size=0;
            for segment in props.segments{
                let ids=opt(segment,"cite");if ids==()||!(source.id in ids){continue;}
                size+=utf8_byte_count(segment.text);
                if size<=65536{terms.push(segment.text);}
            }
        }
        let annotations=if terms.len==0{[]}else{text_match_ranges(literal,terms,match_words)};
        let passage=markdown::MarkdownViewer(#{key:"quote-document",label:ctx.t_or("cite.quote","Evidence excerpt"),
            source:markdown_literal(literal),height:"content",max_height:200,annotations:annotations,
            part_styles:#{root:document_body_style(),
                paragraph:style().padding(px(0)),annotation:ctx.component_style("quote_annotation",
                    style().background(theme_color("selection"))) }});
        body.push(column([passage]).with_key("quote").with_style(ctx.component_style("quote",style()
            .min_width(px(0)).typography("body").border_left(px(2)).border_color(theme_color("accent")).padding_x(theme_length("space.spacing_unit").times(2.5)))));
    }
    let payload=(if quote==(){source.title}else{"“"+quote+"”"})+" — "+source.title+", "+source.site+
        (if url==(){""}else{" ("+url+")"});
    let actions=[button::Button(#{key:"copy",text:if copied{ctx.t_or("cite.copied","Quote copied")}else{ctx.t_or("cite.copy","Copy quote")},
        variant:"secondary",size:"sm",on_click:Fn("copy_quote").curry(source.id,payload)})];
    if url!=()&&url!=""&&allowed(props,"open_source"){actions.push(button::Button(#{key:"open",text:ctx.t_or("cite.open","Open source"),
        variant:"primary",size:"sm",on_click:Fn("open_source").curry(source.id,url)}));}
    body.push(row(actions).with_style(ctx.component_style("preview_actions",style().gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap())));
    column(body).with_key(source.id).accessibility_role("region").accessibility_label(ctx.t_or("cite.preview","Source preview"))
        .with_style(ctx.component_style("preview",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2.5)).gap(theme_length("space.spacing_unit").times(2)).radius(theme_radius("lg"))
            .border(px(1)).border_color(theme_color("border")).background(theme_color("surface_raised"))))
}
fn render_CitedAnswer(ctx,props){
    let data=index_sources(props.sources);
    let requested=opt(props,"preview");let previous=ctx.get_state("open");
    let raw=if requested!=(){requested}else if previous!=(){previous}else{opt(props,"default_open")};
    let open=if raw!=()&&raw in data.index{raw}else{""};
    let hot=ctx.get_state("hot");if !(hot in data.index){hot="";}
    let active=if hot!=""{hot}else{open};
    let config=#{open:open,ids:data.ids,links:data.links,controlled:requested!=(),enabled:requested==()||allowed(props,"preview_change")};
    effect("retain_preview",#{open:open,ids:data.ids,controlled:requested!=()},Fn("update_preview"),Fn("cleanup"));
    let target_source=opt(props,"source");let content=rich_body(ctx,props.segments,data.index,active);
    if target_source!=()&&props.segments.len>0{throw "CitedAnswer takes either source or segments";}
    let sources=caption_number(ctx,data.ids.len);
    let title=opt(props,"label");if title==(){title=ctx.t_or("cite.title","Cited answer");}
    let children=[];
    if props.show_header{
        let pages=opt(props,"pages_read");
        let summary=ctx.t_or("cite.sources","Sources: {count}",#{count:sources});
        if pages!=(){summary+=" · "+ctx.t_or("cite.pages","Pages read: {count}",#{count:caption_number(ctx,pages)});}
        children.push(row([
            text(title).with_key("title").accessibility_role("heading").accessibility_level(3)
                .with_style(ctx.component_style("title",style().typography("control").font_weight(600))),
            text(summary).with_key("summary").with_style(ctx.component_style("summary",style().typography("caption").text_color(theme_color("text_muted")))),
        ]).with_key("header").with_style(ctx.component_style("header",style().items_center().justify_between().flex_wrap().gap(theme_length("space.spacing_unit").times(1.5)))));
    }
    if content.source!=""||target_source!=(){
        let body=markdown::MarkdownViewer(#{key:"answer-document",label:title,source:if target_source!=(){target_source}else{content.source},
            height:"content",tag_link_prefix:"gpui-cite:",active_tag_link:if active==""{()}else{data.index[active].uri},
            annotations:content.ranges,on_link:Fn("target").curry(config),
            part_styles:#{root:ctx.component_style("body",document_body_style()),
                paragraph:ctx.component_style("paragraph",style().typography("body").padding_y(theme_length("space.spacing_unit").times(0.5))),
                tag:ctx.component_style("tag",style().text_color(theme_color("text.accent")).background(theme_color("surface_hover")).font_weight(600)),
                tag_active:ctx.component_style("tag_active",style().text_color(theme_color("text.accent")).background(theme_color("selection")).font_weight(600)),
                annotation:ctx.component_style("annotation",style().background(theme_color("selection"))) }});
        children.push(body);
    }
    if props.sources.len>0{
        let chips=[];
        for i in 0..props.sources.len{chips.push(source_chip(ctx,props.sources[i],i,config));}
        children.push(column([text(ctx.t_or("cite.source_label","Sources")).with_style(ctx.component_style("sources_label",style().typography("caption")
            .font_weight(500))),row(chips).with_key("chips").with_style(style().min_width(px(0)).flex_wrap().gap(theme_length("space.spacing_unit").times(1.5)))])
            .with_key("sources").with_style(ctx.component_style("sources",style().min_width(px(0)).gap(theme_length("space.spacing_unit")))));
    }
    let copied=ctx.get_state("copied");
    if copied!=(){timeout("quote-copy",1800,false,Fn("expire"),copied);}
    if open!=""{
        let source=props.sources[data.index[open].number-1];
        children.push(accordion::disclosure_panel(ctx,"preview",true,(),preview_view(ctx,props,source,config,copied!=()&&copied.id==open)));
    }
    let embedded=!props.show_header&&!props.show_tray;
    let presentation=if embedded{children}else{[column(children).with_key("card")
        .with_style(ctx.component_style("card",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(2))))]};
    if props.show_tray{
        let caption=if target_source!=(){ctx.t_or("cite.sources","Sources: {count}",#{count:sources})}
            else{ctx.t_or("cite.coverage","{cited} of {total} statements backed by a source",
                #{cited:caption_number(ctx,content.cited),total:caption_number(ctx,content.total)})};
        if hot!=""{
            let source=props.sources[data.index[hot].number-1];
            caption=source.site+" · "+source.title;
        }
        if copied!=(){caption=ctx.t_or("cite.copied","Quote copied");}
        let items=[text(caption).with_key("caption").accessibility_role("status")
            .with_style(ctx.component_style("caption",style().typography("caption").text_color(theme_color("text_muted"))))
            .motion(motion_transition("opacity",0.6,1.0,#{duration_ms:220,intent:"feedback"})).motion_on_change(caption)];
        let label=opt(props,"action_label");
        if label!=()&&label!=""&&allowed(props,"action"){
            items.push(button::Button(#{key:"action",text:label,variant:"secondary",size:"sm",on_click:Fn("tray_action")})
                .with_style(ctx.component_style("action",style())));
        }
        presentation.push(row(items).with_key("tray").with_style(ctx.component_style("tray",style().gap(theme_length("space.spacing_unit").times(2)).justify_between().items_center().flex_wrap())));
    }
    let root=column(presentation).with_key(props.key).accessibility_role("region").accessibility_label(title).tab_stop(false)
        .with_style(ctx.component_style(if embedded{["root","embedded"]}else{["root"]},style()
            .min_width(px(0)).gap(theme_length("space.spacing_unit").times(2)).text_color(theme_color("text_primary"))))
        .on_key_value("escape",Fn("close").curry(config),());
    // The standalone Modern card and the caption tray have distinct gutters.
    if !embedded&&ctx.theme_variant()!=()&&ctx.theme_token("design.name")=="modern"{root.audit_allow(["text-edge-misaligned"])}else{root}
}
