/* gpui-rhai
{"id":"components/stepper_form","export":"StepperForm","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":["components/accordion","components/button","components/sliding_panel"],"capabilities":{},"assets":[],"tokens":["surface_raised","surface_hover","text_primary","text_muted","text.danger","border","focus_ring","accent","radius.sm","radius.lg","spacing.xs","spacing.sm","typography.body","typography.caption","typography.control","space.spacing_unit","space.unit","typography.stepper_title"],"environment":["density"]}
*/
// StepperForm presents a reliable workflow around caller-owned keyed form content.
// Example: StepperForm(#{key:"setup",steps:[#{id:"team",title:"Team",content:team_fields}],on_submit:Fn("save")})
import "components/accordion" as accordion;
import "components/button" as button;
import "components/sliding_panel" as sliding;

define_component(#{
    metadata:#{id:"components/stepper_form","export":"StepperForm",version:"0.1.0",
        runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:["components/accordion","components/button","components/sliding_panel"],
        capabilities:#{},assets:[],
        tokens:["surface_raised","surface_hover","text_primary","text_muted","text.danger","border","focus_ring","accent","radius.sm","radius.lg","spacing.xs","spacing.sm","typography.body","typography.caption","typography.control","space.spacing_unit","space.unit","typography.stepper_title"],
        environment:["density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of the form and retained panes."},
        steps:#{schema:#{type:"array",max_items:16,items:#{type:"object",allow_unknown:false,fields:#{
            id:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Unique stable step ID."},
            title:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Short name in progress and compact headings."},
            heading:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Pane heading, defaults to title."},
            description:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Quiet description below the heading."},
            content:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Caller-owned fields. Keep values and field-validation policy in the application."},
            error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Current validation error that blocks forward navigation. It is shown after an invalid attempt."},
            review:#{schema:#{type:"optional",value:#{type:"array",max_items:32,items:#{type:"object",allow_unknown:false,fields:#{
                key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity within the review."},
                label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Label of the reviewed field."},
                text:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Readable current review value."},
                edit_step:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Earlier step to edit. Continue there returns directly to this review."},
            }}}},required:false,sensitive:false,doc:"Review rows below optional content."},
        }}},required:true,sensitive:false,doc:"Ordered keyed workflow; empty renders no workflow."},
        step:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Controlled step ID; apply an accepted step_change proposal to this prop."},
        initial_step:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Initial and reset step for local navigation; otherwise the first step."},
        status:#{schema:#{type:"string",allowed:["editing","submitting","failed","succeeded"]},"default":#{type:"string",value:"editing"},
            required:false,sensitive:false,doc:"Host-owned submission status. Echo submitting when starting an asynchronous request."},
        submission_error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Message on failed submission. Failed requests may be retried."},
        submit_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Last action label, defaults to Finish."},
        submitting_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Label while waiting for submission."},
        success:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Custom success content; shown only when Host reports succeeded."},
        disabled:#{schema:#{type:"bool"},"default":#{type:"bool",value:false},required:false,sensitive:false,doc:"Disables step navigation and submission."},
        on_step_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Next step proposal with step, from, and reason. The visible step changes only after acceptance."},
        on_invalid:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"A blocked attempt, with step and message. The application can reveal per-field errors in its pane."},
        on_submit:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Emitted once per accepted submit attempt; payload has step and request. Read your application's values here."},
        on_reset:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Request to reset the Host-owned data/status from the success pane."},
    },state:#{fields:#{
        local:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        model:#{schema:#{type:"ui_value"},"default":#{type:"null"},sensitive:true},
        reached:#{schema:#{type:"array",max_items:16,items:#{type:"string"}},"default":#{type:"array",value:[]}},
        returning:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        pending:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        attempted:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        attempt:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
        focused_attempt:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
        request:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
        inflight:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        reset_pending:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        shown:#{schema:#{type:"string"},"default":#{type:"string",value:""}},
        direction:#{schema:#{type:"integer"},"default":#{type:"integer",value:0}},
    }},events:#{
        step_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
            step:#{schema:#{type:"string"},required:true,sensitive:false},from:#{schema:#{type:"string"},required:true,sensitive:false},
            reason:#{schema:#{type:"string",allowed:["next","back","progress","edit","return"]},required:true,sensitive:false},
        }},doc:"Navigation proposal. Writing step back is the Host's acceptance."},
        invalid:#{payload:#{type:"object",allow_unknown:false,fields:#{
            step:#{schema:#{type:"string"},required:true,sensitive:false},message:#{schema:#{type:"string"},required:true,sensitive:false},
        }},doc:"Forward navigation was rejected by the current step's validation."},
        submit:#{payload:#{type:"object",allow_unknown:false,fields:#{
            step:#{schema:#{type:"string"},required:true,sensitive:false},request:#{schema:#{type:"integer"},required:true,sensitive:false},
        }},doc:"A request to submit caller-owned data. Request is monotonic within this mounted form; the Host owns asynchronous results."},
        reset:#{payload:#{type:"null"},doc:"Reset requested from success. The Host must restore the form values, status, and any controlled step."},
    },slots:#{},parts:["root","header","progress","step","step_current","step_done","step_future","marker","marker_current",
        "marker_done","check","step_name","bar","bar_active","compact","card","pane","heading","description","body",
        "validation","review","review_row","review_label","review_value","edit","footer","back","primary","primary_busy","submit_error","success","success_mark"],
        effects:["committed"]},
    render:Fn("render_StepperForm")
});

