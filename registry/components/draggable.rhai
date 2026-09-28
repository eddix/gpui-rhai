/* gpui-rhai
{
  "id": "components/draggable",
  "export": "Draggable",
  "version": "0.1.8",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled in-window positioning. Native pointer movement updates only
// optional translation signals; release proposes one final {x,y} value.

define_component(#{
    metadata: #{ id: "components/draggable", "export": "Draggable", version: "0.1.8",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            position: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            } }, required: true, sensitive: false },
            content: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            handle: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
            axes: #{ schema: #{ type: "string", allowed: ["both", "horizontal", "vertical"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "both" } },
            contain: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            threshold: #{ schema: #{ type: "number", min: 0.0, max: 64.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 4.0 } },
            snap_x: #{ schema: #{ type: "optional", value: #{ type: "number", exclusive_min: 0.0 } },
                required: false, sensitive: false },
            snap_y: #{ schema: #{ type: "optional", value: #{ type: "number", exclusive_min: 0.0 } },
                required: false, sensitive: false },
            keyboard_step: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 512.0 },
                required: false, sensitive: false, "default": #{ type: "float", value: 8.0 } },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_move: #{ schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{ move: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
            x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
            y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
        } } } },
        slots: #{}, parts: ["root", "surface", "handle", "content", "interaction"],
    },
    render: Fn("render_Draggable")
});

fn Draggable(props) { render_component("components/draggable", props) }
fn draggable_commit(ctx, position) { ctx.emit("move", position); }

fn render_Draggable(ctx, props) {
    let boundary_key = `${props.key}-boundary`;
    let object_key = `${props.key}-object`;
    let boundary_ref = element_ref(boundary_key);
    let object_ref = element_ref(object_key);
    let handle_ref = if "handle" in props && props.handle != () {
        element_ref(`${props.key}-handle`)
    } else { object_ref };
    let x_signal = optional_float_signal("drag-x");
    let y_signal = optional_float_signal("drag-y");
    let native = #{
        key: `${props.key}-interaction`, x: props.position.x, y: props.position.y,
        axes: props.axes, contain: props.contain, threshold: props.threshold,
        keyboard_step: props.keyboard_step, disabled: props.disabled,
        boundary_ref: boundary_ref, object_ref: object_ref, handle_ref: handle_ref,
        x_signal: x_signal, y_signal: y_signal,
        on_move: if "on_move" in props && props.on_move != () { Fn("draggable_commit") } else { () }
    };
    if "snap_x" in props && props.snap_x != () { native.snap_x = props.snap_x; }
    if "snap_y" in props && props.snap_y != () { native.snap_y = props.snap_y; }
    let interaction = gpui_rhai::DraggablePrimitive(native)
        .with_key(`${props.key}-drag-interaction`)
        .with_style(ctx.component_style("interaction", style().absolute()
            .left(px(0)).right(px(0)).top(px(0)).bottom(px(0))))
        .disabled(props.disabled).tab_stop(!props.disabled)
        .accessibility_role("group").accessibility_label(props.label);
    let children = [];
    if "handle" in props && props.handle != () {
        children.push(props.handle.with_key(`${props.key}-handle`).with_ref(handle_ref)
            .with_style(ctx.component_style("handle", style())));
    }
    children.push(props.content.with_key(`${props.key}-content`)
        .with_style(ctx.component_style("content", style())));
    children.push(interaction);
    let surface = column(children).with_key(object_key).with_ref(object_ref)
        .with_style(ctx.component_style("surface", style().absolute()
            .left(px(props.position.x)).top(px(props.position.y))))
        .bind_signal("translate_x_override", x_signal)
        .bind_signal("translate_y_override", y_signal);
    box([surface]).with_key(boundary_key).with_ref(boundary_ref)
        .with_style(ctx.component_style("root", style().relative()
            .width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0))))
}
