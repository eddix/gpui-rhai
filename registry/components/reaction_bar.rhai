/* gpui-rhai
{"id":"components/reaction_bar","export":"ReactionBar","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"assets":[],"tokens":["accent","focus_ring","surface_hover","text_primary","typography.caption","radius.sm","reaction.ink","space.spacing_unit","space.unit","reaction.comment","reaction.like","reaction.repost","reaction.share"],"environment":["density"]}
*/
// ReactionBar shows a compact set of social actions and their exact accessible counts.
// State: local unless the application provides a reactions value.
// Example: ReactionBar(#{key:"release",actions:["like","share"]})
define_component(#{
    metadata:#{id:"components/reaction_bar","export":"ReactionBar",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:[],capabilities:#{},assets:[],tokens:["accent","focus_ring","surface_hover","text_primary","typography.caption","radius.sm","reaction.ink","space.spacing_unit","space.unit","reaction.comment","reaction.like","reaction.repost","reaction.share"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable reaction group identity."},
        label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Accessible name of the reaction group."},
        actions:#{schema:#{type:"array",max_items:4,items:#{type:"string",allowed:["like","comment","repost","share"]}},
            required:false,sensitive:false,"default":#{type:"array",value:[#{type:"string",value:"like"},#{type:"string",value:"comment"},
                #{type:"string",value:"repost"},#{type:"string",value:"share"}]},doc:"Visible actions in order; duplicates are shown once."},
        reactions:#{schema:#{type:"optional",value:#{type:"array",max_items:4,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string",allowed:["like","comment","repost","share"]},required:true,sensitive:false,doc:"Reaction ID."},
            count:#{schema:#{type:"integer",min:0,max:999999999999999},required:true,sensitive:false,doc:"Exact accepted count."},
            active:#{schema:#{type:"bool"},required:true,sensitive:false,doc:"Accepted pressed state."},
        }}}},required:false,sensitive:false,doc:"Controlled states; omitted reactions start at zero. Omit the whole property for local reactions."},
        disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Blocks all reactions."},
        on_react:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives one proposal: #{id,count,active}."},
    },state:#{fields:#{
        local:#{schema:#{type:"ui_value"},"default":#{type:"map",value:#{}}},
    }},events:#{react:#{payload:#{type:"object",allow_unknown:false,fields:#{
        id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Reaction ID."},
        count:#{schema:#{type:"integer"},required:true,sensitive:false,doc:"Proposed exact count."},
        active:#{schema:#{type:"bool"},required:true,sensitive:false,doc:"Proposed pressed state."},
    }},doc:"One proposed reaction state."}},slots:#{},
        parts:["root","action","action_active","like","comment","repost","share","icon","count","digit"]},
    render:Fn("render_ReactionBar")
});

