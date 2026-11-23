/* gpui-rhai
{"id":"patterns/prompt_library","export":"PromptLibrary","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/badge","components/button","components/input","components/markdown_viewer","components/textarea","patterns/list_detail"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","surface_raised","surface_hover","text_primary","text_muted","radius.lg","radius.sm","selection","typography.body","typography.caption","space.spacing_unit","space.unit","text.accent","typography.control","typography.title"],"environment":["density"]}
*/
// PromptLibrary lets the user find templates, edit variables and propose a prompt.
// State: accepted selection can be controlled; query and drafts are component-local.
// Example: PromptLibrary(#{key:"prompts",templates:[#{id:"summary",title:"Summarize",body:"Write for {audience}."}]})
import "components/badge" as badge;
import "components/button" as button;
import "components/input" as input;
import "components/markdown_viewer" as markdown;
import "components/textarea" as textarea;
import "patterns/list_detail" as catalog;

define_component(#{
    metadata:#{id:"patterns/prompt_library","export":"PromptLibrary",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/badge","components/button","components/input","components/markdown_viewer","components/textarea","patterns/list_detail"],
        capabilities:#{},assets:[],tokens:["border","focus_ring","surface_raised","surface_hover","text_primary","text_muted","radius.lg","radius.sm","selection","typography.body","typography.caption","space.spacing_unit","space.unit","text.accent","typography.control","typography.title"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable library identity."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accessible and visible library name."},
        templates:#{schema:#{type:"array",max_items:128,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique, nonempty template ID."},
            title:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Template title and primary search term."},
            category:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Short badge and additional search term."},
            tone:#{schema:#{type:"optional",value:#{type:"string",allowed:["blue","violet","amber","green","red","gray"]}},required:false,sensitive:false,
                doc:"Optional category color; otherwise assigned by category order."},
            body:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Literal template, up to 32 KiB; {name} placeholders are not code."},
            variables:#{schema:#{type:"map",values:#{type:"object",allow_unknown:false,fields:#{
                "default":#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Initial variable text, at most 512 graphemes and 4096 UTF-8 bytes."},
                description:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Hint while the editor has focus."},
            }}},required:false,sensitive:false,doc:"Optional details keyed by placeholder name."},
        }}},required:true,sensitive:false,doc:"Ordered templates to search by title, category or literal body."},
        value:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accepted template ID; empty means no accepted selection. Omit for local selection."},
        default_value:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initial local selection; defaults to the first catalog entry."},
        use_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional label for the proposed-use action."},
        on_select:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Template ID proposal."},
        on_use:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives #{id,text} with the filled prompt. No evaluation or submission is performed."},
        on_copy:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Template ID copied in the input transaction; does not duplicate raw text in the event."},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Blocks input and proposals while keeping the preview visible."},
    },state:#{fields:#{
        selected:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        query:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        drafts:#{schema:#{type:"ui_value"},"default":#{type:"map",value:#{}},sensitive:true},
        navigation:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        editing:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
        hint:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        notice:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        serial:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    }},events:#{
        select:#{payload:#{type:"string"},doc:"Accept or reject the requested template ID."},
        copy:#{payload:#{type:"string"},doc:"The template ID for a requested clipboard write."},
        "use":#{payload:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false},
            text:#{schema:#{type:"string"},required:true,sensitive:true},
        }},doc:"A fully filled prompt, proposed as literal text."},
    },slots:#{},parts:["root","card","sidebar","side_head","title","total","search","search_input","search_icon","list","row","selected","row_label","badge",
        "detail","detail_title","body","variable","variable_active","fields","field","field_label","field_editor","foot","caption","copy","use","empty"],
        effects:["catalog"]},
    render:Fn("render_PromptLibrary")
});