fn StepperForm(props){render_component("components/stepper_form",props)}
fn opt(map,key){if key in map{map[key]}else{()}}
fn value_or(map,key,fallback){let value=opt(map,key);if value==(){fallback}else{value}}
fn pos(ids,id){for i in 0..ids.len{if ids[i]==id{return i;}}-1}
fn page_key(id){`step:${id}`}
fn scope_key(i){`pane-${i}`}
fn live(ctx){ctx.get_state("model")}
fn locked(ctx,model){model==()||model.disabled||model.status=="submitting"||model.status=="succeeded"||ctx.get_state("inflight")}
fn farthest(ids,values,at){let last=at;for id in values{last=max(last,pos(ids,id));}last}
fn committed(ctx,data){
    let model=data[0];let direction=data[1];let pending=data[2];let attempt=data[3];let reset_pending=data[5];
    let prior=live(ctx);
    ctx.set_state("model",model);
    let reached=[];
    if reset_pending&&model.status=="editing"{
        ctx.set_state("attempted","");ctx.set_state("returning","");
        if !model.controlled{ctx.set_state("local",model.initial);}
    } else {
        for id in ctx.get_state("reached"){if id in model.ids{reached.push(id);}}
    }
    if reset_pending{ctx.set_state("reset_pending",false);}
    if !(model.step in reached){reached.push(model.step);}
    ctx.set_state("reached",reached);
    if pending!=(){
        if model.step==pending.target{ctx.set_state("returning",pending.returning);}
        ctx.set_state("pending",());
    }
    if model.status!="submitting"{ctx.set_state("inflight",false);}
    let shown=ctx.get_state("shown");
    if attempt!=ctx.get_state("focused_attempt"){
        ctx.set_state("focused_attempt",attempt);
        if ctx.get_state("attempted")==model.step&&model.status!="succeeded"{ctx.focus_descendant(model.scope,true);}
    }else if prior!=()&&model.page!=shown&&!model.disabled{
        ctx.focus_descendant(model.scope,false);
    }
    ctx.set_state("shown",model.page);ctx.set_state("direction",direction);
}
fn cleanup(ctx,data){}
fn check_step(ctx,model){
    if model.error==""{ctx.set_state("attempted","");return true;}
    ctx.set_state("attempted",model.step);ctx.set_state("attempt",ctx.get_state("attempt")+1);
    ctx.emit("invalid",#{step:model.step,message:model.error});
    false
}
fn request_step(ctx,model,target,reason,returning){
    if locked(ctx,model)||!model.can_navigate||target<0||target>=model.ids.len||target==model.index{return handled();}
    let to=model.ids[target];
    if !model.controlled{ctx.set_state("local",to);}
    ctx.set_state("pending",#{target:to,returning:returning});
    ctx.emit("step_change",#{step:to,from:model.step,reason:reason});
    handled()
}
fn advance(ctx,payload){
    let model=live(ctx);
    if locked(ctx,model)||!check_step(ctx,model){return handled();}
    if model.index+1<model.ids.len{
        let returning=pos(model.ids,ctx.get_state("returning"));
        if returning>model.index{return request_step(ctx,model,returning,"return","");}
        return request_step(ctx,model,model.index+1,"next","");
    }
    if model.can_submit{
        let request=ctx.get_state("request")+1;
        ctx.set_state("request",request);ctx.set_state("inflight",true);
        ctx.emit("submit",#{step:model.step,request:request});
    }
    handled()
}
fn back(ctx,payload){
    let model=live(ctx);
    if locked(ctx,model){return handled();}
    request_step(ctx,model,model.index-1,"back","")
}
fn jump(ctx,id){
    let model=live(ctx);
    if locked(ctx,model){return handled();}
    let i=pos(model.ids,id);
    if i<0||i>farthest(model.ids,ctx.get_state("reached"),model.index){return handled();}
    if i>model.index&&!check_step(ctx,model){return handled();}
    request_step(ctx,model,i,"progress",if i<model.index{model.step}else{""})
}
fn edit(id,ctx,payload){
    let model=live(ctx);
    if locked(ctx,model){return handled();}
    let to=pos(model.ids,id);
    if to<0||to>=model.index{return handled();}
    request_step(ctx,model,to,"edit",model.step)
}
fn reset(ctx,payload){
    let model=live(ctx);
    if model!=()&&model.status=="succeeded"&&model.can_reset&&!model.disabled{
        ctx.set_state("reset_pending",true);
        ctx.emit("reset",());
    }
    handled()
}
fn check_glyph(ctx){
    let line=[path_move(5.5,12.5),path_line(9.5,16.5),path_line(18.5,8)];
    canvas(canvas_scene([canvas_stroke_path("check",line,2.2,current_color()).stroke_cap("round").stroke_join("round")]).view_box(0,0,24,24))
        .with_key("check").accessibility_hidden(true)
        .with_style(ctx.component_style("check",style().width(px(14)).height(px(14))))
        .motion(motion_transition("path_progress",0.0,1.0,#{duration_ms:360,easing:cubic_bezier(0.22,1.0,0.36,1.0),intent:"feedback"}))
}
fn step_marker(ctx,model,step,i,furthest,clean){
    let state=if model.status=="succeeded"||(i<furthest&&i!=model.index){"done"}else if i==model.index{"current"}else{"future"};
    let mark=if state=="done"{check_glyph(ctx)}else{text(ctx.format_number(i+1)).with_key("digit")};
    let mark_part=if state=="current"{["marker","marker_current"]}else if state=="done"{["marker","marker_done"]}else{["marker"]};
    let base=ctx.component_style(mark_part,style().width(theme_length("space.unit").times(6)).height(theme_length("space.unit").times(6)).items_center().justify_center()
        .radius(theme_length("space.unit").times(249.75)).text_color(theme_color("text_muted")).typography("caption").flex_shrink(false));
    let stripe=row([]).with_key("bar").with_style(ctx.component_style(if state=="future"{["bar"]}else{["bar","bar_active"]},
        style().height(px(2)).width(relative(1.0)).background(theme_color(if state=="future"{"border"}else{"accent"}))
            .radius(px(1)))).accessibility_hidden(true);
    let name=text(step.title).with_key("name").with_style(ctx.component_style("step_name",style().typography("caption")
        .min_width(px(0)).whitespace_nowrap().text_ellipsis()));
    let control=row([row([mark]).with_key("mark").with_style(base).accessibility_hidden(true),name])
        .with_key("control").with_style(style().items_center().min_width(px(0)).gap(theme_spacing("xs")));
    let can_jump=model.can_navigate&&!model.disabled&&model.status!="submitting"&&model.status!="succeeded"&&i!=model.index&&i<=furthest;
    let children=if clean{[stripe,control]}else{[control]};
    let display=column(children).with_key(step.id).with_style(ctx.component_style(["step",`step_${state}`],
        style().min_width(px(0)).flex_grow().flex_basis(px(0)).gap(theme_length("space.spacing_unit").times(1.25)).padding(theme_length("space.spacing_unit").times(0.75)).radius(theme_radius("sm"))
            .focus_ring(px(2),offset_px(-2),theme_color("focus_ring"))))
        .accessibility_role(if can_jump{"button"}else{"listitem"}).accessibility_label(step.title);
    if state=="current"{display=display.accessibility_current("step");}
    if state=="done"{display=display.accessibility_description(ctx.t_or("stepper.done","Completed"));}
    if can_jump{display=display.on_click_value(Fn("jump"),step.id);}else{display=display.tab_stop(false);}
    display
}
fn progress_view(ctx,props,model,compact){
    let furthest=farthest(model.ids,ctx.get_state("reached"),model.index);
    let theme=ctx.theme_variant();let clean=theme!=()&&theme.family=="Clean";
    let list=[];for i in 0..props.steps.len{list.push(step_marker(ctx,model,props.steps[i],i,furthest,clean));}
    let wide=row(list).with_key("steps").retained_page(!compact).with_style(ctx.component_style("progress",
        style().gap(theme_spacing("xs")).min_width(px(0)).width(relative(1.0))))
        .accessibility_role("list").accessibility_label(ctx.t_or("stepper.progress","Progress"));
    let sentence=if model.status=="succeeded"{ctx.t_or("stepper.completed","All {count} steps done",#{count:ctx.format_number(list.len)})}
        else{ctx.t_or("stepper.position","Step {step} of {count} · {title}",
            #{step:ctx.format_number(model.index+1),count:ctx.format_number(list.len),title:props.steps[model.index].title})};
    let narrow=text(sentence).with_key("compact").retained_page(compact).accessibility_role("status")
        .with_style(ctx.component_style("compact",style().typography("caption").text_color(theme_color("text_muted"))));
    column([wide,narrow]).with_key("header").with_style(ctx.component_style("header",style().min_width(px(0)).width(relative(1.0))))
}
fn review_view(ctx,records,model,index){
    let entries=[];let seen=#{};
    for item in records{
        if item.key==""||item.key in seen{throw "StepperForm review keys must be unique and nonempty";}
        seen[item.key]=true;
        let edit_id=opt(item,"edit_step");
        let at=pos(model.ids,edit_id);
        let label=text(item.label).with_style(ctx.component_style("review_label",
            style().min_width(px(0)).width(relative(0.30)).typography("caption").text_color(theme_color("text_muted"))));
        let result=text(item.text).with_style(ctx.component_style("review_value",style().min_width(px(0)).flex_grow().typography("control")));
        let cells=[label,result];
        if at>=0&&at<index&&model.can_navigate{
            cells.push(button::Button(#{key:`edit:${item.key}`,text:ctx.t_or("stepper.edit","Edit"),variant:"ghost",size:"sm",
                disabled:model.disabled||model.status=="submitting",on_click:Fn("edit").curry(edit_id)})
                .accessibility_label(ctx.t_or("stepper.edit_item","Edit {field}",#{field:item.label}))
                .with_style(ctx.component_style("edit",style().flex_shrink(false))));
        }
        entries.push(row(cells).with_key(item.key).with_style(ctx.component_style("review_row",style().min_width(px(0))
            .gap(theme_spacing("sm")).items_center().padding_y(theme_spacing("xs")).border_bottom(px(1)).border_color(theme_color("border")))));
    }
    column(entries).with_key("review").with_style(ctx.component_style("review",style().min_width(px(0))));
}
fn editor_pane(ctx,step,model,i,active){
    let title=value_or(step,"heading",step.title);
    let children=[text(title).with_key("heading").accessibility_role("heading").accessibility_level(2).tab_stop(false)
            .with_style(ctx.component_style("heading",style().typography("stepper_title").font_weight(600)))];
    let description=opt(step,"description");
    if description!=(){children.push(text(description).with_key("description")
        .with_style(ctx.component_style("description",style().typography("body").text_color(theme_color("text_muted")))));}
    let contents=[];
    let content=opt(step,"content");if content!=(){contents.push(content);}
    let records=opt(step,"review");if records!=(){contents.push(review_view(ctx,records,model,i));}
    children.push(column(contents).with_key("body").with_style(ctx.component_style("body",style().min_width(px(0)).gap(theme_spacing("sm")))));
    if active&&ctx.get_state("attempted")==model.step&&model.error!=""{
        children.push(text(model.error).with_key("validation").accessibility_role("alert")
            .with_style(ctx.component_style("validation",style().typography("caption").text_color(theme_color("text.danger")))));
    }
    let motion=column(children).with_key("feedback").with_style(style().min_width(px(0)).gap(theme_spacing("xs")));
    if active&&ctx.get_state("attempted")==model.step&&model.error!=""{
        motion=motion.motion(motion_keyframes("translate_x",[
            #{offset:0.0,value:0.0},#{offset:0.20,value:-4.0},#{offset:0.39,value:4.0},
            #{offset:0.68,value:-2.0},#{offset:1.0,value:0.0}
        ],#{duration_ms:340,intent:"feedback"})).motion_replay_key(`invalid:${ctx.get_state("attempt")}`);
    }
    column([motion]).with_key(page_key(step.id)).with_ref(element_ref(scope_key(i))).retained_page(active)
        .with_style(ctx.component_style("pane",style().min_width(px(0)).padding(theme_spacing("sm"))))
        .accessibility_role("region").accessibility_label(title).tab_stop(false)
        .on_key_value("enter",Fn("advance"),());
}
fn success_pane(ctx,props,model){
    let custom=opt(props,"success");let children=[];
    if custom==(){
        children.push(row([check_glyph(ctx)]).with_key("success-mark").accessibility_hidden(true)
            .with_style(ctx.component_style("success_mark",style().width(theme_length("space.unit").times(9)).height(theme_length("space.unit").times(9))
                .radius(theme_length("space.unit").times(249.75)).items_center().justify_center().background(theme_color("surface_hover")).text_color(theme_color("accent")))));
        children.push(text(ctx.t_or("stepper.success","All set")).with_key("heading")
            .accessibility_role("heading").accessibility_level(2).tab_stop(false)
            .with_style(ctx.component_style("heading",style().typography("control").font_weight(600))));
        children.push(text(ctx.t_or("stepper.saved","Everything is saved.")).with_key("description")
            .with_style(ctx.component_style("description",style().typography("body").text_color(theme_color("text_muted")))));
    }else{children.push(custom);}
    if model.can_reset{children.push(button::Button(#{key:"reset",text:ctx.t_or("stepper.reset","Start over"),variant:"secondary",size:"sm",
        disabled:model.disabled,on_click:Fn("reset")}));}
    column(children).with_key("completed").with_ref(element_ref("completed")).retained_page(model.status=="succeeded")
        .with_style(ctx.component_style("success",style().min_width(px(0)).gap(theme_spacing("sm"))
            .items_center().padding(theme_spacing("sm")))).tab_stop(false)
}
fn footer(ctx,props,model){
    let pending=model.status=="submitting";let last=model.index+1==model.ids.len;
    let label=value_or(props,"submit_label",ctx.t_or("stepper.finish","Finish"));
    if pending{label=value_or(props,"submitting_label",ctx.t_or("stepper.submitting","Submitting…"));}
    else if !last{
        let target=pos(model.ids,ctx.get_state("returning"));
        label=if target>model.index{ctx.t_or("stepper.return","Back to {step}",#{step:props.steps[target].title})}
            else{ctx.t_or("stepper.continue","Continue")};
    }
    let actions=[];
    if model.index>0{
        let path=if ctx.text_direction()=="rtl"{"m9.5 6 6 6-6 6"}else{"m14.5 6-6 6 6 6"};
        let arrow=svg(`<svg viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='1.65' stroke-linecap='round' stroke-linejoin='round'><path d='${path}'/></svg>`)
            .with_style(style().width(px(14)).height(px(14))).accessibility_hidden(true);
        actions.push(button::Button(#{key:"back",text:ctx.t_or("stepper.back","Back"),variant:"ghost",size:"sm",
            prefix:arrow,
            disabled:model.disabled||pending||!model.can_navigate,on_click:Fn("back")})
            .with_style(ctx.component_style("back",style().flex_shrink(false))));
    }
    let error=opt(props,"submission_error");
    if model.status=="failed"&&last{
        if error==(){error=ctx.t_or("stepper.failed","Submission failed. Try again.");}
        actions.push(text(error).with_key("submit-error").accessibility_role("alert")
            .with_style(ctx.component_style("submit_error",style().min_width(px(0)).flex_grow().typography("caption")
                .text_color(theme_color("text.danger")))));
    }
    let primary=ctx.component_style(if pending{["primary","primary_busy"]}else{["primary"]},
        style().min_width(px(0)).margin_start(auto()),
        if pending{style().disabled(style().opacity(0.7))}else{style()});
    actions.push(button::Button(#{key:"advance",text:label,variant:"primary",size:"sm",
        disabled:model.disabled||(last&&!model.can_submit)||(!last&&!model.can_navigate),
        prefix:if pending{row([])}else{()},loading:pending,loading_text:label,
        part_styles:#{root:primary,label:if pending{style().opacity(1.0)}else{style()}},on_click:Fn("advance")}));
    let shown=model.status!="succeeded";
    let row=row(actions).with_key("actions").with_style(ctx.component_style("footer",style().min_width(px(0))
        .items_center().gap(theme_spacing("sm")).padding(theme_spacing("sm")).flex_wrap()
        .border_top(px(1)).border_color(theme_color("border")))).disabled(!shown).accessibility_hidden(!shown);
    accordion::disclosure_panel(ctx,"footer",shown,(),row)
}
fn render_StepperForm(ctx,props){
    let ids=[];for step in props.steps{
        if step.id==""||step.id in ids{throw "StepperForm step IDs must be unique and nonempty";}
        ids.push(step.id);
    }
    if ids.len==0{return fragment([]);}
    let controlled=opt(props,"step")!=();
    let initial=value_or(props,"initial_step",ids[0]);if !(initial in ids){initial=ids[0];}
    let reset_pending=ctx.get_state("reset_pending");
    let current=if controlled{props.step}else if reset_pending&&props.status=="editing"{initial}else{ctx.get_state("local")};
    if !(current in ids){if controlled{throw "StepperForm controlled step does not exist";}current=initial;}
    let index=pos(ids,current);let step=props.steps[index];
    let page=if props.status=="succeeded"{"completed"}else{page_key(current)};
    let scope=if props.status=="succeeded"{"completed"}else{scope_key(index)};
    let model=#{ids:ids,initial:initial,step:current,index:index,page:page,scope:scope,status:props.status,
        disabled:props.disabled,controlled:controlled,error:value_or(step,"error",""),
        can_navigate:!controlled||opt(props,"on_step_change")!=(),can_submit:opt(props,"on_submit")!=(),can_reset:opt(props,"on_reset")!=()};
    let order=[];for id in ids{order.push(page_key(id));}order.push("completed");
    let prior=ctx.get_state("shown");
    let direction=if prior==page{ctx.get_state("direction")}else{sliding::slide_direction(order,prior,page,false)};
    effect("committed",[model,direction,ctx.get_state("pending"),ctx.get_state("attempt"),ctx.get_state("request"),reset_pending],Fn("committed"),Fn("cleanup"));
    let panes=[];
    for i in 0..props.steps.len{panes.push(editor_pane(ctx,props.steps[i],model,i,page==page_key(props.steps[i].id)));}
    panes.push(success_pane(ctx,props,model));
    let side=if ctx.text_direction()=="rtl"{-1.0}else{1.0};
    let timing=#{duration_ms:360,easing:cubic_bezier(0.22,1.0,0.36,1.0),intent:"feedback"};
    let active=if props.status=="succeeded"{panes.len-1}else{index};
    if direction!=0{
        panes[active]=panes[active].motion(motion_transition("translate_x",side*direction*ctx.motion_distance("moderate"),0.0,timing))
            .motion(motion_transition("opacity",0.0,1.0,timing)).motion_replay_key(page);
    }
    let view=accordion::disclosure_panel(ctx,"viewport",true,(),column(panes).with_key("panes").with_style(style().min_width(px(0))));
    let reference=element_ref("stepper-root");let width=ctx.element_width_class(reference,[420]);
    let compact=if width==(){ctx.viewport_class()=="compact"}else{width==0};
    let body=column([view,footer(ctx,props,model)]).with_key("card")
        .with_style(ctx.component_style("card",style().min_width(px(0)).radius(theme_radius("lg")).background(theme_color("surface_raised"))));
    let root=column([progress_view(ctx,props,model,compact),body]).with_key(props.key).with_ref(reference)
        .with_style(ctx.component_style("root",style().width(relative(1.0)).max_width(px(560)).min_width(px(0))
            .gap(theme_spacing("xs")).text_color(theme_color("text_primary")))).disabled(props.disabled);
    let theme=ctx.theme_variant();
    // The tray progress has its own gutter; it does not share the card's text inset.
    if theme!=()&&theme.family=="Modern"{root.audit_allow(["text-edge-misaligned"])}else{root}
}