fn ReactionBar(props){render_component("components/reaction_bar",props)}
fn optional(props,name){if name in props{props[name]}else{()}}
fn short_count(ctx,n){
    if n<1000{return ctx.format_number(n);}
    let unit=1000;let suffix="K";
    if n>=999950{unit=1000000;suffix="M";}
    if n>=999950000{unit=1000000000;suffix="B";}
    if n>=999950000000{unit=1000000000000;suffix="T";}
    let tenth=(n+(unit/20))/(unit/10);
    if tenth%10==0{return ctx.format_number(tenth/10)+suffix;}
    ctx.format_number(tenth.to_float()/10.0)+suffix
}
fn reaction_name(ctx,id){
    if id=="like"{ctx.t_or("reaction.like","Like")}
    else if id=="comment"{ctx.t_or("reaction.comment","Comment")}
    else if id=="repost"{ctx.t_or("reaction.repost","Repost")}
    else{ctx.t_or("reaction.share","Share")}
}
fn reaction_glyph(id,active){
    let path=if id=="like"{"M12 20.3 4.7 13a4.5 4.5 0 0 1 6.4-6.3l.9.9.9-.9a4.5 4.5 0 0 1 6.4 6.3z"}
        else if id=="comment"{"M20.5 11.2a8 8 0 0 1-8.4 8.2 8.4 8.4 0 0 1-3.1-.6L4 20l1.2-4.3a7.9 7.9 0 0 1-1.5-4.5 8 8 0 0 1 8.4-8.2 8 8 0 0 1 8.4 8.2z"}
        else if id=="repost"{"m2.5 8 3.5-3.5L9.5 8M6 4.5v12a2 2 0 0 0 2 2h4m9.5-2.5L18 19.5 14.5 16m3.5 3.5v-12a2 2 0 0 0-2-2h-4"}
        else{"M12 15V3m-4 4 4-4 4 4M6 10H5v10h14V10h-1"};
    let fill=if active&&(id=="like"||id=="comment"){"currentColor"}else{"none"};
    svg(`<svg viewBox='0 0 24 24' fill='${fill}' stroke='currentColor' stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true)
}
fn activate(current,controlled,ctx,payload){
    let proposed=#{id:current.id,count:min(max(current.count+(if current.active{-1}else{1}),0),999999999999999),active:!current.active};
    if !controlled{
        let local=ctx.get_state("local");local[current.id]=proposed;ctx.set_state("local",local);
    }
    ctx.emit("react",proposed);
    handled()
}
fn render_ReactionBar(ctx,props){
    let value=optional(props,"reactions");let controlled=value!=();
    let model=if controlled{#{}}else{ctx.get_state("local")};
    if controlled{for entry in value{if entry.id in model{throw "ReactionBar reaction IDs must be unique";}model[entry.id]=entry;}}
    let nodes=[];let seen=[];
    let disabled=props.disabled||(controlled&&optional(props,"on_react")==());
    for id in props.actions{
        if id in seen{continue;}seen.push(id);
        let current=if id in model{model[id]}else{#{id:id,count:0,active:false}};
        let ink=if !current.active{theme_color("reaction.ink")}else{switch id{
            "like"=>theme_color("reaction.like"),
            "comment"=>theme_color("reaction.comment"),
            "repost"=>theme_color("reaction.repost"),
            _=>theme_color("reaction.share"),
        }};
        let icon=reaction_glyph(id,current.active).with_key("glyph")
            .with_style(ctx.component_style("icon",style().width(theme_length("space.unit").times(4.5)).height(theme_length("space.unit").times(4.5)).flex_shrink(false)))
            .motion(motion_transition("scale_x",0.85,1.0,#{duration_ms:220,intent:"feedback"}))
            .motion(motion_transition("scale_y",0.85,1.0,#{duration_ms:220,intent:"feedback"}))
            .motion_on_change(if current.active{"active"}else{"idle"});
        let content=[icon];
        if current.count>0{
            content.push(row([text(short_count(ctx,current.count)).with_key("digits")
                .with_style(ctx.component_style("digit",style().font_feature("tnum",1)))
                .motion(motion_transition("translate_y",7.0,0.0,#{duration_ms:400,intent:"feedback"}))
                .motion_on_change(`${current.count}`)]).with_key("count")
                .accessibility_hidden(true).with_style(ctx.component_style("count",style().clip())));
        }
        let action=row(content).with_key(id).accessibility_role("button").accessibility_label(reaction_name(ctx,id))
            .accessibility_description(ctx.t_or("reaction.total","Count: {count}",#{count:ctx.format_number(current.count)}))
            .accessibility_pressed(current.active).disabled(disabled).tab_stop(!disabled)
            .with_style(ctx.component_style(if current.active{["action",id,"action_active"]}else{["action",id]},style().min_height(theme_length("space.unit").times(8)).min_width(px(28)).padding_x(theme_length("space.spacing_unit").times(1.5))
                .gap(theme_length("space.spacing_unit")).items_center().justify_center().radius(theme_radius("sm")).typography("caption").text_color(ink)
                .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))
                .hover(style().background(theme_color("surface_hover")))))
            .paint_transition(150,cubic_bezier(0.22,1.0,0.36,1.0));
        if !disabled{action=action.on_click(Fn("activate").curry(current,controlled));}
        nodes.push(action);
    }
    let name=optional(props,"label");if name==(){name=ctx.t_or("reaction.title","Reactions");}
    row(nodes).with_key(props.key).accessibility_role("group").accessibility_label(name).tab_stop(false)
        .with_style(ctx.component_style("root",style().items_center().flex_wrap().gap(theme_length("space.spacing_unit")).min_width(px(0))))
}
