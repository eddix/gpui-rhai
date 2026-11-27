/* gpui-rhai
{"id":"patterns/run_controls","export":"RunControls","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/button","components/icon_button","components/textarea","components/thinking_loader","components/toggle_group"],"capabilities":{},"assets":[],"tokens":["accent","border","focus_ring","surface_hover","surface_raised","text.danger","text_muted","text_primary","radius.lg","radius.sm","typography.caption","typography.body","metrics.run_prompt_tip","run.mark","space.spacing_unit","space.unit"],"environment":["density"]}
*/
// RunControls presents a Host-owned run transcript, queue and draft editor.
// State: the Host owns run identity, lifecycle, draft, queue, mode and policy.
// Example: RunControls(#{key:"run",run_id:"job/7",draft:"",status:"idle"})
import "components/button" as button;
import "components/icon_button" as icon_button;
import "components/textarea" as textarea;
import "components/thinking_loader" as thinking_loader;
import "components/toggle_group" as toggle_group;

define_component(#{
    metadata:#{id:"patterns/run_controls","export":"RunControls",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/button","components/icon_button","components/textarea","components/thinking_loader","components/toggle_group"],capabilities:#{},assets:[],
        tokens:["accent","border","focus_ring","surface_hover","surface_raised","text.danger","text_muted","text_primary","radius.lg","radius.sm","typography.caption","typography.body","metrics.run_prompt_tip","run.mark","space.spacing_unit","space.unit"],environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable workspace identity. The editor survives run status changes."},
        run_id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Current Host run identity (or empty before the first run); echoed in run events."},
        draft:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Accepted editable text. The Host clears it after accepting a submission."},
        status:#{schema:#{type:"string",allowed:["idle","running","stopped","error","done"]},
            "default":#{type:"string",value:"idle"},required:false,sensitive:false,doc:"Accepted run status; never advanced by the control."},
        agent_id:#{schema:#{type:"string"},"default":#{type:"string",value:"assistant"},required:false,sensitive:false,doc:"Stable seed for the decorative agent mark."},
        agent_name:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Display name. Omit for a localized fallback."},
        prompt:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:true,doc:"Accepted prompt of the current transcript."},
        answer:#{schema:#{type:"string"},"default":#{type:"string",value:""},required:false,sensitive:true,doc:"Accepted literal response as it streams; up to 96000 UTF-8 bytes."},
        log_height:#{schema:#{type:"integer",min:80,max:1200},"default":#{type:"integer",value:252},
            required:false,sensitive:false,doc:"Fixed native transcript viewport height in logical pixels; the composer remains independently sized."},
        steers:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable, unique accepted note ID."},
            at:#{schema:#{type:"integer",min:0},required:true,sensitive:false,doc:"Unicode scalar offset in the accepted answer, not a byte offset."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Accepted steering note shown as literal text."},
        }}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Accepted notes in chronological order of nondecreasing answer offset."},
        queue:#{schema:#{type:"array",max_items:64,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable, unique Host-owned queue ID."},
            text:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Queued draft shown without markup interpretation."},
        }}},"default":#{type:"array",value:[]},required:false,sensitive:false,doc:"Accepted queue in dispatch order. Removing or dispatching remains Host-owned."},
        mode:#{schema:#{type:"string",allowed:["chat","act"]},"default":#{type:"string",value:"chat"},required:false,sensitive:false,
            doc:"Accepted workspace mode. Chat disables permission choices. This property never authorizes work."},
        permission:#{schema:#{type:"string",allowed:["ask","edits","full"]},"default":#{type:"string",value:"ask"},required:false,sensitive:false,
            doc:"Accepted, application-owned policy. Proposals never grant access or execute anything."},
        context:#{schema:#{type:"optional",value:#{type:"number",min:0.0,max:1.0}},required:false,sensitive:false,doc:"Optional share of the context window already used."},
        elapsed_ms:#{schema:#{type:"optional",value:#{type:"integer",min:0,max:9000000000000}},required:false,sensitive:false,doc:"Application-measured duration of completed work."},
        placeholder:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Hint for the native editor."},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disable editing and all actionable controls."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Draft edited; receives #{run,text}."},
        on_send:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Send a new prompt; receives #{run,text}."},
        on_steer:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Steer the current run; receives #{run,text}."},
        on_queue:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Queue an input; receives #{run,text}."},
        on_unqueue:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Remove request with the current run and queue ID."},
        on_stop:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Stop or Escape; receives run_id."},
        on_resume:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Resume stopped or failed work; receives run_id."},
        on_retry:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Retry the current run; receives run_id."},
        on_dismiss:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Dismiss the current run; receives run_id."},
        on_mode_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"A proposal; the Host accepts or rejects the workspace mode."},
        on_permission_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"A proposal; only the Host can authorize work."},
    },state:#{fields:#{
        permission_open:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        following:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        change:#{payload:#{type:"object",allow_unknown:false,fields:#{run:#{schema:#{type:"string"},required:true,sensitive:false},
            text:#{schema:#{type:"string"},required:true,sensitive:true}}},doc:"Draft edited; receives #{run,text}."},
        send:#{payload:#{type:"object",allow_unknown:false,fields:#{run:#{schema:#{type:"string"},required:true,sensitive:false},
            text:#{schema:#{type:"string"},required:true,sensitive:true}}},doc:"Send a new prompt; receives #{run,text}."},
        steer:#{payload:#{type:"object",allow_unknown:false,fields:#{run:#{schema:#{type:"string"},required:true,sensitive:false},
            text:#{schema:#{type:"string"},required:true,sensitive:true}}},doc:"Steer the current run; receives #{run,text}."},
        queue:#{payload:#{type:"object",allow_unknown:false,fields:#{run:#{schema:#{type:"string"},required:true,sensitive:false},
            text:#{schema:#{type:"string"},required:true,sensitive:true}}},doc:"Queue an input; receives #{run,text}."},
        unqueue:#{payload:#{type:"object",allow_unknown:false,fields:#{run:#{schema:#{type:"string"},required:true,sensitive:false},
            id:#{schema:#{type:"string"},required:true,sensitive:false}}},doc:"Remove request with the current run and queue ID."},
        stop:#{payload:#{type:"string"},doc:"Stop or Escape; receives run_id."},
        resume:#{payload:#{type:"string"},doc:"Resume stopped or failed work; receives run_id."},
        retry:#{payload:#{type:"string"},doc:"Retry the current run; receives run_id."},
        dismiss:#{payload:#{type:"string"},doc:"Dismiss the current run; receives run_id."},
        mode_change:#{payload:#{type:"string"},doc:"A proposal; the Host accepts or rejects the workspace mode."},
        permission_change:#{payload:#{type:"string"},doc:"A proposal; only the Host can authorize work."},
    },slots:#{},parts:["root","log","empty","prompt","agent","agent_mark","agent_cell","name","status","answer","steer","after","note",
        "composer","queue","queued","queued_index","remove","write","editor","tools","modes","permission","permission_trigger","permission_panel",
        "permission_item","permission_selected","permission_alert","permission_hint","context","actions","send","stop","resume","retry","dismiss","icon","jump"],
        effects:["close_policy"]},
    render:Fn("render_RunControls")
});

