/* gpui-rhai
{"id":"patterns/email_draft","export":"EmailDraft","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/popover","components/textarea","components/spinner"],"capabilities":{},"assets":[],"tokens":["accent","border","design.name","email_draft.account","email_draft.address","email_draft.card","email_draft.tray","email_draft.warn","email_draft.warn_bg","email_draft.ok","email_draft.ok_bg","email_draft.chip","email_draft.file","email_draft.link","email_draft.link_underline","email_draft.send","email_draft.on_send","email_draft.hint","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text.danger","text_muted","text_primary","typography.body","typography.caption","typography.key_legend","typography.metadata","typography.label"],"environment":[]}
*/
// Review an authored email without giving this view transport or file access.
// Example: EmailDraft(#{key:"renewal",draft:"renewal",revision:0,contacts:[#{id:"finance",name:"Ada Lovelace",email:"ada@example.test"}],tones:[#{id:"brief",label:"Brief",body:"Hi {first}, your plan renews soon."}]})
import "components/popover" as pop;
import "components/textarea" as editor;
import "components/spinner" as spinner;

define_component(#{
    metadata:#{id:"patterns/email_draft","export":"EmailDraft",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/popover","components/textarea","components/spinner"],capabilities:#{},assets:[],
        tokens:["accent","border","design.name","email_draft.account","email_draft.address","email_draft.card","email_draft.tray","email_draft.warn","email_draft.warn_bg","email_draft.ok","email_draft.ok_bg","email_draft.chip","email_draft.file","email_draft.link","email_draft.link_underline","email_draft.send","email_draft.on_send","email_draft.hint","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text.danger","text_muted","text_primary","typography.body","typography.caption","typography.key_legend","typography.metadata","typography.label"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable card instance."},
        draft:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Business ID. Changing it resets local edits, recipient, attachment and tone."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted Host revision carried by requests."},
        phase:#{schema:#{type:"string",allowed:["draft","sending","sent","discarded"]},required:false,sensitive:false,
            "default":#{type:"string",value:"draft"},doc:"Only the Host may enter a pending or receipt phase."},
        reference:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Record reference."},
        account:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Account caption."},
        amount:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Formatted amount."},
        status:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Record status chip."},
        agent:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Draft author caption."},
        contacts:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Distinct recipient ID."},
            name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Recipient name."},
            email:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Displayed address. The Host resolves the recipient before sending."}
        }}},required:true,sensitive:true,doc:"Available recipients. An empty list disables sending."},
        tones:#{schema:#{type:"array",max_items:12,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Distinct variant ID."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Tone button caption."},
            body:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Literal text with optional {first} and {name} tokens; not executable."}
        }}},required:true,sensitive:true,doc:"Generated text variants in display order. A recipient change retains manual edits, updating the previous name; choosing a tone resets the edit."},
        attachment:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Opaque Host-owned file ID."},
            label:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Attachment name."}
        }}},required:false,sensitive:true,doc:"Optional removable attachment. The component never resolves files."},
        link:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Opaque Host link identity."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible link text."}
        }}},required:false,sensitive:false,doc:"Optional appended link. The Host owns both opening and composing its destination."},
        receipt:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            recipient_name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Name confirmed on delivery, independent of the current contact roster."},
            attachment_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Attachment label confirmed on delivery."}
        }}},required:false,sensitive:true,doc:"Accepted receipt in the sent phase. Without it the card shows only a generic sent status."},
        notice:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Optional accepted or rejected status from the Host."},
        on_send:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the proposed recipient, content and attachment. Does not send mail."},
        on_discard:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Requests discarding this draft and revision."},
        on_undo:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Requests restoring a draft; does not recall already delivered mail."},
        on_link:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Handles a request to open the appended link."}
    },state:#{fields:#{local:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}}},
    events:#{
        send:#{payload:#{type:"object",allow_unknown:false,fields:#{
            draft:#{schema:#{type:"string"},required:true,sensitive:false},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            recipient:#{schema:#{type:"string"},required:true,sensitive:false},
            body:#{schema:#{type:"string"},required:true,sensitive:true},
            attachment:#{schema:#{type:"string"},required:true,sensitive:false},
            link:#{schema:#{type:"string"},required:true,sensitive:false}
        }},doc:"A send proposal. Optional attachment and link IDs are empty when absent."},
        discard:#{payload:#{type:"object",allow_unknown:false,fields:#{
            draft:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false}
        }},doc:"A discard proposal."},
        undo:#{payload:#{type:"object",allow_unknown:false,fields:#{
            draft:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false}
        }},doc:"A restore proposal from a settled receipt."},
        link:#{payload:#{type:"object",allow_unknown:false,fields:#{
            draft:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            id:#{schema:#{type:"string"},required:true,sensitive:false}
        }},doc:"Open an opaque link through the Host."}
    },slots:#{},parts:["root","record","reference","status","account","amount","switcher","card","meta","to_label","recipient","recipient_open","recipient_arrow","recipient_name","recipient_email",
        "menu","option","option_selected","option_name","option_email","option_avatar","attachment","attachment_icon","file_name","remove","remove_icon","attach","attach_label","body","body_text",
        "editor","link","actions","tones","tone","tone_selected","edit","buttons","discard","send","control_label","decision_label","receipt","receipt_icon","receipt_text","undo",
        "footer","footer_text","agent","badge","badge_done","hint","hint_rail","notice","glyph"]},render:Fn("render_email")
});

