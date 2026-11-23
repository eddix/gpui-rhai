/* gpui-rhai
{"id":"components/thinking_loader","export":"ThinkingLoader","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"assets":[],"tokens":["border","surface_raised","text_primary","text_muted","text.success","radius.pill","typography.caption","space.spacing_unit","space.unit","thinking.sheen"],"environment":["size","density"]}
*/
// ThinkingLoader presents a phase label and motion-owned work/completion cue.
// State: controlled phase and completion; optional local phase playback is for previews.
// Example: ThinkingLoader(#{key:"work",phases:["Reading","Writing"],phase:0})
define_component(#{
    metadata:#{id:"components/thinking_loader","export":"ThinkingLoader",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:[],capabilities:#{},assets:[],tokens:["border","surface_raised","text_primary","text_muted","text.success","radius.pill","typography.caption","space.spacing_unit","space.unit","thinking.sheen"],
        environment:["size","density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable activity identity. Use a new key for a new run."},
        phases:#{schema:#{type:"array",max_items:16,items:#{type:"string"}},"default":#{type:"array",value:[]},
            required:false,sensitive:false,doc:"Phase captions. An empty array uses a localized Thinking label."},
        phase:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:15}},required:false,sensitive:false,
            doc:"Accepted zero-based phase; capped by the supplied labels. When present this disables local playback."},
        autoplay:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,
            doc:"Opt into local phase playback for demonstrations. It changes labels only, not completion."},
        interval_ms:#{schema:#{type:"integer",min:200,max:60000},"default":#{type:"integer",value:2200},required:false,sensitive:false,
            doc:"How long each preview label is shown."},
        repeat:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,
            doc:"Preview playback returns to its first label after the final one."},
        done:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,
            doc:"Accepted completion; stops activity and replaces its spinner with a check."},
        stopped:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,
            doc:"Freeze the work cue and show the stopped caption; done takes precedence."},
        elapsed_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9000000000000}},required:false,sensitive:false,
            doc:"Application measured duration used for the final caption. Does not create a wall-clock timer."},
        done_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Overrides the final completion caption."},
        size:#{schema:#{type:"optional",value:#{type:"string",allowed:["sm","md"]}},required:false,sensitive:false,
            doc:"Override the inherited size with sm or md."},
        variant:#{schema:#{type:"string",allowed:["pill","plain"]},"default":#{type:"string",value:"pill"},
            required:false,sensitive:false,doc:"Draw a contained pill or render flush inside a larger surface."},
    },state:#{fields:#{
        local:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
        was_terminal:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
    }},events:#{},slots:#{},parts:["root","plain","icon","ray","check","stop","clip","label","finished","stopped","sheen"],effects:["restart"]},
    render:Fn("render_ThinkingLoader")
});

