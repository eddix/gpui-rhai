/* gpui-rhai
{"id":"components/number_input","export":"NumberInput","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"assets":["icons/minus.svg","icons/plus.svg"],"tokens":["border","control.border","danger","focus_ring","metrics.field_h","metrics.field_pad","metrics.field_radius","surface_raised","text_primary","text_muted","typography.control","radius.sm","space.spacing_unit","space.unit","typography.caption","typography.label"],"environment":["size","density"]}
*/
// NumberInput is a controlled numeric field with a native editor, locale and steppers.
// State: a local draft and transient step feedback; the caller owns the committed number.
// Example: NumberInput(#{key:"budget",label:"Token budget",value:4000,min:0,max:8000,step:100,unit:"tokens"})
define_component(#{metadata:#{
    id:"components/number_input","export":"NumberInput",version:"0.1.0",
    runtime_api:#{min_inclusive:3,max_exclusive:4},dependencies:[],capabilities:#{},
    assets:["icons/minus.svg","icons/plus.svg"],
    tokens:["border","control.border","danger","focus_ring","metrics.field_h","metrics.field_pad","metrics.field_radius","surface_raised","text_primary","text_muted","typography.control","radius.sm","space.spacing_unit","space.unit","typography.caption","typography.label"],
    environment:["size","density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of the numeric field."},
        label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Visible and accessible field name."},
        value:#{schema:#{type:"optional",value:#{type:"number"}},required:true,sensitive:false,doc:"Controlled number. Unit means an empty field."},
        min:#{schema:#{type:"optional",value:#{type:"number"}},required:false,sensitive:false,doc:"Optional inclusive lower bound and origin of the step grid."},
        max:#{schema:#{type:"optional",value:#{type:"number"}},required:false,sensitive:false,doc:"Optional inclusive upper bound."},
        step:#{schema:#{type:"number",min:0.000000000001},required:false,sensitive:false,"default":#{type:"float",value:1.0},doc:"Step size. Steps land on the grid based at min or zero."},
        large_step:#{schema:#{type:"optional",value:#{type:"number",min:0.000000000001}},required:false,sensitive:false,doc:"Page Up and Page Down distance; defaults to ten steps."},
        unit:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Visible unit following the value; not part of its text or numeric payload."},
        prefix:#{schema:#{type:"optional",value:#{type:"node"}},required:false,sensitive:false,doc:"Decorative prefix placed just before the number."},
        placeholder:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Hint drawn within an empty value."},
        hint:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Quiet helper text below the field."},
        error:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Marks the field invalid and replaces the helper text."},
        optional:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Shows a localized optional marker beside the label."},
        size:#{schema:#{type:"optional",value:#{type:"string",allowed:["xs","sm","md","lg"]}},required:false,sensitive:false,doc:"Explicit control size or inherited size."},
        disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Blocks focus and all edits."},
        read_only:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Allows selection and copy without editing or steppers."},
        grouping:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:true},doc:"Whether the formatted value includes grouping marks."},
        fraction_digits:#{schema:#{type:"integer",min:0,max:12},required:false,sensitive:false,"default":#{type:"integer",value:6},doc:"Maximum fractional digits presented after committing edits."},
        minimum_fraction_digits:#{schema:#{type:"integer",min:0,max:12},required:false,sensitive:false,"default":#{type:"integer",value:0},doc:"Minimum visible fractional digits after commit; must not exceed fraction_digits."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called when a committed edit or step proposes a different numeric value."},
    },state:#{fields:#{
        draft:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
        roll:#{schema:#{type:"ui_value"},"default":#{type:"null"}},
    }},events:#{
        change:#{payload:#{type:"optional",value:#{type:"number"}},doc:"A finite numeric proposal or unit if the field was cleared. Not emitted for partial drafts."},
    },slots:#{},parts:["root","label","optional","field","field_invalid","field_read_only","input","placeholder","prefix","unit","steps","step","message","error"],
    effects:["draft_revision"]},
    render:Fn("render_NumberInput")
});

