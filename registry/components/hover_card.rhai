/* gpui-rhai
{
  "id":"components/hover_card", "export":"HoverCard", "version":"0.4.3",
  "runtime_api":{"min_inclusive":3,"max_exclusive":4},
  "dependencies":[], "capabilities":{},
  "tokens":["surface_raised","text_primary","border","typography.hover_card","space.spacing_unit","space.unit"],
  "environment":["density"]
}
*/
// A non-modal, interactive preview that opens on intentional hover or keyboard focus.
// Example: HoverCard(#{key:"service",label:"Service details",trigger:text("Checkout"),content:text("Owned by Platform")})

define_component(#{
    metadata:#{id:"components/hover_card","export":"HoverCard",version:"0.4.3",
        runtime_api:#{min_inclusive:3,max_exclusive:4}, dependencies:[],capabilities:#{},
        tokens:["surface_raised","text_primary","border","typography.hover_card","space.spacing_unit","space.unit"],environment:["density"]},
    schema:#{
        props:#{
            key:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Stable preview anchor identity."},
            label:#{schema:#{type:"string"},required:true,sensitive:false,doc:"Accessible name of the preview content."},
            trigger:#{schema:#{type:"node"},required:true,sensitive:false,
                doc:"Usually a labeled button or link. Its own press action is kept; a press dismisses rather than opens the preview."},
            content:#{schema:#{type:"node"},required:true,sensitive:false,
                doc:"The preview body. It may contain text, links, and keyboard-focusable controls."},
            open:#{schema:#{type:"optional",value:#{type:"bool"}},required:false,sensitive:false,
                doc:"Controlled visibility. Omit it to let the component own local visibility."},
            on_open_change:#{schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,
                doc:"Receives a bool proposal. A controlled caller accepts by passing it back in open."},
            placement:#{schema:#{type:"string",allowed:["bottom","top","left","right"]},
                required:false,sensitive:false,"default":#{type:"string",value:"bottom"},
                doc:"Preferred side; placement flips when there is no room."},
            align:#{schema:#{type:"string",allowed:["start","center","end"]},
                required:false,sensitive:false,"default":#{type:"string",value:"start"},
                doc:"Cross-axis alignment. On horizontal edges start/end follow the reading direction."},
            gap:#{schema:#{type:"number",min:0.0,max:64.0},required:false,sensitive:false,
                "default":#{type:"float",value:8.0},doc:"Space between the anchor and preview in logical pixels."},
            width:#{schema:#{type:"optional",value:#{type:"length"}},required:false,sensitive:false,
                doc:"Optional requested width. By default the card fits its content up to 320 logical pixels at reading scale 100. The viewport still caps it."},
            show_delay_ms:#{schema:#{type:"integer",min:0,max:60000},required:false,sensitive:false,
                "default":#{type:"integer",value:600},doc:"Dwell delay for pointer hover or keyboard focus, in milliseconds."},
            hide_delay_ms:#{schema:#{type:"integer",min:0,max:60000},required:false,sensitive:false,
                "default":#{type:"integer",value:300},doc:"Hide delay after leaving the trigger, panel and crossing corridor."},
            disabled:#{schema:#{type:"bool"},required:false,sensitive:false,
                "default":#{type:"bool",value:false},doc:"Renders the trigger alone without starting or showing a preview."},
        },
        slots:#{
            trigger:#{required:true,multiple:false,doc:"Visible focusable anchor."},
            content:#{required:true,multiple:false,doc:"The interactive overlay body."},
        },
        state:#{fields:#{shown:#{schema:#{type:"bool"},"default":#{type:"bool",value:false}}}},
        events:#{open_change:#{payload:#{type:"bool"},doc:"Requested visibility after native hover, focus or dismissal."}},
        parts:["root","trigger","content"],
    },
    render:Fn("render_HoverCard")
});

fn HoverCard(props) { render_component("components/hover_card",props) }
fn request_open(controlled,ctx,value) {
    if !controlled {ctx.set_state("shown",value);}
    ctx.emit("open_change",value);
}
fn render_HoverCard(ctx,props) {
    let trigger=props.trigger.with_style(ctx.component_style("trigger",style()));
    if props.disabled {return trigger;}
    let controlled="open" in props&&props.open!=();
    let open=if controlled {props.open}else{ctx.get_state("shown")};
    let base=style().max_width(px(320)).min_width(px(0)).overflow_y_scroll().padding(theme_length("space.spacing_unit").times(4))
        .radius(theme_length("space.unit").times(4)).background(theme_color("surface_raised"))
        .typography("hover_card").text_color(theme_color("text_primary"));
    let style=ctx.component_style("content",base);
    let style=if "width" in props&&props.width!=(){style.width(props.width).max_width(relative(1.0))}
        else{style};
    // Opening is a new presentation of the same content owner, not a new
    // component instance. Decorative motion follows the host's preference.
    let timing=#{duration_ms:200,easing:cubic_bezier(0.22,1.0,0.36,1.0)};
    let content=column([props.content]).with_key("preview")
        .with_style(style).accessibility_role("group").accessibility_label(props.label);
    overlay(trigger,content,#{id:props.key,kind:"hover_card",open:open,
        placement:props.placement,align:props.align,gap:props.gap,show_delay_ms:props.show_delay_ms,
        hide_delay_ms:props.hide_delay_ms,activate_on_trigger:false,modal:false,
        dismiss_on_escape:true,dismiss_on_outside:true}).with_key(props.key)
        .with_style(ctx.component_style("root",style()))
        .on_open_change(Fn("request_open").curry(controlled))
        .motion(motion_transition("overlay_reveal",0.0,1.0,timing))
        .motion_on_change(if open {"open"}else{"closed"})
}
