/* gpui-rhai
{"id":"patterns/rate_limit_planner","export":"RateLimitPlanner","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/input","components/time_input","components/number_input"],"capabilities":{},"assets":[],"tokens":["focus_ring","metrics.rate_plan_sheen","radius.lg","radius.sm","space.unit","space.spacing_unit","text_primary","typography.body","typography.caption","typography.metadata","rate_plan.card","rate_plan.ink","rate_plan.muted","rate_plan.line","rate_plan.guide","rate_plan.stroke","rate_plan.actual","rate_plan.wash","rate_plan.accent","rate_plan.warm","rate_plan.warn","rate_plan.warm_bg","rate_plan.danger","rate_plan.danger_bg","rate_plan.okay","rate_plan.okay_bg","rate_plan.hatch","rate_plan.hatch_bg","rate_plan.popup","rate_plan.button","rate_plan.on_button","rate_plan.tag","rate_plan.tag_over","rate_plan.blue","rate_plan.purple","rate_plan.teal","rate_plan.pink","rate_plan.cyan","rate_plan.green","rate_plan.amber"],"environment":[]}
*/
// A 24-hour load forecast with native drag previews and reversible Host proposals.
// Example: RateLimitPlanner(#{key:"plan",session:"sync",limit:40000,runs:[#{id:"sync",name:"Sync index",at:480,peak:20000}]})
import "components/input" as input;
import "components/time_input" as clockfield;
import "components/number_input" as number;

