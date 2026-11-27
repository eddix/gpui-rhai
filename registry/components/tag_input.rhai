/* gpui-rhai
{"id":"components/tag_input","export":"TagInput","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/tag"],"capabilities":{},"assets":[],"tokens":["surface_raised","surface_hover","text_primary","text_muted","field.placeholder","border","focus_ring","danger","accent","selection","spacing.xs","radius.sm","radius.md","typography.control","typography.caption","space.spacing_unit","space.unit","typography.initials"],"environment":["size","density"]}
*/
// A single native editor for a controlled, ordered list of removable tags.
// Example: TagInput(#{ key:"labels", label:"Labels", values:["design"], on_change:Fn("labels_changed") })
import "components/tag" as tag;

define_component(#{
    metadata: #{
        id:"components/tag_input","export":"TagInput",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/tag"],capabilities:#{},assets:[],
        tokens:["surface_raised","surface_hover","text_primary","text_muted","field.placeholder","border","focus_ring","danger","accent","selection","spacing.xs","radius.sm","radius.md","typography.control","typography.caption","space.spacing_unit","space.unit","typography.initials"],
        environment:["size","density"],
    },
    schema: #{
        props: #{
            key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of the field and native editor."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible label and accessible name of the editor."},
            values:#{schema:#{type:"array",max_items:128,items:#{type:"string"}},required:true,sensitive:false,
                doc:"Controlled ordered, distinct tags. Apply each accepted on_change list as the new value."},
            hint:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Quiet explanation below the field until an error takes its place."},
            error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"External validation message; marks the editor invalid without erasing the draft."},
            placeholder:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Hint in an empty editor, including when chips already exist."},
            optional:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Shows Optional by the label."},
            avatar:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Draws a decorative initial avatar for each tag."},
            disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Displays the tags, but disables editor and removal."},
            split_on_space:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Treat spaces and tabs as separators in addition to commas, semicolons and line breaks."},
            normalize:#{schema:#{type:"string",allowed:["trim","lowercase"]},"default":#{type:"string",value:"trim"},required:false,sensitive:false,doc:"Trim incoming tags, optionally lowercasing before duplicate checks."},
            validation:#{schema:#{type:"string",allowed:["text","email"]},"default":#{type:"string",value:"text"},required:false,sensitive:false,doc:"Email requires a nonempty local part and a dotted domain, without whitespace."},
            max:#{schema:#{type:"optional",value:#{type:"integer",min:1,max:128}},required:false,sensitive:false,doc:"Maximum additions; a visible count appears when supplied."},
            max_characters:#{schema:#{type:"integer",min:1,max:256},"default":#{type:"integer",value:96},required:false,sensitive:false,doc:"Maximum grapheme clusters in each newly entered tag."},
            on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Proposes one full tag list for a single edit, batch paste or removal."},
        },
        state:#{fields:#{
            draft:#{schema:#{type:"string"},"default":#{type:"string",value:""},sensitive:true},
            error:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
            armed:#{schema:#{type:"string"},"default":#{type:"string",value:""},sensitive:true},
            flash:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
            arrival:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
            sequence:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
            announcement:#{schema:#{type:"string"},"default":#{type:"string",value:""},sensitive:true},
            pending:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
            live_model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
        }},
        events:#{change:#{payload:#{type:"array",max_items:128,items:#{type:"string"}},doc:"A complete controlled list proposal. A batch paste never emits intermediate lists."}},
        slots:#{},
        parts:["root","header","label","optional","field","invalid","editor","placeholder","chip","chip_body","chip_avatar","chip_armed","chip_remove","chip_remove_icon","chip_flash","footer","hint","error","count","count_full"],
        effects:["accepted_list"],
    },
    render:Fn("render_TagInput"),
});