fn ThinkingLoader(props){render_component("components/thinking_loader",props)}
fn opt(props,key){if key in props{props[key]}else{()}}
fn track_terminal(ctx,terminal){
    if ctx.get_state("was_terminal")&&!terminal{ctx.set_state("local",0);}
    ctx.set_state("was_terminal",terminal);
}
fn cleanup(ctx,terminal){}
fn advance(ctx,next){if !ctx.get_state("was_terminal"){ctx.set_state("local",next);}}
fn finish_label(ctx,label,elapsed){
    if label!=()&&label!=""{return label;}
    if elapsed!=(){return ctx.t_or("thinking.elapsed","Worked for {duration}",#{duration:ctx.format_duration(elapsed)});}
    ctx.t_or("thinking.complete","Finished thinking")
}
fn phase_ink(ctx,stage){
    let value=if stage=="done"{"text.success"}else{"text_primary"};
    theme_color(value)
}
fn check_shape(ctx,stage,part){
    let check=stage=="done";
    let points=if check{[path_move(3.5,8.4),path_line(6.6,11.3),path_line(12.5,4.9)]}
        else{[path_move(4.2,4.2),path_line(11.8,4.2),path_line(11.8,11.8),path_line(4.2,11.8),path_close()]};
    let mark=canvas(canvas_scene([canvas_stroke_path("glyph",points,1.5,current_color()).stroke_cap("round").stroke_join("round")]).view_box(0,0,16,16))
        .with_key(stage).accessibility_hidden(true)
        .with_style(ctx.component_style(part,style().width(relative(1.0)).height(relative(1.0)).text_color(phase_ink(ctx,stage))));
    if check{mark.motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:330,intent:"feedback"}))}else{mark}
}
fn work_rays(ctx){
    let coordinates=[[8,1.75,8,4.75],[12.42,3.58,10.30,5.70],[14.25,8,11.25,8],[12.42,12.42,10.30,10.30],
        [8,14.25,8,11.25],[3.58,12.42,5.70,10.30],[1.75,8,4.75,8],[3.58,3.58,5.70,5.70]];
    let brightness=[1.0,0.82,0.64,0.49,0.38,0.28,0.20,0.14];
    let rays=[];
    for i in 0..8{
        let point=coordinates[i];
        let path=[path_move(point[0],point[1]),path_line(point[2],point[3])];
        let frames=[];for k in 0..=8{frames.push(#{offset:k.to_float()/8.0,value:brightness[(i+8-k)%8]});}
        rays.push(canvas(canvas_scene([canvas_stroke_path("ray",path,1.6,current_color()).stroke_cap("round")]).view_box(0,0,16,16))
            .with_key(`ray-${i}`).accessibility_hidden(true)
            .with_style(ctx.component_style("ray",style().absolute().top(px(0)).left(px(0)).width(relative(1.0)).height(relative(1.0))
                .opacity(brightness[i]).text_color(theme_color("text_primary"))))
            .motion(motion_keyframes("opacity",frames,#{duration_ms:800,iterations:"infinite",intent:"essential"})));
    }
    rays
}
fn glyph_sheen(ctx,leaf,base,duration){
    leaf.with_style(ctx.component_style("sheen",base.glyph_sweep(theme_color("thinking.sheen"),0.36,0)))
        .motion(motion_keyframes("glyph_sweep",[
            #{offset:0.0,value:0.0},#{offset:1.0,value:1.0,easing:"linear"}],
            #{duration_ms:duration,iterations:"infinite",intent:"decorative"}))
}
fn text_sheen_at(ctx,caption,part,active,duration){
    let base=style().min_width(px(0)).whitespace_nowrap().text_ellipsis();
    let label=text(caption).with_key("base").with_style(base);
    if active{label=glyph_sheen(ctx,label,base,duration);}
    row([label]).with_style(ctx.component_style(part,base.relative().typography("caption").clip()))
}
fn text_sheen(ctx,caption,part,active){text_sheen_at(ctx,caption,part,active,1600)}
fn animated_label(ctx,caption,part,transition_key,active){
    let label=text_sheen_at(ctx,caption,part,active,ctx.motion_duration("thinking_sweep")).with_key(transition_key)
        .motion(motion_transition("opacity",0.35,1.0,#{duration_ms:300,intent:"feedback"}))
        .motion(motion_transition("translate_y_fraction",0.35,0.0,#{duration_ms:330,easing:cubic_bezier(0.22,1.0,0.36,1.0),intent:"feedback"}))
        .motion(motion_transition("foreground_blur",2.0,0.0,#{duration_ms:280,intent:"feedback"}))
        .exit_motion(motion_transition("opacity",1.0,0.0,#{duration_ms:190,intent:"feedback"}))
        .exit_motion(motion_transition("foreground_blur",0.0,2.0,#{duration_ms:200,intent:"feedback"}))
        .exit_motion(motion_transition("translate_y_fraction",0.0,-0.32,#{duration_ms:220,intent:"feedback"}));
    label
}
fn render_ThinkingLoader(ctx,props){
    let terminal=props.done||props.stopped;
    effect("restart",terminal,Fn("track_terminal"),Fn("cleanup"));
    let labels=props.phases;
    if labels.len==0{labels=[ctx.t_or("thinking.label","Thinking")];}
    let selected=opt(props,"phase");
    let local=if ctx.get_state("was_terminal")&&!terminal{0}else{ctx.get_state("local")};
    let phase=min(labels.len-1,if selected==(){local}else{selected});
    if !terminal&&props.autoplay&&selected==()&&labels.len>1&&(phase+1<labels.len||props.repeat){
        timeout("next-phase",props.interval_ms,false,Fn("advance"),(phase+1)%labels.len);
    }
    let stage=if props.done{"done"}else if props.stopped{"stopped"}else{"run"};
    let message=if props.done{finish_label(ctx,opt(props,"done_label"),opt(props,"elapsed_ms"))}
        else if props.stopped{ctx.t_or("thinking.stopped","Stopped")}else{labels[phase]};
    if message==""{message=ctx.t_or("thinking.label","Thinking");}
    let icon=row(if terminal{[check_shape(ctx,stage,if props.done{"check"}else{"stop"})]}else{work_rays(ctx)})
        .with_key("indicator").accessibility_hidden(true)
        .with_style(ctx.component_style("icon",style().relative().width(theme_length("space.unit").times(3.75)).height(theme_length("space.unit").times(3.75)).flex_shrink(false)));
    let label=row([animated_label(ctx,message,if props.done{["label","finished"]}else if props.stopped{["label","stopped"]}else{["label"]},
        stage+":"+phase.to_string(),!terminal)]).with_key("clip").accessibility_hidden(true)
        .with_style(ctx.component_style("clip",style().items_center().min_width(px(0)).clip()));
    let root=row([icon,label]).with_key(props.key).accessibility_role("status").accessibility_label(message)
        .intrinsic_width_motion(message,380,"ease_out")
        .with_style(ctx.component_style(if props.variant=="plain"{["root","plain"]}else{["root"]},style()
            .self_start().items_center().min_width(px(0)).max_width(relative(1.0)).gap(theme_length("space.spacing_unit").times(2)).min_height(theme_length("space.unit").times(7)).padding_x(theme_length("space.spacing_unit").times(2))
            .radius(theme_radius("pill")).border(px(1)).border_color(theme_color("border")).background(theme_color("surface_raised"))
            .text_color(theme_color("text_muted"))));
    if opt(props,"size")!=(){root.env(#{size:props.size})}else{root}
}