define_component(#{
    metadata:#{id:"patterns/rate_limit_planner","export":"RateLimitPlanner",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/input","components/time_input","components/number_input"],capabilities:#{},assets:[],tokens:["focus_ring","metrics.rate_plan_sheen","radius.lg","radius.sm","space.unit","space.spacing_unit","text_primary","typography.body","typography.caption","typography.metadata","rate_plan.card","rate_plan.ink","rate_plan.muted","rate_plan.line","rate_plan.guide","rate_plan.stroke","rate_plan.actual","rate_plan.wash","rate_plan.accent","rate_plan.warm","rate_plan.warn","rate_plan.warm_bg","rate_plan.danger","rate_plan.danger_bg","rate_plan.okay","rate_plan.okay_bg","rate_plan.hatch","rate_plan.hatch_bg","rate_plan.popup","rate_plan.button","rate_plan.on_button","rate_plan.tag","rate_plan.tag_over","rate_plan.blue","rate_plan.purple","rate_plan.teal","rate_plan.pink","rate_plan.cyan","rate_plan.green","rate_plan.amber"],environment:[]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false},
        session:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Identity of this plan; changing it resets local selection and editing."},
        revision:#{schema:#{type:"integer",min:0},"default":#{type:"integer",value:0},required:false,sensitive:false,doc:"Version of the Host's accepted inputs and schedule."},
        runs:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique run key; retain it when moving or editing the run."},
            name:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Caption used in the chart, editor and selected-run summary."},
            at:#{schema:#{type:"integer",min:0,max:1439},required:true,sensitive:false,doc:"Minute after midnight in the planner's declared wall-clock day."},
            peak:#{schema:#{type:"integer",min:0,max:1000000000},required:true,sensitive:false,doc:"Peak tokens per minute."},
            rise:#{schema:#{type:"optional",value:#{type:"integer",min:1,max:720}},required:false,sensitive:false,doc:"Minutes to reach the peak; defaults to 30."},
            half_life:#{schema:#{type:"optional",value:#{type:"integer",min:1,max:1440}},required:false,sensitive:false,doc:"Minutes of decay for one half-life; defaults to 45."},
            days:#{schema:#{type:"optional",value:#{type:"string",allowed:["daily","weekdays","weekend"]}},required:false,sensitive:false,doc:"Selects which kind of day includes this run in its forecast; omission means daily."}
        }}},required:true,sensitive:true,doc:"Ordered forecast runs. IDs remain stable when reshuffled."},
        controlled:#{schema:#{type:"bool"},"default":#{type:"bool",value:true},required:false,sensitive:false,doc:"When false, runs seed a local plan for this session."},
        limit:#{schema:#{type:"integer",min:1,max:1000000000},required:true,sensitive:false,doc:"Hard shared limit in tokens per minute; excess contributes to overload bands."},
        buffer:#{schema:#{type:"number",min:0.05,max:1.0},"default":#{type:"float",value:0.8},required:false,sensitive:false,doc:"Fraction of the hard limit used as the warning line, not a change to the limit."},
        start:#{schema:#{type:"integer",min:0,max:1439},"default":#{type:"integer",value:360},required:false,sensitive:false,doc:"Minute after midnight at the start of the rolling 24-hour plot."},
        now:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:1439}},required:false,sensitive:false,doc:"Wall-clock minute now. Earlier offsets within this 24-hour window cannot be changed."},
        day:#{schema:#{type:"string",allowed:["weekdays","weekend"]},"default":#{type:"string",value:"weekdays"},required:false,sensitive:false,doc:"Day category used to select recurring runs for this forecast."},
        actual:#{schema:#{type:"array",max_items:1440,items:#{type:"optional",value:#{type:"number"}}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Minute readings from the window start. Null gaps carry the previous reading; no unmeasured past is drawn."},
        checkpoint:#{schema:#{type:"optional",value:#{type:"object",allow_unknown:false,fields:#{
            at:#{schema:#{type:"integer",min:0,max:1439},required:true,sensitive:false,doc:"Checkpoint minute after midnight, in the same wall-clock day as the runs."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Human-readable checkpoint caption used next to the reported time."}
        }}},required:false,sensitive:false,doc:"Optional marker and load summary at a weekday or weekend checkpoint."},
        selected:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional run ID for the detailed curve; omission uses the local selection."},
        title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Overrides the visible plan heading."},
        pending:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disables plan mutations while a controlled Host decides."},
        on_runs_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a complete proposed plan plus its session, revision and operation; the SDK never schedules those runs."},
        on_selected_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the ID of the run chosen for inspection."}
    },state:#{fields:#{
        ui:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
        frame:#{schema:#{type:"ui_value"},"default":#{type:"null"}}
    }},events:#{
        runs_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            session:#{schema:#{type:"string"},required:true,sensitive:false},
            revision:#{schema:#{type:"integer"},required:true,sensitive:false},
            operation:#{schema:#{type:"string",allowed:["move","spread","fit","add","edit","remove"]},required:true,sensitive:false},
            id:#{schema:#{type:"optional",value:#{type:"string"}},required:true,sensitive:false},
            runs:#{schema:#{type:"array",max_items:64,items:#{type:"ui_value"}},required:true,sensitive:true}
        }},doc:"One complete plan proposal. Applies no external schedule."},
        selected_change:#{payload:#{type:"string"},doc:"Selected run ID for the detailed blue curve."}
    },slots:#{},parts:["root","head","title","summary","badge","button","plot_row","scale","plot","plot_hatch","plan_fill","selected_fill","axis","tick","peak","tip","flag",
        "lane","lane_selected","name","start","mark","dot","bar","bar_selected","bar_locked","bar_hatch","bar_sheen","bar_text","row_tools","icon","editor","field",
        "add","foot","verdict","fit","empty"],effects:["remember","new_editor"]},render:Fn("view_plan")
});
fn RateLimitPlanner(p){render_component("patterns/rate_limit_planner",p)}
fn get(m,k,d){if m!=()&&k in m&&m[k]!=(){m[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn u(n){theme_length("space.unit").times(n)}
fn ink(k){let key="rate_plan."+k;theme_color(key)}
fn tx(ctx,k,v){ctx.t_or("rate_plan."+k,v)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn faint(){style().typography("caption").min_width(px(0)).text_color(ink("muted")).whitespace_nowrap().text_ellipsis()}
fn apply(ctx,n,k,s){n.with_style(ctx.component_style(k,s))}
fn label(ctx,k,msg,s){apply(ctx,text(msg),k,s)}
fn focus(){style().focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))}
fn  click_key(n,f){n.on_click(f).on_key_value("plain+enter",f,()).on_key_value("plain+space",f,())}
fn logical(at,start){(at+1440-start)%1440}
fn clock(at){let n=(at+1440)%1440;let h=(n/60).to_string();let m=(n%60).to_string();(if h.len==1{"0"}else{""})+h+":"+(if m.len==1{"0"}else{""})+m}
fn short(ctx,n){let v=n.to_float();if v>=1000000.0{ctx.format_number(v/1000000.0,#{max_fraction_digits:1})+"M"}
    else if v>=1000.0{ctx.format_number(v/1000.0,#{max_fraction_digits:0})+"k"}else{ctx.format_number(v,#{max_fraction_digits:0})}}
fn pct(ctx,n,m){ctx.format_number(100.0*n.to_float()/m.to_float(),#{max_fraction_digits:0})+"%"}
fn glyph(which){
    let d=switch which{"spread"=>"M2.5 11.5c1.6 0 2-6 3.5-6s1.9 6 3.5 6 1.9-6 3.5-6M2.5 13.5h11",
      "add"=>"M8 3.5v9M3.5 8h9","remove"=>"m4.5 4.5 7 7m0-7-7 7",_=>"M3.5 8h.1m4.3 0h.1m4.3 0h.1"};
    svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><path d='"+d+"'/></svg>")
      .accessibility_hidden(true).with_style(style().width(u(3.5)).height(u(3.5)).flex_shrink(false))
}
fn state(ctx,session){
    let v=ctx.get_state("ui");
    if v!=()&&v.session==session{v}else{#{session:session,runs:(),select:"",editor:(),next_editor:(),hover:(),serial:0,epoch:0}}
}
fn select_plan_run(t,id,ctx,e){
    let s=state(ctx,t.session);
    if !t.select_controlled&&s.select!=id{s.select=id;ctx.set_state("ui",s);}
    if t.selected!=id{ctx.emit("selected_change",id);}
    handled()
}
fn focus_select(t,id,ctx,inside){if inside{select_plan_run(t,id,ctx,());}}
fn editor_open(t,rec,ctx,e){
    let s=state(ctx,t.session);let draft=rec;draft.clock=ctx.format_time(rec.at*60000,"short","UTC");s.editor=#{revision:t.revision,record:draft};
    ctx.set_state("ui",s);handled()
}
fn editor_close(t,key,ctx,e){
    let s=state(ctx,t.session);s.editor=();ctx.set_state("ui",s);
    if key!=""{ctx.focus(key);}
    handled()
}
fn editor_visibility(t,key,ctx,open){if !open{editor_close(t,key,ctx,());}}
fn edit_value(t,field,ctx,value){
    let s=state(ctx,t.session);if s.editor==()||s.editor.revision!=t.revision||value==(){return;}
    s.editor.record[field]=if type_of(value)=="string"{value}else{value.to_float().round().to_int()};
    ctx.set_state("ui",s);
}
fn at_from(ctx,value){
    let time=ctx.parse_time(value,"minute");if time==(){return ();}
    let ms=time_local_candidates("1970-01-01",time,"UTC");if ms.len==0{()}else{ms[0]/60000}
}
fn commit(t,op,id,next,ctx){
    let s=state(ctx,t.session);
    if !t.controlled{s.runs=next;s.epoch+=1;}
    ctx.set_state("ui",s);
    ctx.emit("runs_change",#{session:t.session,revision:t.revision,operation:op,id:if id==""{()}else{id},runs:next});
}
fn lookup(runs,id){for i in 0..runs.len{if runs[i].id==id{return i;}}-1}
fn clamp_start(t,offset){
    let lo=if t.now==(){0}else{t.now};
    max(lo,min(1380,offset))
}
fn move(t,runs,id,offset,op,ctx){
    let i=lookup(runs,id);if i<0{return;}
    let r=runs[i];let pos=logical(r.at,t.start);
    if t.now!=()&&pos<t.now{return;}
    let new_pos=if op=="fit"{max(get(t,"now",0),min(1439,offset))}else{clamp_start(t,offset)};
    let at=(new_pos+t.start)%1440;
    if at==r.at{return;}
    r.at=at;runs[i]=r;
    commit(t,op,id,runs,ctx);
}
fn drag_move(t,runs,id,width,ctx,pos){
    if width<=0.0{return;}
    let offset=(pos.x/width*288.0).round().to_int()*5;
    move(t,runs,id,offset,"move",ctx);
    let s=state(ctx,t.session);
    if !t.select_controlled&&s.select!=id{s.select=id;ctx.set_state("ui",s);}
    if id!=t.selected{ctx.emit("selected_change",id);}
}
fn to_minute(t,runs,id,offset,op,ctx,e){move(t,runs,id,offset,op,ctx);handled()}
fn remove(t,runs,id,ctx,e){
    let i=lookup(runs,id);if i<0{return handled();}
    if t.now!=()&&logical(runs[i].at,t.start)<t.now{return handled();}
    runs.remove(i);commit(t,"remove",id,runs,ctx);ctx.focus("add-run");handled()
}
fn save(t,runs,ctx,e){
    let s=state(ctx,t.session);
    if s.editor==()||s.editor.revision!=t.revision{return handled();}
    let r=s.editor.record;let i=lookup(runs,r.id);let at=at_from(ctx,r.clock);
    let name=r.name;name.trim();
    if i<0||at==()||name==""{return handled();}
    if t.now!=()&&logical(runs[i].at,t.start)<t.now{return handled();}
    r.name=name;r.at=(clamp_start(t,logical(at,t.start))+t.start)%1440;r.remove("clock");runs[i]=r;
    s.editor=();ctx.set_state("ui",s);commit(t,"edit",r.id,runs,ctx);ctx.focus("edit:"+r.id);
    handled()
}
fn spread(t,runs,cfg,ctx,e){
    let changes=rate_plan_spread(runs,cfg);
    if changes.len>0{
        for delta in changes{runs[delta.index].at=delta.at;}
        commit(t,"spread","",runs,ctx);
    }
    handled()
}
fn add(t,runs,cfg,ctx,e){
    if runs.len>=64{return handled();}
    let at=rate_plan_add_slot(runs,cfg,max(1,cfg.limit/3));if at==(){return handled();}
    let s=state(ctx,t.session);let id="";
    while id==""||lookup(runs,id)>=0{s.serial+=1;id="run:"+s.serial.to_string();}
    if !t.select_controlled{s.select=id;}
    s.next_editor=#{id:id,revision:t.revision};ctx.set_state("ui",s);
    runs.push(#{id:id,name:tx(ctx,"new_run","New run"),at:at,peak:max(1,cfg.limit/3),rise:40,half_life:70,days:cfg.day});
    commit(t,"add",id,runs,ctx);ctx.emit("selected_change",id);handled()
}
fn added_editor(ctx,e){
    let s=state(ctx,e.ticket.session);
    if s.next_editor==()||s.next_editor.id!=e.id{return;}
    s.next_editor=();ctx.set_state("ui",s);
    if e.record!=(){editor_open(e.ticket,e.record,ctx,());}
}
fn hovered(t,ctx,e){
    // Both the extent and local pointer belong to the current visual
    // target. Mixing this pointer with an untransformed layout size
    // would also misread the minute in a scaled container.
    let target=get(e,"target",());
    if target==()||target.width<=1.0{return propagate();}
    let position=e.local.x/target.width*1440.0;
    let m=if position>=0.0&&position<1440.0{position.floor().to_int()}else{()};
    let s=state(ctx,t.session);if s.hover!=m{s.hover=m;ctx.set_state("ui",s);}
    propagate()
}
fn leave(t,ctx,yes){
    if !yes{let s=state(ctx,t.session);if s.hover!=(){s.hover=();ctx.set_state("ui",s);}}
    // This is a passive listener. Consuming hover entry also consumes
    // the first pointer move that produced that transition.
    propagate()
}
fn remember(ctx,n){
    let old=ctx.get_state("frame");
    if old!=()&&old.current.tag==n.tag{return;}
    let prev=if old==()||old.current.session!=n.session||old.current.grid!=n.grid{()}else{old.current};
    ctx.set_state("frame",#{current:n,previous:prev});
}
fn noop(ctx,n){}
fn plot_x(m,n){m.w*n.to_float()/1440.0}
fn headroom(){1.3}
fn plot_height(){u(33)}
fn plot_y(m,p,n){m.h*(1.0-min(headroom(),max(0.0,n.to_float()/p.limit.to_float()))/headroom())}
fn points(m,p,series){
    rate_plan_curve(series,m.w,m.h,p.limit.to_float()*headroom())
}
fn region_path(m,line,left,right){
    let out=line;
    out.push(path_line(right,m.h));out.push(path_line(left,m.h));out.push(path_close());
    out
}
fn axis_line(m,a,c){
    canvas_line("axis",0,a,m.w,a,1.0,c)
}
fn limit_line(m,p,buffer){
    let y=plot_y(m,p,if buffer{p.limit.to_float()*p.buffer.to_float()}else{p.limit});
    let path=[path_move(0,y),path_line(m.w,y)];
    canvas_stroke_path(if buffer{"buffer"}else{"limit"},path,
        1.0,if buffer{ink("guide")}else{alpha(ink("warn"),0.6)})
        .stroke_dash(if buffer{2}else{3},if buffer{3}else{4})
}
fn guides(m,d){
    let lines=[];
    if d.now!=(){let x=plot_x(m,d.now);lines.push(canvas_line("now",x,0,x,m.h,1.0,ink("actual")));}
    if d.checkpoint!=(){let x=plot_x(m,d.checkpoint.at);lines.push(canvas_stroke_path("checkpoint",
        [path_move(x,0),path_line(x,m.h)],1.0,ink("guide")).stroke_dash(2,3));}
    lines
}
fn quiet_guide(m,d,x){d.peak>0.0&&abs(plot_x(m,d.at)-x)<110.0}
fn flag_style(cap){faint().font_feature("tnum",1).max_width(px(cap))}
fn flag_edge(){
    // Anchor the bottom to the plot origin; taller text grows upward
    // instead of covering the guide when reading size is increased.
    style().absolute().bottom(relative(1.0)).margin_bottom(u(1))
}
fn checkpoint_flag_slot(m,x,cap,now){
    let prefer_left=x+170.0>m.w;
    let l=x-6.0;let r=x+6.0;
    if now==(){return #{left:prefer_left,edge:if prefer_left{l}else{r},width:cap};}
    // The measured Now label owns this interval to both sides of its
    // guide. Search the two free intervals without measuring the
    // checkpoint itself (which would depend on the chosen side).
    let a=now.x-now.half-4.0;let b=now.x+now.half+4.0;
    let le=if l>a&&l<b{a}else{l};
    let rs=if r>a&&r<b{b}else{r};
    let left=#{left:true,edge:le,width:max(0.0,min(cap,le-if le>=b{b}else{0.0}))};
    let right=#{left:false,edge:rs,width:max(0.0,min(cap,(if rs<=a{a}else{m.w})-rs))};
    if prefer_left&&le==l&&left.width==cap{return left;}
    if !prefer_left&&rs==r&&right.width==cap{return right;}
    if left.width!=right.width{return if left.width>right.width{left}else{right};}
    if l-le!=rs-r{return if l-le<rs-r{left}else{right};}
    if prefer_left{left}else{right}
}
fn guide_flags(ctx,p,m,d){
    let flags=[];
    let cap=min(180.0,m.w*0.48);
    let now=();
    if d.now!=(){
        let x=plot_x(m,d.now);
        let quiet=quiet_guide(m,d,x);
        let anchor=element_ref("planner-now-label");
        let measured=ctx.element_size(anchor);
        if !quiet{now=#{x:x,half:min(cap,get(measured,"width",cap))/2.0};}
        let caption=ctx.t_or("rate_plan.now_flag","Now {time}",#{time:clock(d.now+p.start)});
        let text=label(ctx,"flag",caption,flag_style(cap).font_weight(500))
            .with_key("now-caption").with_ref(anchor).accessibility_hidden(true);
        let edge=flag_edge().left(offset_px(x-cap/2.0)).width(px(cap)).justify_center();
        flags.push(row([text]).with_key("now-flag").accessibility_hidden(true)
            .with_style(if quiet{edge.hidden()}else{edge}));
    }
    if d.checkpoint!=(){
        let x=plot_x(m,d.checkpoint.at);
        let place=checkpoint_flag_slot(m,x,cap,now);
        let edge=if place.left{flag_edge().right(offset_px(m.w-place.edge))}else{flag_edge().left(offset_px(place.edge))};
        let caption=ctx.t_or("rate_plan.checkpoint_flag","{label} {time}",
            #{label:p.checkpoint.label,time:clock(d.checkpoint.at+p.start)});
        let quiet=quiet_guide(m,d,x)||place.width<1.0;
        flags.push(label(ctx,"flag",caption,flag_style(place.width)).with_key("checkpoint-flag")
            .accessibility_hidden(true).with_style(if quiet{edge.hidden()}else{edge}));
    }
    flags
}
fn layer(){style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(relative(1.0))}
fn overflow_hatch(plot){
    let wash=ink("hatch_bg");
    // The chart draws the stroke over the tile wash; its tile edge clips
    // half of the stroke. Lane gradients alternate the two paints directly.
    let stripe=if plot{ink("hatch").over(wash)}else{ink("hatch")};
    let edge=if plot{0.75/6.0}else{1.5/4.5};
    layered_gradient([#{angle:if plot{135}else{315},repeat_period:if plot{6.0}else{4.5},
        origin:"border",clip:"border",stops:[
            #{offset:0,color:stripe},#{offset:edge,color:stripe},
            #{offset:edge,color:wash},#{offset:1,color:wash}
        ]}])
}
fn load_unit(){1000000000.0}
fn source_load(v){-v.to_float()/load_unit()}
fn load_points(m,series){
    // Do not clamp the load rank or thresholds to display headroom.
    rate_plan_load_path(series,m.w,load_unit())
}
fn plan_load_route(m,series,previous,tween){
    let path=load_points(m,series);
    if tween{canvas_morph_stroke_path("load-source",load_points(m,previous),path,1.0,ink("warn"))}
    else{canvas_stroke_path("load-source",path,1.0,ink("warn"))}
}
fn chart_overflow(ctx,m,p,load,tween){
    let fill=ctx.component_style("plot_hatch",style().layered_gradient(overflow_hatch(true)));
    let scene=canvas_scene([]).view_box(0,0,m.w,m.h).threshold_bands(load,source_load(p.limit),fill);
    chart_motion(canvas(scene).with_key("forecast-bands").with_style(layer()).accessibility_hidden(true),m,tween)
}
fn chart_tween(series,old){
    old!=()&&old.len==series.len&&old[0].at==series[0].at
}
fn chart_series(m,p,series,old){
    if series.len<2{return ();}
    let prev=if chart_tween(series,old){points(m,p,old)}else{()};
    #{path:points(m,p,series),prev:prev,left:plot_x(m,series[0].at),right:plot_x(m,series[series.len-1].at)}
}
fn chart_motion(n,m,tween){
    // New topology snaps fully drawn, but keeps its terminal track ready.
    let track=if tween{motion_transition("path_progress",0.0,1.0,#{duration_ms:690,easing:ease(),intent:"feedback"})}
        else{motion_transition("path_progress",1.0,1.0,#{duration_ms:1,easing:"linear",intent:"feedback"})};
    n.motion(track).motion_on_change(m.motion_tag)
}
fn chart_paint(m,tag,commands){
    canvas(canvas_scene(commands).view_box(0,0,m.w,m.h))
        .with_key(tag).with_style(layer()).accessibility_hidden(true)
}
fn chart_area(m,tag,line,fill,clip){
    if line==(){return ();}
    let tween=line.prev!=();
    let path=region_path(m,line.path,line.left,line.right);
    let cmd=if tween{canvas_morph_fill_path(tag,region_path(m,line.prev,line.left,line.right),path,fill)}
        else{canvas_fill_path(tag,path,fill)};
    if clip!=(){cmd=cmd.clip_rect(clip.x,0,m.w-clip.x,clip.h);}
    chart_motion(chart_paint(m,tag,[cmd]),m,tween)
}
fn curve_command(m,tag,line,color,opts){
    let cmd=if line.prev!=(){canvas_morph_stroke_path(tag,line.prev,line.path,1.5,color)}
        else{canvas_stroke_path(tag,line.path,1.5,color)};
    let cmd=cmd.stroke_cap("round").stroke_join("round");
    if get(opts,"dashed",false){cmd=cmd.stroke_dash(4,4);}
    let clip=get(opts,"clip",());
    if clip!=(){cmd=cmd.clip_rect(clip.x,0,m.w-clip.x,clip.h);}
    cmd
}
fn chart_curve(m,tag,line,color,opts){
    if line==(){return ();}
    let tween=line.prev!=();
    let commands=if get(opts,"draw",true){[curve_command(m,tag,line,color,opts)]}else{[]};
    let n=chart_paint(m,tag,commands);
    if get(opts,"motion",true){chart_motion(n,m,tween)}else{n}
}
fn chart_peak(ctx,m,p,d,source,tween){
    let limit=source_load(p.limit);
    let peak=source.peak_route().project_y(limit*headroom(),0,0,m.h);
    let show=if d.peak>0.0{style()}else{style().hidden()};
    let mark=chart_paint(m,"peak-mark",[
        canvas_circle("peak",0,0,3.5,current_color()),
        canvas_circle("peak-center",0,0,2.25,ink("card"))
    ]).with_style(show.text_color(ink("stroke"))).follow_canvas_peak(peak,0,0,[])
        .canvas_peak_paint_above(limit,ink("warn"),());
    let caption=ctx.t_or("rate_plan.peak","Peak {value} · {time}",#{value:short(ctx,d.peak),time:clock(d.at+p.start)});
    let tag=label(ctx,"peak",caption,faint().text_color(ink("ink"))
        .background(ink("tag")).padding_x(sp(1.5)).radius(theme_radius("sm")).absolute().left(px(0)).top(px(0)))
        .with_key("peak-caption").with_style(show).accessibility_hidden(true)
        .follow_canvas_peak(peak,5,-21,[0,0,max(0.0,m.w-126.0),m.h])
        .canvas_peak_paint_above(limit,ink("danger"),ink("tag_over"));
    [chart_motion(mark,m,tween),chart_motion(tag,m,tween)]
}
fn chart_tip(ctx,p,m,reading){
    let x=plot_x(m,m.hover);let y=plot_y(m,p,reading.value);
    let args=#{time:clock(m.hover+p.start),value:short(ctx,reading.value)};
    let caption=if reading.measured{ctx.t_or("rate_plan.hover_measured","{time} · {value} / min measured",args)}
        else{ctx.t_or("rate_plan.hover_forecast","{time} · {value} / min",args)};
    let base=style().typography("caption").whitespace_nowrap().background(ink("ink")).text_color(ink("card"))
        .font_feature("tnum",1).padding_x(u(1.5)).padding_y(u(0.5)).radius(theme_radius("sm"))
        .absolute().top(offset_px(y-30.0));
    // Anchoring by the right edge uses the tip's real layout width,
    // including localization and the current reading size.
    let anchor=if m.hover.to_float()>1440.0*0.72{base.right(offset_px(m.w-x+8.0))}else{base.left(offset_px(x+8.0))};
    [chart_paint(m,"cursor",[canvas_line("cursor",x,0,x,m.h,1.0,ink("guide"))]),
        label(ctx,"tip",caption,anchor).with_key("hover-tip").accessibility_hidden(true)]
}
fn chart(ctx,p,m,d,track,t){
    let prior=ctx.get_state("frame");
    let old=if prior==()||prior.current.session!=p.session||get(prior.current,"grid",0)!=d.chart_grid_size{()}
      else if prior.current.tag==m.motion_tag{prior.previous}else{prior.current};
    let raw=get(d,"unjoined_plan",d.plan);
    effect("remember",#{session:p.session,tag:m.motion_tag,grid:d.chart_grid_size,plan:d.plan,unjoined:d.unjoined_plan,selected:d.selected_line},Fn("remember"),Fn("noop"));
    let measured=d.actual.len>1;
    let from=if measured{plot_x(m,d.plan[0].at)}else{0.0};
    let future_clip=if measured{#{x:from,h:m.h}}else{()};
    let previous=get(old,"plan",());
    let previous_selected=get(old,"selected",());
    let tween=chart_tween(d.plan,previous);
    let load=plan_load_route(m,raw,get(old,"unjoined",previous),tween);
    let forecast=chart_series(m,p,d.plan,previous);
    let selection=if d.selected!=(){chart_series(m,p,d.selected_line,previous_selected)}else{()};
    let measured_line=chart_series(m,p,d.actual,());
    let bits=[chart_paint(m,"buffer",[limit_line(m,p,true)]),
        chart_overflow(ctx,m,p,load,tween)];
    let area=chart_area(m,"plan-fill",forecast,
        ctx.component_style("plan_fill",style().background(alpha(ink("wash"),0.4))),future_clip);
    if area!=(){bits.push(area);}
    if d.selected!=(){
        let area=chart_area(m,"selected-fill",selection,
            ctx.component_style("selected_fill",style().background(alpha(ink("accent"),0.08))),());
        if area!=(){bits.push(area);}
    }
    bits.push(chart_paint(m,"grid",[limit_line(m,p,false)]));
    bits.push(chart_curve(m,"forecast",forecast,ink("stroke"),#{dashed:measured,clip:future_clip}));
    let actual=chart_curve(m,"actual",measured_line,ink("actual"),#{motion:false});
    if actual!=(){bits.push(actual);}
    let selected=chart_curve(m,"selected",selection,ink("accent"),#{});
    if selected!=()&&d.selected!=(){bits.push(selected);}
    // The retained warning clock follows the forecast even while its
    // scene is empty. Mounting the overlay on the first crossing would
    // otherwise initialize its newly declared motion at the final value.
    let red=chart_curve(m,"over-limit",forecast,ink("warn"),
        #{draw:d.plan.some(|q|q.v>p.limit)||(previous!=()&&previous.some(|q|q.v>p.limit)),
          clip:#{x:from,h:plot_y(m,p,p.limit)}});
    if red!=(){bits.push(red);}
    bits.push(canvas(canvas_scene(guides(m,d)).view_box(0,0,m.w,m.h))
        .with_key("guides").accessibility_hidden(true).with_style(layer()));
    // Both the bands and peak describe the unjoined plan. Neither of
    // them treats the measured-to-forecast seam as a planned maximum.
    bits+=chart_peak(ctx,m,p,d,load,tween);
    let layers=[box(bits).with_key("plot-paint").with_style(layer().overflow_hidden())]+guide_flags(ctx,p,m,d);
    if d.hover!=(){layers+=chart_tip(ctx,p,m,d.hover);}
    let drawable=box(layers).with_key("load-plot").with_ref(track).accessibility_role("figure").accessibility_label(tx(ctx,"forecast","Planned load"))
        .accessibility_description("Peak "+short(ctx,d.peak)+" · "+clock(d.at+p.start))
        .on("pointer_move",Fn("hovered").curry(t)).on_hover_change(Fn("leave").curry(t))
        .with_style(style().relative().flex_grow().width(relative(1.0)).min_height(px(0)).cursor_crosshair());
    // The plot's rule belongs to the frame, not the measured canvas or
    // pointer target. The child consumes the actual remaining content box.
    let plot=apply(ctx,column([drawable]).with_key("plot-frame"),"plot",
        style().flex_grow().width(relative(1.0)).min_width(px(0)).height(plot_height())
            .border_bottom(u(0.25).without_reading_scale()).border_color(ink("line")));
    let scale=[];
    for i in 0..2{
        let value=if i==0{p.limit}else{p.limit.to_float()*p.buffer.to_float()};
        scale.push(label(ctx,"scale",short(ctx,value),faint().absolute().inset_end(sp(2)).top(px(max(0.0,plot_y(m,p,value)-6.0)))
            .text_color(if i==0{ink("danger")}else{ink("muted")})));
    }
    // Reserve the label rail even while the labels are quiet. The measured
    // chart and its pointer target start below it, not under the header.
    row([rail(m,box(scale).with_style(style().relative().height(plot_height()))),plot]).with_key("plot-row")
        .with_style(ctx.component_style("plot_row",style().min_width(px(0)).padding_top(u(7.5)).padding_end(sp(4))))
}
fn rail(m,n){n.with_style(style().width(u(if m.narrow{8.5}else{49})).min_width(px(0)).flex_shrink(false))}
fn controls(ctx,key,name,icon,part,base,enabled,cb){
    let n=apply(ctx,row(if icon==""{[text(name)]}else{[glyph(icon),text(name)]}).with_key(key).tab_stop(enabled).accessibility_role("button")
        .accessibility_label(name).disabled(!enabled),part,focus().min_width(px(0)).min_height(u(7)).items_center().justify_center()
        .gap(sp(1.5)).padding_x(sp(2)).radius(theme_radius("sm")).typography("caption").cursor_pointer().merge(base));
    if enabled{click_key(n,cb)}else{n}
}
fn square(ctx,key,caption,g,f){
    let node=apply(ctx,row([glyph(g)]).with_key(key).accessibility_role("button").accessibility_label(caption).tab_stop(true),"icon",
        focus().width(u(5)).height(u(5)).items_center().justify_center().radius(theme_radius("sm")).background(ink("wash"))
            .text_color(ink("muted")).hover(style().background(ink("line"))).cursor_pointer());
    click_key(node,f)
}
fn header(ctx,p,m,d){
    let name=get(p,"title",tx(ctx,"title","Rate limit"));
    let top=column([label(ctx,"title",name,style().typography("body").font_weight(500)).accessibility_role("heading").accessibility_level(2),
        label(ctx,"summary",ctx.format_number(d.rows.len)+" runs · "+short(ctx,d.amount)+" tokens · "+short(ctx,p.limit)+" / min",faint())])
        .with_style(style().min_width(px(0)).flex_grow());
    let tone=if d.over.len>0{"danger"}else if d.peak>p.limit.to_float()*p.buffer.to_float(){"warm"}else{"okay"};
    let state=switch tone{"danger"=>tx(ctx,"over","Over limit"),"warm"=>tx(ctx,"close","Close to limit"),_=>tx(ctx,"under","Under limit")};
    let value=if tone=="danger"{" · "+clock(d.over[0].from+p.start)+"–"+clock(d.over[d.over.len-1].to+p.start)}
        else{" · peak "+pct(ctx,d.peak,p.limit)};
    let status=apply(ctx,row([text(state+value)]).with_key("badge").accessibility_role("status").accessibility_label(state+value),"badge",
        faint().min_height(u(5.5)).items_center().padding_x(sp(2.5)).radius(theme_radius("lg")).background(ink(tone+"_bg")).text_color(ink(tone)).flex_shrink(false));
    let spread=controls(ctx,"spread",tx(ctx,"spread","Spread out"),"spread","button",style().background(ink("card")),m.can&&d.rows.some(|r|!r.locked),
        Fn("spread").curry(m.ticket,m.runs,m.cfg));
    apply(ctx,row([top,row([status,spread]).with_style(style().flex_wrap().gap(sp(2)).items_center())]).with_key("heading"),"head",
        style().min_width(px(0)).padding_x(sp(if m.narrow{3.5}else{4})).padding_top(sp(if m.narrow{3.5}else{4}))
            .gap(sp(3)).flex_wrap().items_start())
}
fn bar_ref(id){ "bar:"+id }
fn dot(ctx,id,i,color,locked){
    let hash=utf8_byte_count(id)+i*17;let turn=hash%4;
    let bits=[];
    for j in 0..3{
        for k in 0..3{
            let on=(j+k+turn)%3!=2;
            bits.push(apply(ctx,box([]),"dot",style().grid_cell(k+1,j+1).width(u(0.75)).height(u(0.75)).radius(u(0.1875))
                .background(if on{color}else{rgba(0x00000000)})));
        }
    }
    apply(ctx,box(bits).with_key("mark").accessibility_hidden(true),"mark",style().grid_cols(3).grid_rows(3)
        .width(u(4)).height(u(4)).padding(u(0.5)).gap(u(0.375)).flex_shrink(false).opacity(if locked{0.55}else{1.0}))
}
fn color_at(i){let palette=["blue","purple","teal","pink","cyan","green","amber"];ink(palette[i%palette.len])}
fn running(v,now){v.locked&&now!=()&&now<v.offset+v.busy}
fn running_sheen(ctx,animate){
    let clear=rgba(0x00000000);
    let shine=layered_gradient([#{angle:90,stops:[
        #{offset:0,color:clear},#{offset:0.5,color:alpha(ink("card"),0.70)},#{offset:1,color:clear}
    ]}]);
    let width=theme_length("metrics.rate_plan_sheen").without_reading_scale();
    let patch=apply(ctx,box([]).with_key("light"),"bar_sheen",layer().width(width).layered_gradient(shine));
    // Both terms share the clock: p * bar width - (1-p) * measured sheen width.
    let motion=#{duration_ms:1500,easing:"linear",iterations:"infinite",intent:"decorative"};
    let patch=if animate{patch.motion(motion_transition("translate_x_fraction",-1.0,0.0,motion))}else{patch};
    let sweep=box([patch]).with_key("sheen").accessibility_hidden(true).with_style(layer());
    if animate{sweep.motion(motion_transition("translate_x_fraction",0.0,1.0,motion))}else{sweep}
}
fn drag_bar(ctx,p,m,d,r,v,color,on){
    let w=max(18.0,plot_x(m,v.busy));let x=min(max(0.0,m.w-18.0),plot_x(m,v.offset));
    let base=style().width(px(w)).height(u(4)).items_center().justify_end().padding_x(sp(1)).min_width(px(0)).overflow_hidden()
        .radius(theme_radius("sm")).border(px(0.5)).border_color(alpha(color,if on{0.5}else{0.30}))
        .background(alpha(color,if on{0.15}else{0.08}))
        .text_color(mix_premultiplied(color,ink("ink"),if on{0.15}else{0.30}));
    let base=if v.locked{base.border_color(ink("line")).background(ink("wash")).text_color(ink("muted"))}else{base};
    let amount=if running(v,m.ticket.now){[running_sheen(ctx,m.animate)]}else{[]};
    for over in d.over{
        let a=max(over.from,v.offset);let b=min(over.to,v.offset+v.busy);
        if b>a&&v.busy>0{
            amount.push(apply(ctx,box([]).with_key("excess:"+a.to_string()),"bar_hatch",style().absolute().top(px(0)).height(relative(1.0))
                .left(relative((a-v.offset).to_float()/v.busy.to_float())).width(relative((b-a).to_float()/v.busy.to_float()))
                .layered_gradient(m.bar_hatch)));
        }
    }
    if w>=38.0{amount.push(label(ctx,"bar_text",short(ctx,r.peak),
        style().typography("caption").whitespace_nowrap().text_ellipsis()).with_key("amount"));}
    let visual=apply(ctx,row(amount).with_key("bar-display"),if v.locked{["bar","bar_locked"]}else if on{["bar","bar_selected"]}else{["bar"]},base)
         .accessibility_hidden(true);
    // Started runs do not participate in a schedule glide; their paint and
    // native slider share the resized lane immediately.
    let visual=if v.locked{visual}else{visual.layout_motion(630,ease())};
    let lower=if m.ticket.now==(){0}else{m.ticket.now};
    let title=r.name+" · "+clock(r.at)+" · "+short(ctx,r.peak)+" / min"+(if v.locked{" · "+tx(ctx,"locked","Already started")}else{""});
    if v.locked||!m.can{
        return row([visual]).with_key("locked:"+r.id).accessibility_role("slider").accessibility_label(r.name)
            .accessibility_description(title).accessibility_value(v.offset).disabled(true)
            .with_style(style().absolute().left(px(x)).top(u(2.25)).width(px(w)).height(u(4)));
    }
    let area=element_ref("lane:"+r.id);let item=element_ref("item:"+r.id);let dx=optional_float_signal("drag:"+r.id);
    let cb=Fn("drag_move").curry(m.ticket,m.runs,r.id,m.w);
    let cfg=#{key:"drag:"+r.id,x:x,y:9.0,axes:"horizontal",contain:false,threshold:3.0,keyboard_step:max(0.25,m.w/96.0),
        snap_x:max(0.01,m.w/288.0),disabled:false,boundary_ref:area,object_ref:item,handle_ref:item,
        x_signal:dx,y_signal:optional_float_signal("drag-y:"+r.id),on_move:cb};
    let slider=gpui_rhai::DraggablePrimitive(cfg).with_key(bar_ref(r.id)).with_ref(element_ref(bar_ref(r.id))).tab_stop(true)
        .accessibility_role("slider").accessibility_label(r.name).accessibility_description(title)
        .accessibility_value(v.offset).accessibility_value_min(lower).accessibility_value_max(1439)
        .with_style(focus().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0)));
    slider=slider.on_focus_within_change(Fn("focus_select").curry(m.ticket,r.id))
        .on_key_value("plain+home",Fn("to_minute").curry(m.ticket,m.runs,r.id,lower,"move"),())
        .on_key_value("plain+end",Fn("to_minute").curry(m.ticket,m.runs,r.id,1380,"move"),())
        .on_key_value("plain+enter",Fn("editor_open").curry(m.ticket,r),())
        .on_key_value("plain+delete",Fn("remove").curry(m.ticket,m.runs,r.id),());
    let thing=box([visual,slider]).with_key("drag-item").with_ref(item).bind_signal("translate_x_override",dx)
        .with_style(style().absolute().left(px(x)).top(u(2.25)).width(px(w)).height(u(4)));
    box([thing]).with_key("drag-window").with_ref(area).with_style(style().absolute().left(px(0)).top(px(0)).width(relative(1.0)).height(u(8.5)))
}
fn edit_field(ctx,m,key,name,value,max,unit){
    let field=number::NumberInput(#{key:key,label:name,value:value,min:if key=="peak"{0}else{1},max:max,step:1.0,unit:unit,size:"xs",
        fraction_digits:0,on_change:Fn("edit_value").curry(m.ticket,key),
        part_styles:#{label:faint(),steps:style().hidden()}});
    apply(ctx,box([field]).with_key("number:"+key),"field",style().min_width(px(0)).flex_grow())
}
fn editor(ctx,m,r){
    let e=m.editor.record;let done=Fn("save").curry(m.ticket,m.runs);
    let start=column([label(ctx,"scale",tx(ctx,"starts","Starts"),faint()),
        clockfield::TimeInput(#{key:"start",label:tx(ctx,"starts","Starts"),value:e.clock,size:"xs",on_change:Fn("edit_value").curry(m.ticket,"clock")})])
        .with_key("start-clock").with_style(style().min_width(px(0)).gap(sp(1)));
    let columns=box([start,edit_field(ctx,m,"peak",tx(ctx,"peak_field","Peak"),e.peak,1000000000,"/ min"),
        edit_field(ctx,m,"rise",tx(ctx,"rise","Climb"),get(e,"rise",30),720,"min"),
        edit_field(ctx,m,"half_life",tx(ctx,"half","Half-life"),get(e,"half_life",45),1440,"min")])
        .with_key("fields").with_style(style().grid().grid_cols(2).min_width(px(0)).gap(sp(2)));
    let fields=[
        label(ctx,"scale",tx(ctx,"name","Name"),faint()),
        input::Input(#{key:"run-name",label:tx(ctx,"name","Name"),value:e.name,autofocus:true,on_change:Fn("edit_value").curry(m.ticket,"name"),on_submit:done}),
        columns,
        row([controls(ctx,"save",tx(ctx,"save","Save run"),"","fit",style().background(ink("button")).text_color(ink("on_button")),at_from(ctx,e.clock)!=(),done)])
            .with_style(style().justify_end())];
    apply(ctx,column(fields).with_key("editor").accessibility_role("dialog").accessibility_label("Edit "+r.name),"editor",
        style().width(u(76)).min_width(px(0)).gap(sp(1.25)).padding(sp(3)).radius(theme_radius("lg")).background(ink("popup")))
}
fn lane(ctx,p,m,d,v){
    let r=m.runs[v.index];let on=d.chosen==r.id;let hue=color_at(v.index);
    let can=m.can&&!v.locked;let edit="edit:"+r.id;let shown=m.editor!=()&&m.editor.record.id==r.id&&can;
    let status=if running(v,m.ticket.now){tx(ctx,"running","Running")}else{clock(r.at)};
    let name=label(ctx,"name",r.name,faint().typography("body").text_color(if v.locked{ink("muted")}else{theme_color("text_primary")}).flex_grow());
    let start=label(ctx,"start",status,faint().typography("metadata").line_height(u(5)).font_feature("tnum",1).flex_shrink(false)
        .text_color(if on{hue}else{ink("muted")})).with_key("start");
    let who=row([dot(ctx,r.id,v.index,hue,v.locked),name,start]).with_key("who")
        .tab_stop(false).on_click(Fn("select_plan_run").curry(m.ticket,r.id))
        .with_style(style().min_width(px(0)).items_center().gap(sp(2))
            .padding_start(sp(if m.narrow{3.5}else{4})).padding_end(sp(if m.narrow{16}else{3})).height(u(8.5)));
    let bits=[];
    for i in 1..8{
        if m.narrow&&i%2!=0{continue;}
        bits.push(box([]).with_key("guide:"+i.to_string()).accessibility_hidden(true).with_style(style().absolute().top(px(0))
            .left(relative(i.to_float()/8.0)).height(relative(1.0)).width(px(0.5)).background(ink("line"))));
    }
    if d.now!=(){
        bits.push(box([]).with_key("now").accessibility_hidden(true).with_style(style().absolute().top(px(0))
            .left(relative(d.now.to_float()/1440.0)).height(relative(1.0)).width(px(1)).background(alpha(ink("actual"),0.55))));
    }
    bits.push(drag_bar(ctx,p,m,d,r,v,hue,on));
    if can{
        let edit_n=square(ctx,edit,"Edit "+r.name,"edit",Fn("editor_open").curry(m.ticket,r)).with_ref(element_ref(edit)).accessibility_expanded(shown);
        let panel=if shown{editor(ctx,m,r)}else{box([])};
        let edit_wrap=overlay(edit_n,panel,#{id:"rate-plan:"+r.id,kind:"popover",placement:"top",align:"end",gap:6.0,open:shown,modal:false,
            initial_focus:"first",activate_on_trigger:false,dismiss_on_escape:true,dismiss_on_outside:true})
            .with_key("editor-overlay").on_open_change(Fn("editor_visibility").curry(m.ticket,edit));
        let remove=square(ctx,"remove","Remove "+r.name,"remove",Fn("remove").curry(m.ticket,m.runs,r.id));
        bits.push(apply(ctx,row([remove,edit_wrap]).with_key("tools"),"row_tools",style().absolute().right(px(0)).top(u(1.75)).gap(sp(1))
            .opacity(if shown||m.narrow&&on{1.0}else{0.0}).group_hover(style().opacity(1.0)).group_focus(style().opacity(1.0)).focus_within(style().opacity(1.0))));
    }
    let track=box(bits).with_key("track").with_style(style().relative().flex_grow().min_width(px(0)).height(u(8.5)));
    let bor=ctx.component_style(if on{["lane","lane_selected"]}else{["lane"]},style().min_width(px(0)).padding_end(sp(4))
        .background(if on{alpha(hue,0.05)}else{rgba(0x00000000)}).focus(style()));
    // Change layout, not native ownership: a resized lane keeps its
    // slider, editor and decorative motion tracks mounted.
    let trail=style().min_width(px(0));
    let trail=if m.narrow{trail.width(relative(1.0)).padding_start(u(8.5))}else{trail.flex_grow()};
    let tracks=row([track]).with_key("lane-track").with_style(trail);
    row([if m.narrow{who}else{rail(m,who)},tracks]).with_key("run:"+r.id)
        .with_style(if m.narrow{bor.flex_col()}else{bor})
        .paint_transition(160,cubic_bezier(0.25,0.1,0.25,1.0))
}
fn ticks(ctx,p,m){
    let cells=[];
    for i in 0..8{
        if m.narrow&&i%2!=0{continue;}
        cells.push(label(ctx,"tick",clock(p.start+i*180),faint().absolute().top(sp(1))
            .left(relative(i.to_float()/8.0))));
    }
    row([rail(m,box([])),box(cells).with_style(style().relative().min_width(px(0)).flex_grow().height(u(6.5)))])
        .with_key("axis").with_style(ctx.component_style("axis",style().min_width(px(0)).padding_end(sp(4))))
}
fn verdict(ctx,p,d){
    if d.over.len>0{
        if d.over_without{return tx(ctx,"without","Over the limit even without the selected run. Try spreading the plan.");}
        let at=clock(d.over[0].from+p.start);
        return if d.fit==(){ctx.t_or("rate_plan.hit","The plan exceeds the limit at {time}.",#{time:at})}
            else{ctx.t_or("rate_plan.fits","The plan exceeds the limit at {time}. The selected run fits at {fit}.",#{time:at,fit:clock(d.fit)})};
    }
    if d.pace!=()&&abs(d.pace.ratio-1.0)>=0.03{
        return ctx.t_or("rate_plan.pace","Measured pace is {percent} of plan; projected peak {peak} at {at}.",
            #{percent:pct(ctx,d.pace.ratio,1),peak:pct(ctx,d.pace.peak,p.limit),at:clock(d.pace.at+p.start)});
    }
    if d.checkpoint!=(){
        let c=p.checkpoint;
        return ctx.t_or("rate_plan.checkpoint","{load} / min still scheduled at {label} · {time}.",
            #{load:short(ctx,d.checkpoint.load),label:c.label,time:clock(c.at)});
    }
    tx(ctx,"stays","Stays under the limit all day")
}
fn view_plan(ctx,p){
    if utf8_byte_count(p.session)==0||utf8_byte_count(p.session)>96{throw "Rate planner needs a stable session ID";}
    let s=state(ctx,p.session);
    let runs=if p.controlled||s.runs==(){p.runs}else{s.runs};
    let checkpoint=get(p,"checkpoint",());
    let cfg=#{start:p.start,now:get(p,"now",()),day:p.day,limit:p.limit,buffer:p.buffer,
        selected:get(p,"selected",s.select),checkpoint:if checkpoint==(){()}else{checkpoint.at}};
    let ref=element_ref("planner");let track=element_ref("load-plot");let size=ctx.element_size(track);
    let width=if size==(){512.0}else{max(1.0,size.width)};
    let d=rate_plan_read(runs,cfg+#{hover:s.hover,plot_width:width},p.actual);
    let chosen=if d.selected==(){""}else{runs[d.rows[d.selected].index].id};
    let t=#{session:p.session,start:p.start,revision:p.revision,now:d.now,controlled:p.controlled,selected:chosen,select_controlled:get(p,"selected",())!=()};
    let can=!p.pending&&(!p.controlled||get(p,"on_runs_change",())!=());
    let next=get(s,"next_editor",());
    if next!=()&&(!p.controlled||p.revision>next.revision){
        let i=lookup(runs,next.id);
        let open=if i>=0&&can&&(d.now==()||logical(runs[i].at,p.start)>=d.now){runs[i]}else{()};
        effect("new_editor",#{id:next.id,ticket:t,record:open},Fn("added_editor"),Fn("noop"));
    }
    let m=#{ticket:t,cfg:cfg,runs:runs,can:can,narrow:ctx.element_width_class(ref,[560])==0,hover:s.hover,
      editor:if s.editor!=()&&s.editor.revision==p.revision{s.editor}else{()},
      animate:ctx.motion_preference()=="normal",
      bar_hatch:if d.over.len>0{overflow_hatch(false)}else{()},
      w:width,h:if size==(){131.0}else{max(1.0,size.height)},
      motion_tag:p.revision.to_string()+":"+s.epoch.to_string()+":"+chosen+":"+d.chart_grid_size.to_string()};
    let display=header(ctx,p,m,d);
    let load=chart(ctx,p,m,d,track,t);
    let time=ticks(ctx,p,m);
    let rows=[];d.chosen=chosen;
    for v in d.rows{rows.push(lane(ctx,p,m,d,v));}
    if rows.len==0{rows.push(label(ctx,"empty",tx(ctx,"empty","No runs planned for this day"),faint().padding(sp(3))));}
    let add=controls(ctx,"add-run",tx(ctx,"add","Add run"),"add","add",style().text_color(ink("muted"))
      .padding_start(sp(1.5)).padding_end(sp(2)),can&&runs.len<64,
      Fn("add").curry(t,runs,cfg)).with_ref(element_ref("add-run"));
    // Its 6px leading inset aligns the visible icon with the panel content;
    // the button's hit frame intentionally starts before that shared edge.
    rows.push(row([add]).with_key("add-row").with_style(style().min_width(px(0)).items_center()
        .padding_start(sp(if m.narrow{2}else{2.5})).height(u(8.5))));
    let words=verdict(ctx,p,d);
    let foot=[label(ctx,"verdict",words,style().typography("caption").text_color(ink("muted")).min_width(px(0)).flex_grow())
      .accessibility_role("status").accessibility_label(words)];
    if d.fit!=()&&chosen!=""{
      let label_text=tx(ctx,"move_to","Move to")+" "+clock(d.fit);
      foot.push(controls(ctx,"fit",label_text,"","fit",style().background(ink("button")).text_color(ink("on_button")),can,
        Fn("to_minute").curry(t,runs,chosen,logical(d.fit,p.start),"fit")));
    }
    let footer=apply(ctx,row(foot).with_key("foot"),"foot",style().min_width(px(0))
        .padding_start(sp(if m.narrow{3.5}else{4})).padding_end(sp(if m.narrow{3.5}else{3}))
        .padding_top(sp(1)).padding_bottom(sp(if m.narrow{3}else{2.5}))
        .justify_between().items_center().flex_wrap().gap(sp(3)));
    apply(ctx,column([display,load,time]+rows+[footer]).with_key(p.key).with_ref(ref).accessibility_role("region")
      .accessibility_label(get(p,"title",tx(ctx,"title","Rate limit"))),"root",
      style().width(relative(1.0)).min_width(px(0)).radius(theme_radius("lg")).background(ink("card")))
}