fn TagInput(props){render_component("components/tag_input",props)}
fn opt(props,name){if name in props{props[name]}else{()}}
fn join_parts(parts,separator){
    let result="";
    for part in parts{
        if part!=""{if result!=""{result+=separator;}result+=part;}
    }
    result
}
fn split_tokens(raw, spaces){
    let normalized=raw;
    normalized.replace("\r\n",",");normalized.replace("\r",",");normalized.replace("\n",",");
    normalized.replace(";",",");
    if spaces{normalized.replace(" ",",");normalized.replace("\t",",");}
    normalized.split(",",257)
}
fn normalized_token(raw, mode) {
    let token=raw;token.trim();
    if mode=="lowercase"{token.to_lower()}else{token}
}
fn email_like(token){
    if token.contains(" ")||token.contains("\t"){return false;}
    let parts=token.split("@");
    if parts.len!=2||parts[0]==""{return false;}
    let domain=parts[1].split(".");
    if domain.len<2{return false;}
    for label in domain{if label==""{return false;}}
    true
}
fn same_tags(left,right){
    if left.len!=right.len{return false;}
    for i in 0..left.len{if left[i]!=right[i]{return false;}}
    true
}
fn accepted_list(ctx,data){
    let model=data[0];let actual=model.values;let pending=data[1];
    ctx.set_state("live_model",model);
    if ctx.get_state("armed")!=""&&(actual.len==0||ctx.get_state("armed")!=actual[actual.len-1]){
        ctx.set_state("armed","");
    }
    if pending==(){return;}
    if same_tags(actual,pending.want){
        ctx.set_state("announcement",pending.announcement);
        if pending.focus!=""{ctx.focus(pending.focus);}
        ctx.scroll_into_view(if pending.focus==""{"editor"}else{pending.focus});
        if pending.added.len>0{
            let sequence=ctx.get_state("sequence")+1;
            ctx.set_state("sequence",sequence);
            ctx.set_state("arrival",#{values:pending.added,sequence:sequence});
        }
    }else{ctx.set_state("draft",pending.rejected);}
    ctx.set_state("pending",());
}
fn accepted_cleanup(ctx,data){}
fn propose(ctx,list,rejected,announcement,focus,added){
    ctx.set_state("pending",#{want:list,rejected:rejected,announcement:announcement,focus:focus,added:added});
    ctx.emit("change",list);
}
fn flash_chip(ctx,token){
    let sequence=ctx.get_state("sequence")+1;
    ctx.set_state("sequence",sequence);
    ctx.set_state("flash",#{value:token,sequence:sequence});
    ctx.set_state("announcement",ctx.t_or("tag_input.duplicate","{tag} is already present",#{tag:token}));
}
// One proposal after validating the full batch, including duplicates within
// that batch. Keep every rejected token and incomplete suffix editable.
fn commit(model,ctx,parts,rest,rejected){
    if model.blocked{return;}
    if parts.len>=256{
        ctx.set_state("draft",rejected);
        ctx.set_state("error",ctx.t_or("tag_input.batch_limit","The draft is too long to add in one batch."));
        return;
    }
    let next=model.values;let seen=#{};
    for value in next{seen[value]=true;}
    let left=[];let added=[];let first_error="";let duplicate="";
    let separator=if model.spaces{" "}else{", "};
    for raw in parts{
        let token=normalized_token(raw,model.normalize);
        if token==""{continue;}
        let reason="";
        if grapheme_count(token)>model.characters{
            reason=ctx.t_or("tag_input.too_long","Each tag can have at most {max} characters.",#{max:ctx.format_number(model.characters)});
        }else if model.validation=="email"&&!email_like(token){
            reason=ctx.t_or("tag_input.email","Enter an email with a local part and dotted domain.");
        }else if token in seen{
            duplicate=token;continue;
        }else if next.len>=model.max{
            reason=ctx.t_or("tag_input.limit","Up to {max} tags.",#{max:ctx.format_number(model.max)});
        }
        if reason!=""{
            if first_error==""{first_error=reason;}
            left.push(token);
        }else{seen[token]=true;next.push(token);added.push(token);}
    }
    left.push(rest);
    ctx.set_state("draft",join_parts(left,separator));
    ctx.set_state("error",first_error);
    ctx.set_state("armed","");
    if duplicate!=""&&duplicate in model.values{flash_chip(ctx,duplicate);}
    let delta=next.len-model.values.len;
    if delta>0{
        let said=if delta==1{ctx.t_or("tag_input.added","{tag} added",#{tag:next[next.len-1]})}
            else{ctx.t_or("tag_input.added_many","{count} tags added",#{count:ctx.format_number(delta)})};
        propose(ctx,next,rejected,said,"",added);
    }
}
fn submit(model,ctx,raw){
    if raw.len>8192{
        ctx.set_state("draft",raw);
        ctx.set_state("error",ctx.t_or("tag_input.batch_limit","The draft is too long to add in one batch."));return;
    }
    commit(model,ctx,split_tokens(raw,model.spaces),"",raw);
}
fn edited(model,ctx,raw){
    if model.blocked{return;}
    ctx.set_state("armed","");
    ctx.set_state("draft",raw);
    ctx.set_state("error","");
    if raw.len>8192{
        ctx.set_state("error",ctx.t_or("tag_input.batch_limit","The draft is too long to add in one batch."));return;
    }
    let parts=split_tokens(raw,model.spaces);
    if parts.len>1{
        let rest=parts.pop();
        commit(model,ctx,parts,rest,raw);
    }
}
fn pasted(model,ctx,data){
    if model.blocked{return;}
    if data.clipboard.len>8192||data.value.len>8192{edited(model,ctx,data.value);return;}
    let probe=data.clipboard;probe.trim();
    if split_tokens(probe,model.spaces).len==1{edited(model,ctx,data.value);return;}
    let current=ctx.get_state("draft");
    let batch=join_parts([current,data.clipboard],", ");
    submit(model,ctx,batch);
}
fn blurred(model,ctx,payload){
    ctx.set_state("armed","");
    let raw=ctx.get_state("draft");
    if raw!=""&&ctx.get_state("error")==""&&!model.blocked{submit(model,ctx,raw);}
}
fn canceled(ctx,payload){
    ctx.set_state("draft","");ctx.set_state("error","");ctx.set_state("armed","");
}
fn focus_part(ctx,key){ctx.focus(key);ctx.scroll_into_view(key);handled()}
fn preceding(index,ctx,payload){focus_part(ctx,`chip-${index}`);}
fn remove_chip(keep_chip_focus,ctx,value){
    let model=ctx.get_state("live_model");
    if model==()||model.blocked{return handled();}
    let next=[];let removed=-1;
    for i in 0..model.values.len{
        if model.values[i]==value{removed=i;}else{next.push(model.values[i]);}
    }
    if removed<0{return handled();}
    ctx.set_state("armed","");
    let focus=if keep_chip_focus&&removed<next.len{`chip-${removed}`}else{"editor"};
    propose(ctx,next,ctx.get_state("draft"),
        ctx.t_or("tag_input.removed","{tag} removed",#{tag:value}),focus,[]);
    handled()
}
fn empty_backspace(model,ctx,payload){
    if model.blocked||model.values.len==0{return;}
    let value=model.values[model.values.len-1];
    if ctx.get_state("armed")==value{remove_chip(false,ctx,value);}
    else{ctx.set_state("armed",value);}
}
fn tag_chip(ctx,props,model,i){
    let value=model.values[i];let focus=`chip-${i}`;
    let armed=ctx.get_state("armed")==value;
    let shape=ctx.component_style(if armed{["chip","chip_armed"]}else{["chip"]},
        style().max_width(relative(1.0)).min_width(px(0)).height(theme_length("space.unit").times(6)).radius(theme_radius("sm"))
            .background(theme_color("surface_hover")).text_color(theme_color("text_primary")),
        if armed{style().background(theme_color("selection"))}else{style()});
    let avatar=if props.avatar{
        row([text(text_initial(value))]).with_style(ctx.component_style("chip_avatar",
            style().width(theme_length("space.unit").times(4.5)).height(theme_length("space.unit").times(4.5)).radius(theme_length("space.unit").times(249.75)).items_center().justify_center()
                .background(theme_color("surface_raised")).typography("initials")))
            .accessibility_hidden(true)
    }else{()};
    let chip=tag::Tag(#{key:value,text:value,prefix:avatar,closable:!model.blocked,close_tab_stop:false,
        part_styles:#{root:shape,value:ctx.component_style("chip_body",style()
            .padding_start(theme_length("space.spacing_unit").times(if props.avatar{0.75}else{2.0}))
            .padding_end(theme_length("space.spacing_unit").times(0.75)).gap(theme_length("space.spacing_unit").times(1.25))),
            close:ctx.component_style("chip_remove",style().width(theme_length("space.unit").times(4.5)).height(theme_length("space.unit").times(4.5)).radius(px(4))),
            close_icon:ctx.component_style("chip_remove_icon",style().width(px(12)).height(px(12)))},
        on_close:if model.blocked{()}else{Fn("remove_chip").curry(false)}});
    let contents=[chip];
    let flash=ctx.get_state("flash");
    if flash!=()&&flash.value==value{
        contents.push(box([]).with_key("duplicate-flash").with_style(ctx.component_style("chip_flash",
            style().absolute().top(px(0)).bottom(px(0)).left(px(0)).right(px(0))
                .radius(theme_radius("sm")).background(alpha(theme_color("accent"),0.14)).opacity(0.0)))
            .accessibility_hidden(true)
            .motion(motion_keyframes("opacity",[#{offset:0.0,value:0.0},#{offset:0.2,value:1.0},
                #{offset:0.6,value:1.0},#{offset:1.0,value:0.0}],#{duration_ms:700}))
            .motion_on_change(`${flash.sequence}`));
    }
    let node=row(contents).with_key(value).with_ref(element_ref(focus))
        .with_style(style().relative().flex_shrink(false).min_width(px(0)).max_width(relative(1.0))
            .group_focus(style()).focus_ring(px(2),px(2),theme_color("focus_ring")))
        .tab_stop(false).accessibility_role("group").accessibility_label(value);
    let arrival=ctx.get_state("arrival");
    if arrival!=()&&value in arrival.values{
        let timing=#{duration_ms:220,easing:cubic_bezier(0.22,1.0,0.36,1.0)};
        node=node.motion(motion_transition("opacity",0.0,1.0,timing))
            .motion(motion_transition("scale_x",0.85,1.0,timing))
            .motion(motion_transition("scale_y",0.85,1.0,timing))
            .motion(motion_transition("foreground_blur",2.0,0.0,timing))
            .motion_on_change(`${arrival.sequence}`);
    }
    if model.blocked{return node;}
    let backwards=if ctx.text_direction()=="rtl"{"right"}else{"left"};
    let forwards=if backwards=="left"{"right"}else{"left"};
    node.on_key_value(backwards,Fn("focus_part"),`chip-${max(0,i-1)}`)
        .on_key_value(forwards,Fn("focus_part"),if i+1==model.values.len{"editor"}else{`chip-${i+1}`})
        .on_key_value("backspace",Fn("remove_chip").curry(true),value)
        .on_key_value("delete",Fn("remove_chip").curry(true),value);
}
fn render_TagInput(ctx,props){
    let max=opt(props,"max");
    let blocked=props.disabled||opt(props,"on_change")==();
    let model=#{values:props.values,blocked:blocked,max:if max==(){128}else{max},spaces:props.split_on_space,
        normalize:props.normalize,validation:props.validation,characters:props.max_characters};
    let known=#{};
    for value in props.values{
        let trimmed=value;trimmed.trim();
        if trimmed==""||value in known{throw "TagInput values require distinct nonempty strings";}
        known[value]=true;
    }
    let draft=ctx.get_state("draft");
    let pending=ctx.get_state("pending");
    effect("accepted_list",[model,pending],Fn("accepted_list"),Fn("accepted_cleanup"));
    let failure=opt(props,"error");
    if failure==()||failure==""{failure=ctx.get_state("error");}
    let invalid=failure!="";
    let hint=opt(props,"hint");
    let message=if invalid{failure}else if hint==(){""}else{hint};
    let full=props.values.len>=model.max;
    let placeholder=opt(props,"placeholder");
    if placeholder==(){placeholder=if props.values.len==0{ctx.t_or("tag_input.add","Add…")}else{ctx.t_or("tag_input.add_more","Add more…")};}
    if full{placeholder="";}
    let editor=gpui_rhai::TextInputPrimitive(#{key:"editor",value:draft,placeholder:placeholder,typography:"control",
        placeholder_style:ctx.component_style("placeholder",style().text_color(theme_color("field.placeholder"))),
        disabled:blocked,submit_on_tab:true,on_change:Fn("edited").curry(model),
        on_submit:if draft==""{()}else{Fn("submit").curry(model)},on_paste:Fn("pasted").curry(model),
        on_blur:Fn("blurred").curry(model),on_cancel:Fn("canceled"),
        on_move_before:if model.values.len==0{()}else{Fn("preceding").curry(model.values.len-1)},
        on_empty_backspace:if model.values.len==0{()}else{Fn("empty_backspace").curry(model)},
    }).with_ref(element_ref("editor")).with_style(ctx.component_style("editor",style().min_width(px(80))
        .flex_basis(px(80)).flex_grow().height(theme_length("space.unit").times(6)).padding_x(theme_length("space.spacing_unit").times(1.5)).typography("control")))
        .disabled(blocked).accessibility_role("text_field").accessibility_label(props.label)
        .accessibility_value(draft).accessibility_placeholder(placeholder).accessibility_invalid(invalid)
        .accessibility_description(message);
    let children=[];
    for i in 0..props.values.len{children.push(tag_chip(ctx,props,model,i));}
    children.push(editor);
    let field=style().flex_wrap().items_center().gap(theme_length("space.spacing_unit")).min_height(theme_length("space.unit").times(9)).max_height(px(168))
        .padding_x(theme_length("space.spacing_unit").times(1.5)).padding_y(theme_length("space.spacing_unit").times(1.25)).min_width(px(0)).width(relative(1.0))
        .radius(theme_radius("md")).background(theme_color("surface_raised"))
        .border(px(1)).border_color(theme_color("border")).cursor_text()
        .group_focus(style().border_color(theme_color("focus_ring"))).overflow_y_scroll();
    let well=row(children).with_key("field").with_style(ctx.component_style(if invalid{["field","invalid"]}else{["field"]},field,
        if invalid{style().border_color(theme_color("danger")).group_focus(style().border_color(theme_color("danger")))}else{style()}))
        .paint_transition(140,cubic_bezier(0.22,1.0,0.36,1.0))
        .tab_stop(false).accessibility_role("group");
    if !blocked{well=well.on_click_value(Fn("focus_part"),"editor");}
    let heading=[text(props.label).with_style(ctx.component_style("label",style().font_weight(500).typography("control").min_width(px(0))))];
    if props.optional{heading.push(text(ctx.t_or("tag_input.optional","Optional")).with_key("optional-marker")
        .with_style(ctx.component_style("optional",style().typography("caption").text_color(theme_color("text_muted")).flex_shrink(false))));}
    let header=row(heading).with_key("header").with_style(ctx.component_style("header",style().items_baseline().gap(theme_spacing("xs"))));
    // The optional marker is a secondary typographic voice on the label baseline.
    if props.optional{header=header.audit_allow(["mixed-type-in-row"]);}
    if !blocked{header=header.on_click_value(Fn("focus_part"),"editor").tab_stop(false).accessibility_role("presentation");}
    let rows=[header,well];
    let foot=[];
    if message!=""{
        let line=text(message).with_key(if invalid{"error"}else{"hint"}).with_style(ctx.component_style(if invalid{"error"}else{"hint"},
            style().typography("caption").min_width(px(0)).flex_grow()
                .text_color(theme_color(if invalid{"danger"}else{"text_muted"}))));
        if invalid{line=line.accessibility_role("status");}
        foot.push(line);
    }
    if max!=(){
        if foot.len==0{foot.push(row([]).with_style(style().flex_grow()).accessibility_hidden(true));}
        foot.push(text(`${ctx.format_number(props.values.len)}/${ctx.format_number(max)}`).with_key("count")
            .with_style(ctx.component_style(if full{["count","count_full"]}else{["count"]},
                style().typography("caption").text_color(theme_color("text_muted")).flex_shrink(false)))
            .accessibility_role("status"));
    }
    if foot.len>0{rows.push(row(foot).with_key("footer").with_style(ctx.component_style("footer",style().gap(theme_length("space.spacing_unit").times(3)).justify_between())));}
    let said=ctx.get_state("announcement");
    if said!=""{rows.push(text(said).with_style(style().absolute().width(px(1)).height(px(1)).clip().opacity(0.0)).accessibility_role("status"));}
    let control=column(rows).with_key(props.key).disabled(props.disabled).with_style(ctx.component_style("root",style().relative().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.75))
        .text_color(theme_color("text_primary")).merge(if props.disabled{style().opacity(0.55)}else{style()})));
    // Modern intentionally insets label/hint by 2px relative to the well.
    if ctx.theme_variant()!=()&&ctx.theme_token("design.name")=="modern"{control.audit_allow(["text-edge-misaligned"])}else{control}
}