fn NumberInput(props) { render_component("components/number_input",props) }
fn opt(props,key) { if key in props { props[key] } else { () } }
fn clip(model,n) {
    if model.min != () && n < model.min { return model.min; }
    if model.max != () && n > model.max { return model.max; }
    n
}
fn draft_for(ctx,model) {
    let draft=ctx.get_state("draft");
    if draft != () && draft.base == model.value && draft.marks == model.marks { draft.raw } else { () }
}
fn synchronize_draft(ctx,model) {
    if model.blocked||draft_for(ctx,model)==() {ctx.set_state("draft",());}
}
fn release_draft(ctx,model) {}
fn edited(model,ctx,raw) {
    ctx.set_state("draft",#{base:model.value,marks:model.marks,raw:raw});
}
fn propose(model,ctx,value,dir) {
    ctx.set_state("draft",());
    if value != model.value {
        ctx.set_state("roll",if dir==0 {()}else{#{target:value,direction:dir}});
        ctx.emit("change",value);
    }
}
fn committed(model,ctx,raw) {
    if raw==() || model.blocked { return; }
    let entered=raw;
    entered.trim();
    if entered=="" { propose(model,ctx,(),0); return; }
    let number=ctx.parse_number(entered);
    if number==() { ctx.set_state("draft",()); return; }
    commit_number(model,ctx,number);
}
fn commit_number(model,ctx,number) {
    let number=clip(model,number);
    if abs(number.to_float())<=9007199254740991.0 {propose(model,ctx,number,0);}
    else {ctx.set_state("draft",());}
}
fn set_access_value(model,ctx,value) {
    if model.blocked {return;}
    if type_of(value)=="string" {committed(model,ctx,value);}
    else {commit_number(model,ctx,value);}
}
fn blurred(model,ctx,payload) { committed(model,ctx,draft_for(ctx,model)); }
fn cancel_edit(model,ctx,payload) {
    if draft_for(ctx,model)!=() { ctx.set_state("draft",()); return handled(); }
}
fn stepped(model,ctx,kind) {
    if model.blocked {return;}
    let origin=if model.min==(){0.0}else{model.min.to_float()};
    let dir=if kind=="down"||kind=="page_down"||kind=="home"{-1}else{1};
    let raw=draft_for(ctx,model);
    let typed=if raw==(){model.value}else{ctx.parse_number(raw)};
    let base=if typed==(){model.value}else{typed};
    let step=model.step.to_float();
    let next=origin;
    if kind=="home" { next=model.min; }
    else if kind=="end" { next=model.max; }
    else if base!=() {
        let ratio=(base.to_float()-origin)/step;
        if kind=="page_up"||kind=="page_down" {
            let by=if model.large_step==(){10.0}else{model.large_step.to_float()/step};
            let grid=(ratio+dir.to_float()*by).round();
            next=origin+grid*step;
        } else {
            let nearest=ratio.round();
            let grid=if abs(nearest-ratio)<0.000000001 { nearest+dir.to_float() }
                else if dir>0 { ratio.ceiling() } else { ratio.floor() };
            next=origin+grid*step;
        }
        // Preserve the supported 12 decimal places without amplifying large
        // magnitudes through a fixed decimal multiplier.
        if abs(next)<10000.0 {next=(next*1000000000000.0).round()/1000000000000.0;}
    }
    if next==() {return;}
    next=clip(model,next);
    if abs(next.to_float())<=9007199254740991.0 { propose(model,ctx,next,dir); }
    handled()
}
fn native_step(model,kind,ctx,payload) {stepped(model,ctx,kind);}
fn capture_step(model,kind,ctx,payload) { stepped(model,ctx,kind) }
fn press_step(policy,kind,ctx,value) {
    let model=policy;
    model.value=value;
    stepped(model,ctx,kind)
}
fn keep_editor_focus(ctx,payload) { event_response().prevent_default() }
fn focus_editor(ctx,payload) {
    if payload.button=="left" {ctx.focus("number-editor");}
}

fn step_button(ctx,model,kind,icon,name,limit) {
    // Interaction identity includes the bounds, locale, step size and actual
    // forwarded callback. Only the checked controlled value is live data.
    let policy=model;
    policy.value=();
    row([image_source(asset(icon)).with_style(style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4))).accessibility_hidden(true)]).with_key(kind)
        .with_style(ctx.component_style("step",style().width(theme_length("space.unit").times(6.5)).height(theme_length("space.unit").times(6.5)).items_center().justify_center()
            .flex_shrink(false).radius(theme_radius("sm")).text_color(theme_color("text_muted"))
            .hover(style().background(theme_color("surface_raised"))).disabled(style().opacity(0.35))))
        .accessibility_role("button").accessibility_label(name)
        .on_capture("pointer_down",Fn("keep_editor_focus"))
        .on_click_value(Fn("press_step").curry(policy,kind),model.value)
        .repeat_press().tab_stop(false).disabled(model.blocked||limit)
}

fn render_NumberInput(ctx,props) {
    let value=props.value;
    let low=opt(props,"min"); let high=opt(props,"max");
    if low!=() && high!=() && low>high { throw "NumberInput min must not exceed max"; }
    for n in [value,low,high] {if n!=() && abs(n.to_float())>9007199254740991.0 {throw "NumberInput exceeds the exact integer range of a decimal number";}}
    let marks=ctx.number().to_string();
    let blocked=props.disabled||props.read_only;
    let model=#{value:value,min:low,max:high,step:props.step,
        large_step:opt(props,"large_step"),marks:marks,blocked:blocked};
    // Idle fields have no editor draft to synchronize. A large form should
    // not schedule a commit effect merely because each input mounted.
    if ctx.get_state("draft")!=(){
        effect("draft_revision",model,Fn("synchronize_draft"),Fn("release_draft"));
    }
    let draft=if blocked {()}else{draft_for(ctx,model)};
    if props.minimum_fraction_digits>props.fraction_digits {throw "minimum_fraction_digits must not exceed fraction_digits";}
    let shown=if draft!=(){draft}else if value==(){""}else{ctx.format_number(value,
        #{grouping:props.grouping,min_fraction_digits:props.minimum_fraction_digits,max_fraction_digits:props.fraction_digits})};
    let err=opt(props,"error"); let hint=opt(props,"hint");
    let invalid=err!=() && err!="";
    let message=if invalid {err}else{hint};
    let entry_ref="number-editor";
    let editor=gpui_rhai::TextInputPrimitive(#{
        key:props.key,value:shown,typography:"control",intrinsic_width:true,
        placeholder:opt(props,"placeholder"),disabled:props.disabled,read_only:props.read_only,
        placeholder_style:ctx.component_style("placeholder",style().text_color(theme_color("text_muted"))),
        on_change:Fn("edited").curry(model),on_submit:Fn("committed").curry(model),
        on_blur:Fn("blurred").curry(model),on_set_value:Fn("set_access_value").curry(model),
        on_increment:Fn("native_step").curry(model,"up"),on_decrement:Fn("native_step").curry(model,"down"),
    }).with_ref(element_ref(entry_ref))
        .with_style(ctx.component_style("input",style().min_width(px(0)).typography("control")
            .font_feature("tnum",1).opacity(1.0)))
        .accessibility_role("spinbutton").accessibility_label(props.label)
        .accessibility_invalid(invalid).accessibility_read_only(props.read_only)
        .on_key_value("up",Fn("stepped").curry(model),"up")
        .on_key_value("down",Fn("stepped").curry(model),"down")
        .on_key_value("pageup",Fn("stepped").curry(model),"page_up")
        .on_key_value("pagedown",Fn("stepped").curry(model),"page_down")
        .on_key_value("escape",Fn("cancel_edit").curry(model),());
    if !blocked {
        if low!=() {editor=editor.on_capture("key:home",Fn("capture_step").curry(model,"home"));}
        if high!=() {editor=editor.on_capture("key:end",Fn("capture_step").curry(model,"end"));}
    }
    if value!=(){editor=editor.accessibility_value(value);}
    if low!=(){editor=editor.accessibility_value_min(low);}
    if high!=(){editor=editor.accessibility_value_max(high);}
    if message!=()&&message!=""{editor=editor.accessibility_description(message);}
    let roll=ctx.get_state("roll");
    let foreground=row([editor]).with_key("ink").with_style(style().min_width(px(0)));
    if roll!=()&&roll.target==value&&draft==() {
        let ease=cubic_bezier(0.22,1.0,0.36,1.0);
        foreground=foreground
            .motion(motion_transition("translate_y",5.0*roll.direction.to_float(),0.0,#{duration_ms:220,easing:ease}))
            .motion(motion_transition("opacity",0.35,1.0,#{duration_ms:220,easing:ease}))
            .motion(motion_transition("foreground_blur",1.5,0.0,#{duration_ms:220,easing:ease}))
            .motion_replay_key(value.to_string());
    }
    let inner=[];
    let prefix=opt(props,"prefix"); let unit_text=opt(props,"unit");
    if prefix!=(){
        inner.push(row([prefix]).with_key("prefix").with_style(ctx.component_style(
            "prefix",style().text_color(theme_color("text_muted")))).accessibility_hidden(true));
    }
    inner.push(foreground);
    if unit_text!=() {
        inner.push(text(unit_text).with_key("unit").with_style(ctx.component_style("unit",style().text_color(theme_color("text_muted"))
            .max_width(relative(0.4)).min_width(px(0)).whitespace_nowrap().text_ellipsis())));
    }
    let values=row(inner).with_style(style().items_center().gap(theme_length("space.spacing_unit")).min_width(px(0)).flex_grow());
    if !props.disabled {values=values.on("pointer_down",Fn("focus_editor"));}
    let content=[values];
    if !props.read_only {
        content.push(row([
            step_button(ctx,model,"down","app/icons/minus",ctx.t_or("number.decrease","Decrease"),value!=()&&low!=()&&value<=low),
            step_button(ctx,model,"up","app/icons/plus",ctx.t_or("number.increase","Increase"),value!=()&&high!=()&&value>=high),
        ]).with_style(ctx.component_style("steps",style().items_center().gap(theme_length("space.spacing_unit").times(0.5)).flex_shrink(false))));
    }
    let frame_style=style().min_height(theme_length("metrics.field_h")).min_width(px(0))
        .padding_start(theme_length("metrics.field_pad")).padding_end(theme_length("space.spacing_unit").times(1.25)).items_center()
        .gap(theme_length("space.spacing_unit")).radius(theme_length("metrics.field_radius")).border(px(1))
        .border_color(theme_color("border")).background(theme_color("surface_raised"))
        .focus_within(style().border_color(theme_color("control.border")));
    if invalid {frame_style=frame_style.border_color(theme_color("danger")).focus_within(style().border_color(theme_color("danger")));}
    let frame_parts=if invalid {["field","field_invalid"]}
        else if blocked {["field","field_read_only"]}else{["field"]};
    let frame=row(content).with_key("field").with_style(ctx.component_style(frame_parts,frame_style)).disabled(props.disabled);
    let head=[text(props.label).with_style(ctx.component_style("label",style().typography("label"))).accessibility_hidden(true)];
    if props.optional {
        head.push(text(ctx.t_or("common.optional","Optional")).with_key("optional-marker")
            .with_style(ctx.component_style("optional",style().typography("caption").text_color(theme_color("text_muted")))));
    }
    let heading=row(head).with_key("label-row").with_style(style().items_center().gap(theme_length("space.spacing_unit").times(1.5)));
    let result=[heading,frame];
    if message!=()&&message!="" {
        result.push(text(message).with_key(if invalid{"error-message"}else{"hint"})
            .accessibility_hidden(true).with_style(ctx.component_style(
            if invalid{"error"}else{"message"},style().typography("caption")
                .text_color(theme_color(if invalid{"danger"}else{"text_muted"})))));
    }
    let root=column(result).with_key(props.key).with_style(ctx.component_style("root",
        style().gap(theme_length("space.spacing_unit").times(1.75)).min_width(px(0)).disabled(style().opacity(0.55)))).disabled(props.disabled);
    let size=opt(props,"size");
    if size==(){root}else{root.env(#{size:size})}
}
