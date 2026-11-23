/* gpui-rhai
{"id":"components/otp_input","export":"OtpInput","version":"0.1.0","runtime_api":{"min_inclusive":3,"max_exclusive":4},"dependencies":[],"capabilities":{},"assets":["icons/warning.svg"],"tokens":["accent","border","danger","focus_ring","selection","success","surface","surface_raised","text_muted","text_primary","typography.control","space.spacing_unit","space.unit","typography.body","typography.caption","typography.otp_cell","typography.otp_success"],"environment":["size","density"]}
*/
// OtpInput edits one controlled verification code using a single native text field.
// State: error-clear and resend timing only. The caller owns the verification attempt and code.
// Example: OtpInput(#{key:"verification",label:"Verification code",value:"",length:6,separator:true})
define_component(#{metadata:#{
    id:"components/otp_input","export":"OtpInput",version:"0.1.0",runtime_api:#{min_inclusive:3,max_exclusive:4},
    dependencies:[],capabilities:#{},assets:["icons/warning.svg"],
    tokens:["accent","border","danger","focus_ring","selection","success","surface","surface_raised","text_muted","text_primary","typography.control","space.spacing_unit","space.unit","typography.body","typography.caption","typography.otp_cell","typography.otp_success"],
    environment:["size","density"]},
    schema:#{props:#{
        key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable identity of this editor."},
        label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accessible name of the one editor; its cells are decorative."},
        value:#{schema:#{type:"string"},required:true,sensitive:true,doc:"Controlled code, default empty string at the caller. Leading zeroes are retained."},
        length:#{schema:#{type:"integer",min:1,max:24},required:false,sensitive:false,"default":#{type:"integer",value:6},doc:"Number of code cells."},
        separator:#{schema:#{type:"one_of",variants:[#{type:"bool"},#{type:"integer",min:0,max:23}]},required:false,sensitive:false,
            "default":#{type:"bool",value:false},doc:"True divides at the midpoint, or specify the cell index. The divider never enters the value."},
        alphanumeric:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Enables Latin letters alongside digits; committed letters are uppercased."},
        status:#{schema:#{type:"string",allowed:["idle","verifying","error","success"]},required:false,sensitive:false,
            "default":#{type:"string",value:"idle"},doc:"Controlled presentation; verifying keeps focus and blocks edits, success shows the confirmed label."},
        error_message:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Message presented when verification fails."},
        clear_on_error:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:true},
            doc:"Propose an empty code after a short error interval. A replacement code or status withdraws the pending clear."},
        success_label:#{schema:#{type:"optional",value:#{type:"string"}},required:false,sensitive:false,doc:"Text rendered in the success pill; defaults to Verified."},
        align:#{schema:#{type:"string",allowed:["start","center"]},required:false,sensitive:false,"default":#{type:"string",value:"start"},doc:"Align the editor, feedback and resend affordance."},
        disabled:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Disables input and resend."},
        autofocus:#{schema:#{type:"bool"},required:false,sensitive:false,"default":#{type:"bool",value:false},doc:"Focuses the native field on its first mount."},
        resend_seconds:#{schema:#{type:"integer",min:0,max:86400},required:false,sensitive:false,"default":#{type:"integer",value:30},
            doc:"Cooldown while this view is active, initially and after an accepted Resend button activation."},
        on_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called with a new normalized code; store it and pass it back as value."},
        on_complete:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called when an edit produces a full code, never merely because the Host supplied a complete value."},
        on_resend:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,doc:"Called when the resend action is available and activated."},
    },state:#{fields:#{
        left:#{schema:#{type:"integer"},"default":#{type:"integer",value:-1}},
        saw_error:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        retyped:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}},
        clear_snapshot:#{schema:#{type:"optional",value:#{type:"string"}},"default":#{type:"null"},sensitive:true},
    }},events:#{
        change:#{payload:#{type:"string"},doc:"The full normalized code or the empty string on clear."},
        complete:#{payload:#{type:"string"},doc:"A newly entered full-length code."},
        resend:#{payload:#{type:"null"},doc:"A request to send another code."},
    },slots:#{},parts:["root","editor","cell","filled","active","separator","caret","verifying","invalid","invalid_active","error","error_icon","success","success_icon","resend","resend_hint","countdown"],
        effects:["resend_setup","error_session"]},render:Fn("render_OtpInput")
});

