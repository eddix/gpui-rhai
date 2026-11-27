/* gpui-rhai
{"id":"patterns/integration_settings","export":"IntegrationSettings","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/accordion","components/button","components/icon_button","components/input","components/select","components/switch"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","surface_raised","surface_hover","text_primary","text_muted","text.danger","text.success","radius.lg","radius.sm","typography.body","typography.caption","space.spacing_unit","space.unit","typography.control","typography.title"],"environment":["density"]}
*/
// IntegrationSettings edits versioned site configuration and requests Host credential actions.
// State: local field edits; the Host owns accepted values, sync and credentials.
// Example: IntegrationSettings(#{key:"settings",integration:"store",name:"Store",revision:0,value:#{enabled:true,site:"",zone:""}})
import "components/accordion" as accordion;
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/input" as input;
import "components/select" as select;
import "components/switch" as toggle;

define_component(#{
    metadata:#{id:"patterns/integration_settings","export":"IntegrationSettings",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/accordion","components/button","components/icon_button","components/input","components/select","components/switch"],
        capabilities:#{},assets:[],tokens:["border","focus_ring","surface_raised","surface_hover","text_primary","text_muted","text.danger","text.success","radius.lg","radius.sm","typography.body","typography.caption","space.spacing_unit","space.unit","typography.control","typography.title"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable settings surface identity."},
        integration:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Current integration ID, echoed in every proposal. Switching IDs isolates local drafts."},
        name:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Integration display name."},
        logo:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative logo; otherwise an initial is shown."},
        meta:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Subtitle while sync is enabled."},
        count:#{schema:#{type:"optional",value:#{type:"integer",min:0}},required:false,sensitive:false,doc:"Optional localized activity count."},
        value:#{schema:#{type:"object",allow_unknown:false,fields:#{
            enabled:#{schema:#{type:"bool"},required:true,sensitive:false,doc:"Accepted sync state."},
            site:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted site domain or empty."},
            zone:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accepted zone ID."},
        }},required:true,sensitive:false,doc:"Accepted ordinary settings; never includes a credential."},
        revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Opaque accepted version echoed in Save requests for compare-and-set."},
        zones:#{schema:#{type:"array",max_items:1024,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Application zone identifier."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Searchable zone label."},
        }}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Caller-supplied time zones; labels are not computed UTC offsets."},
        site_hint:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional valid-field hint."},
        saving:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Accepted in-flight save state; preserves and temporarily locks current edits."},
        save_message:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Safe Host feedback for a save result. Do not supply raw secrets."},
        credential_present:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Whether the Host has an API key."},
        credential_version:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Nonsecret credential revision echoed in key actions."},
        credential_busy:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Accepted generation or rotation in progress."},
        credential_visible:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Whether the Host authorizes mounting credential_preview now."},
        credential_preview:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:true,
            doc:"Optional Host-projected display. For real secrets, use native paint or a native reveal surface; ordinary Rhai text is not a secret container. Absent unless explicitly revealed."},
        credential_message:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Nonsecret outcome or status for generation, copy or reveal."},
        on_save:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives #{integration,revision,site,zone}; the Host may reject it."},
        on_toggle:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives #{integration,enabled}; never toggles sync on its own."},
        on_generate_key:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives integration ID and credential version for a generation request."},
        on_copy_key:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives integration ID and credential version; only the Host handles actual copy."},
        on_reveal_key:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives integration ID, credential version and requested visible state."},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Keeps accepted values visible but blocks edits and actions."},
    },state:#{fields:#{
        drafts:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        model:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        zone_open:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        zone_query:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
    }},events:#{
        "save":#{payload:#{type:"object",allow_unknown:false,fields:#{
            integration:#{schema:#{type:"string"},required:true,sensitive:false},
            revision:#{schema:#{type:"integer"},required:true,sensitive:false},
            site:#{schema:#{type:"string"},required:true,sensitive:false},
            zone:#{schema:#{type:"string"},required:true,sensitive:false},
        }},doc:"Versioned nonsecret configuration proposal."},
        toggle:#{payload:#{type:"object",allow_unknown:false,fields:#{
            integration:#{schema:#{type:"string"},required:true,sensitive:false},
            enabled:#{schema:#{type:"bool"},required:true,sensitive:false},
        }},doc:"Sync state proposal."},
        generate_key:#{payload:#{type:"object",allow_unknown:false,fields:#{
            integration:#{schema:#{type:"string"},required:true,sensitive:false},
            version:#{schema:#{type:"integer"},required:true,sensitive:false},
        }},doc:"Ask the Host to generate or rotate a key; no raw key goes through this event."},
        copy_key:#{payload:#{type:"object",allow_unknown:false,fields:#{
            integration:#{schema:#{type:"string"},required:true,sensitive:false},
            version:#{schema:#{type:"integer"},required:true,sensitive:false},
        }},doc:"Explicit user request to copy a key at a specific revision."},
        reveal_key:#{payload:#{type:"object",allow_unknown:false,fields:#{
            integration:#{schema:#{type:"string"},required:true,sensitive:false},
            version:#{schema:#{type:"integer"},required:true,sensitive:false},
            visible:#{schema:#{type:"bool"},required:true,sensitive:false},
        }},doc:"Explicit user request to show or hide the key."},
    },slots:#{},parts:["root","header","logo","name","count","meta","sync","body","credential","credential_label","key_frame","key_text","generate","generate_full","key_action","key_note",
        "section","label","site","site_invalid","prefix","address","valid","hint","invalid","zone","footer","save_note","discard","resolve","save","icon"],
        effects:["reconcile"]},
    render:Fn("render_IntegrationSettings")
});

