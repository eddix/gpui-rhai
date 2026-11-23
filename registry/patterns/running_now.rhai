/* gpui-rhai
{"id":"patterns/running_now","export":"RunningNow","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["patterns/agent_tree"],"capabilities":{},"assets":[],"tokens":["border","focus_ring","radius.lg","radius.pill","radius.sm","running_now.amber","running_now.badge","running_now.badge_text","running_now.blue","running_now.orange","running_now.teal","running_now.trigger_ink","running_now.faint","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],"environment":[]}
*/
// A quiet trigger and a live, nonmodal task list. Task state is Host-owned.
// Example: RunningNow(#{key:"run-chip",board:"jobs",revision:0,tasks:[]})
import "patterns/agent_tree" as ids;

define_component(#{
    metadata:#{id:"patterns/running_now","export":"RunningNow",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
        dependencies:["patterns/agent_tree"],capabilities:#{},assets:[],tokens:["border","focus_ring","radius.lg","radius.pill","radius.sm","running_now.amber","running_now.badge","running_now.badge_text","running_now.blue","running_now.orange","running_now.teal","running_now.trigger_ink","running_now.faint","space.spacing_unit","space.unit","surface_hover","surface_raised","text_muted","text_primary","typography.body","typography.caption"],environment:[]},
    schema:#{
        props:#{
            key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable trigger instance and overlay key."},
            board:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable business board identity, included in action proposals."},
            revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted task snapshot revision."},
            tasks:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable task ID; unique within the board."},
                title:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Two-line row's first label."},
                step:#{schema:#{type:"string"},required:false,sensitive:true,"default":#{type:"string",value:""},doc:"Current step or what the task is waiting for."},
                done:#{schema:#{type:"integer",min:0,max:1000000},required:true,sensitive:false,doc:"Completed steps, clamped to total."},
                total:#{schema:#{type:"integer",min:0,max:1000000},required:true,sensitive:false,doc:"Expected number of steps; zero means the number of steps is unknown."},
                status:#{schema:#{type:"string",allowed:["running","paused","needs_you","done"]},required:true,sensitive:false,doc:"Accepted task status."}
            }}},required:true,sensitive:true,doc:"Task rows in the caller's stable display order."},
            pending:#{schema:#{type:"array",max_items:64,items:#{type:"string"}},required:false,sensitive:false,"default":#{type:"array",value:[]},doc:"Task IDs whose mutations are awaiting Host acknowledgement. Duplicate pause/resume requests are suppressed."},
            default_open:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Initial disclosure in local mode."},
            open:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,doc:"Controlled disclosure. The Host accepts changes through on_open_change."},
            align:#{schema:#{type:"string",allowed:["start","end"]},required:false,sensitive:false,"default":#{type:"string",value:"end"},doc:"Logical popover alignment at the trigger edge."},
            title:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Optional localized name of the task list."},
            on_open_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the requested disclosure boolean."},
            on_action:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives a versioned pause or resume request; task state is not changed automatically."},
            on_open_task:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Receives the selected task identity."},
            on_open_all:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Optional action on the footer to show the full task list."}
        },
        state:#{fields:#{
            disclosure:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
            cursor:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
            panel_focus:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
            request_focus:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
            seen:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
            notice:#{schema:#{type:"string"},"default":#{type:"string",value:""},sensitive:true}
        }},
        events:#{
            open_change:#{payload:#{type:"bool"},doc:"Requested disclosure state."},
            action:#{payload:#{type:"object",allow_unknown:false,fields:#{
                board:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Board ID."},
                revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision used for the request."},
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Task ID."},
                kind:#{schema:#{type:"string",allowed:["pause","resume"]},required:true,sensitive:false,doc:"Requested operation."}
            }},doc:"Propose a task transition; the caller decides whether it takes effect."},
            open_task:#{payload:#{type:"object",allow_unknown:false,fields:#{
                board:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Board ID."},
                revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted revision used for the request."},
                id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Task ID."}
            }},doc:"Open an individual task by business identity."},
            open_all:#{payload:#{type:"object",allow_unknown:false,fields:#{
                board:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Board ID."},
                revision:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Accepted board revision."}
            }},doc:"Open the full task list."}
        },
        slots:#{},parts:["root","trigger","trigger_open","trigger_caption","separator","part","count","caption","chevron","panel","heading","label","body","list","row","main","name","finished","step","ring","badge","action","action_text","action_label","done_mark","footer","empty","announcement"],
        effects:["remember","restore"],timers:["focus_task"]
    },render:Fn("render_running")
});
fn RunningNow(p){render_component("patterns/running_now",p)}
fn get(p,k,d){if k in p&&p[k]!=(){p[k]}else{d}}
fn sp(n){theme_length("space.spacing_unit").times(n)}
fn u(n){theme_length("space.unit").times(n)}
fn ease(){cubic_bezier(0.22,1.0,0.36,1.0)}
fn word(ctx,key,label){ctx.t_or("running_now."+key,label)}
fn ink(ctx,key,value,base){text(value).with_style(ctx.component_style(key,base.min_width(px(0))))}
fn quiet(){style().typography("caption").text_color(theme_color("text_muted")).whitespace_nowrap().text_ellipsis()}
fn shell(ctx,key,items,base){row(items).with_style(ctx.component_style(key,base))}
fn svg_mark(path){svg("<svg viewBox='0 0 16 16' fill='none' stroke='currentColor' stroke-width='1.6' stroke-linecap='round' stroke-linejoin='round'><path d='"+path+"'/></svg>").accessibility_hidden(true);}
fn graphic(path){svg_mark(path).with_style(style().width(u(3.5)).height(u(3.5)).flex_shrink(false))}
fn control(cfg,ctx,p){
    if !cfg.controlled{ctx.set_state("disclosure",#{board:cfg.board,open:p});}
    if !p{ctx.set_state("request_focus",());ctx.set_state("cursor",());ctx.set_state("panel_focus",());}
    if cfg.emit{ctx.emit("open_change",p);}
}
fn trigger_key(cfg,index,ctx,event){
    if cfg.open{
        ctx.focus(index);return handled();
    }
    ctx.set_state("request_focus",#{board:cfg.board,target:index});
    control(cfg,ctx,true);handled()
}
fn keyboard_close(cfg,ctx,event){
    control(cfg,ctx,false);ctx.focus("trigger");handled()
}
fn panel_focus_changed(cfg,ctx,inside){
    if inside{ctx.set_state("panel_focus",cfg.board);}
    else if ctx.get_state("panel_focus")==cfg.board{control(cfg,ctx,false);}
}
fn move_focus(key,ctx,event){if key!=""{ctx.focus(key);}handled()}
fn navigate(ctx,target){if target!=""{ctx.focus(target);}}
fn focus_timer(ctx,p){
    let pending=ctx.get_state("request_focus");
    if pending==p.request{ctx.set_state("request_focus",());navigate(ctx,p.target);}
}
fn observe_focus(board,id,i,action,ctx,inside){
    if inside{ctx.set_state("cursor",#{board:board,id:id,index:i,action:action});}
}
fn request(kind,p,ctx,event){ctx.emit(kind,p);handled()}
fn snapshot(ctx,p){
    if !ids::valid(p.board){throw "RunningNow requires a stable board ID";}
    let seen=#{};let order=[];let n=0;let waiting=0;let paused=0;
    for task in p.tasks{
        if !ids::valid(task.id)||task.id in seen||task.title==""||utf8_byte_count(task.title)>512||utf8_byte_count(task.step)>1024{
           throw "RunningNow requires distinct task IDs and bounded labels";
        }
        seen[task.id]=#{status:task.status,title:task.title};
        order.push(task.id);
        if task.status=="running"{n+=1;}else if task.status=="needs_you"{waiting+=1;}else if task.status=="paused"{paused+=1;}
    }
    #{order:order,seen:seen,running:n,waiting:waiting,paused:paused}
}
fn noop(ctx,p){}
fn remember(ctx,p){
    let previous=ctx.get_state("seen");
    let note="";
    let changed=0;
    if previous!=()&&previous.board==p.board{
        for k in p.order{
            let next=p.seen[k];
            if k in previous.seen&&previous.seen[k].status!=next.status&&next.status in ["needs_you","done"]{
                changed+=1;
                if changed<=3{
                    if note!=""{note+=" · ";}
                    note+=next.title+": "+(if next.status=="done"{word(ctx,"done","Done")}else{word(ctx,"needs_you","Needs you")});
                }
            }
        }
    }
    if changed>3{note+=" · "+phrase(ctx,"more_updates",changed-3,"{n} more updates");}
    if note!=""||previous==()||previous.board!=p.board{ctx.set_state("notice",note);}
    ctx.set_state("seen",p);
}
fn restore(ctx,p){
    ctx.set_state("cursor",());navigate(ctx,p.target);
}
fn task_ref(id){if id==""{"panel"}else{"task:"+id}}
fn row_keys(node,m,i,cfg){
    let last=m.order.len-1;
    node.on_key_value("plain+down",Fn("move_focus").curry(task_ref(m.order[min(last,i+1)])),())
        .on_key_value("plain+up",Fn("move_focus").curry(task_ref(m.order[max(0,i-1)])),())
        .on_key_value("plain+home",Fn("move_focus").curry(task_ref(m.order[0])),())
        .on_key_value("plain+end",Fn("move_focus").curry(task_ref(m.order[last])),())
        .on_key_value("plain+escape",Fn("keyboard_close").curry(cfg),())
}
fn phrase(ctx,key,n,fallback){ctx.t_or("running_now."+key,fallback,#{n:ctx.format_number(n)})}
fn trigger_piece(ctx,count,phrase,shade,filled){
    let bits=[];
    let mark=style().width(u(if filled{1.5}else{2})).height(u(if filled{1.5}else{2})).radius(theme_radius("pill")).flex_shrink(false);
    if filled{mark=mark.background(theme_color(shade)).margin_start(sp(0.25)).margin_end(sp(0.75));}
    else{mark=mark.border(px(1.5)).border_color(theme_color(shade)).margin_end(sp(0.5));}
    bits.push(box([]).accessibility_hidden(true).with_style(mark));
    let num=if count>0{ctx.format_number(count)}else{""};
    if num!=""{
        bits.push(ink(ctx,"count",num,quiet().font_feature("tnum",1).font_weight(500).text_color(theme_color("text_primary")))
            .with_key("count").motion(motion_transition("translate_y",4.0,0.0,#{duration_ms:230,easing:ease(),intent:"decorative"}))
            .motion_on_change(num));
    }
    bits.push(ink(ctx,"trigger_caption",phrase,quiet().text_color(theme_color("running_now.trigger_ink"))).with_key("caption"));
    shell(ctx,"part",bits,style().min_width(px(0)).gap(sp(1)).items_center())
}
fn disclosure(ctx,p,m,cfg,title,first,last){
    let bits=[];let spoken=[];
    if m.running>0{
        bits.push(trigger_piece(ctx,m.running,word(ctx,"running","running"),"running_now.teal",false).with_key("running"));
        spoken.push(phrase(ctx,"count_running",m.running,"{n} running"));
    }
    if m.waiting>0{
        if m.running>0{bits.push(ink(ctx,"separator","·",quiet().flex_shrink(false)
            .text_color(theme_color("running_now.faint"))).with_key("dot").accessibility_hidden(true));}
        let singular=m.waiting==1;
        bits.push(trigger_piece(ctx,m.waiting,if singular{word(ctx,"needs_one","needs you")}else{word(ctx,"need_many","need you")},"running_now.amber",true).with_key("waiting"));
        spoken.push(if singular{phrase(ctx,"count_needs",m.waiting,"{n} needs you")}else{phrase(ctx,"count_need",m.waiting,"{n} need you")});
    }
    if bits.len==0{
        bits.push(trigger_piece(ctx,m.paused,if m.paused>0{word(ctx,"paused_count","paused")}else{word(ctx,"idle","No tasks running")},"running_now.faint",false));
        spoken.push(if m.paused>0{phrase(ctx,"count_paused",m.paused,"{n} paused")}else{word(ctx,"idle","No tasks running")});
    }
    let summary=title+": "+spoken[0];if spoken.len>1{summary+=", "+spoken[1];}
    let deg=if cfg.open{180.0}else{0.0};
    bits.push(graphic("M4 6l4 4 4-4").with_key("chevron")
        .with_style(ctx.component_style("chevron",style().transform(#{rotate:deg}).text_color(theme_color("text_muted"))))
        .motion(motion_transition("rotate",deg,deg,#{duration_ms:200,easing:ease(),intent:"decorative"})));
    shell(ctx,if cfg.open{["trigger","trigger_open"]}else{["trigger"]},bits,style().min_width(px(0)).max_width(relative(1.0))
        .min_height(u(7.5)).padding_x(sp(2.5)).items_center().gap(sp(2)).radius(theme_radius("sm"))
        .background(theme_color("surface_hover")).focus_ring(px(2),offset_px(-1),theme_color("focus_ring")).cursor_pointer())
      .with_key("trigger").with_ref(element_ref("trigger")).tab_stop(true).accessibility_role("button").accessibility_label(summary)
      .accessibility_expanded(cfg.open)
      .paint_transition(140,cubic_bezier(0.25,0.1,0.25,1.0))
      .on_key_value("plain+down",Fn("trigger_key").curry(cfg,first),())
      .on_key_value("plain+up",Fn("trigger_key").curry(cfg,last),())
}
fn ring(ctx,t,fraction){
    let tone=if t.status=="done"{"running_now.blue"}else if t.status=="paused"{"text_muted"}
        else if fraction<=0.25{"running_now.orange"}else if fraction<=0.5{"running_now.amber"}else{"running_now.teal"};
    let rim=[path_move(8,2),path_cubic(14,8,11.31,2,14,4.69),path_cubic(8,14,14,11.31,11.31,14),
        path_cubic(2,8,4.69,14,2,11.31),path_cubic(8,2,2,4.69,4.69,2),path_close()];
    let path=[path_move(8,6),path_cubic(10,8,9.1,6,10,6.9),path_cubic(8,10,10,9.1,9.1,10),
        path_cubic(6,8,6.9,10,6,9.1),path_cubic(8,6,6,6.9,6.9,6),path_close()];
    let paint=theme_color(tone);
    let full=style().absolute().top(px(0)).inset_start(px(0)).width(relative(1.0)).height(relative(1.0));
    stack([canvas(canvas_scene([canvas_stroke_path("rim",rim,1.5,paint)]).view_box(0,0,16,16)).with_key("rim").with_style(full),
        canvas(canvas_scene([canvas_stroke_path("pie",path,4.0,paint)]).view_box(0,0,16,16)).with_key("pie").with_style(full)
            .motion(motion_transition("path_progress",fraction,fraction,#{duration_ms:650,easing:ease(),intent:"feedback"}))])
        .with_key("ring").accessibility_hidden(true)
        .with_style(ctx.component_style("ring",style().relative().width(u(4)).height(u(4)).flex_shrink(false)))
}
fn task_control(ctx,p,t,ticket){
    let action=if t.status=="paused"{"resume"}else if t.status=="needs_you"{"open"}else if t.status=="done"{"done"}else{"pause"};
    let label=switch action{"resume"=>word(ctx,"resume","Resume"),"open"=>word(ctx,"open","Open"),"done"=>word(ctx,"done","Done"),_=>word(ctx,"pause","Pause")};
    let path=switch action{"resume"=>"M5 3l8 5-8 5z","done"=>"M3 8l3 3 7-7",_=>"M6 4v8m4-8v8"};
    let content=if action=="open"{ink(ctx,"action_label",label,quiet().font_weight(500).text_color(theme_color("text_primary")))}else{graphic(path)};
    let mutable=action in ["pause","resume"];
    let available=(mutable&&get(p,"on_action",())!=())||(action=="open"&&get(p,"on_open_task",())!=());
    let pending=mutable&&t.id in p.pending;
    let base=style().min_width(u(7)).height(u(7)).padding_x(sp(1.25)).items_center().justify_center().flex_shrink(false)
        .radius(theme_radius("sm")).focus_ring(px(2),offset_px(-1),theme_color("focus_ring")).text_color(theme_color("text_muted"));
    let parts=if action=="done"{["action","done_mark"]}else if action=="open"{["action","action_text"]}else{["action"]};
    let button=shell(ctx,parts,[content],base).with_key("action").with_ref(element_ref("action:"+t.id));
    if available{
        button=button.tab_stop(true).accessibility_role("button").accessibility_label(label+" "+t.title);
        if pending{button=button.accessibility_description(word(ctx,"pending","Waiting for acknowledgement")).with_style(style().opacity(0.55));}
        if !pending{
            let event=if mutable{"action"}else{"open_task"};
            if mutable{ticket.kind=action;}
            let cb=Fn("request").curry(event,ticket);
            button=button.on_click(cb).on_key_value("plain+enter",cb,()).on_key_value("plain+space",cb,());
        }
        button
    }else{button.accessibility_hidden(true)}
}
fn row_view(ctx,p,m,t,i,cfg){
    let total=t.total;let done=if t.status=="done"{total}else{min(t.done,total)};
    let ratio=if t.status=="done"{1.0}else if total==0{0.0}else{done.to_float()/total.to_float()};
    let stage=switch t.status{"paused"=>word(ctx,"paused","Paused"),"needs_you"=>word(ctx,"needs_you","Needs you"),
        "done"=>word(ctx,"done","Done"),_=>word(ctx,"running","Running")};
    let detail=if t.status=="done"{stage}else if t.status=="paused"{stage+(if t.step!=""{" · "+t.step}else{""})}else{t.step};
    let description=stage+(if t.step!=""&&t.status!="done"{". "+t.step}else{""});
    if total>0{
        description+=". "+ctx.t_or("running_now.steps","{done} of {total} steps",#{done:ctx.format_number(done),total:ctx.format_number(total)});
    }
    let opts=if t.status=="needs_you"{
        ink(ctx,"badge",stage,quiet().padding_x(sp(1.5)).padding_y(sp(0.5)).radius(theme_radius("sm"))
            .text_color(theme_color("running_now.badge_text")).background(theme_color("running_now.badge")).flex_shrink(false))
    }else{
        ink(ctx,"caption",if total>0{ctx.format_number(done)+"/"+ctx.format_number(total)}else{"—"},quiet().font_feature("tnum",1).flex_shrink(false))
    };
    let names=column([ink(ctx,if t.status=="done"{["name","finished"]}else{["name"]},t.title,
            style().typography("body").font_weight(500).whitespace_nowrap().text_ellipsis()).with_key("name"),
        ink(ctx,"step",detail,quiet()).with_key("step")
            .motion(motion_transition("translate_y",4.0,0.0,#{duration_ms:230,easing:ease(),intent:"decorative"})).motion_on_change(detail)])
        .with_key("names").with_style(style().flex_grow().flex_basis(px(0)).min_width(px(0)));
    let ticket=#{board:p.board,revision:p.revision,id:t.id};
    let actionable=get(p,"on_open_task",())!=();
    let label=shell(ctx,"main",[ring(ctx,t,ratio),names,opts],style().flex_grow().flex_basis(px(0)).min_width(px(0)).height(relative(1.0))
        .padding_x(sp(1.5)).items_center().gap(sp(2)).radius(theme_radius("sm")).focus_ring(px(2),offset_px(-1),theme_color("focus_ring")))
        .with_key("main").with_ref(element_ref(task_ref(t.id))).tab_stop(true).accessibility_role(if actionable{"button"}else{"group"})
        .accessibility_label(t.title).accessibility_description(description)
        .on_focus_within_change(Fn("observe_focus").curry(p.board,t.id,i,false));
    if actionable{
        let cb=Fn("request").curry("open_task",ticket);
        label=label.on_click(cb).on_key_value("plain+enter",cb,()).on_key_value("plain+space",cb,());
    }
    let button=task_control(ctx,p,t,ticket).on_focus_within_change(Fn("observe_focus").curry(p.board,t.id,i,true));
    shell(ctx,"row",[row_keys(label,m,i,cfg),row_keys(button,m,i,cfg)],style().min_width(px(0)).height(u(13)).width(relative(1.0)).items_center()
        .gap(sp(0.75)).padding_end(sp(1.5)).radius(theme_radius("sm")).hover(style().background(theme_color("surface_hover"))))
        .with_key("row:"+t.id).accessibility_role("listitem")
}
fn panel(ctx,p,m,title,cfg){
    let headline=shell(ctx,"heading",[ink(ctx,"label",title,quiet().font_weight(500).text_color(theme_color("text_primary"))),
        ink(ctx,"count",ctx.format_number(p.tasks.len),quiet().font_feature("tnum",1))],
        style().min_height(u(7)).padding_x(sp(2)).gap(sp(1.5)).items_center()).with_key("heading");
    let items=[];
    for i in 0..p.tasks.len{items.push(row_view(ctx,p,m,p.tasks[i],i,cfg));}
    if items.len==0{items.push(ink(ctx,"empty",word(ctx,"empty","Nothing running right now"),quiet().padding(sp(3))));}
    let list=column(items).with_key("list").accessibility_role("list").accessibility_label(title)
        .with_style(ctx.component_style("list",style().max_height(u(71.5)).min_width(px(0)).overflow_y_scroll()));
    let children=[headline,column([list]).with_key("body").with_style(ctx.component_style("body",style().min_width(px(0)).radius(theme_radius("lg"))))];
    let has_footer=get(p,"on_open_all",())!=();
    if has_footer{
        let name=word(ctx,"open_all","Open Tasks");let cb=Fn("request").curry("open_all",#{board:p.board,revision:p.revision});
        let path=if ctx.text_direction()=="rtl"{"M9 4l-4 4 4 4"}else{"M6 4l4 4-4 4"};
        children.push(shell(ctx,"footer",[graphic("M3 4h10M3 8h10M3 12h7"),
            ink(ctx,"name",name,quiet().flex_grow().font_weight(500)),graphic(path)],style().min_height(u(8))
            .padding_x(sp(2)).gap(sp(2)).items_center().radius(theme_radius("sm")).text_color(theme_color("text_muted"))
            .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-1),theme_color("focus_ring")))
            .with_key("footer").with_ref(element_ref("footer")).tab_stop(true).accessibility_role("button").accessibility_label(name)
            .on_key_value("plain+escape",Fn("keyboard_close").curry(cfg),())
            .on_click(cb).on_key_value("plain+enter",cb,()).on_key_value("plain+space",cb,()));
    }
    column(children).with_key("panel").with_ref(element_ref("panel")).accessibility_role("dialog").accessibility_label(title)
        .tab_stop(p.tasks.len==0&&!has_footer)
        .on_focus_within_change(Fn("panel_focus_changed").curry(cfg))
        .on_key_value("plain+escape",Fn("keyboard_close").curry(cfg),())
        .with_style(ctx.component_style("panel",style().width(px(348)).max_width(relative(1.0)).min_width(px(0)).padding(sp(1))
            .radius(theme_radius("lg")).background(theme_color("surface_raised"))))
}
fn render_running(ctx,p){
    let m=snapshot(ctx,p);let previous=ctx.get_state("disclosure");let controlled=get(p,"open",())!=();
    let shown=if controlled{p.open}else if previous!=()&&previous.board==p.board{previous.open}else{p.default_open};
    let cfg=#{board:p.board,controlled:controlled,open:shown,emit:get(p,"on_open_change",())!=()};
    let title=get(p,"title",word(ctx,"title","Background tasks"));
    let empty=if get(p,"on_open_all",())!=(){"footer"}else{"panel"};
    let first=if m.order.len==0{empty}else{task_ref(m.order[0])};
    let last=if m.order.len==0{empty}else{task_ref(m.order[m.order.len-1])};
    effect("remember",#{board:p.board,seen:m.seen,order:m.order},Fn("remember"),Fn("noop"));
    if shown{
        let pending=ctx.get_state("request_focus");
        if pending!=()&&pending.board==p.board{
            let chosen=if pending.target==last||pending.target==first{pending.target}else{first};
            // Resolve against the latest accepted list, and deliver after the panel is laid out.
            timeout("focus_task",16,false,Fn("focus_timer"),#{request:pending,target:chosen});
        }else{
            let observed=ctx.get_state("cursor");
            if observed!=()&&observed.board==p.board{
                let exists=observed.id in m.seen;
                if !exists||(observed.action&&m.seen[observed.id].status=="done"){
                    let to=if !exists{if m.order.len==0{empty}else{task_ref(m.order[min(observed.index,m.order.len-1)])}}else{task_ref(observed.id)};
                    effect("restore",#{target:to},Fn("restore"),Fn("noop"));
                }
            }
        }
    }
    let frame=overlay(disclosure(ctx,p,m,cfg,title,first,last),if shown{panel(ctx,p,m,title,cfg)}else{column([])},
        #{id:p.key,kind:"popover",open:shown,placement:"bottom",gap:6.0,align:p.align,modal:false,initial_focus:"first",
        dismiss_on_outside:true,dismiss_on_escape:true})
        .with_key("popover").on_open_change(Fn("control").curry(cfg))
        .motion(motion_transition("overlay_reveal",0.0,1.0,#{duration_ms:200,easing:ease(),intent:"decorative"}))
        .motion_on_change(if shown{"open"}else{"closed"});
    if shown{frame=frame.on_key_value("plain+escape",Fn("keyboard_close").curry(cfg),());}
    let seen=ctx.get_state("seen");
    let notice=if seen!=()&&seen.board==p.board{ctx.get_state("notice")}else{""};
    let body=[frame];
    if notice!=""{
        body.push(ink(ctx,"announcement",notice,style().absolute().width(px(1)).height(px(1)).overflow_hidden().opacity(0.0))
            .with_key("announcement").accessibility_role("status").accessibility_label(notice));
    }
    row(body).with_key(p.key).with_style(ctx.component_style("root",style().relative().min_width(px(0)).max_width(relative(1.0))))
}