fn OtpInput(props){render_component("components/otp_input",props)}
fn opt(props,name){if name in props {props[name]}else{()}}
fn translated(ctx,props,key,name,fallback){
    let provided=opt(props,key);
    if provided==(){ctx.t_or(name,fallback)}else{provided}
}
fn cleanup(ctx,payload){}
fn initialize_resend(ctx,seconds){ctx.set_state("left",seconds);}
fn tick(ctx,payload){
    let left=ctx.get_state("left");
    if left>0 {ctx.set_state("left",left-1);}
}
fn resend(seconds,ctx,payload){
    if ctx.get_state("left")>0 {return handled();}
    ctx.set_state("left",seconds);
    ctx.emit("resend",());
    handled()
}
fn synchronize_error(ctx,model){
    if model.status!="error" {
        ctx.set_state("saw_error",false);
        ctx.set_state("retyped",false);
        ctx.set_state("clear_snapshot",());
        return;
    }
    if !ctx.get_state("saw_error") {
        ctx.set_state("saw_error",true);
        if model.clear && model.value!="" {ctx.set_state("clear_snapshot",model.value);}
    } else if !model.clear || ctx.get_state("clear_snapshot")!=model.value {
        ctx.set_state("clear_snapshot",());
    }
}
fn finish_clear(ctx,payload){
    if ctx.get_state("clear_snapshot")!=(){
        ctx.set_state("clear_snapshot",());
        ctx.emit("change","");
    }
}
fn changed(length,under_error,ctx,value){
    if under_error&&value!="" {ctx.set_state("retyped",true);}
    ctx.emit("change",value);
    if value.len==length {ctx.emit("complete",value);}
}
fn support_icon(ctx,part,id,color){
    image_source(asset(id)).with_style(ctx.component_style(part,
        style().width(theme_length("space.unit").times(4)).height(theme_length("space.unit").times(4)).text_color(color).flex_shrink(false))).accessibility_hidden(true)
}
fn resend_row(ctx,props){
    let left=ctx.get_state("left");
    let left=if left<0 {props.resend_seconds}else{left};
    let labels=[text(ctx.t_or("otp.no_code","Didn't get a code?")).with_key("resend-hint").with_style(ctx.component_style("resend_hint",
        style().typography("caption").text_color(theme_color("text_muted"))))];
    if left>0 {
        let seconds=(left%60).to_string();
        if seconds.len<2 {seconds="0"+seconds;}
        labels.push(text(`Resend in ${left/60}:${seconds}`).with_style(ctx.component_style("countdown",
            style().typography("caption").text_color(theme_color("text_primary")).font_feature("tnum",1))));
    } else {
        let label=ctx.t_or("otp.resend","Resend code");
        labels.push(row([text(label)]).with_key("resend").accessibility_role("button").accessibility_label(label)
            .with_style(ctx.component_style("resend",style().font_weight(500).typography("caption")
                .text_color(theme_color("accent")).items_center().min_height(theme_length("space.unit").times(5.5))))
            .on_click(Fn("resend").curry(props.resend_seconds))
            .disabled(props.disabled||props.status=="verifying"));
    }
    row(labels).with_key("resend-row").with_style(style().items_center().gap(theme_length("space.spacing_unit").times(1.25)).flex_wrap())
}