fn PromptLibrary(props){render_component("patterns/prompt_library",props)}
fn opt(map,key){catalog::catalog_opt(map,key)}
fn query_changed(ctx,value){ctx.set_state("query",value);}
fn clear_search(ctx,payload){if ctx.get_state("query")==""{return propagate();}ctx.set_state("query","");handled()}
fn clean_scope(ctx,model){
    if !model.controlled{ctx.set_state("selected",model.accepted);}
    let old=ctx.get_state("drafts");let kept=#{};
    for id in model.ids{if id in old{kept[id]=old[id];}}
    ctx.set_state("drafts",kept);
    ctx.set_state("navigation",model.navigation);
    ctx.set_state("editing",model.editor);
}
fn cleanup(ctx,model){}
fn choose(cfg,ctx,id){
    if cfg.enabled&&id in cfg.refs&&cfg.selected!=id{
        if !cfg.controlled{ctx.set_state("selected",id);}
        ctx.emit("select",id);
    }
    handled()
}
fn row_click(ctx,id){let cfg=ctx.get_state("navigation");if cfg!=(){choose(cfg,ctx,id);}handled()}
fn go_to(origin,step,ctx,payload){
    let cfg=ctx.get_state("navigation");if cfg==(){return propagate();}
    let target=if step==0{if cfg.selected in cfg.refs{cfg.selected}else{catalog::catalog_neighbor(cfg.order,"",1,false)}}
        else{catalog::catalog_neighbor(cfg.order,origin,step,false)};
    if target==""||!cfg.enabled{return propagate();}
    choose(cfg,ctx,target);ctx.focus(cfg.refs[target]);ctx.scroll_into_view(cfg.refs[target]);handled()
}
fn tone_variant(tone){
    if tone=="blue"{"info"}else if tone=="violet"{"purple"}else if tone=="green"{"success"}
    else if tone=="amber"{"warning"}else if tone=="red"{"danger"}else{"neutral"}
}
fn category_tag(ctx,label,tone){
    badge::Badge(#{text:label,variant:tone_variant(tone)})
        .with_style(ctx.component_style("badge",style().flex_shrink(false)))
}
fn index_templates(ctx,templates,term){
    let all=#{};let refs=#{};let order=[];let ids=[];let tones=#{};let chosen=#{};
    let palette=["blue","violet","amber","green","red","gray"];let n=0;
    for item in templates{
        if item.id==""||item.id in all{throw "PromptLibrary template IDs must be unique and nonempty";}
        let category=opt(item,"category");let tone=opt(item,"tone");
        if category!=()&&category!=""&&!(category in tones){tones[category]=palette[n%palette.len];n+=1;}
        chosen[item.id]=if tone!=(){tone}else if category!=()&&category in tones{tones[category]}else{"gray"};
        all[item.id]=item;
        if catalog::catalog_match(term,[item.title,category,item.body]){
            refs[item.id]="template:"+ids.len.to_string();order.push(item.id);
        }
        ids.push(item.id);
    }
    #{all:all,refs:refs,order:order,ids:ids,tones:chosen}
}
fn template_row(ctx,item,tone,cfg){
    let category=opt(item,"category");
    let selected=cfg.selected==item.id;
    let pieces=[text(item.title).with_style(ctx.component_style("row_label",style().min_width(px(0)).flex_grow()
        .whitespace_nowrap().text_ellipsis()))];
    if category!=()&&category!=""{pieces.push(category_tag(ctx,category,tone));}
    let paint=ctx.component_style(if selected{["row","selected"]}else{["row"]},style()
        .min_height(theme_length("space.unit").times(8.5)).min_width(px(0)).items_center().gap(theme_length("space.spacing_unit")).padding_x(theme_length("space.spacing_unit").times(1.5))
        .radius(theme_radius("sm")).typography("control").focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))
        .background(if selected{theme_color("selection")}else{rgba(0x00000000)})
        .hover(style().background(theme_color("surface_hover"))));
    let row=catalog::catalog_option(item.id,item.title,selected,cfg.enabled,pieces,paint)
        .with_ref(element_ref(cfg.refs[item.id])).disabled(!cfg.enabled);
    if !cfg.enabled{return row;}
    row.on_click_value(Fn("row_click"),item.id)
        .on_key_value("down",Fn("go_to").curry(item.id,1),()).on_key_value("up",Fn("go_to").curry(item.id,-1),())
        .on_key_value("home",Fn("go_to").curry(item.id,-256),()).on_key_value("end",Fn("go_to").curry(item.id,256),())
}
fn parse_prompt(ctx,template){
    let drafts=ctx.get_state("drafts");let cached=if template.id in drafts{drafts[template.id]}else{#{}};
    let vars=opt(template,"variables");if vars==(){vars=#{};}
    let parts=prompt_template_parts(template.body);let names=[];let values=#{};let missing=[];
    let raw="";let shown="";let links=#{};let size=0;let overflow=false;
    for item in parts{
        if "text" in item{
            size+=utf8_byte_count(item.text);
            if size>64000{overflow=true;}
            if !overflow{raw+=item.text;shown+=markdown_literal(item.text);}
            continue;
        }
        let name=item.name;
        if !(name in values){
            let spec=if name in vars{vars[name]}else{#{}};
            let value=if name in cached{cached[name]}else{opt(spec,"default")};
            if value==(){value="";}
            if utf8_byte_count(value)>4096||grapheme_count(value)>512{throw "PromptLibrary variable defaults must fit 4096 bytes and 512 graphemes";}
            let trimmed=value;trimmed.trim();if trimmed==""{missing.push(name);}
            names.push(name);values[name]=value;
        }
        let label=values[name];label.trim();
        let result=if label==""{"{"+name+"}"}else{values[name]};
        let refkey="variable:"+names.index_of(name).to_string();
        let href="gpui-prompt:"+names.index_of(name).to_string();
        links[href]=refkey;
        size+=utf8_byte_count(result);
        if size>64000{overflow=true;}
        if !overflow{raw+=result;shown+="["+markdown_literal(result)+"]("+href+")";}
    }
    if overflow{raw="";shown=markdown_literal(ctx.t_or("prompt.large","Prompt exceeds the editing limit."));links=#{};}
    #{raw:raw,shown:shown,names:names,values:values,missing:missing,links:links,vars:vars,parts:parts,overflow:overflow}
}
fn edited(id,name,ctx,value){
    if utf8_byte_count(value)>4096{set_notice(ctx,"large");return;}
    let drafts=ctx.get_state("drafts");let values=if id in drafts{drafts[id]}else{#{}};
    values[name]=value;drafts[id]=values;ctx.set_state("drafts",drafts);
}
fn hint_in(id,name,ctx,event){ctx.set_state("hint",#{id:id,name:name});}
fn hint_out(id,name,ctx,event){
    if ctx.get_state("hint")==#{id:id,name:name}{ctx.set_state("hint",());}
}
fn marker_click(links,ctx,payload){if payload.href in links{ctx.focus(links[payload.href]);}}
fn set_notice(ctx,kind){
    let n=ctx.get_state("serial")+1;ctx.set_state("serial",n);
    ctx.set_state("notice",#{kind:kind,serial:n});
}
fn expire(ctx,expected){if ctx.get_state("notice")==expected{ctx.set_state("notice",());}}
fn use_prompt(id,text,ctx,event){ctx.emit("use",#{id:id,text:text});set_notice(ctx,"used");handled()}
fn copy_prompt(id,text,ctx,event){ctx.copy_text(text);ctx.emit("copy",id);set_notice(ctx,"copy");handled()}
fn missing_after(fields,current,missing){
    if missing.len==0{return "";}
    for offset in 1..=fields.len{
        let next=(current+offset)%fields.len;
        if fields[next] in missing{return "variable:"+next.to_string();}
    }
    ""
}
fn next_field(id,name,ctx,value){
    let editor=ctx.get_state("editing");
    if editor==()||editor.id!=id||!(name in editor.names){return handled();}
    if utf8_byte_count(value)>4096{set_notice(ctx,"large");return handled();}
    let values=editor.values;
    values[name]=value;
    edited(id,name,ctx,value);
    let missing=[];
    for field in editor.names{
        let trimmed=values[field];trimmed.trim();if trimmed==""{missing.push(field);}
    }
    let target=missing_after(editor.names,editor.names.index_of(name),missing);
    if target!=""{ctx.focus(target);}
    else if editor.can_use{
        let resolved="";let size=0;
        for part in editor.parts{
            let piece=if "name" in part{values[part.name]}else{part.text};
            size+=utf8_byte_count(piece);
            if size>64000{set_notice(ctx,"large");return handled();}
            resolved+=piece;
        }
        use_prompt(id,resolved,ctx,());
    }
    handled()
}
fn detail(ctx,props,item,projection,tone){
    let title=text(item.title).with_key("detail-title").accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("detail_title",style().typography("title").min_width(px(0))))
        .exit_motion(motion_transition("opacity",1.0,0.0,#{duration_ms:145,intent:"feedback"}));
    let heading=[title];let cat=opt(item,"category");if cat!=()&&cat!=""{heading.push(category_tag(ctx,cat,tone));}
    let body=[row(heading).with_style(style().items_center().gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap())];
    let hint=ctx.get_state("hint");
    let tag=if hint!=()&&hint.id==item.id&&hint.name in projection.names{
        "gpui-prompt:"+projection.names.index_of(hint.name).to_string()
    }else{()};
    body.push(markdown::MarkdownViewer(#{key:"preview",label:ctx.t_or("prompt.preview","Prompt preview"),source:projection.shown,
        height:"content",max_height:208,tag_link_prefix:"gpui-prompt:",active_tag_link:tag,
        on_link:if props.disabled{()}else{Fn("marker_click").curry(projection.links)},
        part_styles:#{root:style().background(rgba(0x00000000)).border(px(0)).padding(px(0)).shadows([]),
            paragraph:ctx.component_style("body",style().padding_y(theme_length("space.spacing_unit").times(0.5)).typography("body")),
            tag:ctx.component_style("variable",style().background(theme_color("surface_hover")).text_color(theme_color("text.accent"))),
            tag_active:ctx.component_style("variable_active",style().background(theme_color("selection")).text_color(theme_color("text.accent")))}}));
    let fields=[];
    for i in 0..projection.names.len{
        let name=projection.names[i];
        let field=textarea::Textarea(#{key:name,label:name,rows:1,max_length:512,value:projection.values[name],
            editor_ref:element_ref("variable:"+i.to_string()),disabled:props.disabled,
            on_change:Fn("edited").curry(item.id,name),submit_key:"enter",
            on_submit:Fn("next_field").curry(item.id,name),
            on_focus:Fn("hint_in").curry(item.id,name),on_blur:Fn("hint_out").curry(item.id,name),
            part_styles:#{editor:ctx.component_style("field_editor",style().padding_x(theme_length("space.spacing_unit").times(1.75)).padding_y(theme_length("space.spacing_unit").times(0.75))
                .typography("control").radius(theme_radius("sm")))}});
        fields.push(column([text(name).with_style(ctx.component_style("field_label",style().typography("caption"))),field])
            .with_key(name).with_style(ctx.component_style("field",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(0.5)))));
    }
    if fields.len>0{body.push(column(fields).with_style(ctx.component_style("fields",style().gap(theme_length("space.spacing_unit").times(1.5)).min_width(px(0)))));}
    column(body).with_key(item.id).with_style(ctx.component_style("detail",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(3)).gap(theme_length("space.spacing_unit").times(2))))
        .motion(motion_transition("opacity",0.5,1.0,#{duration_ms:240,intent:"feedback"}))
}
fn render_PromptLibrary(ctx,props){
    let query=ctx.get_state("query");let data=index_templates(ctx,props.templates,catalog::catalog_query(query));
    let external=opt(props,"value");let local=ctx.get_state("selected");
    let seed=if local!=(){local}else{opt(props,"default_value")};
    let selected=if external!=(){external}else if seed!=()&&seed in data.all{seed}else if data.ids.len==0{""}else{data.ids[0]};
    let item=if selected in data.all{data.all[selected]}else{()};
    let cfg=#{order:data.order,refs:data.refs,selected:selected,controlled:external!=(),
        enabled:!props.disabled&&(external==()||opt(props,"on_select")!=())};
    let reference=element_ref("library");let narrow=ctx.element_width_class(reference,[570])==0;
    let name=opt(props,"title");if name==(){name=ctx.t_or("prompt.title","Prompts");}
    let search_label=ctx.t_or("prompt.search","Search templates");
    let search=input::Input(#{key:"search",label:search_label,value:query,placeholder:search_label,appearance:"embedded",
        disabled:props.disabled,on_change:Fn("query_changed")}).with_style(ctx.component_style("search_input",style().min_width(px(0)).flex_grow()))
        .on_key_value("down",Fn("go_to").curry("",0),()).on_key_value("escape",Fn("clear_search"),());
    let sidebar=[row([text(name).with_style(ctx.component_style("title",style().typography("control").font_weight(600))),
        text(ctx.format_number(data.ids.len)).with_style(ctx.component_style("total",style().typography("caption").text_color(theme_color("text_muted"))))])
        .with_style(ctx.component_style("side_head",style().items_center().justify_between().gap(theme_length("space.spacing_unit"))))
        .audit_allow(["mixed-type-in-row"]),
        row([catalog::catalog_search_icon().with_style(ctx.component_style("search_icon",style().width(theme_length("space.unit").times(3.75)).height(theme_length("space.unit").times(3.75)).flex_shrink(false))),search])
            .with_style(ctx.component_style("search",style().items_center().gap(theme_length("space.spacing_unit").times(1.25)).min_width(px(0))))];
    let rows=[];for id in data.order{rows.push(template_row(ctx,data.all[id],data.tones[id],cfg));}
    if rows.len==0{rows.push(text(ctx.t_or("prompt.empty","No templates match")).with_style(ctx.component_style("empty",style().padding(theme_length("space.spacing_unit").times(1.5)))));}
    sidebar.push(column(rows).with_key("list").accessibility_role("listbox").accessibility_label(ctx.t_or("prompt.templates","Templates"))
        .audit_allow(["text-edge-misaligned"])
        .with_style(ctx.component_style("list",style().min_height(px(0)).max_height(px(if narrow{92}else{344})).min_width(px(0)).overflow_y_scroll())));
    let project=if item==(){()}else{parse_prompt(ctx,item)};
    let edited_model=if item==(){()}else{#{id:item.id,names:project.names,values:project.values,parts:project.parts,
        can_use:!props.disabled&&opt(props,"on_use")!=()}};
    effect("catalog",#{ids:data.ids,controlled:external!=(),accepted:selected,navigation:cfg,editor:edited_model},Fn("clean_scope"),Fn("cleanup"));
    let pane=if item==(){text(ctx.t_or("prompt.none","Choose a template")).with_style(ctx.component_style("empty",style().padding(theme_length("space.spacing_unit").times(3))))}
        else{detail(ctx,props,item,project,data.tones[item.id])};
    let main=catalog::ListDetail(#{label:name,list:column(sidebar).with_style(ctx.component_style("sidebar",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2)).gap(theme_length("space.spacing_unit")))),
        detail:pane,orientation:if narrow{"vertical"}else{"horizontal"},list_size:px(if narrow{170}else{226})})
        .with_key("body").with_style(ctx.component_style("card",style().min_width(px(0))));
    let views=[main];let notice=ctx.get_state("notice");
    if notice!=(){timeout("notice",1800,false,Fn("expire"),notice);}
    if item!=(){
        let label=opt(props,"use_label");if label==(){label=ctx.t_or("prompt.use","Use prompt");}
        let hint=ctx.get_state("hint");
        let caption=ctx.t_or("prompt.estimate","≈{count} tokens",#{count:ctx.format_number(max(1,(grapheme_count(project.raw)+2)/4))});
        if project.missing.len>0{caption=ctx.t_or("prompt.missing","Fill {count} more fields",#{count:ctx.format_number(project.missing.len)});}
        if hint!=()&&hint.id==item.id&&hint.name in project.vars{
            let description=opt(project.vars[hint.name],"description");
            if description!=(){caption=hint.name+" · "+description;}
        }
        if notice!=(){caption=if notice.kind=="copy"{ctx.t_or("prompt.copied","Copied to clipboard")}
            else if notice.kind=="large"{ctx.t_or("prompt.large","Prompt exceeds the editing limit.")}
            else{ctx.t_or("prompt.requested","Prompt use requested")};}
        if project.overflow{caption=ctx.t_or("prompt.large","Prompt exceeds the editing limit.");}
        let footer=[
            text(caption).with_key("status").accessibility_role("status").with_style(ctx.component_style("caption",style().min_width(px(0)).typography("caption").text_color(theme_color("text_muted"))))
                .motion(motion_transition("opacity",0.6,1.0,#{duration_ms:190,intent:"feedback"})).motion_on_change(caption),
            row([
                button::Button(#{key:"copy",text:ctx.t_or("prompt.copy","Copy prompt"),size:"sm",variant:"ghost",disabled:props.disabled||project.overflow,
                    on_click:Fn("copy_prompt").curry(item.id,project.raw)}).with_style(ctx.component_style("copy",style())),
                button::Button(#{key:"use",text:label,size:"sm",disabled:props.disabled||project.overflow||project.missing.len>0||opt(props,"on_use")==(),
                    on_click:Fn("use_prompt").curry(item.id,project.raw)}).with_style(ctx.component_style("use",style())),
            ]).with_style(style().gap(theme_length("space.spacing_unit")).items_center().flex_wrap())
        ];
        views.push(row(footer).with_key("foot").with_style(ctx.component_style("foot",style().min_width(px(0)).justify_between().items_center().gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap())));
    }
    let root=column(views).with_key(props.key).with_ref(reference).with_style(ctx.component_style("root",style().width(relative(1.0)).min_width(px(0))
        .gap(theme_length("space.spacing_unit")).padding(theme_length("space.spacing_unit")).radius(theme_radius("lg")).background(theme_color("surface_raised"))));
    let theme=ctx.theme_variant();
    // The standalone Modern tray and the library card have separate gutters.
    if theme!=()&&theme.family=="Modern"{root.audit_allow(["text-edge-misaligned"])}else{root}
}