fn EmailDraft(p){render_component("patterns/email_draft",p)}
fn get(p,k,d){if p!=()&&k in p&&p[k]!=(){p[k]}else{d}}
fn sp(x){theme_length("space.spacing_unit").times(x)}
fn u(x){theme_length("space.unit").times(x)}
fn tx(ctx,k,s){ctx.t_or("email_draft."+k,s)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn bounded(value,cap){value!=""&&utf8_byte_count(value)<=cap}
fn text_line(){style().typography("body").min_width(px(0)).whitespace_nowrap().text_ellipsis()}
fn line(){text_line().text_color(theme_color("text_primary"))}
fn quiet(){line().typography("caption").text_color(theme_color("text_muted"))}
fn label(ctx,part,s,style){text(s).with_style(ctx.component_style(part,style))}
fn mark(ctx,path){svg("<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='"+path+"'/></svg>")
    .accessibility_hidden(true).with_style(ctx.component_style("glyph",style().width(u(3.75)).height(u(3.75)).flex_shrink(false)))}
fn first_word(name){let name=name;name.trim();let parts=name.split(" ");if parts.len>0{parts[0]}else{""}}
fn paint_ease(node){node.paint_transition(160,cubic_bezier(0.25,0.1,0.25,1.0))}
fn personalized(template,contact){
    let name=get(contact,"name","");
    let first=first_word(name);let result="";
    for part in prompt_template_parts(template){
        if "text" in part{result+=part.text;}
        else if part.name=="first"{result+=first;}
        else if part.name=="name"{result+=name;}
        else{result+="{"+part.name+"}";}
        if utf8_byte_count(result)>16000{throw "EmailDraft generated copy exceeds 16000 bytes";}
    }
    result
}
fn indexed(rows,is_contact){
    let by=#{};
    for r in rows{
        let v=if is_contact{r.name}else{r.label};
        if !bounded(r.id,96)||!bounded(v,512)||r.id in by{throw "EmailDraft items need distinct, bounded IDs and labels";}
        if is_contact&&!bounded(r.email,512){throw "EmailDraft recipients need a bounded, nonempty address";}
        if !is_contact&&utf8_byte_count(r.body)>16000{throw "EmailDraft tones must fit 16000 bytes";}
        by[r.id]=r;
    }
    by
}
fn bootstrap(draft,to,tone){#{draft:draft,to:to,tone:tone,edited:(),menu:false,editing:false,drop:""}}
fn local(ctx,base){
    let old=ctx.get_state("local");if old!=()&&old.draft==base.draft{old}else{base}
}
fn model(ctx,p){
    if !bounded(p.draft,96){throw "EmailDraft draft needs a nonempty ID";}
    let contacts=indexed(p.contacts,true);let tones=indexed(p.tones,false);
    let file=get(p,"attachment",());let link=get(p,"link",());
    for item in [file,link]{if item!=()&&(!bounded(item.id,96)||!bounded(item.label,512)){throw "EmailDraft attachments and links need an ID and label";}}
    let base=bootstrap(p.draft,if p.contacts.len>0{p.contacts[0].id}else{""},if p.tones.len>0{p.tones[0].id}else{""});
    let s=local(ctx,base);
    if !(s.to in contacts){s.to=base.to;}
    if !(s.tone in tones){s.tone=base.tone;}
    let file_id=get(file,"id","");
    let attached=file_id!=""&&s.drop!=file_id;
    let contact=get(contacts,s.to,());
    let generated=personalized(get(get(tones,s.tone,()),"body",""),contact);
    let body=get(s,"edited",generated);
    let ticket=#{draft:p.draft,revision:p.revision};
    let payload=#{draft:p.draft,revision:p.revision,recipient:s.to,body:body,attachment:if attached{file_id}else{""},link:get(link,"id","")};
    #{local:s,base:base,contact:contact,body:body,file:file,link:link,attached:attached,ticket:ticket,payload:payload,live:p.phase=="draft",
      can_send:p.phase=="draft"&&contact!=()&&get(p,"on_send",())!=()}
}
fn change(base,kind,value,ctx,event){
    let s=local(ctx,base);
    switch kind{
        "tone"=>{s.tone=value;s.edited=();s.editing=false;},
        "edit"=>{s.edited=value;s.editing=true;s.menu=false;},
        "close_edit"=>{s.editing=false;},
        "drop"=>{s.drop=value;},
        "attach"=>{s.drop="";}
    }
    ctx.set_state("local",s);handled()
}
fn pick_recipient(base,id,previous,next,ctx,event){
    let s=local(ctx,base);
    if s.to!=id&&s.edited!=(){
        let changed=replace_recipient_name(s.edited,previous,next);
        if grapheme_count(changed)<=16000{s.edited=changed;}
    }
    s.to=id;s.menu=false;ctx.set_state("local",s);handled()
}
fn open_menu(base,ctx,value){let s=local(ctx,base);s.menu=value;ctx.set_state("local",s);}
fn edited(base,ctx,text){
    if utf8_byte_count(text)>64000{return;}
    let s=local(ctx,base);s.edited=text;ctx.set_state("local",s);
}
fn request(base,intent,payload,ctx,event){
    let s=local(ctx,base);s.editing=false;s.menu=false;ctx.set_state("local",s);ctx.emit(intent,payload);handled()
}
fn submit(base,payload,ctx,text){
    if utf8_byte_count(text)>64000{return handled();}
    let s=local(ctx,base);s.edited=text;ctx.set_state("local",s);
    payload.body=text;
    request(base,"send",payload,ctx,())
}
fn jump(ref,ctx,event){ctx.focus(ref);ctx.scroll_into_view(ref);handled()}
fn button(ctx,key,parts,name,children,live,cb){
    let n=row(children).with_key(key).accessibility_role("button").accessibility_label(name).disabled(!live)
      .with_style(ctx.component_style(parts,style().min_width(px(0)).min_height(u(7)).items_center().justify_center().gap(sp(1))
        .padding_x(sp(2)).radius(theme_radius("sm")).cursor_pointer().hover(style().background(theme_color("surface_hover")))
        .disabled(style().opacity(0.55)).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))));
    if live{n.tab_stop(true).on_click(cb).on_key_value("plain+enter",cb,()).on_key_value("plain+space",cb,()).paint_transition(160,"ease_out")}else{n}
}
fn record(ctx,p){
    let head=[];
    let r=get(p,"reference","");let status=get(p,"status","");let amount=get(p,"amount","");
    if r!=""{head.push(label(ctx,"reference",r,line().font_weight(500)));}
    if status!=""{head.push(label(ctx,"status",status,quiet().typography("metadata").padding_x(sp(1.5)).radius(theme_radius("sm"))
       .background(theme_color("email_draft.warn_bg")).text_color(theme_color("email_draft.warn"))));}
    if amount!=""{head.push(row([]).with_style(style().flex_grow()));head.push(label(ctx,"amount",amount,line().font_weight(500).flex_shrink(false)));}
    let info=[row(head).with_style(style().items_center().min_width(px(0)).gap(sp(1.5)))];
    let account=get(p,"account","");
    if account!=""{info.push(label(ctx,"account",account,quiet().text_color(theme_color("email_draft.account"))));}
    column(info).with_key("record").with_style(ctx.component_style("record",style().min_width(px(0)).gap(sp(0.5)).padding(sp(2))))
}
fn initials(name){
    let out="";let n=0;
    for word in name.split(" "){
        if word!=""{for c in word.chars(0,1){out+=c.to_string();}n+=1;if n==2{break;}}
    }
    out.to_upper()
}
fn recipient_menu(ctx,p,m){
    let options=[];
    for i in 0..p.contacts.len{
        let c=p.contacts[i];let selected=c.id==m.local.to;
        let avatar=row([label(ctx,"option_avatar",initials(c.name),quiet().text_color(theme_color("accent")))])
            .with_style(style().width(u(7)).height(u(7)).radius(u(3.5)).items_center().justify_center()
                .background(theme_color("email_draft.chip")).flex_shrink(false)).accessibility_hidden(true);
        let cell=column([label(ctx,"option_name",c.name,line()),label(ctx,"option_email",c.email,quiet())])
            .with_style(style().flex_grow().min_width(px(0)));
        let parts=if selected{["option","option_selected"]}else{["option"]};
        let cb=Fn("pick_recipient").curry(m.base,c.id,get(m.contact,"name",""),c.name);
        let content=[avatar,cell];
        if selected{content.push(mark(ctx,"m6 12 4 4 8-9"));}
        let key="contact:"+i.to_string();
        let option=row(content).with_key("recipient:"+c.id).with_ref(element_ref(key)).accessibility_role("option")
            .accessibility_label(c.name).accessibility_description(c.email).accessibility_selected(selected).tab_stop(true)
            .with_style(ctx.component_style(parts,style().min_width(px(0)).items_center().padding(sp(1.5)).gap(sp(2)).radius(theme_radius("sm"))
                .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
            .on_click(cb).on_key_value("plain+enter",cb,()).on_key_value("plain+space",cb,());
        option=option.on_key_value("plain+down",Fn("jump").curry("contact:"+((i+1)%p.contacts.len).to_string()),())
            .on_key_value("plain+up",Fn("jump").curry("contact:"+((i+p.contacts.len-1)%p.contacts.len).to_string()),());
        options.push(option);
    }
    column(options).with_key("list").accessibility_role("listbox").accessibility_label(tx(ctx,"recipient","Recipient"))
        .with_style(style().gap(sp(0.5)).min_width(px(0)).max_height(u(65)).overflow_y_scroll()).scrollbars("hidden","auto")
}
fn address(ctx,value){label(ctx,"recipient_email",value,quiet().text_color(theme_color("email_draft.address")))}
fn recipient_arrow(ctx,open){
    let angle=if open{180.0}else{0.0};
    mark(ctx,"m7 10 5 5 5-5").with_key("recipient-arrow")
        .with_style(ctx.component_style("recipient_arrow",
            style().text_color(theme_color("text_muted")).transform(#{rotate:angle})))
        .motion(motion_transition("rotate",angle,angle,#{duration_ms:220,easing:ease(),intent:"feedback"}))
}
fn meta(ctx,p,m,compact,clean){
    let to_label=label(ctx,"to_label",tx(ctx,"to","to"),quiet().flex_shrink(false)).accessibility_role("label");
    let to=if m.contact==(){address(ctx,tx(ctx,"no_recipient","No recipient"))}else{
        let c=m.contact;
        let to=[label(ctx,"recipient_name",c.name,line().font_weight(500))];
        if !compact{to.push(address(ctx,if clean{c.email}else{"("+c.email+")"}));}
        let open=m.live&&m.local.menu;
        if m.live{to.push(recipient_arrow(ctx,open));}
        let trigger=paint_ease(row(to).with_key("recipient").accessibility_role("button").accessibility_label(tx(ctx,"recipient","Recipient"))
            .accessibility_description(c.name+" · "+c.email).accessibility_expanded(open));
        let rail=ctx.component_style(if open{["recipient","recipient_open"]}else{["recipient"]},
            style().min_width(px(0)).gap(sp(1)).items_center()
            .radius(theme_radius("sm")).padding_x(sp(1)).padding_y(sp(0.5)).hover(style().background(theme_color("surface_hover"))));
        if !m.live{trigger.with_style(rail).accessibility_role("text")}else{
            pop::Popover(#{key:"recipient:"+p.draft,label:tx(ctx,"choose_recipient","Choose recipient"),open:open,initial_focus:"first",
                gap:5,trigger:trigger,content:if open{recipient_menu(ctx,p,m)}else{box([])},
                part_styles:#{root:style().flex_row().min_width(px(0)),
                    trigger:rail,content:ctx.component_style("menu",style().min_width(u(53)).max_width(u(90)).padding(sp(1)))},
                on_open_change:Fn("open_menu").curry(m.base)})
        }
    };
    // Bound the inline group to this flex line. The shrinking Popover root
    // then passes the remaining width to the very same trigger/button.
    let out=[row([to_label,to]).with_key("to").with_style(style().items_center().min_width(px(0))
        .max_width(relative(1.0)).gap(sp(1)))];
    if m.file!=(){
        let file_name=if m.attached{m.file.label}else{tx(ctx,"attach","Attach")+" "+m.file.label};
        let glyph=if m.attached{"m15.5 7.5-6.4 6.4a1.8 1.8 0 0 0 2.5 2.5l6.7-6.7a3.5 3.5 0 0 0-5-5l-6.9 6.9a5.2 5.2 0 0 0 7.4 7.4l6-6"}else{"M12 6v12M6 12h12"};
        let icon=mark(ctx,glyph).with_style(ctx.component_style("attachment_icon",style()));
        if m.attached{
            let rm=tx(ctx,"remove","Remove")+" "+m.file.label;
            let remove_icon=mark(ctx,"M7 7l10 10M17 7 7 17").with_style(ctx.component_style("remove_icon",style()));
            let remove=paint_ease(button(ctx,"remove","remove",rm,[remove_icon],m.live,Fn("change").curry(m.base,"drop",m.file.id)));
            out.push(row([icon,label(ctx,"file_name",file_name,quiet()),remove]).with_key("file")
                .with_style(ctx.component_style("attachment",style().min_width(px(0)).items_center().gap(sp(1))
                    .text_color(theme_color("email_draft.file")))));
        }else{
            out.push(paint_ease(button(ctx,"attach","attach",file_name,[icon,label(ctx,"attach_label",file_name,text_line())],
                m.live,Fn("change").curry(m.base,"attach",""))));
        }
    }
    // The header paints into its own grid band. The recipient menu itself
    // is a native Popover portal, not overflow from the header's inline paint.
    row(out).with_key("meta").with_style(ctx.component_style("meta",style().flex_wrap().items_center().justify_between()
        .min_width(px(0)).gap(sp(1.5)).padding(sp(2)).clip()))
}
fn body(ctx,p,m,clean){
    let children=[];
    if m.live&&m.local.editing{
        children.push(editor::Textarea(#{key:"active-body:"+p.draft,label:tx(ctx,"email_text","Email text"),value:m.body,editor_ref:element_ref("body-editor"),
            min_rows:1,max_rows:12,max_length:16000,autofocus:true,on_change:Fn("edited").curry(m.base),
            on_blur:Fn("change").curry(m.base,"close_edit",""),submit_key:if m.can_send{"mod_enter"}else{()},
            on_submit:if m.can_send{Fn("submit").curry(m.base,m.payload)}else{()},
            part_styles:#{editor:ctx.component_style("editor",style().border(px(0)).padding(px(0)).background(rgba(0x00000000)))}})
                .on_key_value("plain+escape",Fn("change").curry(m.base,"close_edit",""),()));
    }else{
        let paper=style().typography("body").min_width(px(0)).whitespace_normal().text_color(theme_color("text_primary"));
        let base=ctx.component_style("body_text",if m.live{paper.cursor_text()}else{paper});
        let start=0;
        let copy=if m.link!=(){
            // A break has no width, but keeps a leading combining character
            // in the link label from extending the preceding text span.
            let lead=if m.body==""{""}else{m.body+" \u200b"};
            start=grapheme_count(lead);
            let link_ink=ctx.component_style("link",style().text_color(theme_color("email_draft.link")));
            let link=span(m.link.label).ink_from(link_ink)
                .underline(if clean{1.0}else{1.5},theme_color("email_draft.link_underline"),if clean{3.0}else{4.0})
                .underline_hover(160,cubic_bezier(0.25,0.1,0.25,1.0));
            text([span(lead),if clean{link.typography("label")}else{link}])
        }else{text(m.body)};
        let words=copy.with_style(base).with_key("body-text");
        if m.live{
            let edit=Fn("change").curry(m.base,"edit",m.body);
            words=words.on_click(edit);
        }
        if m.link!=(){
            let info=m.ticket;info.id=m.link.id;
            let link=box([]).with_key("link")
                .accessibility_role("link").accessibility_label(m.link.label);
            if m.live&&get(p,"on_link",())!=(){
                let open=Fn("request").curry(m.base,"link",info);
                link=link.tab_stop(true).on_click(open).on_key_value("plain+enter",open,());
            }else{link=link.disabled(true);}
            children.push(inline_prose(words,[#{start:start,end:start+grapheme_count(m.link.label),node:link}]).with_key("prose"));
        }else{children.push(words);}
    }
    // Prevent edited or translating copy from painting over the following
    // control band. Editor and inline links retain their existing hit owners.
    column(children).with_key("body").with_style(ctx.component_style("body",style().min_width(px(0)).gap(sp(0.5)).padding(sp(2)).clip()))
}
fn tuning(ctx,p,m){
    let options=[];
    for t in p.tones{
        let selected=t.id==m.local.tone&&m.local.edited==();
        // Let the action surface's live ink reach the text and its native
        // hover transition without changing the selected-tone callback.
        options.push(button(ctx,"tone:"+t.id,if selected{["tone","tone_selected"]}else{["tone"]},t.label,[label(ctx,"control_label",t.label,text_line())],
            m.live,Fn("change").curry(m.base,"tone",t.id)).accessibility_pressed(selected)
            .paint_transition(160,cubic_bezier(0.25,0.1,0.25,1.0)));
    }
    options.push(button(ctx,"edit","edit",tx(ctx,"edit","Edit text"),[mark(ctx,"m14 6 4 4M5 19l1-4L15 6l4 4-9 9z")],
        m.live,Fn("change").curry(m.base,"edit",m.body)).accessibility_pressed(m.live&&m.local.editing));
    let group=row(options).with_key("tones").accessibility_role("group").accessibility_label(tx(ctx,"tone","Tone"))
        .with_style(ctx.component_style("tones",style().min_width(px(0)).flex_wrap().items_center().gap(sp(1))));
    row([group]).with_key("actions").with_style(ctx.component_style("actions",style().min_width(px(0)).items_center().padding(sp(2))))
}
fn decisions(ctx,p,m){
    let discard=tx(ctx,"discard","Discard");
    let sending=p.phase=="sending";let send=if sending{tx(ctx,"sending","Sending")}else{tx(ctx,"send","Send email")};
    let send_content=[];
    if sending{send_content.push(spinner::Spinner(#{key:"send-work",size:"xs",label:send}).accessibility_hidden(true));}
    send_content.push(label(ctx,["control_label","decision_label"],send,quiet().text_color(theme_color("email_draft.on_send"))));
    row([
        button(ctx,"discard","discard",discard,[label(ctx,["control_label","decision_label"],discard,quiet())],m.live&&get(p,"on_discard",())!=(),Fn("request").curry(m.base,"discard",m.ticket)),
        button(ctx,"send","send",send,send_content,m.can_send,Fn("request").curry(m.base,"send",m.payload)),
    ]).with_key("buttons").with_style(ctx.component_style("buttons",style().min_width(px(0)).flex_wrap()
        .gap(sp(2)).items_center().justify_end().padding(sp(2))))
}
fn receipt(ctx,p,m){
    let sent=p.phase=="sent";
    let accepted=get(p,"receipt",());
    let file=get(accepted,"attachment_label","");
    let caption=if !sent{tx(ctx,"discarded","Draft discarded")}
        else if accepted==(){tx(ctx,"email_sent","Email sent")}
        else{tx(ctx,"sent","Sent to")+" "+accepted.recipient_name+(if file!=""{" · "+file}else{""})};
    let glyph=if sent{"m6 12 4 4 8-9"}else{"m6 6 12 12M18 6 6 18"};
    let symbol=row([mark(ctx,glyph)]).with_style(ctx.component_style("receipt_icon",style().width(u(7)).height(u(7)).flex_shrink(false)
        .items_center().justify_center().radius(u(3.5)).background(theme_color(if sent{"email_draft.ok_bg"}else{"email_draft.chip"}))
        .text_color(theme_color(if sent{"email_draft.ok"}else{"text_muted"}))));
    let out=[symbol,label(ctx,"receipt_text",caption,line().flex_grow()).accessibility_role("status").accessibility_label(caption)];
    if get(p,"on_undo",())!=(){
        let undo=tx(ctx,"undo","Undo");
        out.push(button(ctx,"undo","undo",undo,[label(ctx,"control_label",undo,quiet())],true,Fn("request").curry(m.base,"undo",m.ticket)));
    }
    row(out).with_key("receipt").with_style(ctx.component_style("receipt",style().min_width(px(0)).items_center().gap(sp(2)).padding(sp(2.5))
        .background(theme_color("email_draft.card")).radius(theme_radius("lg"))))
}
fn at(node,x,y,w,h){
    node.with_style(style().grid_cell(x,y).col_span(w).row_span(h))
}
fn footer_words(ctx,p,show_author){
    let review=if p.phase=="sent"{tx(ctx,"sent_badge","Sent")}else if p.phase=="discarded"{tx(ctx,"discarded_badge","Discarded")}
        else if p.phase=="sending"{tx(ctx,"sending","Sending")}else{tx(ctx,"ready","Ready for review")};
    let owner=get(p,"agent","");
    let footer=[];
    if show_author&&owner!=""{footer.push(label(ctx,"agent",owner,line()));}
    footer.push(label(ctx,if p.phase=="discarded"{["badge","badge_done"]}else{["badge"]},review,
        quiet().typography("metadata").padding_x(sp(1.5)).radius(theme_radius("sm")).background(theme_color("email_draft.ok_bg")).text_color(theme_color("email_draft.ok"))));
    row(footer).with_key("footer-words").with_style(ctx.component_style("footer_text",
        style().min_width(px(0)).gap(sp(1.5)).padding(sp(2)).items_center().flex_wrap()))
}
fn shortcut(ctx){
    row([label(ctx,"hint",tx(ctx,"shortcut","Cmd/Ctrl+Enter to send"),
        quiet().typography("key_legend").text_color(theme_color("email_draft.hint")))])
        .with_key("shortcut").with_style(ctx.component_style("hint_rail",style().min_width(px(0)).justify_end()
            .items_center().padding(sp(2))))
}
fn surface(ctx,p,m,open,clean,width){
    let compact=width!=()&&width<=2;
    let narrow=width!=()&&width<=1;
    let split=open&&!(if clean{width==0}else{narrow});
    let cols=if split{2}else{1};
    let rtl=ctx.text_direction()=="rtl";
    let lead=if rtl{cols}else{1};
    let tail=if rtl{1}else{cols};
    let footer_y=if !open{2}else if !clean&&!split{5}else{4};
    let footer_rows=if open&&clean&&!split{2}else{1};
    let children=[];
    if open{
        // Surfaces span the same grid as their single, keyed content owners.
        // In particular, moving "buttons" to the footer does not reparent them.
        children.push(at(box([]).with_key("card").accessibility_hidden(true)
            .with_style(ctx.component_style("card",style().min_width(px(0)).background(theme_color("email_draft.card"))
                .radius(theme_radius("lg")))),1,1,cols,footer_y-1));
    }
    children.push(at(box([]).with_key("footer").accessibility_hidden(true)
        .with_style(ctx.component_style("footer",style().min_width(px(0)))),1,footer_y,cols,footer_rows));
    if open{
        children.push(at(meta(ctx,p,m,compact,clean),1,1,cols,1));
        children.push(at(body(ctx,p,m,clean),1,2,cols,1));
        let full_tuning=clean&&compact;
        children.push(at(tuning(ctx,p,m),if full_tuning{1}else{lead},3,if full_tuning{cols}else{1},1));
        if !compact{
            children.push(at(shortcut(ctx),tail,if clean{3}else{footer_y},1,1));
        }
        let full_footer=!clean&&compact;
        children.push(at(footer_words(ctx,p,!(clean&&narrow)),if full_footer{1}else{lead},footer_y,if full_footer{cols}else{1},1));
        let button_y=if clean{footer_y+footer_rows-1}else if split{3}else{4};
        children.push(at(decisions(ctx,p,m),tail,button_y,1,1));
    }else{
        children.push(at(receipt(ctx,p,m),1,1,cols,1));
        children.push(at(footer_words(ctx,p,true),1,footer_y,1,1));
    }
    // Intrinsic tracks, pushed to the logical edges, avoid reserving half
    // the width for translated controls or losing their RTL alignment.
    let tracks=if split{style().grid_cols_max_content(cols).justify_between()}else{style().grid_cols(1)};
    box(children).with_key("draft-grid").with_style(tracks.grid_rows_intrinsic(footer_y+footer_rows-1)
        .min_width(px(0)).width(relative(1.0)))
}
fn render_email(ctx,p){
    let m=model(ctx,p);
    let open=p.phase=="draft"||p.phase=="sending";
    let clean=ctx.theme_variant()!=()&&ctx.theme_token("design.name")=="clean";
    let frame=element_ref("draft-frame");
    let width=ctx.element_width_class(frame,[280,380,440]);
    let shape=if !open{"receipt"}else if m.local.editing&&m.live{"editor"}
        else{"draft:"+utf8_byte_count(m.local.to).to_string()+":"+m.local.to+":"+m.local.tone};
    let parts=[record(ctx,p),column([surface(ctx,p,m,open,clean,width)]).with_key("switcher")
        .with_style(ctx.component_style("switcher",style().min_width(px(0)).width(relative(1.0))))
        .layout_size_motion(shape,"height",320,ease())];
    let notice=get(p,"notice","");
    if notice!=""{parts.push(label(ctx,"notice",notice,quiet().text_color(theme_color("text.danger")))
        .with_key("notice").accessibility_role("status").accessibility_label(notice));}
    column(parts).with_key("email").with_ref(frame).accessibility_role("group").accessibility_label(tx(ctx,"card","Email draft"))
        .with_style(ctx.component_style("root",style().min_width(px(0)).width(relative(1.0)).max_width(u(130))
            .padding(sp(1)).background(theme_color("email_draft.tray")).radius(theme_radius("lg"))))
}