fn IntegrationSettings(props){render_component("patterns/integration_settings",props)}
fn opt(map,key){if key in map{map[key]}else{()}}
fn can(props,event){!props.disabled&&opt(props,"on_"+event)!=()}
fn canonical(value){let clean=normalize_site_domain(value);if clean==(){value}else{clean}}
fn kept(entries,key,accepted){
    if !(key in entries){return ();}
    let entry=entries[key];
    let current=if key=="site"{normalize_site_domain(entry.entered)}else{entry.entered};
    if current!=()&&current==accepted{()}else{entry}
}
fn project(ctx,props){
    let saved=#{site:canonical(props.value.site),zone:props.value.zone};
    let previous=ctx.get_state("drafts");
    let entries=if previous!=()&&previous.integration==props.integration{previous.fields}else{#{}};
    let site=kept(entries,"site",saved.site);let zone=kept(entries,"zone",saved.zone);
    let draft=#{};
    if site!=(){draft.site=site;}
    if zone!=(){draft.zone=zone;}
    let value=#{site:if site==(){props.value.site}else{site.entered},zone:if zone==(){props.value.zone}else{zone.entered}};
    let clean=normalize_site_domain(value.site);
    let choices=[];let zone_ids=#{};
    for item in props.zones{
        if item.id==""||item.id in zone_ids{throw "IntegrationSettings zone IDs must be unique and nonempty";}
        zone_ids[item.id]=true;choices.push(#{value:item.id,label:item.label});
    }
    let conflict=site!=()&&site.baseline!=saved.site||zone!=()&&zone.baseline!=saved.zone;
    let editable=props.value.enabled&&!props.disabled&&!props.saving;
    let valid=clean!=()&&(value.zone==saved.zone||value.zone in zone_ids);
    #{integration:props.integration,revision:props.revision,accepted:saved,enabled:props.value.enabled,editable:editable,
        draft:#{integration:props.integration,fields:draft},value:value,site:clean,conflict:conflict,valid:valid,
        dirty:site!=()||zone!=(),zones:choices,zone_ids:zone_ids,can_save:editable&&!conflict&&valid&&(site!=()||zone!=())&&can(props,"save")}
}
fn reconcile(ctx,model){
    let old=ctx.get_state("model");
    ctx.set_state("drafts",model.draft);ctx.set_state("model",model);
    if old==()||old.integration!=model.integration||!model.editable{
        ctx.set_state("zone_open",false);ctx.set_state("zone_query","");
    }
}
fn cleanup(ctx,model){}
fn edited(integration,field,ctx,value){
    let m=ctx.get_state("model");if m==()||m.integration!=integration||!m.editable{return;}
    if field=="zone"&&(value==()||!(value in m.zone_ids)){return;}
    let draft=ctx.get_state("drafts");let fields=draft.fields;
    let baseline=if field in fields{fields[field].baseline}else{m.accepted[field]};
    fields[field]=#{baseline:baseline,entered:value};
    draft.fields=fields;ctx.set_state("drafts",draft);
}
fn open_zone(ctx,value){
    let model=ctx.get_state("model");if model==()||!model.editable{return;}
    ctx.set_state("zone_open",value);if !value{ctx.set_state("zone_query","");}
}
fn zone_query(ctx,value){ctx.set_state("zone_query",value);}
fn discard(ctx,event){
    let m=ctx.get_state("model");if m!=()&&m.editable{ctx.set_state("drafts",#{integration:m.integration,fields:#{}});ctx.focus("site");}
    handled()
}
fn reconcile_conflict(ctx,event){
    let m=ctx.get_state("model");if m==()||!m.editable{return handled();}
    let draft=ctx.get_state("drafts");let fields=draft.fields;
    for key in ["site","zone"]{if key in fields{fields[key].baseline=m.accepted[key];}}
    draft.fields=fields;ctx.set_state("drafts",draft);ctx.focus("site");handled()
}
fn submit(ctx,event){
    let m=ctx.get_state("model");
    if m!=()&&m.can_save{ctx.emit("save",#{integration:m.integration,revision:m.revision,site:m.site,zone:m.value.zone});}
    handled()
}
fn toggle_sync(integration,ctx,enabled){ctx.emit("toggle",#{integration:integration,enabled:enabled});handled()}
fn request(event,value,ctx,payload){ctx.emit(event,value);handled()}
fn glyph(ctx,id){
    let path=switch id{
        "generate"=>"M14 3a6 6 0 1 0 2 11.7L22 21v-3l-2-2h-2v-2l-1.3-1.3A6 6 0 0 0 14 3zm0 4h.01",
        "show"=>"M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6-10-6-10-6zm13 0a3 3 0 1 1-6 0 3 3 0 0 1 6 0z",
        "hide"=>"M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6-10-6-10-6zM4 4l16 16",
        "copy"=>"M9 9h12v12H9zM5 16H3V3h13v2",
        _=>"m5 12 4 4L19 6",
    };
    svg(`<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("icon",style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4)).flex_shrink(false)))
}
fn credentials(ctx,props,m){
    let payload=#{integration:props.integration,version:props.credential_version};
    let ready=m.enabled&&!props.credential_busy;
    let visible=props.credential_present&&props.credential_visible&&m.enabled&&!props.disabled;
    let nodes=[];
    if !props.credential_present&&opt(props,"on_generate_key")==(){return ();}
    nodes.push(text(ctx.t_or("integration.key","API key")).with_style(ctx.component_style("credential_label",style()
        .typography("control").font_weight(500))));
    if props.credential_present{
        let preview=opt(props,"credential_preview");
        let display=if visible&&preview!=(){preview.accessibility_hidden(true)}
            else{text("••••••••••••").accessibility_hidden(true).with_style(ctx.component_style("key_text",style()
                .typography("caption").font_feature("tnum",1).whitespace_nowrap().text_ellipsis()))};
        let actions=[];
        if opt(props,"on_reveal_key")!=(){
            let label=if visible{ctx.t_or("integration.hide_key","Hide key")}else{ctx.t_or("integration.show_key","Show key")};
            actions.push(icon_button::IconButton(#{key:"reveal",label:label,size:"sm",icon:glyph(ctx,if visible{"hide"}else{"show"}),
                disabled:!ready||props.disabled,selected:visible,
                on_click:Fn("request").curry("reveal_key",#{integration:props.integration,version:props.credential_version,visible:!visible})})
                .with_style(ctx.component_style("key_action",style())));
        }
        if opt(props,"on_copy_key")!=(){
            actions.push(icon_button::IconButton(#{key:"copy",label:ctx.t_or("integration.copy_key","Copy key"),size:"sm",icon:glyph(ctx,"copy"),
                disabled:!ready||props.disabled,on_click:Fn("request").curry("copy_key",payload)})
                .with_style(ctx.component_style("key_action",style())));
        }
        if opt(props,"on_generate_key")!=(){
            actions.push(button::Button(#{key:"generate",text:ctx.t_or("integration.regenerate","Regenerate"),size:"sm",variant:"ghost",
                disabled:!m.enabled||props.disabled,loading:props.credential_busy,loading_text:ctx.t_or("integration.generating","Generating…"),
                on_click:Fn("request").curry("generate_key",payload)}).with_style(ctx.component_style("generate",style())));
        }
        nodes.push(row([row([display]).accessibility_role("group").accessibility_label(ctx.t_or("integration.key_present","API key stored"))
            .with_style(style().min_width(px(0)).flex_grow()),
            row(actions).with_style(style().items_center().gap(theme_length("space.spacing_unit")).flex_wrap())])
            .with_style(ctx.component_style("key_frame",style().items_center().min_width(px(0)).padding(theme_length("space.spacing_unit").times(1.5)).gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap()
                .radius(theme_radius("sm")).background(theme_color("surface_hover")))));
    }else if opt(props,"on_generate_key")!=(){
        nodes.push(button::Button(#{key:"generate-new",text:ctx.t_or("integration.generate","Generate a new API key"),size:"sm",variant:"secondary",
            suffix:glyph(ctx,"generate"),disabled:!m.enabled||props.disabled,loading:props.credential_busy,
            loading_text:ctx.t_or("integration.generating","Generating…"),on_click:Fn("request").curry("generate_key",payload)})
            .with_key("generate-new")
            .with_style(ctx.component_style("generate_full",style().width(relative(1.0)).justify_between())));
    }
    let note=opt(props,"credential_message");
    if note!=()&&note!=""{
        nodes.push(text(note).accessibility_role("status").with_style(ctx.component_style("key_note",style()
            .typography("caption").text_color(theme_color("text_muted")))));
    }
    column(nodes).with_key("credential").with_style(ctx.component_style("credential",style().gap(theme_length("space.spacing_unit")).min_width(px(0))))
}
fn site_editor(ctx,props,m){
    let label=ctx.t_or("integration.site","Site address");
    let invalid=m.site==();
    let hint=if invalid{ctx.t_or("integration.invalid","Enter a domain, not a URL path, IP address or port.")}
        else if opt(props,"site_hint")!=(){props.site_hint}else{ctx.t_or("integration.site_hint","Your primary domain, without https://")};
    let editor=input::Input(#{key:"address",label:label,value:m.value.site,placeholder:"example.store",
        error:invalid,appearance:"embedded",disabled:!m.enabled||props.disabled,read_only:props.saving,
        on_change:Fn("edited").curry(props.integration,"site")})
        .with_ref(element_ref("site")).with_style(ctx.component_style("address",style()
            .min_width(px(0)).flex_grow().padding(px(0)).border(px(0)).shadows([])));
    let line=[text("https://").accessibility_hidden(true).with_style(ctx.component_style("prefix",style().typography("body")
        .text_color(theme_color("text_muted")).flex_shrink(false))),editor];
    if !invalid&&m.site!=""{line.push(glyph(ctx,"check").with_style(ctx.component_style("valid",style().text_color(theme_color("text.success")))));}
    column([
        text(label).with_style(ctx.component_style("label",style().typography("control").font_weight(500))),
        row(line).with_style(ctx.component_style(if invalid{["site","site_invalid"]}else{["site"]},style().min_width(px(0))
            .min_height(theme_length("space.unit").times(8)).gap(theme_length("space.spacing_unit")).padding_x(theme_length("space.spacing_unit").times(1.5)).items_center().radius(theme_radius("sm"))
            .border(px(1)).border_color(theme_color(if invalid{"text.danger"}else{"border"}))
            .focus_within(style().focus_ring(px(1),offset_px(-1),theme_color("focus_ring"))))),
        text(hint).with_style(ctx.component_style(if invalid{["hint","invalid"]}else{["hint"]},style().min_width(px(0))
            .typography("caption").text_color(theme_color(if invalid{"text.danger"}else{"text_muted"}))))
    ]).with_key("site").with_style(ctx.component_style("section",style().gap(theme_length("space.spacing_unit")).min_width(px(0))))
}
fn zone_editor(ctx,props,m){
    let label=ctx.t_or("integration.zone","Time zone");
    let picker=select::Select(#{key:"zone-choice",label:label,options:m.zones,value:m.value.zone,width:relative(1.0),panel_width:"trigger",
        open:m.editable&&ctx.get_state("zone_open"),query:ctx.get_state("zone_query"),searchable:true,max_visible:6,
        disabled:!m.editable||m.zones.len==0,on_change:Fn("edited").curry(props.integration,"zone"),
        on_open_change:Fn("open_zone"),on_query_change:Fn("zone_query")})
        .with_style(ctx.component_style("zone",style().min_width(px(0))));
    column([text(label).with_style(ctx.component_style("label",style().typography("control").font_weight(500))),picker])
        .with_key("zone-field").with_style(ctx.component_style("section",style().gap(theme_length("space.spacing_unit")).min_width(px(0))))
}
fn footer(ctx,props,m){
    let note=if m.conflict{ctx.t_or("integration.conflict","The Host changed an edited field. Review before saving.")}
        else if opt(props,"save_message")!=(){props.save_message}
        else if m.dirty{ctx.t_or("integration.dirty","Unsaved changes")}else{""};
    let actions=[];
    if note==""&&!m.dirty{return ();}
    if m.dirty{
        actions.push(button::Button(#{key:"discard",text:ctx.t_or("integration.discard","Discard"),variant:"ghost",size:"sm",
            disabled:!m.editable,on_click:Fn("discard")}).with_style(ctx.component_style("discard",style())));
        if m.conflict{
            actions.push(button::Button(#{key:"resolve",text:ctx.t_or("integration.keep","Keep my edits"),variant:"secondary",size:"sm",
                disabled:!m.editable,on_click:Fn("reconcile_conflict")}).with_style(ctx.component_style("resolve",style())));
        }
        actions.push(button::Button(#{key:"save",text:ctx.t_or("integration.save","Save"),size:"sm",disabled:!m.can_save,
            loading:props.saving,loading_text:ctx.t_or("integration.saving","Saving…"),on_click:Fn("submit")})
            .with_style(ctx.component_style("save",style())));
    }
    row([
        text(note).accessibility_role("status").with_style(ctx.component_style("save_note",style()
            .typography("caption").min_width(px(0)).text_color(theme_color(if m.conflict{"text.danger"}else{"text_muted"})))),
        row(actions).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit")).items_center().flex_wrap())
    ]).with_key("footer").with_style(ctx.component_style("footer",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(1.5))
        .gap(theme_length("space.spacing_unit").times(1.5)).justify_between().items_center().flex_wrap()))
}
fn render_IntegrationSettings(ctx,props){
    if props.integration==""{throw "IntegrationSettings integration ID must not be empty";}
    let m=project(ctx,props);
    effect("reconcile",m,Fn("reconcile"),Fn("cleanup"));
    let mark=opt(props,"logo");
    let tile=row([if mark==(){text(text_initial(props.name)).with_style(style().font_weight(600))}else{
        mark.with_style(style().width(theme_length("space.unit").times(5.75)).height(theme_length("space.unit").times(5.75)))
    }]).accessibility_hidden(true).with_style(ctx.component_style("logo",style().width(theme_length("space.unit").times(10.5)).height(theme_length("space.unit").times(10.5))
        .flex_shrink(false).items_center().justify_center().radius(theme_radius("sm")).background(theme_color("surface_hover"))));
    let titles=[text(props.name).accessibility_role("heading").accessibility_level(3)
        .with_style(ctx.component_style("name",style().min_width(px(0)).typography("title")))];
    if opt(props,"count")!=(){
        let count_style=ctx.component_style("count",style().typography("caption").text_color(theme_color("text_muted")));
        titles.push(text(ctx.format_number(props.count)).with_key("count").with_style(count_style));
    }
    let meta=if !m.enabled{ctx.t_or("integration.paused","Paused — no data is synced")}
        else if opt(props,"meta")!=(){props.meta}else{""};
    let info=column([row(titles).with_style(style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).items_center().flex_wrap()),
        text(meta).with_key("meta").with_style(ctx.component_style("meta",style().typography("caption").text_color(theme_color("text_muted"))))])
        .with_style(style().min_width(px(0)).flex_grow());
    let state=toggle::Switch(#{checked:m.enabled,label:ctx.t_or("integration.sync","{name} sync",#{name:props.name}),size:"sm",
        disabled:!can(props,"toggle"),on_change:Fn("toggle_sync").curry(props.integration),
        part_styles:#{text:style().hidden(),root:style().gap(px(0))}})
        .with_key("sync").with_style(ctx.component_style("sync",style().self_center().flex_shrink(false)));
    let heading=row([tile,info,state]).with_key("header")
        .with_style(ctx.component_style("header",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2)).items_center().gap(theme_length("space.spacing_unit").times(2))));
    let editors=[site_editor(ctx,props,m),zone_editor(ctx,props,m)];
    let secret=credentials(ctx,props,m);if secret!=(){editors.insert(0,secret);}
    let body=column(editors)
        .with_key("values").accessibility_hidden(!m.enabled)
        .with_style(ctx.component_style("body",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2.5)).gap(theme_length("space.spacing_unit").times(3))
            .radius(theme_radius("lg")).background(theme_color("surface_raised"))));
    let panel=[body];let actions=footer(ctx,props,m);if actions!=(){panel.push(actions);}
    column([heading,accordion::disclosure_panel(ctx,"sync-body",m.enabled,(),column(panel).accessibility_hidden(!m.enabled))]).with_key(props.key)
        .accessibility_role("region").accessibility_label(ctx.t_or("integration.settings","{name} settings",#{name:props.name}))
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(560)).min_width(px(0))
            .padding(theme_length("space.spacing_unit")).gap(theme_length("space.spacing_unit")).radius(theme_radius("lg")).background(theme_color("surface_hover"))))
}
