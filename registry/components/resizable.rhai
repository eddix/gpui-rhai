/* gpui-rhai
{
  "id": "components/resizable",
  "export": "Resizable",
  "version": "0.1.7",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled single-element rectangle resizing inside one local boundary.
// Props: key, label, rect, content, handles, constraints, aspect_ratio, contain, disabled.
// Events: resize({x,y,width,height,handle}) proposes one complete controlled rectangle.
// State: only native pointer preview; application rectangle remains controlled.
// Parts: root, surface, content, handle.
// Example: Resizable(#{ key:"card", label:"Resize card",
//     rect:#{x:40.0,y:32.0,width:280.0,height:180.0}, content:card,
//     on_resize:Fn("resized") })

define_component(#{
    metadata: #{ id: "components/resizable", "export": "Resizable", version: "0.1.7",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            rect: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                width: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false },
                height: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false },
            } }, required: true, sensitive: false },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            handles: #{ schema: #{ type: "array", max_items: 8,
                items: #{ type: "string", allowed: ["n", "s", "e", "w", "ne", "nw", "se", "sw"] } },
                required: false, sensitive: false,
                "default": #{ type: "array", value: [
                    #{ type: "string", value: "n" }, #{ type: "string", value: "s" },
                    #{ type: "string", value: "e" }, #{ type: "string", value: "w" },
                    #{ type: "string", value: "ne" }, #{ type: "string", value: "nw" },
                    #{ type: "string", value: "se" }, #{ type: "string", value: "sw" }
                ] } },
            min_width: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 24.0 } },
            min_height: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 24.0 } },
            max_width: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 16384.0 } },
            max_height: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 16384.0 } },
            aspect_ratio: #{ schema: #{ type: "optional", value: #{ type: "number", exclusive_min: 0.0 } },
                required: false, sensitive: false },
            contain: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            keyboard_step: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 512.0 },
                required: false, sensitive: false, "default": #{ type: "float", value: 8.0 } },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_resize: #{ schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{ resize: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
            x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            width: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false },
            height: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false },
            handle: #{ schema: #{ type: "string", allowed: ["n", "s", "e", "w", "ne", "nw", "se", "sw"] },
                required: true, sensitive: false },
        } } } },
        slots: #{}, parts: ["root", "surface", "content", "handle"],
    },
    render: Fn("render_Resizable")
});

fn Resizable(props) { render_component("components/resizable", props) }
fn resizable_commit(ctx, value) { ctx.emit("resize", value); }

fn handle_style(handle) {
    let value = style().absolute().flex_shrink(false)
        .focus(style().background(theme_color("selection")));
    if handle == "n" {
        value.left(px(12)).right(px(12)).top(offset_px(-5)).height(px(10))
    } else if handle == "s" {
        value.left(px(12)).right(px(12)).bottom(offset_px(-5)).height(px(10))
    } else if handle == "e" {
        value.top(px(12)).bottom(px(12)).right(offset_px(-5)).width(px(10))
    } else if handle == "w" {
        value.top(px(12)).bottom(px(12)).left(offset_px(-5)).width(px(10))
    } else if handle == "ne" {
        value.top(offset_px(-7)).right(offset_px(-7)).width(px(14)).height(px(14))
    } else if handle == "nw" {
        value.top(offset_px(-7)).left(offset_px(-7)).width(px(14)).height(px(14))
    } else if handle == "se" {
        value.bottom(offset_px(-7)).right(offset_px(-7)).width(px(14)).height(px(14))
    } else {
        value.bottom(offset_px(-7)).left(offset_px(-7)).width(px(14)).height(px(14))
    }
}

fn handle_orientation(handle) {
    if handle == "n" || handle == "s" { "horizontal" }
    else if handle == "e" || handle == "w" { "vertical" }
    else { () }
}

fn render_Resizable(ctx, props) {
    if props.max_width < props.min_width || props.max_height < props.min_height {
        throw "Resizable max dimensions must be at least their minimums";
    }
    let seen = [];
    for handle in props.handles {
        if handle in seen { throw `Resizable handle ${handle} is duplicated`; }
        seen.push(handle);
    }
    let boundary_key = `${props.key}-boundary`;
    let boundary_ref = element_ref(boundary_key);
    let x_signal = optional_float_signal("resize-x");
    let y_signal = optional_float_signal("resize-y");
    let width_signal = optional_float_signal("resize-width");
    let height_signal = optional_float_signal("resize-height");
    let surface_children = [box([props.content]).with_key(`${props.key}-content`)
        .with_style(ctx.component_style("content",
            style().width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0))))];
    for handle in props.handles {
        let native_props = #{ key: `${props.key}-${handle}`, handle: handle,
            x: props.rect.x, y: props.rect.y, width: props.rect.width, height: props.rect.height,
            min_width: props.min_width, min_height: props.min_height,
            max_width: props.max_width, max_height: props.max_height,
            aspect_ratio: if "aspect_ratio" in props { props.aspect_ratio } else { () },
            contain: props.contain, keyboard_step: props.keyboard_step, disabled: props.disabled,
            boundary_ref: boundary_ref, x_signal: x_signal, y_signal: y_signal,
            width_signal: width_signal, height_signal: height_signal };
        if "on_resize" in props && props.on_resize != () {
            native_props.on_resize = Fn("resizable_commit");
        }
        let node = gpui_rhai::ResizableHandlePrimitive(native_props)
            .with_style(ctx.component_style("handle", handle_style(handle)))
            .disabled(props.disabled)
            .tab_stop(!props.disabled)
            .accessibility_role("separator")
            .accessibility_label(`${props.label}: ${handle} resize handle`);
        let orientation = handle_orientation(handle);
        if orientation != () { node = node.accessibility_orientation(orientation); }
        surface_children.push(node);
    }
    let surface = box(surface_children)
        .with_key(`${props.key}-surface`)
        .with_style(ctx.component_style("surface", style().absolute()
            .left(px(props.rect.x)).top(px(props.rect.y))
            .width(px(props.rect.width)).height(px(props.rect.height))
            .min_width(px(0)).min_height(px(0))))
        .bind_signal("translate_x_override", x_signal)
        .bind_signal("translate_y_override", y_signal)
        .bind_signal("width_override", width_signal)
        .bind_signal("height_override", height_signal);
    box([surface])
        .with_key(boundary_key)
        .with_ref(boundary_ref)
        .with_style(ctx.component_style("root", style().relative()
            .width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0))))
}
