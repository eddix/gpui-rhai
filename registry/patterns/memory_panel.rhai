/* gpui-rhai
{"id":"patterns/memory_panel","export":"MemoryPanel","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/input","components/textarea"],"capabilities":{},"assets":[],"tokens":["border","design.name","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","memory_panel.tray","memory_panel.card","memory_panel.well","memory_panel.hover","memory_panel.fill","memory_panel.ink","memory_panel.text","memory_panel.muted","memory_panel.track","memory_panel.track_on","memory_panel.knob","memory_panel.danger","memory_panel.danger_fill","memory_panel.paused","memory_panel.paused_fill","memory_panel.solid","memory_panel.on_solid"],"environment":[]}
*/
// Edit an explicit Host memory snapshot. The panel does not discover, persist or use memories.
// Example: MemoryPanel(#{key:"memory",workspace:"profile",memories:[#{id:"tea",text:"Prefers tea.",category:"Preferences"}]})
import "components/input" as input;
import "components/textarea" as textarea;

define_component(#{metadata:#{id:"patterns/memory_panel","export":"MemoryPanel",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
    dependencies:["components/input","components/textarea"],capabilities:#{},assets:[],tokens:["border","design.name","focus_ring","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","memory_panel.tray","memory_panel.card","memory_panel.well","memory_panel.hover","memory_panel.fill","memory_panel.ink","memory_panel.text","memory_panel.muted","memory_panel.track","memory_panel.track_on","memory_panel.knob","memory_panel.danger","memory_panel.danger_fill","memory_panel.paused","memory_panel.paused_fill","memory_panel.solid","memory_panel.on_solid"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable panel instance."},
        workspace:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of the displayed memory workspace."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Accepted Host revision; increment to acknowledge or reject a controlled proposal."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Defaults to Memory."},
        memories:#{schema:#{type:"array",max_items:128,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable memory ID."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Short memory value; at most 2,048 graphemes and 8,192 bytes."},
            category:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Group name."},
            source:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Visible provenance label."},
            date:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Caller-formatted date caption."},
            paused:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Retained but not used, according to the Host."}
        }}},required:true,sensitive:true,doc:"Accepted snapshot when controlled; otherwise the local seed."},
        categories:#{schema:#{type:"array",max_items:32,items:#{type:"string"}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Category ordering and extra choices for new memories."},
        enabled:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Controlled use-memory toggle when present."},
        default_enabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Initial local toggle."},
        controlled:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Override snapshot ownership; by default on_change makes memories controlled."},
        max_height:#{schema:#{type:"integer",min:96,max:2000},"default":#{type:"integer",value:360},required:false,sensitive:false,doc:"Visible list height before native vertical scrolling."},
        undo_ms:#{schema:#{type:"integer",min:250,max:60000},"default":#{type:"integer",value:5000},required:false,sensitive:false,doc:"Time to offer Undo after a confirmed removal."},
        added_source:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Source caption for a new item; defaults to You added."},
        added_date:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional caller-formatted date for new items."},
        action_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional unobtrusive footer action label."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the operation, entry, revision and full resulting list."},
        on_enabled_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Use-memory toggle proposal."},
        on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Optional footer action."}
    },state:#{fields:#{
        ui:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
        approved:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true}
    }},events:#{
        change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            workspace:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            kind:#{schema:#{type:"string"},required:true,sensitive:false},id:#{schema:#{type:"string"},required:true,sensitive:false},
            entry:#{schema:#{type:"ui_value"},required:true,sensitive:true},memories:#{schema:#{type:"array",max_items:128,items:#{type:"ui_value"}},required:true,sensitive:true}
        }},doc:"A revisioned add, edit, pause, forget or restore proposal. Payload memory text is sensitive."},
        enabled_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            workspace:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            enabled:#{schema:#{type:"bool"},required:true,sensitive:false}}},doc:"Toggle proposal without modifying records."},
        action:#{payload:#{type:"object",allow_unknown:false,fields:#{
            workspace:#{schema:#{type:"string"},required:true,sensitive:false},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false}
        }},doc:"Named footer action for this snapshot."}
    },slots:#{},parts:["root","card","head","title","count","switch","track","knob","switch_label","notice","filters","chips","chip","chip_selected","chip_count",
        "search","search_field","icon","list","list_off","filters_off","group","item","hot","fact","fact_paused","meta","metadata","metadata_separator","paused","actions","danger",
        "editor","editor_field","edit_buttons","plain","solid","new","bottom","foot","caption","undo","action","empty"],effects:["snapshot","focus"]},
    render:Fn("render_memory")});

