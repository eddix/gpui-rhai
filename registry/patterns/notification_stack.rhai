/* gpui-rhai
{"id":"patterns/notification_stack","export":"NotificationStack","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","patterns/agent_tree"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","metrics.icon","notification_stack.art","notification_stack.art_dot","notification_stack.badge","notification_stack.peek","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],"environment":[]}
*/
// Only the front notice is in the input and accessibility trees.
// Example: NotificationStack(#{key:"notices",stack:"updates",revision:0,notices:[#{id:"release",title:"Released",text:"See what changed."}]})
import "components/button" as btn;
import "patterns/agent_tree" as ids;

define_component(#{
    metadata:#{id:"patterns/notification_stack","export":"NotificationStack",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/button","patterns/agent_tree"],capabilities:#{},assets:[],tokens:["border","focus_ring","metrics.icon","notification_stack.art","notification_stack.art_dot","notification_stack.badge","notification_stack.peek","radius.lg","radius.pill","radius.sm","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],environment:[]},
    schema:#{
        props:#{
            key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable stack layout key."},
            stack:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of the notice queue."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision."},
            notices:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique notice ID."},title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Notice heading."},text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Notice body."},badge:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional short caption."},pattern:#{schema:#{type:"optional",value:#{type:"string",allowed:["orbit","grid","rays"]}},required:false,sensitive:false,doc:"Top trailing line art; defaults to orbit."},action:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Action label."},id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Opaque action ID interpreted by the Host."},external:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Show the external-action arrow; the Host still owns navigation."}}}},required:false,sensitive:false,doc:"Optional single action."}}}},required:true,sensitive:true,doc:"Accepted notice queue, front first. Only the front notice is interactive."},
            label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accessible stack label."},
            busy:#{schema:#{type:"bool"},required:false,sensitive:false,doc:"Preserve the front and disable proposals while the Host settles.","default":#{type:"bool",value:false}},
            on_dismiss:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Propose moving past the front notice. The Host accepts by removing it."},
            on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Propose the current notice's action; no built-in URL opening."},
        },
        state:#{fields:#{}},
        events:#{
            dismiss:#{payload:#{type:"object",allow_unknown:false,fields:#{stack:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of the notice queue."},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision."},id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Current front notice ID."}}},doc:"Dismissal proposal scoped to the current front and revision."},
            action:#{payload:#{type:"object",allow_unknown:false,fields:#{stack:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of the notice queue."},revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted snapshot revision."},id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Current front notice ID."},action:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Action ID supplied with this notice."}}},doc:"Request the action on the accepted front notice."},
        },
        slots:#{},parts:["root","panel","rear","rear_far","art","body","top","badge","title","text","dismiss","dismiss_icon","action","arrow","empty"],
    },
    render:Fn("render_stack"),
});
fn NotificationStack(p){render_component("patterns/notification_stack",p)}
fn get(m,k,d){if k in m&&m[k]!=(){m[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn u(n){theme_length("space.unit").times(n)}
fn spline(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn notice_event(name,p,ctx,e){ctx.emit(name,p);handled()}
fn ink(ctx,value,part,base){text(value).with_style(ctx.component_style(part,base.min_width(px(0))))}
fn symbol(ctx,kind,part){
    let p=if kind=="dismiss"{"m4 4 8 8M12 4l-8 8"}else{"M5 11 11 5M6 5h5v5"};
    let size=theme_length("metrics.icon");
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><path d='"+p+"'/></svg>")
        .accessibility_hidden(true).with_style(ctx.component_style(part,style().width(size).height(size).flex_shrink(false)))
}
fn drawing(ctx,kind,rtl){
    let paths=switch kind{
        "rays"=>"M210 6 0 18M210 6 0 56M210 6 0 95M210 6 0 139M210 6 39 160M210 6 98 160M210 6 149 160M210 6 193 160M120 6a90 90 0 0 0 90 90",
        "grid"=>"M30 0v160M60 0v160M90 0v160M120 0v160M150 0v160M180 0v160M210 0v160M0 20h220M0 50h220M0 80h220M0 110h220M0 140h220",
        _=>"M144 10a46 46 0 1 0 92 0 46 46 0 1 0-92 0M104 10a86 86 0 1 0 172 0 86 86 0 1 0-172 0M62 10a128 128 0 1 0 256 0 128 128 0 1 0-256 0M60 160 220 0M0 58h220M150 0v160",
    };
    let dot=if kind=="rays"{"146 70"}else if kind=="grid"{"180 50"}else{"150 58"};
    let xy=dot.split(" ");
    let theme=ctx.theme_variant();
    let corner=if theme!=()&&theme.family=="Clean"{12}else{20};
    // Clip the art against the actual corner as well as the card's rectangular
    // overflow mask. The whole graphic mirrors in the logical trailing corner.
    let c=corner.to_string();let edge=(220-corner).to_string();
    let clip="M0 0H"+edge+"A"+c+" "+c+" 0 0 1 220 "+c+"V160H0Z";
    let source="<svg viewBox='0 0 220 160' fill='none'><defs><clipPath id='corner'><path d='"+clip+"'/></clipPath><linearGradient id='fade' x1='0' x2='1' y1='1' y2='0'><stop stop-color='currentColor' stop-opacity='0'/><stop offset='.62' stop-color='currentColor' stop-opacity='.70'/><stop offset='1' stop-color='currentColor'/></linearGradient></defs><path clip-path='url(#corner)' stroke='url(#fade)' stroke-width='1' d='"+paths+"'/></svg>";
    let spot="<svg viewBox='0 0 220 160'><circle cx='"+xy[0]+"' cy='"+xy[1]+"' r='3.5' fill='currentColor'/></svg>";
    let overlay=style().absolute().top(px(0)).left(px(0)).width(relative(1.0)).height(relative(1.0));
    let base=style().absolute().inset_end(px(0)).top(px(0)).width(u(55)).height(u(40));
    if rtl{base=base.transform(#{scale_x:-1.0});}
    stack([svg(source).with_key("line-art").with_style(overlay.text_color(theme_color("notification_stack.art"))),
           svg(spot).with_key("dot").with_style(overlay.text_color(theme_color("notification_stack.art_dot")))])
        .with_key("art").accessibility_hidden(true).with_style(ctx.component_style("art",base))
}
fn shell(ctx,index){
    let inset=relative(if index==1{0.025}else{0.05});
    box([]).with_key("back:"+index.to_string()).accessibility_hidden(true)
        .with_style(ctx.component_style(if index==2{["rear","rear_far"]}else{["rear"]},style().absolute().top(inset)
            .bottom(u(if index==1{3}else{0})).inset_start(inset).inset_end(inset).radius(theme_radius("lg"))
            .border(px(1)).border_color(theme_color("border")).background(theme_color("notification_stack.peek"))))
}
fn panel(ctx,p,n){
    let tint=style().typography("caption").text_color(theme_color("text_muted"));
    let phrasing=[];
    let badge=get(n,"badge","");
    if badge!=""{
        phrasing.push(ink(ctx,badge.to_upper(),"badge",tint.whitespace_nowrap().text_ellipsis().padding_x(sp(2))
            .radius(theme_radius("pill")).border(px(1)).border_color(theme_color("border"))).with_key("badge"));
    }
    let top=row(phrasing).with_key("top").with_style(ctx.component_style("top",style().min_height(u(6.5))
        .min_width(px(0)).items_center().padding_end(u(8))));
    let title=ink(ctx,n.title,"title",style().typography("body").font_weight(600).text_color(theme_color("text_primary")))
        .with_key("title").accessibility_role("heading").accessibility_level(2);
    let prose=ink(ctx,n.text,"text",style().typography("body").text_color(theme_color("text_muted"))).with_key("details");
    let body=[top,title,prose];let a=get(n,"action",());
    if a!=()&&get(p,"on_action",())!=(){
        let props=#{key:"action",text:a.label,size:"sm",variant:"secondary",disabled:p.busy,
            on_click:Fn("notice_event").curry("action",#{stack:p.stack,revision:p.revision,id:n.id,action:a.id})};
        if get(a,"external",false){props.suffix=symbol(ctx,"arrow","arrow");}
        body.push(btn::Button(props).with_style(ctx.component_style("action",style().self_start().max_width(relative(1.0)))));
    }
    box([drawing(ctx,get(n,"pattern","orbit"),ctx.text_direction()=="rtl"),
         column(body).with_key("body").with_style(ctx.component_style("body",style()
            .min_width(px(0)).width(relative(1.0)).gap(sp(2)).padding(sp(5)).isolated_opacity(true)))
            .motion(motion_transition("opacity",0.0,1.0,#{delay_ms:155,duration_ms:245,easing:spline(),intent:"decorative"}))])
        .with_key("notice:"+n.id).with_style(ctx.component_style("panel",style().min_width(px(0))
            .width(relative(1.0)).relative().overflow_hidden().radius(theme_radius("lg"))
            .border(px(1)).border_color(theme_color("border")).background(theme_color("surface_raised")).isolated_opacity(true)))
        .exit_motion(motion_transition("opacity",1.0,0.0,#{duration_ms:240,intent:"decorative"}))
        .exit_motion(motion_transition("translate_x",0.0,40.0,#{duration_ms:320,easing:spline(),intent:"decorative"}))
        .exit_motion(motion_transition("scale_x",1.0,0.98,#{duration_ms:320,easing:spline(),intent:"decorative"}))
        .exit_motion(motion_transition("scale_y",1.0,0.98,#{duration_ms:320,easing:spline(),intent:"decorative"}))
}
fn dismiss(ctx,p,n){
    let caption=ctx.t_or("notification_stack.dismiss","Dismiss {title}",#{title:n.title});
    let control=row([symbol(ctx,"dismiss","dismiss_icon")]).with_key("dismiss").with_ref(element_ref("dismiss"))
        .accessibility_role("button").accessibility_label(caption).disabled(p.busy).tab_stop(true)
        .with_style(ctx.component_style("dismiss",style().absolute().top(sp(4)).inset_end(sp(4))
            .width(u(6.5)).height(u(6.5)).items_center().justify_center().radius(theme_radius("sm"))
            .cursor_pointer().text_color(theme_color("text_muted")).focus_ring(px(2),offset_px(-1),theme_color("focus_ring"))));
    if !p.busy{
        let fnp=Fn("notice_event").curry("dismiss",#{stack:p.stack,revision:p.revision,id:n.id});
        control=control.on_click(fnp).on_key_value("plain+enter",fnp,()).on_key_value("plain+space",fnp,());
    }
    control
}
fn render_stack(ctx,p){
    if !ids::valid(p.stack){throw "NotificationStack requires a stable stack ID";}
    let seen=#{};
    for it in p.notices{
        if !ids::valid(it.id)||it.id in seen||it.title==""||utf8_byte_count(it.title)>512||utf8_byte_count(it.text)>4096{
            throw "Notifications need distinct IDs and bounded text";
        }
        seen[it.id]=true;
    }
    let depth=max(0,min(2,p.notices.len-1));
    let body=[];let id="empty";
    if p.notices.len>0{
        let n=p.notices[0];id=n.id;
        for k in 0..depth {body.push(shell(ctx,depth-k));}
        body.push(panel(ctx,p,n));
        if get(p,"on_dismiss",())!=(){body.push(dismiss(ctx,p,n));}
    }
    let words=get(p,"label",ctx.t_or("notification_stack.title","Notifications"));
    column(body).with_key(p.key).accessibility_role("group").accessibility_label(words)
        .with_style(ctx.component_style("root",style().relative().width(relative(1.0)).max_width(px(380)).min_width(px(0))
            .padding_bottom(u(depth*3))))
        .layout_size_motion(id+":"+depth.to_string(),"height",460,spline())
}