fn RunControls(props){render_component("patterns/run_controls",props)}
fn opt(props,key){if key in props{props[key]}else{()}}
fn can(props,event){!props.disabled&&opt(props,"on_"+event)!=()}
fn run_change(event,run,ctx,value){ctx.emit(event,#{run:run,text:value});}
fn submit(event,run,ctx,value){
    let clean=value;clean.trim();
    if clean!=""{run_change(event,run,ctx,clean);}
    handled()
}
fn submit_button(event,run,draft,ctx,payload){submit(event,run,ctx,draft);ctx.focus("draft");handled()}
fn request(event,value,ctx,payload){ctx.emit(event,value);handled()}
fn stop_key(open,enabled,run,ctx,event){
    if open||!enabled{return propagate();}
    request("stop",run,ctx,())
}
fn mode_request(ctx,value){if value.len>0{ctx.emit("mode_change",value[0]);}}
fn menu_open(ctx,open){ctx.set_state("permission_open",open);}
fn disallow_menu(ctx,allowed){if !allowed{ctx.set_state("permission_open",false);}}
fn cleanup(ctx,allowed){}
fn permission_choice(ctx,value){ctx.set_state("permission_open",false);ctx.emit("permission_change",value);handled()}
fn follow(run,ctx,value){ctx.set_state("following",#{run:run,active:value.following});}
fn latest(key,ctx,payload){ctx.scroll_collection_to_end(key);handled()}
fn oldest(key,ctx,payload){ctx.scroll_collection_to_start(key);handled()}
fn glyph(ctx,id){
    let path=switch id{
        "send"=>"M12 19V5m-6 6 6-6 6 6",
        "stop"=>"M6 6h12v12H6z",
        "play"=>"m8 5 12 7-12 7z",
        "retry"=>"M20 7V3m0 4h-4m4 0A9 9 0 1 0 21 14",
        "queue"=>"M4 5h16M4 10h16M4 15h16v5H4z",
        "steer"=>"M5 4v6a7 7 0 0 0 7 7h7m-4-4 4 4-4 4",
        "ask"=>"M12 2l8 4v6c0 5-4 8-8 10-4-2-8-5-8-10V6z",
        "edits"=>"M5 16 16 5l3 3L8 19H5zm9-9 3 3",
        "full"=>"m13 2-8 12h6l-1 8 9-12h-6z",
        "chevron"=>"m6 9 6 6 6-6",
        "check"=>"m5 12 4 4 10-11",
        _=>"m6 6 12 12m0-12L6 18",
    };
    svg(`<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.7' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
        .accessibility_hidden(true).with_style(ctx.component_style("icon",style().width(px(14)).height(px(14)).flex_shrink(false)))
}
fn agent_mark(ctx,id,busy){
    let code=0;let n=0;
    for ch in id.chars(){
        if n>=128{break;}
        code=(code*31+ch.to_int())%8191;n+=1;
    }
    let rows=[];
    for y in 0..3{
        let cells=[];
        for x in 0..3{
            let on=x==1||((code >> y) & 1)==1||y==1;
            let cell=row([]).with_key(`cell-${x}`).with_style(ctx.component_style("agent_cell",style()
                .width(px(4)).height(px(4)).radius(px(0.6)).background(theme_color("run.mark")).opacity(if on{1.0}else{0.0})));
            if on&&busy{cell=cell.motion(motion_transition("opacity",1.0,0.28,#{duration_ms:540,delay_ms:(x+y)*95,
                iterations:"infinite",autoreverse:true,intent:"decorative"}));}
            cells.push(cell);
        }
        rows.push(row(cells).with_key(`row-${y}`).with_style(style().gap(theme_length("space.spacing_unit").times(0.25))));
    }
    column(rows).with_key("mark").accessibility_hidden(true).with_style(ctx.component_style("agent_mark",style()
        .width(theme_length("space.unit").times(5)).height(theme_length("space.unit").times(5)).flex_shrink(false).gap(theme_length("space.spacing_unit").times(0.25)).items_center().justify_center()))
        .audit_allow(["text-edge-misaligned"])
}
fn status_text(ctx,status,answered,steered){
    if status=="running"{
        if !answered{ctx.t_or("thinking.label","Thinking")}else if steered{ctx.t_or("run.steered","Writing · steered")}else{ctx.t_or("run.writing","Writing")}
    }else if status=="stopped"{ctx.t_or("thinking.stopped","Stopped")}
    else if status=="error"{ctx.t_or("run.error","Connection lost")}
    else if status=="done"{ctx.t_or("run.done","Done")}else{""}
}
fn render_log_row(ctx,payload){
    let item=payload.item;
    let prompt_style=ctx.component_style("prompt",style().min_width(px(0)).typography("body"));
    let tip=theme_length("metrics.run_prompt_tip");
    prompt_style=if ctx.text_direction()=="rtl"{prompt_style.radius_bottom_left(tip)}else{prompt_style.radius_bottom_right(tip)};
    let leaf=switch item.kind{
        "agent"=>row([agent_mark(ctx,item.agent,item.busy),text(item.name).with_style(ctx.component_style("name",style().min_width(px(0)).font_weight(600))),
            text(item.label).accessibility_role("status").with_style(ctx.component_style("status",style().typography("caption")
                .text_color(theme_color(if item.error{"text.danger"}else{"text_muted"}))))])
            .with_style(ctx.component_style("agent",style().items_center().gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap())),
        "steer"=>row([glyph(ctx,"steer"),text(item.text).with_style(style().min_width(px(0)).typography("caption"))])
            .with_style(ctx.component_style("steer",style().gap(theme_length("space.spacing_unit").times(1.5)).items_center().padding(theme_length("space.spacing_unit").times(1.25))
                .radius(theme_radius("sm")).background(theme_color("surface_hover")))),
        "pending"=>thinking_loader::ThinkingLoader(#{key:"thinking",phase:0,variant:"plain",size:"sm"}),
        "prompt"=>text(item.text).selectable(true).with_style(prompt_style),
        "empty"=>column([agent_mark(ctx,item.agent,false),text(item.text).with_style(style().min_width(px(0)))])
            .with_style(ctx.component_style("empty",style().min_height(theme_length("space.unit").times(29.5)).items_center().justify_center().gap(theme_length("space.spacing_unit").times(2.5))
                .typography("body").text_color(theme_color("text_muted")))),
        _=>text(item.text).selectable(true).with_style(ctx.component_style("answer",style().min_width(px(0)).typography("body"))),
    };
    column([leaf]).with_key(payload.key).with_style(style().min_width(px(0)).padding_bottom(theme_length("space.spacing_unit").times(1.25)))
}
fn add_text(rows,answer,from,to,prefix){
    let n=0;
    while from<to{
        let len=min(1600,to-from);
        rows.push(#{key:prefix+":"+n.to_string(),kind:"answer",text:answer.sub_string(from,len)});
        from+=len;n+=1;
    }
    rows
}
fn collect_log(ctx,props,name,running){
    let prompt=opt(props,"prompt");
    if prompt==()||prompt==""{return [#{key:"empty",kind:"empty",agent:props.agent_id,
        text:ctx.t_or("run.empty","Ask {agent} anything. You can steer or stop it while it works.",#{agent:name})}];}
    let answer=props.answer;
    if utf8_byte_count(answer)>96000{throw "RunControls answer exceeds 96000 bytes";}
    let rows=[#{key:"prompt",kind:"prompt",text:prompt},#{key:"agent",kind:"agent",agent:props.agent_id,name:name,
        label:status_text(ctx,props.status,answer!="",props.steers.len>0),busy:running,error:props.status=="error"}];
    let at=0;let previous=0;let seen=[];
    for note in props.steers{
        if note.id==""||note.id in seen{throw "RunControls steering note IDs must be unique and nonempty";}
        if note.at<previous{throw "RunControls steering notes must be ordered by accepted offsets";}
        previous=note.at;seen.push(note.id);
        if note.at>answer.len{continue;}
        rows=add_text(rows,answer,at,note.at,"before:"+note.id);
        rows.push(#{key:"note:"+note.id,kind:"steer",text:note.text});at=note.at;
    }
    rows=add_text(rows,answer,at,answer.len,"tail");
    if running&&answer==""{rows.push(#{key:"pending",kind:"pending"});}
    rows
}
fn menu_name(ctx,value){
    if value=="ask"{ctx.t_or("run.ask","Ask first")}
    else if value=="edits"{ctx.t_or("run.edits","Auto-edit")}
    else{ctx.t_or("run.full","Full access")}
}
fn menu_hint(ctx,value){
    if value=="ask"{ctx.t_or("run.ask_hint","Ask before making a change")}
    else if value=="edits"{ctx.t_or("run.edits_hint","Edit files; ask before other actions")}
    else{ctx.t_or("run.full_hint","Allow tool use without asking")}
}
fn move_choice(index,delta,ctx,payload){
    let next=if delta==3{2}else if delta== -3{0}else{(index+3+delta)%3};
    ctx.focus("policy:"+next.to_string());handled()
}
fn permission_menu(ctx,props,open,eligible){
    let values=["ask","edits","full"];
    let trigger_label=menu_name(ctx,props.permission);
    let trigger=row([glyph(ctx,props.permission),text(trigger_label).with_style(style().min_width(px(0)).text_ellipsis()),glyph(ctx,"chevron")])
        .with_key("trigger").accessibility_role("button")
        .accessibility_label(ctx.t_or("run.permissions","Permissions: {mode}",#{mode:trigger_label})).accessibility_expanded(open)
        .disabled(!eligible).with_style(ctx.component_style(if props.permission=="full"{["permission_trigger","permission_alert"]}else{["permission_trigger"]},
            style().items_center().gap(theme_length("space.spacing_unit")).min_width(px(0)).min_height(theme_length("space.unit").times(6.5)).padding_x(theme_length("space.spacing_unit").times(1.25)).radius(theme_radius("sm")).typography("caption")
                .hover(style().background(theme_color("surface_hover"))).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))));
    if !eligible{return trigger;}
    let choices=[];
    for i in 0..3{
        let value=values[i];let selected=props.permission==value;let title=menu_name(ctx,value);let hint=menu_hint(ctx,value);
        let parts=["permission_item"];if selected{parts.push("permission_selected");}if value=="full"{parts.push("permission_alert");}
        let end=glyph(ctx,"check").with_style(style().opacity(if selected{1.0}else{0.0}));
        choices.push(row([glyph(ctx,value),column([text(title),text(hint).with_style(ctx.component_style("permission_hint",
            style().typography("caption").text_color(theme_color("text_muted"))))]).with_style(style().min_width(px(0)).flex_grow()),end])
            .with_key(value).with_ref(element_ref("policy:"+i.to_string())).accessibility_role("radio").accessibility_label(title)
            .accessibility_description(hint).accessibility_checked(selected).tab_stop(true)
            .with_style(ctx.component_style(parts,style().gap(theme_length("space.spacing_unit").times(1.75)).padding(theme_length("space.spacing_unit").times(1.5)).min_width(px(0)).items_center()
                .radius(theme_radius("sm")).typography("body").hover(style().background(theme_color("surface_hover")))
                .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
            .on_click_value(Fn("permission_choice"),value)
            .on_key_value("down",Fn("move_choice").curry(i,1),()).on_key_value("up",Fn("move_choice").curry(i,-1),())
            .on_key_value("home",Fn("move_choice").curry(i,-3),()).on_key_value("end",Fn("move_choice").curry(i,3),()));
    }
    let panel=column(choices).accessibility_role("menu").accessibility_label(ctx.t_or("run.policy_title","Application permissions"))
        .with_style(ctx.component_style("permission_panel",style().width(px(270)).max_width(relative(1.0)).min_width(px(0))
            .gap(theme_length("space.spacing_unit").times(0.5)).padding(theme_length("space.spacing_unit").times(1.25)).background(theme_color("surface_raised")).radius(theme_radius("lg"))));
    overlay(trigger,panel,#{id:"policies",kind:"menu",placement:"top",align:"start",gap:6.0,initial_focus:"first",open:open,
        modal:false,dismiss_on_escape:true,dismiss_on_outside:true}).with_key("policies").on_open_change(Fn("menu_open"))
        .with_style(ctx.component_style("permission",style().min_width(px(0))))
}
fn mode_view(ctx,props){
    toggle_group::ToggleGroup(#{key:"mode",size:"xs",label:ctx.t_or("run.mode","Mode"),values:[props.mode],allow_empty:false,
        items:[#{value:"chat",label:ctx.t_or("run.chat","Chat")},#{value:"act",label:ctx.t_or("run.act","Act")}],
        disabled:!can(props,"mode_change"),on_change:if can(props,"mode_change"){Fn("mode_request")}else{()}})
        .with_style(ctx.component_style("modes",style()))
}
fn context_meter(ctx,amount){
    let circle=[path_move(8,2),path_cubic(14,8,11.31,2,14,4.69),path_cubic(8,14,14,11.31,11.31,14),
        path_cubic(2,8,4.69,14,2,11.31),path_cubic(8,2,2,4.69,4.69,2),path_close()];
    let track=canvas(canvas_scene([canvas_stroke_path("track",circle,1.5,theme_color("border"))]).view_box(0,0,16,16))
        .with_key("track").accessibility_hidden(true).with_style(style().width(relative(1.0)).height(relative(1.0)));
    let fill=canvas(canvas_scene([canvas_stroke_path("fill",circle,1.5,theme_color(if amount>=0.9{"text.danger"}else{"run.mark"})).stroke_cap("round")]).view_box(0,0,16,16))
        .with_key("fill").accessibility_hidden(true).with_style(style().absolute().top(px(0)).left(px(0))
            .width(relative(1.0)).height(relative(1.0)))
        .motion(motion_transition("path_progress",amount,amount,#{duration_ms:220,intent:"feedback"}));
    row([track,fill]).with_key("context").accessibility_role("progressbar")
        .accessibility_label(ctx.t_or("run.context","Context used")).accessibility_value((amount*100).to_int())
        .accessibility_value_min(0).accessibility_value_max(100)
        .with_style(ctx.component_style("context",style().width(theme_length("space.unit").times(4.25)).height(theme_length("space.unit").times(4.25)).relative().flex_shrink(false)))
}
fn queued_views(ctx,props){
    let nodes=[];let ids=[];
    for item in props.queue{
        if item.id==""||item.id in ids{throw "RunControls queued IDs must be unique and nonempty";}
        let count=ids.len;ids.push(item.id);
        let label=if count==0{ctx.t_or("run.next","Next")}else{"#"+ctx.format_number(count+1)};
        let remove=icon_button::IconButton(#{key:"remove",size:"xs",label:ctx.t_or("run.remove","Remove “{message}” from queue",#{message:item.text}),
            icon:glyph(ctx,"close"),disabled:!can(props,"unqueue"),
            on_click:Fn("request").curry("unqueue",#{run:props.run_id,id:item.id})})
            .with_style(ctx.component_style("remove",style().flex_shrink(false)));
        nodes.push(row([glyph(ctx,"queue"),text(item.text).with_style(style().min_width(px(0)).flex_grow().text_ellipsis().whitespace_nowrap()),
            text(label).with_style(ctx.component_style("queued_index",style().flex_shrink(false).text_color(theme_color("text_muted")))),remove])
            .with_key(item.id).accessibility_role("listitem").accessibility_label(item.text)
            .with_style(ctx.component_style("queued",style().items_center().gap(theme_length("space.spacing_unit").times(1.5)).padding_x(theme_length("space.spacing_unit").times(1.5)).min_width(px(0)).typography("caption"))));
    }
    column(nodes).with_key("queue").accessibility_role("list").accessibility_label(ctx.t_or("run.queued","Queued messages"))
        .with_style(ctx.component_style("queue",style().max_height(px(126)).min_width(px(0)).overflow_y_scroll()))
}
fn run_action(ctx,props,id,label,icon,variant){
    button::Button(#{key:id,text:label,prefix:glyph(ctx,icon),variant:variant,size:"xs",disabled:!can(props,id),
        on_click:Fn("request").curry(id,props.run_id)}).with_style(ctx.component_style(id,style()))
}
fn status_actions(ctx,props,running){
    let widgets=[];
    if running{
        widgets.push(text(ctx.t_or("run.stop_keys","Esc to stop")).with_style(ctx.component_style("note",style().typography("caption")
            .text_color(theme_color("text_muted")))));
    }else if props.status=="stopped"||props.status=="error"{
        let caption=if props.answer==""{ctx.t_or("run.no_answer","Stopped before an answer")}
            else{ctx.t_or("run.partial","Partial answer retained")};
        widgets.push(text(caption).with_style(ctx.component_style("note",style().typography("caption").text_color(theme_color("text_muted")))));
        if can(props,"resume"){widgets.push(run_action(ctx,props,"resume",ctx.t_or("run.resume","Resume"),"play","secondary"));}
        if can(props,"retry"){widgets.push(run_action(ctx,props,"retry",ctx.t_or("run.retry","Retry"),"retry","ghost"));}
        if can(props,"dismiss"){widgets.push(run_action(ctx,props,"dismiss",ctx.t_or("run.dismiss","Dismiss"),"close","ghost"));}
    }else if props.status=="done"{
        let elapsed=opt(props,"elapsed_ms");
        let caption=if elapsed==(){ctx.t_or("run.done","Done")}else{
            ctx.t_or("run.done_in","Done in {duration}",#{duration:ctx.format_duration(elapsed)})};
        widgets.push(text(caption).with_style(ctx.component_style("note",style().typography("caption").text_color(theme_color("text_muted")))));
        if can(props,"retry"){widgets.push(run_action(ctx,props,"retry",ctx.t_or("run.retry","Retry"),"retry","ghost"));}
    }
    row(widgets).with_key("after").with_style(ctx.component_style("after",style().min_height(theme_length("space.unit").times(5.5)).items_center().gap(theme_length("space.spacing_unit").times(1.5)).flex_wrap()))
}
fn input_tools(ctx,props,running,open,eligible){
    let left=row([mode_view(ctx,props),permission_menu(ctx,props,open,eligible)]).with_key("policy")
        .with_style(style().items_center().min_width(px(0)).gap(theme_length("space.spacing_unit")).flex_wrap());
    let actions=[];let amount=opt(props,"context");
    if amount!=(){actions.push(context_meter(ctx,amount.to_float()));}
    let draft=props.draft;let trimmed=draft;trimmed.trim();
    let can_write=trimmed!=""&&can(props,"change");
    if running{
        actions.push(button::Button(#{key:"queue",text:ctx.t_or("run.queue","Queue"),prefix:glyph(ctx,"queue"),variant:"ghost",size:"xs",
            disabled:!can_write||!can(props,"queue"),on_click:Fn("submit_button").curry("queue",props.run_id,draft)}));
        actions.push(button::Button(#{key:"steer",text:ctx.t_or("run.steer","Steer"),prefix:glyph(ctx,"steer"),variant:"secondary",size:"xs",
            disabled:!can_write||!can(props,"steer"),on_click:Fn("submit_button").curry("steer",props.run_id,draft)}));
        actions.push(run_action(ctx,props,"stop",ctx.t_or("run.stop","Stop run"),"stop","primary"));
    }else{
        actions.push(button::Button(#{key:"send",text:ctx.t_or("run.send","Send"),prefix:glyph(ctx,"send"),variant:"primary",size:"xs",
            disabled:!can_write||!can(props,"send"),on_click:Fn("submit_button").curry("send",props.run_id,draft)})
            .with_style(ctx.component_style("send",style())));
    }
    row([left,row(actions).with_key("actions").with_style(ctx.component_style("actions",style()
        .min_width(px(0)).items_center().gap(theme_length("space.spacing_unit")).flex_wrap()))]).with_key("tools")
        .with_style(ctx.component_style("tools",style().min_width(px(0)).gap(theme_length("space.spacing_unit").times(1.5)).items_center().justify_between().flex_wrap()))
}
fn render_RunControls(ctx,props){
    if utf8_byte_count(props.run_id)>128{throw "RunControls run_id exceeds 128 bytes";}
    let agent=opt(props,"agent_name");if agent==()||agent==""{agent=ctx.t_or("run.agent","Assistant");}
    let running=props.status=="running";
    let eligible=props.mode=="act"&&can(props,"permission_change");
    let open=eligible&&ctx.get_state("permission_open");
    effect("close_policy",eligible,Fn("disallow_menu"),Fn("cleanup"));
    let log_key="run-log:"+props.run_id;
    let log_name=ctx.t_or("run.transcript","Run transcript");
    let log=virtual_collection(#{key:log_key,label:log_name,data:collect_log(ctx,props,agent,running),
        estimated_height:32,height:props.log_height,follow_tail:true,on_follow_change:Fn("follow").curry(props.run_id)},Fn("render_log_row"))
        .on_key_value("home",Fn("oldest").curry(log_key),()).on_key_value("end",Fn("latest").curry(log_key),())
        .accessibility_role("list").accessibility_label(log_name)
        .tab_stop(true).with_style(ctx.component_style("log",style().min_height(px(0)).min_width(px(0))
            .padding_x(theme_length("space.spacing_unit").times(2)).focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))));
    let upper=[log];
    let follow_state=ctx.get_state("following");
    if follow_state!=()&&follow_state.run==props.run_id&&!follow_state.active{
        upper.push(button::Button(#{key:"latest",text:ctx.t_or("run.latest","Jump to latest"),
            variant:"ghost",size:"xs",on_click:Fn("latest").curry(log_key),disabled:props.disabled})
            .with_style(ctx.component_style("jump",style())));
    }
    upper.push(status_actions(ctx,props,running));
    let placeholder=opt(props,"placeholder");
    if placeholder==(){placeholder=if running{ctx.t_or("run.steer_placeholder","Steer or queue a message…")}
        else{ctx.t_or("run.placeholder","Ask the assistant…")};}
    let draft_label=if running{ctx.t_or("run.steer_field","Steer or queue a message")}else{ctx.t_or("run.message","Message")};
    let primary=if running{"steer"}else{"send"};
    let editor=textarea::Textarea(#{key:"draft",label:draft_label,value:props.draft,placeholder:placeholder,editor_ref:element_ref("draft"),
        min_rows:2,max_rows:5,max_length:4000,read_only:!can(props,"change"),disabled:props.disabled,
        on_change:if can(props,"change"){Fn("run_change").curry("change",props.run_id)}else{()},
        submit_key:if can(props,primary)&&can(props,"change"){"enter"}else{()},
        on_submit:if can(props,primary)&&can(props,"change"){Fn("submit").curry(primary,props.run_id)}else{()},
        on_alt_submit:if running&&can(props,"queue")&&can(props,"change"){Fn("submit").curry("queue",props.run_id)}else{()},
        part_styles:#{editor:ctx.component_style("editor",style().border(px(0)).padding(px(0)).radius(px(0))
            .shadows([]).background(rgba(0x00000000)))}});
    let write=column([editor,input_tools(ctx,props,running,open,eligible)]).with_key("write")
        .with_style(ctx.component_style("write",style().min_width(px(0)).padding(theme_length("space.spacing_unit").times(2)).gap(theme_length("space.spacing_unit").times(1.5))
            .radius(theme_radius("lg")).background(theme_color("surface_raised"))));
    let composer=[write];if props.queue.len>0{composer.insert(0,queued_views(ctx,props));}
    upper.push(column(composer).with_key("composer").with_style(ctx.component_style("composer",style().min_width(px(0))
        .padding(theme_length("space.spacing_unit")).gap(theme_length("space.spacing_unit")).radius(theme_radius("lg")).background(theme_color("surface_hover")))));
    column(upper).with_key(props.key).tab_stop(false).accessibility_role("region").accessibility_label(ctx.t_or("run.title","Run controls"))
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(560)).min_width(px(0))
            .padding(theme_length("space.spacing_unit").times(2)).gap(theme_length("space.spacing_unit").times(1.5)).radius(theme_radius("lg")).background(theme_color("surface_raised"))))
        .on_key_value("escape",Fn("stop_key").curry(open,running&&can(props,"stop"),props.run_id),())
}