fn render_OtpInput(ctx,props){
    let parts=[];
    let success=props.status=="success";
    let retry=opt(props,"on_resend")!=();
    if retry {
        effect("resend_setup",props.resend_seconds,Fn("initialize_resend"),Fn("cleanup"));
        if ctx.get_state("left")>0 && !success && !props.disabled {interval("resend_tick",1000,false,Fn("tick"),());}
    }
    let clear_enabled=props.status=="error"&&props.clear_on_error&&!props.disabled;
    effect("error_session",#{status:props.status,value:if clear_enabled {props.value}else{""},clear:clear_enabled},
        Fn("synchronize_error"),Fn("cleanup"));
    let invalid=props.status=="error"&&!ctx.get_state("retyped");
    let clearing=invalid&&props.clear_on_error&&!props.disabled&&ctx.get_state("clear_snapshot")==props.value;
    if clearing {timeout("clear_error",980,false,Fn("finish_clear"),());}
    let error=translated(ctx,props,"error_message","otp.error","That code didn't work. Try again.");
    let title=translated(ctx,props,"success_label","otp.verified","Verified");
    {
        let waiting=props.status=="verifying";
        let separator=if type_of(props.separator)=="bool" {if props.separator {props.length/2}else{0}}else{props.separator};
        let cell=style().width(theme_length("space.unit").times(12)).height(theme_length("space.unit").times(14.16)).gap(theme_length("space.spacing_unit").times(2)).radius(theme_length("space.unit").times(2.88)).typography("otp_cell")
            .border(px(1)).border_color(theme_color("border")).background(theme_color("surface_raised"));
        let common=ctx.component_style("cell",cell);
        let visual_state=if invalid {ctx.component_style("invalid",style().border_color(theme_color("danger")))}
            else if waiting {ctx.component_style("verifying",style().background(theme_color("surface")))}else{style()};
        let ring=style().radius(theme_length("space.unit").times(2.88)).border(px(1.5)).border_color(theme_color("text_primary"));
        let editor=gpui_rhai::TextInputPrimitive(#{
            key:"native-code",value:props.value,typography:"control",code_length:props.length,
            code_alphanumeric:props.alphanumeric,code_separator:separator,
            disabled:props.disabled||success,read_only:waiting||clearing||success,autofocus:props.autofocus,
            code_cell_style:common.merge(visual_state),
            code_filled_style:ctx.component_style("filled",style()).merge(visual_state),
            code_active_style:if invalid {ctx.component_style(["active","invalid_active"],ring.border_color(theme_color("danger")))}
                else {ctx.component_style("active",ring)},
            code_separator_style:ctx.component_style("separator",style().width(px(10)).height(px(2)).text_color(theme_color("border"))),
            code_caret_style:ctx.component_style("caret",style().text_color(theme_color(if invalid{"danger"}else{"text_primary"}))),
            code_success_label:title,
            code_verified:success,code_centered:props.align=="center",
            code_discarding:clearing,
            code_success_style:ctx.component_style("success",style().height(theme_length("space.unit").times(9)).padding_left(theme_length("space.spacing_unit").times(3)).padding_right(theme_length("space.spacing_unit").times(3.75))
                .gap(theme_length("space.spacing_unit").times(1.5)).radius(theme_length("space.unit").times(249.75)).border(px(0)).typography("otp_success").text_color(theme_color("text_primary"))
                .background(theme_color("surface_raised"))),
            code_success_icon_style:ctx.component_style("success_icon",style().width(theme_length("space.unit").times(4.25)).text_color(theme_color("success"))),
            on_change:Fn("changed").curry(props.length,props.status=="error"),
        }).with_style(ctx.component_style("editor",style().min_width(px(0)).max_width(relative(1.0))
            .font_weight(500).font_feature("tnum",1).text_color(theme_color("text_primary")).opacity(1.0)))
            .accessibility_role(if success {"status"}else{"text_field"})
            .accessibility_label(if success {title}else{props.label})
            .accessibility_read_only(waiting||clearing||success).accessibility_invalid(invalid);
        if success {
            editor=editor.motion(motion_transition("primitive_progress",0.0,1.0,#{duration_ms:940,easing:"linear"}))
                .motion_replay_key("verified");
        }
        if waiting {
            editor=editor.accessibility_description(ctx.t_or("otp.verifying","Verifying code"));
            if !props.disabled {
                editor=editor.motion(motion_transition("primitive_progress",0.0,1.0,#{duration_ms:1300,iterations:"infinite"}))
                    .motion_replay_key("verifying");
            }
        }
        if invalid {
            editor=editor.accessibility_description(error)
                .motion(motion_keyframes("translate_x",[
                    #{offset:0.0,value:0.0},#{offset:0.2,value:-6.0},#{offset:0.4,value:5.0},
                    #{offset:0.6,value:-3.0},#{offset:0.8,value:2.0},#{offset:1.0,value:0.0},
                ],#{duration_ms:420})).motion_replay_key("error");
            if clearing {
                // This clock starts with the clear lease, not from a second UI
                // timer. A replacement attempt withdraws both on commit.
                editor=editor.motion(motion_transition("primitive_progress",0.0,1.0,#{duration_ms:980,easing:"linear"}));
            }
        }
        parts.push(editor);
        if invalid {
            parts.push(row([support_icon(ctx,"error_icon","app/icons/warning",theme_color("danger")),text(error)
                .with_style(style().min_width(px(0)))]).with_key("message").accessibility_role("status")
                .with_style(ctx.component_style("error",style().items_center().gap(theme_length("space.spacing_unit").times(1.5)).typography("body"))));
        }
        if retry {
            let resend_view=resend_row(ctx,props);
            if success {
                resend_view=resend_view.with_style(style().invisible()).accessibility_hidden(true).disabled(true);
            }
            parts.push(resend_view);
        }
    }
    column(parts).with_key(props.key).with_style(ctx.component_style("root",
        (if props.align=="center" {style().items_center()}else{style().items_start()})
            .min_width(px(0)).gap(theme_length("space.spacing_unit").times(2.5)).disabled(style().opacity(0.55)))).disabled(props.disabled)
}
