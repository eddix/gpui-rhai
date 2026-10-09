/* gpui-rhai
{
  "id": "components/pan_zoom",
  "export": "PanZoom",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["border", "surface", "focus_ring"],
  "environment": []
}
*/
// Controlled Canvas viewport transform with native pan and pointer-anchored zoom.
// State: transform is caller-owned; native signals contain only current preview.
// Example: PanZoom(#{ key:"map", label:"Map", transform:viewport, content:canvas_node, on_transform_change:Fn("changed") })

define_component(#{
    metadata: #{ id:"components/pan_zoom", "export":"PanZoom", version: "0.2.0",
        runtime_api:#{min_inclusive: 3,max_exclusive: 4}, dependencies:[], capabilities:#{},
        tokens: ["border", "surface", "focus_ring"],
        environment: [] },
    schema: #{ props: #{
        key: #{ schema:#{type:"string"}, required:true, sensitive:false },
        label: #{ schema:#{type:"string"}, required:true, sensitive:false,
            doc:"Accessible name of the viewport region." },
        transform: #{ schema:#{type:"object",allow_unknown:false,fields:#{
            x:#{schema:#{type:"number"},required:true,sensitive:false,
                doc:"Horizontal offset of the content, in logical pixels."},
            y:#{schema:#{type:"number"},required:true,sensitive:false,
                doc:"Vertical offset of the content, in logical pixels."},
            scale:#{schema:#{type:"number",exclusive_min:0.0},required:true,sensitive:false,
                doc:"Zoom factor about the viewport centre, 1 for natural size; within `min_scale` and `max_scale`."},
        }}, required:true, sensitive:false,
            doc:"Content offset and zoom; the caller stores the `transform_change` payload and passes it back." },
        min_scale: #{ schema:#{type:"number",exclusive_min:0.0},required:false,sensitive:false,
            "default":#{type:"float",value:0.25},
            doc:"Smallest zoom factor a wheel or key zoom may propose." },
        max_scale: #{ schema:#{type:"number",exclusive_min:0.0},required:false,sensitive:false,
            "default":#{type:"float",value:8.0},
            doc:"Largest zoom factor a wheel or key zoom may propose; at least `min_scale`." },
        axes: #{ schema:#{type:"string",allowed:["both","horizontal","vertical"]},required:false,
            sensitive:false,"default":#{type:"string",value:"both"},
            doc:"Axes that dragging and arrow keys pan along; zooming is not limited by it." },
        wheel_zoom: #{ schema:#{type:"string",allowed:["off","modifier","always"]},required:false,
            sensitive:false,"default":#{type:"string",value:"modifier"},
            doc:"When the wheel zooms about the pointer: never, only with Ctrl or Cmd held, or always." },
        pan_button: #{ schema:#{type:"string",allowed:["left","middle"]},required:false,
            sensitive:false,"default":#{type:"string",value:"left"},
            doc:"Mouse button that pans when dragged." },
        threshold: #{ schema:#{type:"number",min:0.0,max:64.0},required:false,sensitive:false,
            "default":#{type:"float",value:4.0},
            doc:"Pointer travel in logical pixels before a press becomes a pan." },
        keyboard_pan_step: #{ schema:#{type:"number",min:0.1,max:512.0},required:false,sensitive:false,
            "default":#{type:"float",value:16.0},
            doc:"Logical pixels one arrow-key press pans; Shift pans four times as far." },
        keyboard_zoom_factor: #{ schema:#{type:"number",min:1.001,max:4.0},required:false,sensitive:false,
            "default":#{type:"float",value:1.2},
            doc:"Zoom multiplier of one `+` or `-` press, about the viewport centre; `0` resets to scale 1 at the origin." },
        content: #{ schema:#{type:"node"},required:true,sensitive:false,
            doc:"Node that is panned and zoomed; it fills the viewport." },
        disabled: #{ schema:#{type:"bool"},required:false,sensitive:false,
            "default":#{type:"bool",value:false},
            doc:"Ignores pointer, wheel and keyboard input and removes the viewport from the tab order." },
        on_transform_change: #{ schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false,
            doc:"Called with the proposed `{x, y, scale}` when a pan ends, a wheel zoom settles or a key pans or zooms." },
    }, state:#{fields:#{}}, events:#{ transform_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
        x:#{schema:#{type:"number"},required:true,sensitive:false,
            doc:"Proposed horizontal offset, in logical pixels."},
        y:#{schema:#{type:"number"},required:true,sensitive:false,
            doc:"Proposed vertical offset, in logical pixels."},
        scale:#{schema:#{type:"number",exclusive_min:0.0},required:true,sensitive:false,
            doc:"Proposed zoom factor."},
    }},
        doc:"Emitted when a pan ends, a wheel zoom settles or a key pans or zooms; the payload is the next `transform`."} },
        slots:#{content:#{required:true,multiple:false,doc:"The panned and zoomed content."}},
        parts:["root","content","interaction"] },
    render:Fn("render_PanZoom")
});

