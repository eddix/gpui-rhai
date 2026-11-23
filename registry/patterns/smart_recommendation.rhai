/* gpui-rhai
{"id":"patterns/smart_recommendation","export":"SmartRecommendation","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"assets":[],"tokens":["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","text_primary","typography.body","typography.caption","recommendation.tray","recommendation.card","recommendation.paper","recommendation.reason","recommendation.muted","recommendation.line","recommendation.hover","recommendation.chip","recommendation.chip_ink","recommendation.idle","recommendation.signal0","recommendation.weak","recommendation.good","recommendation.strong","recommendation.success","recommendation.success_bg","recommendation.action","recommendation.action_hover","recommendation.solid","recommendation.on_solid","recommendation.popup","recommendation.selected","recommendation.selected_edge","recommendation.tag"],"environment":[]}
*/
// A recommendation with a confidence signal, alternatives and reversible Host decisions.
// Example: SmartRecommendation(#{key:"proposal",session:"sync",options:[#{id:"delta",label:"Incremental sync",reason:"Only changed files are copied.",signal:3}]})
define_component(#{
    metadata:#{id:"patterns/smart_recommendation","export":"SmartRecommendation",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:[],capabilities:#{},assets:[],tokens:["focus_ring","radius.lg","radius.sm","space.unit","space.spacing_unit","text_primary","typography.body","typography.caption","recommendation.tray","recommendation.card","recommendation.paper","recommendation.reason","recommendation.muted","recommendation.line","recommendation.hover","recommendation.chip","recommendation.chip_ink","recommendation.idle","recommendation.signal0","recommendation.weak","recommendation.good","recommendation.strong","recommendation.success","recommendation.success_bg","recommendation.action","recommendation.action_hover","recommendation.solid","recommendation.on_solid","recommendation.popup","recommendation.selected","recommendation.selected_edge","recommendation.tag"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Proposal group identity; a new session resets local selection and disclosure."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Host revision carried by every decision proposal."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Overrides the heading for the recommendation group, without changing option order."},
        options:#{schema:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique, stable ID used for selection and decisions."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Human-readable name."},
            reason:#{schema:#{type:"one_of",variants:[#{type:"string"},#{type:"node"}]},required:true,sensitive:true,doc:"One sentence or a rich SDK node. The reason is never included in emitted events."},
            signal:#{schema:#{type:"integer",min:0,max:3},required:true,sensitive:false,doc:"No, weak, good or strong signal."}
        }}},required:true,sensitive:true,doc:"Highest-ranked option first, followed by alternatives. An empty list draws nothing."},
        default_open:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"Whether to show the reason initially."},
        accepted_id:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"When present, the accepted option is controlled by the Host; explicit null means none."},
        pending:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Host decision pending. Disables mutating controls without changing the accepted snapshot."},
        on_accept:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Request acceptance of the selected option, without executing it."},
        on_undo:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Request reversal of the current acceptance."}
    },state:#{fields:#{ui:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:false}}},events:#{
        accept:#{payload:#{type:"object",allow_unknown:false,fields:#{
            session:#{schema:#{type:"string"},required:true,sensitive:false},
            revision:#{schema:#{type:"integer"},required:true,sensitive:false},
            id:#{schema:#{type:"string"},required:true,sensitive:false}
        }},doc:"Requested option ID, in the current proposal session and revision."},
        undo:#{payload:#{type:"object",allow_unknown:false,fields:#{
            session:#{schema:#{type:"string"},required:true,sensitive:false},
            revision:#{schema:#{type:"integer"},required:true,sensitive:false},
            id:#{schema:#{type:"string"},required:true,sensitive:false}
        }},doc:"Requested reversal of the accepted option ID."}
    },slots:#{},parts:["root","card","head","mark","title","flag","chevron","body","reason","foot","signal","bars","bar","mini","actions",
       "button","primary","button_text","alternative_glyph","undo_glyph","menu","option","option_selected","option_name","recommended","receipt","pending"],
       effects:["prune"]},render:Fn("view_recommendation")
});
fn SmartRecommendation(p){render_component("patterns/smart_recommendation",p)}
fn get(m,k,d){if m!=()&&k in m&&m[k]!=(){m[k]}else{d}}
fn u(n){theme_length("space.unit").times(n)}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn ink(name){let k="recommendation."+name;theme_color(k)}
fn tx(ctx,k,v){ctx.t_or("recommendation."+k,v)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn line(){style().min_width(px(0)).typography("caption").whitespace_nowrap().text_ellipsis()}
fn apply(ctx,n,k,s){n.with_style(ctx.component_style(k,s))}
fn label(ctx,k,text_value,s){apply(ctx,text(text_value),k,s)}
fn actionable(n,p){n.on_click(p).on_key_value("plain+enter",p,()).on_key_value("plain+space",p,())}
fn focus_style(){style().focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))}
fn ui(ctx,seed){let s=ctx.get_state("ui");if s!=()&&s.session==seed.session{s}else{seed}}
fn flip(seed,shown,ctx,e){let s=ui(ctx,seed);s.open=!shown;ctx.set_state("ui",s);handled()}
fn set_menu(seed,ctx,shown){let s=ui(ctx,seed);s.menu=shown;ctx.set_state("ui",s);}
fn toggle_menu(seed,ctx,e){let s=ui(ctx,seed);s.menu=!s.menu;ctx.set_state("ui",s);handled()}
fn pick(seed,id,ctx,e){
    let s=ui(ctx,seed);s.pick=id;s.menu=false;s.open=true;ctx.set_state("ui",s);ctx.focus("alternatives");
    handled()
}
fn decide(seed,t,accepted,id,controlled,ctx,e){
    let s=ui(ctx,seed);
    if !controlled{s.accepted=if accepted{""}else{id};s.pick=id;}
    s.menu=false;ctx.set_state("ui",s);
    ctx.emit(if accepted{"undo"}else{"accept"},#{session:t.session,revision:t.revision,id:id});
    handled()
}
fn noop(ctx,d){}
fn prune(ctx,d){
    let s=ctx.get_state("ui");if s==()||s.session!=d.session{return;}
    if d.pick!=""&&s.pick==d.pick{s.pick="";}
    if d.accepted!=""&&s.accepted==d.accepted{s.accepted="";}
    if d.close{s.menu=false;}
    ctx.set_state("ui",s);
}
fn move_to(k,ctx,e){ctx.focus(k);ctx.scroll_into_view(k);handled()}
fn glyph(name){
    let paths=switch name{
        "chevron"=>"m4.5 6.25 3.5 3.5 3.5-3.5",
        "status"=>"m3.5 8.5 3 3 6-7",
        "alternatives"=>"M5 12.5v-9m-2.25 2.25L5 3.5l2.25 2.25M11 3.5v9m-2.25-2.25L11 12.5l2.25-2.25",
        _=>"M3.5 6.5h6.25a2.75 2.75 0 0 1 0 5.5H8M5.75 4.25 3.5 6.5l2.25 2.25"};
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.2' stroke-linecap='round' stroke-linejoin='round'><path d='"+paths+"'/></svg>")
        .accessibility_hidden(true).with_style(style().width(u(4)).height(u(4)).flex_shrink(false))
}
fn spark(){
    svg("<svg viewBox='0 0 16 16' fill='currentColor'><path d='M6.5 2.3c.2 0 .3.1.4.4.5 2.6 1.4 3.4 4 4a.4.4 0 0 1 0 .8c-2.6.5-3.5 1.4-4 4a.4.4 0 0 1-.8 0c-.5-2.6-1.4-3.5-4-4a.4.4 0 0 1 0-.8c2.6-.6 3.5-1.4 4-4 .1-.3.2-.4.4-.4ZM12 9.6c.2 0 .2.1.3.3.2 1.2.7 1.7 2 2a.25.25 0 0 1 0 .5c-1.3.2-1.8.7-2 2a.25.25 0 0 1-.5 0c-.3-1.3-.8-1.8-2-2a.25.25 0 0 1 0-.5c1.2-.3 1.7-.8 2-2 .1-.2.1-.3.2-.3Z'/></svg>")
        .accessibility_hidden(true).with_style(style().width(u(4.5)).height(u(4.5)).flex_shrink(false))
}
fn lowered(name){
    let first=grapheme_slice(name,0,1);let second=grapheme_slice(name,1,2);
    if first==()||second==(){return name;}
    let a=first.to_string();let b=second.to_string();
    if a!=a.to_lower()&&b!=b.to_upper(){return a.to_lower()+grapheme_slice(name,1,grapheme_count(name)).to_string();}
    name
}
fn signal_text(ctx,n){
    switch n{0=>tx(ctx,"signal0","No signal"),1=>tx(ctx,"weak","Weak signal"),2=>tx(ctx,"good","Good signal"),_=>tx(ctx,"strong","Strong signal")}
}
fn motion_fade(){motion_transition("opacity",0.0,1.0,#{duration_ms:300,easing:ease(),intent:"decorative"})}
fn bars(ctx,n,mini){
    let bits=[];
    for i in 0..3{
        let key=if n==0&&i==0&&!mini{"signal0"}else if n<=i{"idle"}else if n==1{"weak"}else if n==2{"good"}else{"strong"};
        bits.push(apply(ctx,box([]).with_key("bar:"+i.to_string()),"bar",style().width(u(0.75)).height(u(1.5+0.75*i.to_float()))
            .radius(u(0.25)).flex_shrink(false).background(ink(key))).paint_transition(260,"ease_out"));
    }
    apply(ctx,row(bits).accessibility_hidden(true),if mini{["bars","mini"]}else{["bars"]},style().items_end().gap(sp(0.5)).flex_shrink(false))
}
fn confidence(ctx,n){
    let words=signal_text(ctx,n);
    apply(ctx,row([bars(ctx,n,false),text(words)]).with_key("confidence").accessibility_role("group").accessibility_label(words),
        "signal",line().font_weight(400).gap(sp(2)).items_center().text_color(ink("muted")).flex_shrink(false))
}
fn header(ctx,p,m){
    let name=get(p,"title",tx(ctx,"title","Smart recommendation"));
    let mark=apply(ctx,row([spark()]),"mark",style().text_color(ink("good")).flex_shrink(false));
    let bits=[mark,label(ctx,"title",name,style().min_width(px(0)).typography("body").whitespace_nowrap().text_ellipsis().font_weight(500).flex_grow())];
    if m.alt{
        bits.push(label(ctx,"flag",tx(ctx,"alternative","Alternative"),line().text_color(ink("chip_ink")).background(ink("chip"))
            .padding_x(sp(2)).padding_y(sp(0.25)).radius(theme_radius("lg")).flex_shrink(false)));
    }
    let turn=if m.open{0.0}else if m.rtl{90.0}else{-90.0};
    let cue=apply(ctx,row([glyph("chevron")]).with_key("chevron"),"chevron",style().text_color(ink("muted")).flex_shrink(false))
        .motion(motion_transition("rotate",turn,turn,#{duration_ms:280,easing:ease(),intent:"decorative"}));
    bits.push(cue);
    let h=apply(ctx,row(bits).with_key("heading").accessibility_role("button").accessibility_label(name)
        .accessibility_expanded(m.open).tab_stop(true),"head",focus_style().width(relative(1.0)).min_width(px(0)).min_height(u(10))
            .items_center().gap(sp(2.5)).padding_x(sp(2.5)).radius(theme_radius("lg")).cursor_pointer().hover(style().background(ink("hover"))));
    actionable(h,Fn("flip").curry(m.seed,m.open)).paint_transition(150,"ease_out")
}
fn main_card(ctx,p,m,o){
    let contents=[header(ctx,p,m)];let reason=[];
    if m.open{
        let content=if type_of(o.reason)=="string"{text(o.reason)}else{o.reason};
        reason.push(apply(ctx,column([content]).with_key("reason:"+o.id),"reason",style().min_width(px(0)).typography("body")
            .text_color(ink("reason")).padding_x(sp(2.5)).padding_bottom(sp(2))).enter_motion(motion_fade()));
    }
    let body=apply(ctx,column(reason).with_key("reason-body"),"body",style().min_width(px(0)))
         .layout_size_motion(if m.open{"expanded"}else{"collapsed"},"height",340,ease());
    contents.push(body);
    apply(ctx,column(contents).with_key("card"),"card",style().min_width(px(0)).background(ink("paper"))
        .radius(theme_radius("lg")).padding_x(sp(2)).padding_top(sp(1.5)).padding_bottom(sp(2)))
}
fn control(ctx,k,name,summary,bits,primary,active,cb){
    let entries=bits;
    entries.push(label(ctx,"button_text",name,line().font_weight(500)));
    let base=focus_style().min_width(px(0)).max_width(relative(1.0)).min_height(u(8)).items_center().justify_center().gap(sp(1.5))
        .padding_x(sp(3)).radius(theme_radius("sm")).typography("caption").font_weight(500).cursor_pointer()
        .background(ink(if primary{"solid"}else{"action"})).text_color(if primary{ink("on_solid")}else{theme_color("text_primary")})
        .hover(if primary{style().opacity(0.90)}else{style().background(ink("action_hover"))}).disabled(style().opacity(0.55));
    let parts=if primary{["button","primary"]}else{["button"]};
    let n=apply(ctx,row(entries).with_key(k).accessibility_role("button").accessibility_label(summary).disabled(!active).tab_stop(active),parts,base)
        .paint_transition(160,"ease_out");
    if active{actionable(n,cb)}else{n}
}
fn opt_ref(id){"option:"+id}
fn options_menu(ctx,p,m){
    let label_text=tx(ctx,"alternatives","Alternatives");
    let icon=apply(ctx,row([glyph("alternatives")]),"alternative_glyph",style().flex_shrink(false).text_color(ink("muted")));
    let trigger=control(ctx,"alternatives",label_text,label_text,[icon],false,!m.pending,Fn("toggle_menu").curry(m.seed))
        .with_ref(element_ref("alternatives")).accessibility_expanded(m.menu);
    let rows=[];
    if m.menu{
        for i in 0..p.options.len{
            let o=p.options[i];let key=opt_ref(o.id);let legend=signal_text(ctx,o.signal);
            let children=[label(ctx,"option_name",o.label,line().typography("body").flex_grow())];
            if i==0{children.push(label(ctx,"recommended",tx(ctx,"recommended","Recommended"),line()
                .background(ink("tag")).text_color(ink("good")).font_weight(500).radius(theme_radius("sm")).padding_x(sp(1.25)).flex_shrink(false)));}
            let words=row(children).with_style(style().items_center().gap(sp(1)).min_width(px(0)).flex_grow());
            let current=m.id==o.id;
            let base=focus_style().width(relative(1.0)).min_width(px(0)).min_height(u(8.5)).items_center().gap(sp(2)).justify_between()
                .padding_x(sp(2.5)).radius(theme_radius("sm")).cursor_pointer().background(if current{ink("selected")}else{rgba(0x00000000)})
                .hover(style().background(ink("hover")));
            let option=apply(ctx,row([words,bars(ctx,o.signal,true)]).with_key(key).with_ref(element_ref(key)).tab_stop(true)
                .accessibility_role("menuitem").accessibility_checked(current).accessibility_label(o.label)
                .accessibility_description(legend+(if i==0{" · "+tx(ctx,"recommended","Recommended")}else{""})),
                    if current{["option","option_selected"]}else{["option"]},base);
            option=actionable(option,Fn("pick").curry(m.seed,o.id))
                .on_key_value("plain+down",Fn("move_to").curry(opt_ref(p.options[(i+1)%p.options.len].id)),())
                .on_key_value("plain+up",Fn("move_to").curry(opt_ref(p.options[(i+p.options.len-1)%p.options.len].id)),())
                .on_key_value("plain+home",Fn("move_to").curry(opt_ref(p.options[0].id)),())
                .on_key_value("plain+end",Fn("move_to").curry(opt_ref(p.options[p.options.len-1].id)),());
            rows.push(option);
        }
    }
    let menu=apply(ctx,column(rows).with_key("options-panel").accessibility_role("menu").accessibility_label(label_text)
        .scrollbars("hidden","auto"),"menu",style().width(u(65)).max_width(px(m.menu_width)).max_height(u(80)).min_width(px(0))
        .overflow_y_scroll().gap(sp(0.5)).padding(sp(1)).radius(theme_radius("lg")).background(ink("popup"))).enter_motion(motion_fade());
    let config=#{id:"recommendation-options",kind:"menu",open:m.menu,placement:"top",align:if m.narrow{"start"}else{"end"},gap:6.0,
        modal:false,initial_focus:"first",activate_on_trigger:false,dismiss_on_escape:true,dismiss_on_outside:true};
    overlay(trigger,menu,config).with_key("choices").with_style(style().max_width(relative(1.0)).min_width(px(0)).flex_shrink(false))
        .on_open_change(Fn("set_menu").curry(m.seed))
}
fn footer(ctx,p,m,o){
    let actions=[];let cb=Fn("decide").curry(m.seed,m.ticket,m.done,o.id,m.controlled);
    let enabled=!m.pending&&(!m.controlled||(if m.done{get(p,"on_undo",())!=()}else{get(p,"on_accept",())!=()}));
    if m.done{
        let title=ctx.t_or("recommendation.accepted","{label} accepted",#{label:o.label});
        actions.push(apply(ctx,row([glyph("status"),text(title).with_style(line().font_weight(500))]).with_key("receipt")
             .accessibility_role("status").accessibility_label(title),"receipt",line().items_center().gap(sp(1.5))
             .padding_x(sp(2)).min_height(u(7)).radius(theme_radius("sm")).text_color(ink("success"))
             .background(ink("success_bg")).flex_grow()).enter_motion(motion_fade()));
        let undo=tx(ctx,"undo","Undo");
        let glyph=apply(ctx,row([glyph("undo")]),"undo_glyph",style().flex_shrink(false).text_color(ink("muted")));
        actions.push(control(ctx,"decision",undo,undo,[glyph],false,enabled,cb).with_style(style().flex_shrink(false)));
    }else{
        if p.options.len>1{actions.push(options_menu(ctx,p,m));}
        let accept=ctx.t_or("recommendation.accept","Accept {label}",#{label:lowered(o.label)});
        actions.push(control(ctx,"decision",accept,accept,[],true,enabled,cb));
    }
    let action=apply(ctx,row(actions).with_key("actions"),"actions",style().max_width(relative(1.0)).min_width(px(0))
        .justify_end().items_center().gap(sp(1.5)));
    let signal=confidence(ctx,o.signal);
    let content=if m.narrow{column([signal,action]).with_style(style().items_start())}
        else{row([signal,action]).with_style(style().flex_wrap().justify_between().items_center())};
    let inside=[content];
    if m.pending{
        let waiting=tx(ctx,"pending","Waiting for confirmation");
        inside.push(label(ctx,"pending",waiting,line().text_color(ink("muted"))).with_key("pending")
            .accessibility_role("status").accessibility_label(waiting));
    }
    apply(ctx,column(inside).with_key("foot"),"foot",style().min_width(px(0)).padding_y(sp(2)).padding_x(sp(2)).gap(sp(1.5)))
}
fn valid_id(id){
    if utf8_byte_count(id)==0||utf8_byte_count(id)>96{return false;}
    for c in id.chars(){if c<' '||(c>='\u007f'&&c<='\u009f'){return false;}}
    true
}
fn view_recommendation(ctx,p){
    if !valid_id(p.session){throw "Smart recommendation requires a stable session ID";}
    let indices=#{};
    for i in 0..p.options.len{
        let opt=p.options[i];
        let name=opt.label;name.trim();
        if !valid_id(opt.id)||opt.id in indices||name==""||utf8_byte_count(opt.label)>1024{throw "Recommendation IDs and labels must be valid and IDs unique";}
        indices[opt.id]=i;
    }
    let seed=#{session:p.session,open:p.default_open,pick:"",accepted:"",menu:false};let s=ui(ctx,seed);
    let controlled="accepted_id" in p;
    let accepted=if controlled{get(p,"accepted_id","")}else{s.accepted};
    let done=accepted in indices;
    let menu=!done&&s.menu&&!p.pending&&p.options.len>1;
    let lost_pick=if s.pick!=""&&!(s.pick in indices){s.pick}else{""};
    let lost_accepted=if s.accepted!=""&&!(s.accepted in indices){s.accepted}else{""};
    if lost_pick!=""||lost_accepted!=""||s.menu&&!menu{
        effect("prune",#{session:p.session,pick:lost_pick,accepted:lost_accepted,close:s.menu&&!menu},Fn("prune"),Fn("noop"));
    }
    if p.options.len==0{return column([]).with_key(p.key);}
    let ref=element_ref("shell");let narrow=ctx.element_width_class(ref,[420])==0;
    let size=if menu{ctx.element_size(ref)}else{()};
    let index=if done{indices[accepted]}else if s.pick in indices{indices[s.pick]}else{0};
    let o=p.options[index];
    let m=#{open:s.open,seed:seed,alt:index!=0,id:o.id,done:done,menu:menu,controlled:controlled,pending:p.pending,
        rtl:ctx.text_direction()=="rtl",narrow:narrow,
        menu_width:if size==(){260.0}else{max(1.0,size.width-16.0)},ticket:#{session:p.session,revision:p.revision}};
    apply(ctx,column([main_card(ctx,p,m,o),footer(ctx,p,m,o)]).with_key(p.key).with_ref(ref).accessibility_role("group")
        .accessibility_label(get(p,"title",tx(ctx,"title","Smart recommendation"))),"root",
        style().width(relative(1.0)).max_width(u(110)).min_width(px(0)).padding(sp(1.5))
            .radius(theme_radius("lg")).background(ink("tray")))
}
