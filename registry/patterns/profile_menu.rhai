/* gpui-rhai
{"id":"patterns/profile_menu","export":"ProfileMenu","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/progress"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","profile_menu.accent","profile_menu.badge_bg","profile_menu.badge_ink","profile_menu.muted","profile_menu.setup_ink","profile_menu.track","profile_menu.tray","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.metadata"],"environment":[]}
*/
// A standalone account card; the caller owns navigation, sign-out and setup.
// Example: ProfileMenu(#{key:"account",name:"Ada Lovelace",email:"ada@example.test",setup:#{done:2,total:5,next:"Choose a profile picture"}})
import "components/progress" as progress;

define_component(#{
    metadata:#{id:"patterns/profile_menu","export":"ProfileMenu",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["components/progress"],capabilities:#{},assets:[],
        tokens:["border","focus_ring","profile_menu.accent","profile_menu.badge_bg","profile_menu.badge_ink","profile_menu.muted","profile_menu.setup_ink","profile_menu.track","profile_menu.tray","radius.lg","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption","typography.metadata"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable account card instance. Use another key for a different account."},
        name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Account display name; omit visual title by passing an empty name."},
        email:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Account caption, such as an email address."},
        avatar:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Custom decorative avatar. Its content owns its own loading behavior."},
        avatar_handle:#{schema:#{type:"optional",value:#{type:"handle",kind:"image"}},required:false,sensitive:false,doc:"Native image, over the fallback silhouette. The silhouette remains visible for pending and failed images. Custom avatar takes precedence."},
        items:#{schema:#{type:"optional",value:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Nonempty stable action ID, unique within the card."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Action name."},
            icon:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Optional decorative action glyph."},
            badge:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional short badge."},
            disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Disabled rows stay visible but are skipped by keyboard navigation."}
        }}}},required:false,sensitive:false,doc:"Actions in display order. Omit for the built-in account actions; an empty list hides the actions."},
        setup:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            done:#{schema:#{type:"integer",min:0,max:1000000},required:true,sensitive:false,doc:"Accepted completed steps; clamped to total."},
            total:#{schema:#{type:"integer",min:0,max:1000000},required:true,sensitive:false,doc:"Accepted number of steps."},
            next:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Next step caption. Omit it to hide the shortcut."}
        }}},required:false,sensitive:false,doc:"Optional profile setup section."},
        on_select:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the stable ID of the chosen action."},
        on_continue:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the current accepted counts when the next step is requested."}
    },state:#{fields:#{revealed:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}},
    events:#{
        select:#{payload:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false}}},
            doc:"An account action was selected. The Host carries out the action."},
        "continue":#{payload:#{type:"object",allow_unknown:false,fields:#{
            done:#{schema:#{type:"integer",min:0},required:true,sensitive:false},
            total:#{schema:#{type:"integer",min:0},required:true,sensitive:false}
        }},doc:"A next step was requested. Props change only when the Host accepts it."}
    },
    slots:#{},parts:["root","card","account","avatar","identity","name","email","actions","item","icon","item_label",
        "item_badge","setup","setup_with_next","setup_head","setup_title","setup_count","track","fill","next","next_hint","next_label","next_icon"],
    effects:["reveal_setup"]},render:Fn("render_profile")
});