fn PanZoom(props) { render_component("components/pan_zoom", props) }
fn pan_zoom_commit(ctx, value) { ctx.emit("transform_change", value); }

fn render_PanZoom(ctx, props) {
    if props.max_scale < props.min_scale { throw "PanZoom max_scale must be at least min_scale"; }
    if props.transform.scale < props.min_scale || props.transform.scale > props.max_scale {
        throw "PanZoom transform.scale must be within min_scale and max_scale";
    }
    let viewport_key = `${props.key}-viewport`;
    let viewport_ref = element_ref(viewport_key);
    let content_ref = element_ref(`${props.key}-content`);
    let x_signal = signal("pan-zoom-x", props.transform.x * 1.0);
    let y_signal = signal("pan-zoom-y", props.transform.y * 1.0);
    let scale_x_signal = signal("pan-zoom-scale-x", props.transform.scale * 1.0);
    let scale_y_signal = signal("pan-zoom-scale-y", props.transform.scale * 1.0);
    let wheel_generation_signal = signal("pan-zoom-wheel-generation", 0);
    let wheel_active_signal = signal("pan-zoom-wheel-active", false);
    let wheel_pending_signal = signal("pan-zoom-wheel-pending", false);
    let source_token = `${props.transform.x}|${props.transform.y}|${props.transform.scale}|${props.min_scale}|${props.max_scale}|${props.axes}|${props.wheel_zoom}|${props.pan_button}|${props.disabled}`;
    let source_token_signal = signal("pan-zoom-source-token", source_token);
    let native = #{ key:`${props.key}-interaction`, source_token:source_token,
        x:props.transform.x, y:props.transform.y,
        scale:props.transform.scale, min_scale:props.min_scale, max_scale:props.max_scale,
        axes:props.axes, wheel_zoom:props.wheel_zoom, pan_button:props.pan_button,
        threshold:props.threshold, keyboard_pan_step:props.keyboard_pan_step,
        keyboard_zoom_factor:props.keyboard_zoom_factor, disabled:props.disabled,
        viewport_ref:viewport_ref, content_ref:content_ref, x_signal:x_signal, y_signal:y_signal,
        scale_x_signal:scale_x_signal, scale_y_signal:scale_y_signal,
        wheel_generation_signal:wheel_generation_signal, wheel_active_signal:wheel_active_signal,
        wheel_pending_signal:wheel_pending_signal, source_token_signal:source_token_signal,
        on_transform_change:if "on_transform_change" in props && props.on_transform_change != () {
            Fn("pan_zoom_commit")
        } else { () }
    };
    let content = props.content.with_key(`${props.key}-content`)
        .with_ref(content_ref)
        .with_style(ctx.component_style("content", style().width(relative(1.0)).height(relative(1.0))))
        .bind_signal("translate_x", x_signal).bind_signal("translate_y", y_signal)
        .bind_signal("scale_x", scale_x_signal).bind_signal("scale_y", scale_y_signal);
    let interaction = gpui_rhai::PanZoomPrimitive(native)
        .with_key(`${props.key}-pan-zoom`)
        .with_style(ctx.component_style("interaction", style().absolute()
            .left(px(0)).right(px(0)).top(px(0)).bottom(px(0)).cursor_move()
            .border(px(2)).border_color(rgba(0x00000000))
            .focus(style().border_color(theme_color("focus_ring")))))
        .disabled(props.disabled).tab_stop(!props.disabled)
        .accessibility_role("region").accessibility_label(props.label)
        .accessibility_value(`x ${props.transform.x}, y ${props.transform.y}, scale ${props.transform.scale}`);
    box([content, interaction]).with_key(viewport_key).with_ref(viewport_ref)
        .with_style(ctx.component_style("root", style().relative().clip()
            .min_width(px(0)).min_height(px(0)).border(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface"))))
}
