/* gpui-rhai
{
  "id": "components/pan_zoom",
  "export": "PanZoom",
  "version": "0.1.8",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled Canvas viewport transform with native pan and pointer-anchored zoom.
// Props: key, label, transform{x,y,scale}, min_scale, max_scale, axes,
// wheel_zoom, pan_button, threshold, keyboard steps, content, disabled,
// on_transform_change.
// Events: transform_change({x,y,scale}).
// State: transform is caller-owned; native signals contain only current preview.
// Parts: root, content, interaction.
// Example: PanZoom(#{ key:"map", label:"Map", transform:viewport, content:canvas_node, on_transform_change:Fn("changed") })

define_component(#{
    metadata: #{ id:"components/pan_zoom", "export":"PanZoom", version:"0.1.8",
        runtime_api:#{min_inclusive:2,max_exclusive:3}, dependencies:[], capabilities:#{} },
    schema: #{ props: #{
        key: #{ schema:#{type:"string"}, required:true, sensitive:false },
        label: #{ schema:#{type:"string"}, required:true, sensitive:false },
        transform: #{ schema:#{type:"object",allow_unknown:false,fields:#{
            x:#{schema:#{type:"number"},required:true,sensitive:false},
            y:#{schema:#{type:"number"},required:true,sensitive:false},
            scale:#{schema:#{type:"number",exclusive_min:0.0},required:true,sensitive:false},
        }}, required:true, sensitive:false },
        min_scale: #{ schema:#{type:"number",exclusive_min:0.0},required:false,sensitive:false,
            "default":#{type:"float",value:0.25} },
        max_scale: #{ schema:#{type:"number",exclusive_min:0.0},required:false,sensitive:false,
            "default":#{type:"float",value:8.0} },
        axes: #{ schema:#{type:"string",allowed:["both","horizontal","vertical"]},required:false,
            sensitive:false,"default":#{type:"string",value:"both"} },
        wheel_zoom: #{ schema:#{type:"string",allowed:["off","modifier","always"]},required:false,
            sensitive:false,"default":#{type:"string",value:"modifier"} },
        pan_button: #{ schema:#{type:"string",allowed:["left","middle"]},required:false,
            sensitive:false,"default":#{type:"string",value:"left"} },
        threshold: #{ schema:#{type:"number",min:0.0,max:64.0},required:false,sensitive:false,
            "default":#{type:"float",value:4.0} },
        keyboard_pan_step: #{ schema:#{type:"number",min:0.1,max:512.0},required:false,sensitive:false,
            "default":#{type:"float",value:16.0} },
        keyboard_zoom_factor: #{ schema:#{type:"number",min:1.001,max:4.0},required:false,sensitive:false,
            "default":#{type:"float",value:1.2} },
        content: #{ schema:#{type:"node"},required:true,sensitive:false },
        disabled: #{ schema:#{type:"bool"},required:false,sensitive:false,
            "default":#{type:"bool",value:false} },
        on_transform_change: #{ schema:#{type:"optional",value:#{type:"callback"}},required:false,sensitive:false },
    }, state:#{fields:#{}}, events:#{ transform_change:#{payload:#{type:"object",allow_unknown:false,fields:#{
        x:#{schema:#{type:"number"},required:true,sensitive:false},
        y:#{schema:#{type:"number"},required:true,sensitive:false},
        scale:#{schema:#{type:"number",exclusive_min:0.0},required:true,sensitive:false},
    }}} }, slots:#{content:#{required:true,multiple:false}}, parts:["root","content","interaction"] },
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
            .border(px(1)).border_color(rgba(0x00000000))
            .focus(style().border_color(theme_color("focus_ring")))))
        .disabled(props.disabled).tab_stop(!props.disabled)
        .accessibility_role("region").accessibility_label(props.label)
        .accessibility_value(`x ${props.transform.x}, y ${props.transform.y}, scale ${props.transform.scale}`);
    box([content, interaction]).with_key(viewport_key).with_ref(viewport_ref)
        .with_style(ctx.component_style("root", style().relative().clip()
            .min_width(px(0)).min_height(px(0)).border(px(1)).border_color(theme_color("border"))
            .background(theme_color("surface"))))
}