fn ProfileMenu(p){render_component("patterns/profile_menu",p)}
fn get(p,k,d){if k in p&&p[k]!=(){p[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn u(n){theme_length("space.unit").times(n)}
fn tx(ctx,k,s){ctx.t_or("profile_menu."+k,s)}
fn ink(ctx,part,value,base){text(value).with_style(ctx.component_style(part,base))}
fn line(){style().typography("body").min_width(px(0)).whitespace_nowrap().text_ellipsis().text_color(theme_color("text_primary"))}
fn glyph(path){
    svg("<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><path d='"+path+"'/></svg>")
        .accessibility_hidden(true).with_style(style().width(relative(1.0)).height(relative(1.0)))
}
fn standard(ctx){
    [#{id:"settings",label:tx(ctx,"settings","Settings"),icon:glyph("M10 3h4l1 3 3 1 3 3v4l-3 1-1 3-3 3h-4l-1-3-3-1-3-3v-4l3-1 1-3ZM15 12a3 3 0 1 0-6 0 3 3 0 1 0 6 0")},
     #{id:"docs",label:tx(ctx,"docs","Docs"),icon:glyph("M6 3h8l4 4v14H6zM14 3v5h4M9 11h6m-6 4h6m-6 3h4")},
     #{id:"apps",label:tx(ctx,"apps","Apps"),badge:tx(ctx,"new","New"),icon:glyph("M4 4h6v6H4zm0 10h6v6H4zm10 0h6v6h-6zm3-10v6m-3-3h6")},
     #{id:"logout",label:tx(ctx,"logout","Logout"),icon:glyph("M11 4H5v16h6m4-12 4 4-4 4m-6-4h10")}]
}
fn items(ctx,p){
    let rows=get(p,"items",());
    if rows==(){rows=standard(ctx);}
    let id_set=#{};
    let enabled=[];
    for i in 0..rows.len{
        let item=rows[i];
        if item.id==""||utf8_byte_count(item.id)>96||item.id in id_set||item.label==""||
            utf8_byte_count(item.label)>512||utf8_byte_count(get(item,"badge",""))>80{
            throw "ProfileMenu needs unique, bounded action IDs and nonempty labels";
        }
        id_set[item.id]=true;
        if !get(item,"disabled",false){enabled.push(i);}
    }
    #{rows:rows,enabled:enabled}
}
fn action_ref(i){"action:"+i.to_string()}
fn move_to(target,ctx,event){
    ctx.focus(target);ctx.scroll_into_view(target);handled()
}
fn choose(id,ctx,event){ctx.emit("select",#{id:id});handled()}
fn proceed(counts,ctx,event){ctx.emit("continue",counts);handled()}
fn action(ctx,m,i){
    let a=m.rows[i];let active=!get(a,"disabled",false);
    let mark=get(a,"icon",());
    let mark=if mark==(){[]}else{[mark.accessibility_hidden(true)]};
    let badge=get(a,"badge","");
    let children=[row(mark).with_key("icon").accessibility_hidden(true)
        .with_style(ctx.component_style("icon",style().width(u(5)).height(u(5)).flex_shrink(false).text_color(theme_color("text_muted")))),
        ink(ctx,"item_label",a.label,line().flex_grow())];
    if badge!=""{
        children.push(ink(ctx,"item_badge",badge,line().typography("metadata").max_width(relative(0.5))
            .flex_shrink(false).padding_x(sp(1.5)).radius(theme_radius("lg")).background(theme_color("profile_menu.badge_bg"))
            .text_color(theme_color("profile_menu.badge_ink"))).with_key("badge"));
    }
    let n=row(children).with_key("item:"+a.id).with_ref(element_ref(action_ref(i)))
        .accessibility_role("menuitem").accessibility_label(a.label).accessibility_description(badge).disabled(!active)
        .with_style(ctx.component_style("item",style().width(relative(1.0)).min_width(px(0)).min_height(u(9))
            .flex_shrink(false).items_center().gap(sp(2)).padding_x(sp(2)).radius(theme_radius("sm"))
            .cursor_pointer().hover(style().background(theme_color("surface_hover")))
            .focus(style().background(theme_color("surface_hover")))
            .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))));
    if !active{return n;}
    let position=0;
    for j in 0..m.enabled.len{if m.enabled[j]==i{position=j;break;}}
    let count=m.enabled.len;
    let next=m.enabled[(position+1)%count];let prior=m.enabled[(position+count-1)%count];
    let click=Fn("choose").curry(a.id);
    n.tab_stop(true).on_click(click).on_key_value("plain+enter",click,()).on_key_value("plain+space",click,())
        .on_key_value("plain+down",Fn("move_to").curry(action_ref(next)),())
        .on_key_value("plain+up",Fn("move_to").curry(action_ref(prior)),())
        .on_key_value("plain+home",Fn("move_to").curry(action_ref(m.enabled[0])),())
        .on_key_value("plain+end",Fn("move_to").curry(action_ref(m.enabled[count-1])),())
        .paint_transition(160,"ease_out")
}
fn account(ctx,p,name){
    let mark=get(p,"avatar",());
    let layers=[];
    if mark!=(){layers.push(mark.accessibility_hidden(true));}
    else{
        layers.push(svg("<svg viewBox='0 0 40 40' fill='currentColor'><circle cx='20' cy='20' r='20' opacity='.1'/><circle cx='20' cy='15.5' r='4.8'/><path d='M11.5 28c1-4.2 4.2-6.6 8.5-6.6s7.5 2.4 8.5 6.6c.2.7-.3 1.2-1 1.2h-15c-.7 0-1.2-.5-1-1.2z'/></svg>")
            .with_style(style().width(relative(1.0)).height(relative(1.0))));
        let handle=get(p,"avatar_handle",());
        if handle!=(){
            layers.push(image(handle).image_fallback_text("").with_style(style().absolute().top(px(0)).left(px(0))
                .width(relative(1.0)).height(relative(1.0))));
        }
    }
    let avatar=box(layers).with_key("avatar").accessibility_hidden(true)
        .with_style(ctx.component_style("avatar",style().relative().width(u(10.5)).height(u(10.5))
            .radius(u(5.25)).clip().flex_shrink(false).text_color(theme_color("profile_menu.accent"))));
    let who=[];
    if name!=""{who.push(ink(ctx,"name",name,line().font_weight(500)));}
    who.push(ink(ctx,"email",p.email,line().typography("caption").text_color(theme_color("text_muted"))));
    row([avatar,column(who).with_style(ctx.component_style("identity",style().flex_grow().min_width(px(0))))])
        .with_key("account").with_style(ctx.component_style("account",style().items_center().min_width(px(0)).gap(sp(3))))
}
fn reveal(ctx,payload){if !ctx.get_state("revealed"){ctx.set_state("revealed",true);}}
fn idle(ctx,payload){}
fn exposed(ctx,reference){
    if ctx.get_state("revealed"){return true;}
    if ctx.motion_preference()!="normal"{return true;}
    let g=ctx.element_bounds(reference);
    if g==()||g.visual.width<=0.0||g.visual.height<=0.0{return false;}
    let v=g.visual;let c=g.clip;
    let overlap=if c==(){v.width*v.height}else{
        max(0.0,min(v.x+v.width,c.x+c.width)-max(v.x,c.x))*max(0.0,min(v.y+v.height,c.y+c.height)-max(v.y,c.y))
    };
    if overlap>=v.width*v.height*0.3{
        effect("reveal_setup",true,Fn("reveal"),Fn("idle"));
        return true;
    }
    false
}
fn setup(ctx,s,reference){
    let label=tx(ctx,"setup","Profile setup");
    let done=min(s.total,s.done);
    let percent=if s.total>0{done.to_float()/s.total.to_float()}else{0.0};
    let amount=ctx.format_number(done)+"/"+ctx.format_number(s.total);
    let head=row([ink(ctx,"setup_title",label,line().typography("caption").font_weight(500)
        .text_color(theme_color("profile_menu.setup_ink"))).with_key("label"),
        ink(ctx,"setup_count",amount,line().typography("caption").font_feature("tnum",1).flex_shrink(false)
            .text_color(theme_color("profile_menu.muted"))).with_key("count")]).with_key("heading")
        .with_style(ctx.component_style("setup_head",style().min_width(px(0)).justify_between().items_center().gap(sp(2))));
    // Store the accepted width in layout, and keep the entry reveal in a
    // separate native motion lane. Accepted steps never restart the reveal.
    let origin=if ctx.text_direction()=="rtl"{1.0}else{0.0};
    let fill=ctx.component_style("fill",style().height(relative(1.0)).radius(px(2))
        .background(theme_color("profile_menu.accent")).transform(#{origin_x:origin,origin_y:0.5}));
    let indicator=progress::fraction_fill("fill",percent,fill,[])
        .accessibility_hidden(true)
        .motion(motion_transition("scale_x",0.0,if exposed(ctx,reference){1.0}else{0.0},
            #{delay_ms:150,duration_ms:900,easing:cubic_bezier(0.22,1.0,0.36,1.0),intent:"decorative"}));
    let track=row([indicator]).with_key("track").accessibility_role("progressbar").accessibility_label(label)
        .accessibility_value((percent*100.0).round().to_int()).accessibility_value_min(0).accessibility_value_max(100)
        .accessibility_description(amount).with_style(ctx.component_style("track",style().min_width(px(0))
            .height(u(1)).radius(px(2)).clip().background(theme_color("profile_menu.track"))));
    let out=[head,track];
    let next=get(s,"next","");
    let show_next=next!=""&&done<s.total;
    if show_next{
        let hint=tx(ctx,"next","Next:");
        let arrow=if ctx.text_direction()=="rtl"{"M19 12H5m6 6-6-6 6-6"}else{"M5 12h14m-6-6 6 6-6 6"};
        let click=Fn("proceed").curry(#{done:done,total:s.total});
        let n=row([row([ink(ctx,"next_hint",hint,line().typography("caption").flex_shrink(false)
            .text_color(theme_color("profile_menu.muted"))).with_key("hint"),
            ink(ctx,"next_label",next,line().typography("caption")).with_key("label")])
            .with_key("words").with_style(style().min_width(px(0)).gap(sp(1)).flex_grow()),
            row([glyph(arrow)]).with_style(ctx.component_style("next_icon",style().width(u(3.5)).height(u(3.5))
                .flex_shrink(false).text_color(theme_color("text_muted"))))]).with_key("next")
            .accessibility_role("button").accessibility_label(hint+" "+next).tab_stop(true)
            .on_click(click).on_key_value("plain+enter",click,()).on_key_value("plain+space",click,())
            .with_style(ctx.component_style("next",style().items_center().justify_between().min_width(px(0)).gap(sp(1.5))
                .min_height(u(7)).padding_x(sp(1.5)).radius(theme_radius("sm")).cursor_pointer()
                .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
            .paint_transition(160,"ease_out");
        out.push(n);
    }
    let parts=if show_next{["setup","setup_with_next"]}else{["setup"]};
    column(out).with_key("setup").with_style(ctx.component_style(parts,style().min_width(px(0)).gap(sp(1)).padding(sp(2))))
}
fn render_profile(ctx,p){
    let name=p.name;name.trim();
    let m=items(ctx,p);
    let r=element_ref("profile");
    let contents=[account(ctx,p,name)];
    if m.rows.len>0{
        let actions=[];
        for i in 0..m.rows.len{actions.push(action(ctx,m,i));}
        contents.push(column(actions).with_key("actions").accessibility_role("menu").accessibility_label(tx(ctx,"account","Account"))
            .with_style(ctx.component_style("actions",style().min_width(px(0)).max_height(u(95)).gap(sp(0.5)).overflow_y_scroll()))
            .scrollbars("hidden","auto"));
    }
    let inner=column(contents).with_key("card").with_style(ctx.component_style("card",style().min_width(px(0))
        .padding(sp(2)).radius(theme_radius("lg")).background(theme_color("surface_raised"))));
    let card=[inner];let s=get(p,"setup",());
    if s!=(){card.push(setup(ctx,s,r));}
    column(card).with_key("profile").with_ref(r).accessibility_role("group")
        .accessibility_label(tx(ctx,"profile","Profile menu"))
        .with_style(ctx.component_style("root",style().width(relative(1.0)).min_width(px(0)).max_width(u(80))
            .padding(sp(1)).radius(theme_radius("lg")).background(theme_color("profile_menu.tray"))))
}