fn MemoryPanel(p){render_component("patterns/memory_panel",p)}
fn get(m,k,d){if m!=()&&k in m&&m[k]!=(){m[k]}else{d}}
fn u(n){theme_length("space.unit").times(n)}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn ink(name){let k="memory_panel."+name;theme_color(k)}
fn msg(ctx,k,v){ctx.t_or("memory_panel."+k,v)}
fn ease(){cubic_bezier(0.32,0.72,0.24,1.0)}
fn muted(){style().typography("caption").text_color(ink("muted")).min_width(px(0))}
fn line(){muted().whitespace_nowrap().text_ellipsis()}
fn label(ctx,k,v,base){text(v).with_style(ctx.component_style(k,base))}
fn defaults(workspace){
    #{workspace:workspace,items:(),on:(),filter:"",search:false,query:"",hot:"",focused:"",form:(),draft:"",
        pending:(),undo:(),next:0,seq:0,recover:"",reveal:""}
}
fn state_for(ctx,workspace){
    let v=ctx.get_state("ui");
    if v!=()&&v.workspace==workspace{v}else{defaults(workspace)}
}
fn idx(items,id){
    for i in 0..items.len{if items[i].id==id{return i;}}
    -1
}
fn fresh_undo(s,item,at){
    s.seq+=1;s.undo=#{entry:item,at:at,serial:s.seq};
    s
}
fn reset_editor(s){
    if s.form==(){return s;}
    s.recover=if s.form.kind=="new"{"add-memory"}else{"edit:"+s.form.id};
    s.form=();s.draft="";s
}
fn activate_snapshot(ctx,snap){
    let s=state_for(ctx,snap.workspace);
    let pending=s.pending;
    if pending!=()&&snap.revision>pending.revision{
        let at=idx(snap.items,pending.entry.id);
        switch pending.kind{
            "forget"=>{if at<0{s=fresh_undo(s,pending.entry,pending.at);}},
            "restore"=>{if at>=0&&s.undo!=()&&s.undo.entry.id==pending.entry.id{s.undo=();s.filter="";s.query="";s.reveal=pending.entry.id;}},
            "add"=>{if at>=0&&snap.items[at].text==pending.entry.text{s=reset_editor(s);s.filter="";s.query="";s.reveal=pending.entry.id;}},
            "edit"=>{if at>=0&&snap.items[at].text==pending.entry.text{s=reset_editor(s);}},
        }
        s.pending=();
    }
    if snap.controlled&&s.form!=()&&s.form.kind=="edit"&&idx(snap.items,s.form.id)<0{s=reset_editor(s);}
    ctx.set_state("approved",snap);
    ctx.set_state("ui",s);
}
fn idle(ctx,data){}
fn active_items(ctx,s,t){
    let base=ctx.get_state("approved");
    if base==()||base.workspace!=t.workspace||base.revision!=t.revision{return ();}
    if t.controlled||s.items==(){base.items}else{s.items}
}
fn change(t,s,kind,entry,items,ctx){
    if !t.controlled{s.items=items;}
    ctx.set_state("ui",s);
    ctx.emit("change",#{workspace:t.workspace,revision:t.revision,kind:kind,id:entry.id,entry:entry,memories:items});
}
fn restore_focus(ctx,data){
    let s=state_for(ctx,data.workspace);
    if s.recover==data.intent{
        s.recover="";ctx.set_state("ui",s);
        if data.key!=""{
            ctx.focus(data.key);
            if data.key.starts_with("memory-chip:"){ctx.scroll_into_view(data.key);}
        }
    }
}
fn reveal_row(ctx,data){
    let s=state_for(ctx,data.workspace);
    if s.reveal!=data.id{return;}
    s.reveal="";ctx.set_state("ui",s);
    if ctx.element_bounds(data.key)!=(){ctx.scroll_into_view(data.key);}
}
fn expire_undo(ctx,data){
    let s=state_for(ctx,data.workspace);
    if s.undo!=()&&s.undo.serial==data.serial{s.undo=();ctx.set_state("ui",s);}
}
fn set_mode(t,current,ctx,e){
    let s=state_for(ctx,t.workspace);
    if !t.enabled_controlled{s.on=!current;ctx.set_state("ui",s);}
    ctx.emit("enabled_change",#{workspace:t.workspace,revision:t.revision,enabled:!current});handled()
}
fn external(t,ctx,e){ctx.emit("action",#{workspace:t.workspace,revision:t.revision});handled()}
fn attend(workspace,field,id,ctx,inside){
    let s=state_for(ctx,workspace);
    if inside{if s[field]==id{return;}s[field]=id;}
    else if s[field]==id{s[field]="";}else{return;}
    ctx.set_state("ui",s);
}
fn search_mode(workspace,want,ctx,e){
    let s=state_for(ctx,workspace);s.search=want;s.query="";s.filter="";
    if !want{s.recover="open-search";}
    ctx.set_state("ui",s);handled()
}
fn query(workspace,ctx,text){
    let s=state_for(ctx,workspace);if s.search{s.query=text;ctx.set_state("ui",s);}
}
fn pick_cat(workspace,kind,cat,focus,ctx,e){
    let s=state_for(ctx,workspace);
    if kind=="new"&&s.form!=()&&s.form.kind=="new"{s.form.category=cat;}
    else if kind=="filter"{s.filter=cat;}
    if focus!=""{s.recover=focus;}
    ctx.set_state("ui",s);handled()
}
fn open_form(t,kind,id,first,ctx,e){
    let s=state_for(ctx,t.workspace);
    let items=active_items(ctx,s,t);if items==(){return ();}
    if kind=="edit"{
        let i=idx(items,id);if i<0{return ();}
        s.form=#{kind:"edit",id:id,category:items[i].category};s.draft=items[i].text;
    }else{
        if items.len>=128{return ();}
        s.form=#{kind:"new",id:"",category:first};s.draft="";
    }
    ctx.set_state("ui",s);handled()
}
fn edit_draft(workspace,ctx,text){
    let s=state_for(ctx,workspace);if s.form!=(){s.draft=text;ctx.set_state("ui",s);}
}
fn cancel_form(workspace,ctx,e){
    let s=state_for(ctx,workspace);if s.form!=(){ctx.set_state("ui",reset_editor(s));}handled()
}
fn save_text(t,ctx,raw){
    let s=state_for(ctx,t.workspace);let items=active_items(ctx,s,t);
    if s.form==()||items==(){return;}
    let words=memory_panel_normalize(raw);if words==""{return;}
    let kind=if s.form.kind=="new"{"add"}else{"edit"};
    let entry=();let at=-1;
    if kind=="add"{
        if items.len>=128{return;}
        let id="";
        while id==""||idx(items,id)>=0{s.next+=1;id="local-memory:"+s.next.to_string();}
        entry=#{id:id,text:words,category:s.form.category,paused:false,source:t.source,date:t.date};
        items.push(entry);
    }else{
        at=idx(items,s.form.id);if at<0{return;}
        entry=items[at];if entry.text==words{ctx.set_state("ui",reset_editor(s));return;}
        entry.text=words;items[at]=entry;
    }
    if t.controlled{s.pending=#{kind:kind,entry:entry,at:at,revision:t.revision};}
    else{
        s=reset_editor(s);
        if kind=="add"{s.filter="";s.query="";s.reveal=entry.id;}
    }
    change(t,s,kind,entry,items,ctx);
}
fn save_click(t,ctx,e){save_text(t,ctx,state_for(ctx,t.workspace).draft);handled()}
fn pause_item(t,id,ctx,e){
    let s=state_for(ctx,t.workspace);let items=active_items(ctx,s,t);
    if items==(){return ();}
    let at=idx(items,id);if at<0{return ();}
    let entry=items[at];entry.paused=!get(entry,"paused",false);items[at]=entry;
    change(t,s,"pause",entry,items,ctx);handled()
}
fn forget_item(t,id,ctx,e){
    let s=state_for(ctx,t.workspace);let items=active_items(ctx,s,t);
    if items==(){return ();}
    let at=idx(items,id);if at<0{return ();}
    let entry=items[at];items.remove(at);
    if t.controlled{s.pending=#{kind:"forget",entry:entry,at:at,revision:t.revision};}
    else{s=fresh_undo(s,entry,at);s.hot="";s.focused="";}
    if !t.controlled&&s.form!=()&&s.form.id==id{s=reset_editor(s);}
    change(t,s,"forget",entry,items,ctx);handled()
}
fn undo_forget(t,ctx,e){
    let s=state_for(ctx,t.workspace);let items=active_items(ctx,s,t);
    if items==()||s.undo==()||items.len>=128||idx(items,s.undo.entry.id)>=0{return ();}
    let entry=s.undo.entry;items.insert(min(items.len,s.undo.at),entry);
    if t.controlled{s.pending=#{kind:"restore",entry:entry,at:s.undo.at,revision:t.revision};}
    else{s.undo=();s.filter="";s.query="";s.reveal=entry.id;}
    change(t,s,"restore",entry,items,ctx);handled()
}

fn part(ctx,name,node,base){node.with_style(ctx.component_style(name,base))}
fn glyph(ctx,name){
    let d=switch name{
        "edit"=>"m4 11 7.2-7.2 2 2L6 13H4z m6-6 2 2", "pause"=>"M6 4v8m4-8v8", "play"=>"M6 4.5 12 8 6 11.5Z",
        "forget"=>"M3.5 5.5h9m-6-2h3m-5 2 .6 7h5.8l.6-7", "search"=>"M11 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0Zm-1 3 3.5 3.5",
        "close"=>"m4 4 8 8m0-8-8 8", "plus"=>"M8 3.5v9M3.5 8h9", "undo"=>"m6 4-3 3 3 3M3 7h6a3 3 0 0 1 0 6H6", _=>"m6 4 4 4-4 4"
    };
    part(ctx,"icon",svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.4' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>").accessibility_hidden(true),
        style().width(u(3.5)).height(u(3.5)).flex_shrink(false))
}
fn control(ctx,k,name,bits,b,to,enabled){
    row(bits).with_key(k).with_style(b.cursor_pointer().focus(style().border_color(theme_color("focus_ring"))))
        .accessibility_role("button").accessibility_label(name).tab_stop(enabled).disabled(!enabled)
        .with_style(style().focus_ring(px(2),offset_px(-2),theme_color("focus_ring")))
        .on_click(to).on_key_value("plain+enter",to,()).on_key_value("plain+space",to,())
}
fn small_action(ctx,k,name,glyph_name,cb,enabled){
    let b=style().width(u(6)).height(u(6)).items_center().justify_center().flex_shrink(false)
        .radius(theme_length("radius.sm")).border(px(1)).border_color(rgba(0x00000000))
        .text_color(ink("muted")).hover(style().background(ink("hover")));
    if k=="forget"{b=b.hover(style().text_color(ink("danger")).background(ink("danger_fill")));}
    control(ctx,k,name,[glyph(ctx,glyph_name)],ctx.component_style(if k=="forget"{"danger"}else{"plain"},b),cb,enabled)
}
fn count(ctx,key,value,suffix,b){
    let r=element_ref("count:"+key);let amount=value.to_float();
    let source=box([]).with_key("sample").with_ref(r).number_motion_source().accessibility_hidden(true)
        .with_style(style().absolute().width(px(0)).height(px(0)))
        .motion(motion_transition("number_value",amount,amount,#{duration_ms:420,easing:ease(),intent:"feedback"}));
    let number=ctx.number_text(amount,0,"",suffix,false).sample_numbers(motion_numbers([r]),"amount",0).with_style(b.font_feature("tnum",1));
    row([source,number]).with_key(key).accessibility_hidden(true).with_style(style().relative().min_width(px(0)).flex_shrink(false))
}
fn head(ctx,p,m){
    let name=msg(ctx,"enable","Use memory");
    let c=Fn("set_mode").curry(m.ticket,m.on);
    let track_ref=element_ref("enable-track");
    let bounds=ctx.element_size(track_ref);let dir=if ctx.text_direction()=="rtl"{-1.0}else{1.0};
    let knob_ref=element_ref("enable-knob");let size=ctx.element_size(knob_ref);
    let move=if bounds==()||size==()||!m.on{0.0}else{dir*max(0.0,bounds.width-size.width)};
    let knob=part(ctx,"knob",box([]).with_ref(knob_ref).with_key("knob"),
        style().width(u(3)).height(u(3)).radius(theme_length("radius.lg")).background(ink("knob")))
        .motion(motion_transition("translate_x",move,move,#{duration_ms:200,easing:ease(),intent:"feedback"}));
    let rail=row([knob]).with_key("rail").with_ref(track_ref)
        .with_style(style().height(relative(1.0)).min_width(px(0)).flex_grow(1.0).items_center());
    let track=part(ctx,"track",row([rail]).with_key("track").accessibility_hidden(true),style().width(u(7)).height(u(4)).padding_x(u(0.5)).items_center().flex_shrink(false)
        .radius(theme_length("radius.lg")).background(if m.on{ink("track_on")}else{ink("track")}))
        .paint_transition(240,"ease_out");
    let bits=[];if !m.tiny{bits.push(label(ctx,"switch_label",name,line().text_color(ink("text"))));}bits.push(track);
    let toggle=control(ctx,"enable",name,bits,ctx.component_style("switch",style().gap(sp(2)).items_center().flex_shrink(false)
        .border(px(1)).border_color(rgba(0x00000000)).radius(theme_length("radius.sm"))),c,true)
        .accessibility_role("switch").accessibility_checked(m.on);
    let title=label(ctx,"title",get(p,"title",msg(ctx,"title","Memory")),style().typography("body").font_weight(600).min_width(px(0)).whitespace_nowrap().text_ellipsis())
        .with_key("title").accessibility_role("heading").accessibility_level(2);
    let counter=count(ctx,"total",m.view.total,"",ctx.component_style("count",muted()));
    let names=row([title,counter]).with_style(style().items_center().gap(sp(1.5)).min_width(px(0)).flex_grow(1.0));
    part(ctx,"head",row([names,toggle]),style().gap(sp(2)).items_center().justify_between().min_width(px(0)))
}
fn chip_ref(mode,i){"memory-chip:"+mode+":"+i.to_string()}
fn chips(ctx,m,mode,entries,selected){
    let bits=[];let reverse=ctx.text_direction()=="rtl";let name=if mode=="filter"{msg(ctx,"categories","Memory categories")}else{msg(ctx,"choose","New memory category")};
    for i in 0..entries.len{
        let at=entries[i];let current=selected==at.name;let key=chip_ref(mode,i);
        let title=if at.name==""{msg(ctx,"all","All")}else{at.name};let next=(i+1)%entries.len;let prev=(i+entries.len-1)%entries.len;
        let left=if reverse{next}else{prev};let right=if reverse{prev}else{next};
        let contents=[text(title)];
        if mode=="filter"&&!m.narrow{
            let role=if m.clean&&current{"text"}else{"muted"};
            contents.push(count(ctx,"chip:"+i.to_string(),at.count,"",
                ctx.component_style("chip_count",muted().text_color(ink(role)))));
        }
        let base=line().text_color(ink("text")).height(u(6)).padding_x(sp(2)).radius(theme_length("radius.lg")).items_center().gap(sp(1))
            .border(px(1)).border_color(rgba(0x00000000)).flex_shrink(false).cursor_pointer()
            .hover(style().background(ink("well"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"));
        if current{base=base.background(ink("fill")).text_color(ink("ink"));}
        let path=if current{["chip","chip_selected"]}else{["chip"]};
        let cb=Fn("pick_cat").curry(m.ticket.workspace,mode,at.name,"");
        let chip=row(contents).with_key(key).with_ref(element_ref(key)).with_style(ctx.component_style(path,base)).accessibility_role("radio")
            .accessibility_label(title).accessibility_checked(current).tab_stop(current&&m.on).disabled(!m.on).on_click(cb)
            .on_key_value("plain+space",cb,()).on_key_value("plain+enter",cb,())
            .on_key_value("plain+left",Fn("pick_cat").curry(m.ticket.workspace,mode,entries[left].name,chip_ref(mode,left)),())
            .on_key_value("plain+right",Fn("pick_cat").curry(m.ticket.workspace,mode,entries[right].name,chip_ref(mode,right)),())
            .on_key_value("plain+home",Fn("pick_cat").curry(m.ticket.workspace,mode,entries[0].name,chip_ref(mode,0)),())
            .on_key_value("plain+end",Fn("pick_cat").curry(m.ticket.workspace,mode,entries[entries.len-1].name,chip_ref(mode,entries.len-1)),());
        bits.push(chip);
    }
    let base=style().gap(sp(1)).min_width(px(0)).flex_grow(1.0);
    let strip=row(bits).with_key(mode+"-chips").accessibility_role("radiogroup").accessibility_label(name);
    if mode=="filter"||m.clean{
        let rail=if mode=="filter"{base.flex_basis(px(0))}else{base.width(relative(1.0))};
        let fade=if m.clean{style().scroll_edge_fade("x",0,24)}else{style().edge_fade("x",0,14)};
        return part(ctx,"chips",strip.scrollbars("hidden","hidden"),rail.overflow_x_scroll().merge(fade));
    }
    part(ctx,"chips",strip,base.flex_wrap())
}
fn filters(ctx,m){
    let name=msg(ctx,"search","Search memories");let on=m.on;let w=m.ticket.workspace;
    let search=if m.ui.search&&on{
        let field=input::Input(#{key:"memory-search",label:name,value:m.ui.query,placeholder:name,autofocus:true,
            on_change:Fn("query").curry(w),part_styles:#{root:style().width(relative(1.0)).flex_grow(1.0).min_width(px(0)).shadows([]),input:style().min_width(px(0))}});
        let close=small_action(ctx,"close",msg(ctx,"close_search","Close search"),"close",Fn("search_mode").curry(w,false),true);
        part(ctx,"search_field",row([field,close]).on_key_value("plain+escape",Fn("search_mode").curry(w,false),()),style().gap(sp(1)).items_center().min_width(px(0)))
    }else{
        let entries=[#{name:"",count:m.view.total}];for g in m.view.groups{if g.count>0{entries.push(#{name:g.name,count:g.count});}}
        let b=small_action(ctx,"search",name,"search",Fn("search_mode").curry(w,true),on).with_ref(element_ref("open-search"));
        row([chips(ctx,m,"filter",entries,m.filter),b]).with_style(style().items_center().min_width(px(0)).gap(sp(1)))
    };
    part(ctx,if on{["filters"]}else{["filters","filters_off"]},box([search]).with_key("filters"),
        style().min_width(px(0)).isolated_opacity(true).opacity(if on{1.0}else{0.5}))
        .paint_transition(300,"ease_out",#{group_effects:true})
}
fn editor(ctx,m,is_new){
    let name=if is_new{msg(ctx,"add_field","New memory")}else{msg(ctx,"edit_field","Edit memory")};
    let w=m.ticket.workspace;let cancel=Fn("cancel_form").curry(w);
    let field=textarea::Textarea(#{key:"memory-editor",label:name,value:m.ui.draft,placeholder:msg(ctx,"placeholder","What should be remembered?"),
        autofocus:true,min_rows:1,max_rows:5,max_length:2048,submit_key:"enter",on_change:Fn("edit_draft").curry(w),
        on_submit:Fn("save_text").curry(m.ticket),part_styles:#{root:style().width(relative(1.0)).min_width(px(0)).border(px(0)).shadows([]),
            editor:ctx.component_style("editor_field",style().min_width(px(0)).background(ink("well")))}});
    let back=msg(ctx,"cancel","Cancel");let save=if is_new{msg(ctx,"add","Add")}else{msg(ctx,"save","Save")};
    let b=line().height(u(6)).padding_x(sp(2.5)).items_center().justify_center().radius(theme_length("radius.sm"))
        .border(px(1)).border_color(rgba(0x00000000)).font_weight(500).hover(style().background(ink("hover")));
    let ok=m.ui.draft.trim()!="";
    let buttons=part(ctx,"edit_buttons",row([
        control(ctx,"cancel",back,[text(back)],ctx.component_style("plain",b),cancel,true),
        control(ctx,"save",save,[text(save)],ctx.component_style("solid",b.background(ink("solid")).text_color(ink("on_solid"))
            .hover(style().opacity(0.84)).disabled(style().opacity(0.4))),Fn("save_click").curry(m.ticket),ok)
    ]),style().justify_end().gap(sp(1.5)).min_width(px(0)));
    let contents=[field];if is_new{contents.push(chips(ctx,m,"new",m.categories,m.ui.form.category));}contents.push(buttons);
    part(ctx,"editor",column(contents).on_key_value("plain+escape",cancel,()),style().gap(sp(1.5)).min_width(px(0)).padding(sp(1.5))
        .radius(theme_length("radius.sm")).background(ink("well")))
}
fn row_memory(ctx,m,i){
    let a=m.items[i];let id=a.id;let paused=get(a,"paused",false);
    let is_edit=m.on&&m.ui.form!=()&&m.ui.form.kind=="edit"&&m.ui.form.id==id;
    let hot=m.ui.hot==id||m.ui.focused==id;let w=m.ticket.workspace;
    let main=if is_edit{editor(ctx,m,false)}else{
        let labels=[];let source=get(a,"source","");let date=get(a,"date","");
        if source!=""{labels.push(label(ctx,"metadata",source,line()).with_key("source"));}
        if source!=""&&date!=""{labels.push(label(ctx,["metadata","metadata_separator"],"·",line().flex_shrink(false))
            .with_key("separator").accessibility_hidden(true));}
        if date!=""{labels.push(label(ctx,"metadata",date,line().flex_shrink(false)).with_key("date"));}
        let description=row(labels).with_key("description").with_style(style().min_width(px(0)).items_center().gap(sp(0.75)).overflow_hidden());
        let pieces=[description];
        if paused{pieces.push(label(ctx,"paused",msg(ctx,"paused","Paused"),line().flex_shrink(false).text_color(ink("paused"))
            .background(ink("paused_fill")).padding_x(sp(1)).radius(theme_length("radius.sm"))).accessibility_role("status"));}
        let words=row(pieces).with_style(style().min_width(px(0)).flex_grow(1.0).items_center().gap(sp(1.5)));
        let value=label(ctx,if paused{["fact","fact_paused"]}else{["fact"]},a.text,style().typography("body").min_width(px(0))
            .text_color(if paused{ink("muted")}else{ink("ink")})).with_key("fact");
        let name=if paused{msg(ctx,"resume_item","Resume memory:")}else{msg(ctx,"pause_item","Pause memory:")};
        let enabled=m.on;
        let actions=part(ctx,"actions",row([
            small_action(ctx,"edit",msg(ctx,"edit_item","Edit memory:")+" "+a.text,"edit",Fn("open_form").curry(m.ticket,"edit",id,""),enabled).with_ref(element_ref("edit-row:"+i.to_string())),
            small_action(ctx,"pause",name+" "+a.text,if paused{"play"}else{"pause"},Fn("pause_item").curry(m.ticket,id),enabled),
            small_action(ctx,"forget",msg(ctx,"forget_item","Forget memory:")+" "+a.text,"forget",Fn("forget_item").curry(m.ticket,id),enabled)
        ]).with_key("row-actions"),style().gap(sp(0.5)).items_center().flex_shrink(false));
        let opacity=if hot||m.narrow{1.0}else{0.0};
        let fading=actions.motion(motion_transition("opacity",opacity,opacity,#{duration_ms:150,easing:ease(),intent:"feedback"}));
        let meta=part(ctx,"meta",row([words,fading]).with_key("meta"),style().min_width(px(0)).min_height(u(6)).gap(sp(1.5)).items_center());
        column([value,meta]).with_style(style().min_width(px(0)))
    };
    let item=column([main]).with_key("item:"+id).with_ref(element_ref("record:"+i.to_string()))
        .on_hover_change(Fn("attend").curry(w,"hot",id)).on_focus_within_change(Fn("attend").curry(w,"focused",id));
    part(ctx,if hot&&m.on{["item","hot"]}else{["item"]},item,style().min_width(px(0)).padding_x(sp(1.5)).padding_y(sp(1.25)).radius(theme_length("radius.sm"))
        .background(if hot&&m.on{ink("hover")}else{rgba(0x00000000)}))
}
fn rows(ctx,p,m){
    let out=[];
    for g in m.view.groups{
        if g.visible.len==0{continue;}
        let title=if m.clean{g.name.to_upper()}else{g.name};
        let group=label(ctx,"group",title,line().font_weight(500).padding_x(sp(1.5)).padding_y(sp(0.75)))
            .with_key("group").accessibility_role("heading").accessibility_level(3).accessibility_label(g.name);
        let bits=[group];for i in g.visible{bits.push(row_memory(ctx,m,i));}
        out.push(column(bits).with_key("category:"+g.name).with_style(style().gap(sp(0.5)).min_width(px(0))));
    }
    if out.len==0{out.push(label(ctx,"empty",if m.view.total==0{msg(ctx,"empty","No memories yet")}else{msg(ctx,"no_matches","No matching memories")},
        muted().padding(sp(3)).text_center()));}
    part(ctx,if m.on{["list"]}else{["list","list_off"]},column(out).with_key("memory-list").scrollbars("hidden","auto")
        .accessibility_role("list").accessibility_label(msg(ctx,"list","Saved memories")),style().overflow_y_scroll().max_height(px(p.max_height))
        .min_width(px(0)).gap(sp(1.5)).isolated_opacity(true).opacity(if m.on{1.0}else{0.5}))
        .paint_transition(300,"ease_out",#{group_effects:true})
}
fn footer(ctx,p,m){
    let s=m.ui;
    let caption=if s.undo!=()&&m.on{ctx.t_or("memory_panel.forgot","Forgot “{entry}”",#{entry:s.undo.entry.text})}
        else if !m.on{msg(ctx,"off","Memory is off")}
        else if s.form!=(){msg(ctx,"keys","Enter to save · Esc to cancel")}
        else{ctx.t_or("memory_panel.summary","{total} memories · {paused} paused",#{total:ctx.format_number(m.view.total),paused:ctx.format_number(m.view.paused)})};
    let note=label(ctx,"caption",caption,line());
    let bits=[note];
    if s.undo!=()&&m.on{
        let name=msg(ctx,"undo","Undo");bits.push(control(ctx,"undo",name,[glyph(ctx,"undo"),text(name)],ctx.component_style("undo",line().gap(sp(1)).items_center().text_color(theme_color("text_primary"))
            .padding_x(sp(1)).border(px(1)).border_color(rgba(0x00000000)).flex_shrink(false)),Fn("undo_forget").curry(m.ticket),true));
    }else if get(p,"action_label","")!=""{
        let name=p.action_label;bits.push(control(ctx,"action",name,[text(name),glyph(ctx,"more")],ctx.component_style("action",line().text_color(ink("text")).font_weight(500).gap(sp(1)).items_center().border(px(1))
            .border_color(rgba(0x00000000)).flex_shrink(false)),Fn("external").curry(m.ticket),true));
    }
    part(ctx,"foot",row(bits),style().justify_between().items_center().gap(sp(2)).min_width(px(0)).padding_x(sp(2)).padding_y(sp(1.25)))
}
fn render_memory(ctx,p){
    let s=state_for(ctx,p.workspace);let on=get(p,"enabled",get(s,"on",p.default_enabled));
    let controlled=get(p,"controlled",get(p,"on_change",())!=());
    let items=if controlled||s.items==(){p.memories}else{s.items};
    let filter=if s.search||!on{""}else{s.filter};let search=if s.search&&on{s.query}else{""};
    let view=memory_panel_projection(items,p.categories,filter,search);let cats=[];let active=false;
    for g in view.groups{cats.push(#{name:g.name,count:g.count});if g.name==filter&&g.count>0{active=true;}}
    if cats.len==0{cats.push(#{name:msg(ctx,"general","General"),count:0});}
    if filter!=""&&!active{filter="";view=memory_panel_projection(items,p.categories,"",search);}
    let t=#{workspace:p.workspace,revision:p.revision,controlled:controlled,enabled_controlled:get(p,"enabled",())!=(),source:get(p,"added_source",msg(ctx,"you_added","You added")),date:get(p,"added_date",())};
    let root=element_ref("memory-panel");let width=ctx.element_width_class(root,[320,348]);
    let m=#{ticket:t,ui:s,items:items,on:on,view:view,narrow:width!=()&&width<2,tiny:width==0,
        clean:ctx.theme_token("design.name")=="clean",categories:cats,filter:filter};
    let stack=[head(ctx,p,m)];
    if !on{stack.push(label(ctx,"notice",msg(ctx,"notice","Memory is off. Saved memories are kept."),muted().padding(sp(1.5)).background(ink("well")).radius(theme_length("radius.sm"))));}
    stack.push(filters(ctx,m));stack.push(rows(ctx,p,m));
    let bottom=if on&&s.form!=()&&s.form.kind=="new"{editor(ctx,m,true)}
    else{
        let name=msg(ctx,"add_new","Add memory");
        let b=line().items_center().gap(sp(2)).width(relative(1.0)).padding_x(sp(2)).height(u(8)).radius(theme_length("radius.sm"))
            .border(px(1)).border_color(rgba(0x00000000)).hover(style().background(ink("hover")));
        control(ctx,"add-memory",name,[glyph(ctx,"plus"),text(name)],ctx.component_style("new",b),Fn("open_form").curry(t,"new","",cats[0].name),on&&items.len<128)
            .with_ref(element_ref("add-memory"))
    };
    stack.push(part(ctx,"bottom",column([bottom]).with_key("bottom"),style().min_width(px(0)).padding(sp(1.5))));
    let card=part(ctx,"card",column(stack),style().min_width(px(0)).padding(sp(2.5)).gap(sp(1.75)).background(ink("card")).radius(theme_length("radius.lg")));
    let output=part(ctx,"root",column([card,footer(ctx,p,m)]).with_key(p.key).with_ref(root),style().width(relative(1.0)).max_width(u(102)).min_width(px(0))
        .background(ink("tray")).radius(theme_length("radius.lg")).padding(sp(1)).gap(sp(0.5)));
    let snap=#{workspace:p.workspace,revision:p.revision,controlled:controlled,items:p.memories};
    effect("snapshot",snap,Fn("activate_snapshot"),Fn("idle"));
    if s.recover!=""{
        let key=s.recover;
        if key.starts_with("edit:"){let at=idx(items,key.sub_string(5));key=if at<0{"add-memory"}else{"edit-row:"+at.to_string()};}
        effect("focus",#{workspace:p.workspace,intent:s.recover,key:key},Fn("restore_focus"),Fn("idle"));
    }
    if s.reveal!=""{let at=idx(items,s.reveal);if at>=0{timeout("reveal",32,false,Fn("reveal_row"),#{workspace:p.workspace,id:s.reveal,key:"record:"+at.to_string()});}}
    if s.undo!=(){timeout("undo",p.undo_ms,false,Fn("expire_undo"),#{workspace:p.workspace,serial:s.undo.serial});}
    output
}
