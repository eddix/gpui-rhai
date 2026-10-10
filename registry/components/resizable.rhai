/* gpui-rhai
{
  "id": "components/resizable",
  "export": "Resizable",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["focus_ring", "accent", "border", "surface_raised"],
  "environment": []
}
*/
// Controlled single-element rectangle resizing inside one local boundary.
// State: only native pointer preview; application rectangle remains controlled.
//
// A grip is decoration, centred on its handle and painted above the content; pressing it
// anywhere starts that handle's drag, and it takes `grip_<state>` from a native signal, as
// SplitPane's grip does. `line: false` drops the native marks.
// Example: Resizable(#{ key:"card", label:"Resize card",
//     rect:#{x:40.0,y:32.0,width:280.0,height:180.0}, content:card,
//     on_resize:Fn("resized") })

define_component(#{
    metadata: #{ id: "components/resizable", "export": "Resizable", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["focus_ring", "accent", "border", "surface_raised"],
        environment: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name; each handle reads as `<label>: <handle> resize handle`." },
            rect: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                x: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Left edge, in logical pixels from the root's left edge." },
                y: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                    doc: "Top edge, in logical pixels from the root's top edge." },
                width: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false,
                    doc: "Width in logical pixels." },
                height: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false,
                    doc: "Height in logical pixels." },
            } }, required: true, sensitive: false,
                doc: "Position and size in the root box; the caller stores the `resize` payload and passes it back." },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "Node that fills the rectangle." },
            handles: #{ schema: #{ type: "array", max_items: 8,
                items: #{ type: "string", allowed: ["n", "s", "e", "w", "ne", "nw", "se", "sw"] } },
                required: false, sensitive: false,
                "default": #{ type: "array", value: [
                    #{ type: "string", value: "n" }, #{ type: "string", value: "s" },
                    #{ type: "string", value: "e" }, #{ type: "string", value: "w" },
                    #{ type: "string", value: "ne" }, #{ type: "string", value: "nw" },
                    #{ type: "string", value: "se" }, #{ type: "string", value: "sw" }
                ] },
                doc: "Edges and corners that get a resize handle, each at most once." },
            min_width: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 24.0 },
                doc: "Smallest width a resize may propose, in logical pixels." },
            min_height: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 24.0 },
                doc: "Smallest height a resize may propose, in logical pixels." },
            max_width: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 16384.0 },
                doc: "Largest width a resize may propose, in logical pixels; at least `min_width`." },
            max_height: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 16384.0 },
                doc: "Largest height a resize may propose, in logical pixels; at least `min_height`." },
            aspect_ratio: #{ schema: #{ type: "optional", value: #{ type: "number", exclusive_min: 0.0 } },
                required: false, sensitive: false,
                doc: "Width-to-height ratio to keep; an edge handle also resizes the other side, about its centre." },
            contain: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Keeps the rectangle inside the root box; while `rect` lies outside it, drags and keys do nothing." },
            keyboard_step: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 512.0 },
                required: false, sensitive: false, "default": #{ type: "float", value: 8.0 },
                doc: "Logical pixels an arrow key moves the focused handle's edge; Shift moves four times as far." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Ignores drags and key steps and removes the handles from the tab order." },
            grips: #{ schema: #{ type: "optional", value: #{ type: "object", allow_unknown: false,
                fields: #{
                    n: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Grip for the top edge handle." },
                    s: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Grip for the bottom edge handle." },
                    e: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Grip for the right edge handle." },
                    w: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Grip for the left edge handle." },
                    ne: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Grip for the top-right corner handle." },
                    nw: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Grip for the top-left corner handle." },
                    se: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Grip for the bottom-right corner handle." },
                    sw: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Grip for the bottom-left corner handle." },
                } } }, required: false, sensitive: false,
                doc: "Decorative node per handle, as in `#{ se: node }`; only handles listed in `handles` show theirs." },
            line: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Paints the native marks (edge lines, corner squares); set `false` when grips draw their own." },
            line_inset: #{ schema: #{ type: "number", min: 0.0, max: 256.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 4.0 },
                doc: "How far an edge handle's native line stops short of each end, in logical pixels." },
            on_resize: #{ schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
                doc: "Called with the proposed `rect` when a handle drag ends or an arrow key is pressed." },
        },
        state: #{ fields: #{} },
        events: #{ resize: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
            x: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                doc: "Proposed left edge, in logical pixels." },
            y: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                doc: "Proposed top edge, in logical pixels." },
            width: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false,
                doc: "Proposed width, in logical pixels." },
            height: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: true, sensitive: false,
                doc: "Proposed height, in logical pixels." },
        } },
            doc: "Emitted when a drag ends or a key step resizes; the payload is the next `rect`, ready to store as is." } },
        slots: #{}, parts: ["root", "surface", "content", "handle", "grip", "grip_hover", "grip_drag",
            "grip_focus", "grip_disabled"],
    },
    render: Fn("render_Resizable")
});

fn Resizable(props) { render_component("components/resizable", props) }
fn resizable_commit(ctx, value) { ctx.emit("resize", value); }

fn handle_style(handle) {
    handle_box(handle).focus(style().background(theme_color("focus_ring")))
}

// The handle's box on the surface edge or corner.
fn handle_box(handle) {
    let value = style().absolute().flex_shrink(false);
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

// A decorative grip centred on its handle: later in the surface than the content and the
// handles, so it paints above them.
fn resizable_grip(ctx, props, handle, node, grip_ref, state) {
    let shape = style().flex_row().items_center().justify_center().occlude_except_scroll()
        .background(theme_color("surface_raised")).border(px(1)).border_color(theme_color("border"));
    let grip = box([node]).with_key(`${props.key}-grip-${handle}`).with_ref(grip_ref)
        .with_style(ctx.component_style("grip", shape))
        .signal_style(state, #{
            hover: ctx.component_style("grip_hover", style().border_color(theme_color("accent"))),
            drag: ctx.component_style("grip_drag", style().border_color(theme_color("accent"))),
            focus: ctx.component_style("grip_focus", style().border_color(theme_color("focus_ring"))),
            disabled: ctx.component_style("grip_disabled", style().opacity(0.5)),
        });
    box([grip]).with_key(`${props.key}-grip-lane-${handle}`)
        .with_style(handle_box(handle).flex_row().items_center().justify_center())
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
    let grips = [];
    let grip_nodes = if "grips" in props && props.grips != () { props.grips } else { #{} };
    for handle in props.handles {
        let native_props = #{ key: `${props.key}-${handle}`, handle: handle,
            x: props.rect.x, y: props.rect.y, width: props.rect.width, height: props.rect.height,
            min_width: props.min_width, min_height: props.min_height,
            max_width: props.max_width, max_height: props.max_height,
            aspect_ratio: if "aspect_ratio" in props { props.aspect_ratio } else { () },
            contain: props.contain, keyboard_step: props.keyboard_step, disabled: props.disabled,
            boundary_ref: boundary_ref, x_signal: x_signal, y_signal: y_signal,
            width_signal: width_signal, height_signal: height_signal,
            line: if "line" in props { props.line } else { true },
            line_inset: if "line_inset" in props { props.line_inset } else { 4.0 } };
        if handle in grip_nodes && grip_nodes[handle] != () {
            let grip_ref = element_ref(`${props.key}-grip-${handle}`);
            let state = signal(`grip-state-${handle}`, "idle");
            native_props.state_signal = state;
            native_props.handle_ref = grip_ref;
            grips.push(resizable_grip(ctx, props, handle, grip_nodes[handle], grip_ref, state));
        }
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
    for grip in grips { surface_children.push(grip); }
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
