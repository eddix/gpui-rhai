/* gpui-rhai
{
  "id": "components/split_pane",
  "export": "SplitPane",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": ["border", "surface_hover", "focus_ring", "accent", "surface_raised"],
  "environment": []
}
*/
// Controlled, nestable two-panel split layout with a native drag hot lane.
//
// `size` is the controlled start-panel ratio. Pointer moves update only a
// native signal; `on_resize` fires once when a drag ends or a keyboard step is
// requested. If the Host rejects that value, the preview returns to `size`.
// State: only native pointer preview; application ratio and collapse remain controlled.
//
// `handle` is decoration: it sits in a `grip` box centred on the separator and painted above
// both panes, and pressing anywhere on that box, overhang included, starts the drag. The
// keyboard steps, the separator role and the tab stop stay on the native handle. The grip
// takes the `grip_<state>` part while the handle is hovered, dragged, focused or disabled,
// from a native signal, so pointer moves never re-render Rhai. `line: false` drops the
// native line for a grip that draws its own.
// Example: SplitPane(#{ key:"main", label:"Resize panes", size:0.35,
//     start:navigation, end:content, on_resize:Fn("resized") })

define_component(#{
    metadata: #{ id: "components/split_pane", "export": "SplitPane", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: ["border", "surface_hover", "focus_ring", "accent", "surface_raised"],
        environment: [] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the separator." },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "horizontal" },
                doc: "`horizontal` puts the panes side by side; `vertical` stacks them." },
            size: #{ schema: #{ type: "number", min: 0.0, max: 1.0 }, required: true, sensitive: false,
                doc: "Start-pane share of the group's length; the caller stores the `resize` payload and passes it back." },
            min_start: #{ schema: #{ type: "number", min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 120.0 },
                doc: "Smallest start-pane length a drag or key step may propose, in logical pixels." },
            min_end: #{ schema: #{ type: "number", min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 120.0 },
                doc: "Smallest end-pane length a drag or key step may leave, in logical pixels." },
            max_start: #{ schema: #{ type: "number", min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 16384.0 },
                doc: "Largest start-pane length a drag or key step may propose, in logical pixels; at least `min_start`." },
            start_key: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "start" },
                doc: "Key of the start pane's wrapper; keep it stable so the pane is not remounted." },
            end_key: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "end" },
                doc: "Key of the end pane's wrapper; keep it stable so the pane is not remounted." },
            start: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "Content of the start pane: the leading side, or the top when `vertical`." },
            end: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                doc: "Content of the end pane: the trailing side, or the bottom when `vertical`." },
            start_collapsed: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Hides the start pane and the separator so the end pane fills the group; not with `end_collapsed`." },
            end_collapsed: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Hides the end pane and the separator so the start pane fills the group; not with `start_collapsed`." },
            keyboard_step: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 512.0 },
                required: false, sensitive: false, "default": #{ type: "float", value: 8.0 },
                doc: "Logical pixels one arrow-key press on the focused separator moves it." },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Ignores drags and removes the separator from the tab order." },
            handle: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Decorative grip node centred on the separator above both panes; pressing it starts the drag." },
            line: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Paints the native line down the separator; set `false` when `handle` draws its own." },
            line_inset: #{ schema: #{ type: "number", min: 0.0, max: 256.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 4.0 },
                doc: "How far the native line stops short of each end of the separator, in logical pixels." },
            on_resize: #{ schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false,
                doc: "Called with the proposed start-pane ratio when a drag ends or an arrow key is pressed." },
        },
        state: #{ fields: #{} },
        events: #{ resize: #{ payload: #{ type: "number", min: 0.0, max: 1.0 },
            doc: "Emitted once when a drag ends or a key step is requested; the payload is the next `size`." } },
        slots: #{}, parts: ["root", "start", "handle", "end", "grip", "grip_hover", "grip_drag",
            "grip_focus", "grip_disabled"],
    },
    render: Fn("render_SplitPane")
});

fn SplitPane(props) { render_component("components/split_pane", props) }
fn split_prop(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn split_commit(ctx, value) { ctx.emit("resize", value); }

// The decorative grip: a box the size of the separator on the end pane's start edge, the
// caller's node centred in it; the end pane paints after the start pane and the separator,
// so the grip is above both.
fn split_grip(ctx, props, horizontal, grip_ref, state) {
    let shape = style().flex_row().items_center().justify_center()
        .occlude_except_scroll().background(theme_color("surface_raised"))
        .border(px(1)).border_color(theme_color("border"));
    shape = if horizontal { shape.cursor_resize_x() } else { shape.cursor_resize_y() };
    let grip = box([props.handle]).with_key(`${props.key}-grip`).with_ref(grip_ref)
        .with_style(ctx.component_style("grip", shape))
        .signal_style(state, #{
            hover: ctx.component_style("grip_hover", style().border_color(theme_color("accent"))),
            drag: ctx.component_style("grip_drag", style().border_color(theme_color("accent"))),
            focus: ctx.component_style("grip_focus", style().border_color(theme_color("focus_ring"))),
            disabled: ctx.component_style("grip_disabled", style().opacity(0.5)),
        });
    let lane = style().absolute().flex_row().items_center().justify_center();
    lane = if horizontal { lane.top(px(0)).bottom(px(0)).inset_start(offset_px(-8)).width(px(8)) }
        else { lane.left(px(0)).right(px(0)).top(offset_px(-8)).height(px(8)) };
    box([grip]).with_key(`${props.key}-grip-lane`).with_style(lane)
}

fn split_key(ctx, payload) {
    let group = ctx.element_bounds(payload.group_ref);
    let start = ctx.element_bounds(payload.start_ref);
    let handle = ctx.element_bounds(payload.handle_ref);
    if group == () || start == () || handle == () { return; }
    let group_size = if payload.orientation == "horizontal" { group.layout.width } else { group.layout.height };
    let start_size = if payload.orientation == "horizontal" { start.layout.width } else { start.layout.height };
    let handle_size = if payload.orientation == "horizontal" { handle.layout.width } else { handle.layout.height };
    if group_size <= 0.0 { return; }
    // The renderer maps horizontal keys to logical start/end in RTL, so the
    // same signed delta remains spatially correct without requiring a locale.
    let delta = payload.delta;
    let maximum = if payload.max_start < group_size - handle_size - payload.min_end {
        payload.max_start
    } else { group_size - handle_size - payload.min_end };
    let minimum = if payload.min_start < maximum { payload.min_start } else { maximum };
    let next = if start_size + delta < minimum { minimum }
        else if start_size + delta > maximum { maximum }
        else { start_size + delta };
    ctx.emit("resize", if next <= 0.0 { 0.0 } else { next / group_size });
}

fn render_SplitPane(ctx, props) {
    if props.start_collapsed && props.end_collapsed {
        throw "SplitPane cannot collapse both panels";
    }
    let orientation = props.orientation;
    let horizontal = orientation == "horizontal";
    let root_ref_key = `${props.key}-group`;
    let start_ref_key = `${props.key}-${props.start_key}`;
    let root_ref = element_ref(root_ref_key);
    let start_ref = element_ref(start_ref_key);
    let handle_ref_key = `${props.key}-handle-ref`;
    let handle_ref = element_ref(handle_ref_key);
    let root_style = style().width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0));
    if props.start_collapsed {
        return box([box([props.end]).with_key(props.end_key).with_style(ctx.component_style("end",
            style().width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0))))])
            .with_key(root_ref_key).with_style(ctx.component_style("root", root_style)).with_ref(root_ref);
    }
    // A collapsed side keeps the other pane's key, so collapsing never remounts it.
    if props.end_collapsed {
        return box([box([props.start]).with_key(start_ref_key).with_style(ctx.component_style("start",
            style().width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0))))])
            .with_key(root_ref_key).with_style(ctx.component_style("root", root_style)).with_ref(root_ref);
    }

    let preview = optional_float_signal("start-size");
    let start_style = style().min_width(px(0)).min_height(px(0)).flex_shrink(false);
    let end_style = style().min_width(px(0)).min_height(px(0)).flex_grow();
    // The 8px grab zone: the native handle draws the hairline down its middle (`line`,
    // `line_inset`); hover fills the zone with the tonal step and keyboard focus turns it
    // into an ink bar.
    let handle_style = style().flex_shrink(false)
        .hover(style().background(theme_color("surface_hover")))
        .focus(style().background(theme_color("focus_ring")));
    if horizontal {
        start_style = start_style.width(relative(props.size)).height(relative(1.0));
        end_style = end_style.height(relative(1.0));
        handle_style = handle_style.width(px(8)).height(relative(1.0)).cursor_resize_x();
    } else {
        start_style = start_style.height(relative(props.size)).width(relative(1.0));
        end_style = end_style.width(relative(1.0));
        handle_style = handle_style.height(px(8)).width(relative(1.0)).cursor_resize_y();
    }
    let has_grip = "handle" in props && props.handle != ();
    let start = box([props.start]).with_key(start_ref_key)
        .with_style(ctx.component_style("start", start_style))
        .with_ref(start_ref)
        .bind_signal(if horizontal { "width_override" } else { "height_override" }, preview);
    let resize_props = #{ key: `${props.key}-handle`, orientation: orientation,
        source_ratio: props.size, min_start: props.min_start, min_end: props.min_end,
        max_start: props.max_start, disabled: props.disabled,
        signal: preview, group_ref: root_ref, start_ref: start_ref,
        line: split_prop(props, "line", true), line_inset: split_prop(props, "line_inset", 4.0) };
    let grip = ();
    if has_grip {
        let grip_ref = element_ref(`${props.key}-grip`);
        let state = signal("handle-state", "idle");
        resize_props.state_signal = state;
        resize_props.handle_ref = grip_ref;
        grip = split_grip(ctx, props, horizontal, grip_ref, state);
    }
    if "on_resize" in props && props.on_resize != () { resize_props.on_resize = Fn("split_commit"); }
    let key_payload = #{ orientation: orientation, group_ref: root_ref_key,
        start_ref: start_ref_key, handle_ref: handle_ref_key,
        min_start: props.min_start, min_end: props.min_end,
        max_start: props.max_start, delta: -props.keyboard_step };
    let handle = gpui_rhai::SplitResizePrimitive(resize_props)
        .with_key(`${props.key}-separator`).with_ref(handle_ref)
        .with_style(ctx.component_style("handle", handle_style))
        .disabled(props.disabled)
        .tab_stop(!props.disabled)
        .accessibility_role("separator")
        .accessibility_label(props.label)
        .accessibility_orientation(if horizontal { "vertical" } else { "horizontal" });
    if horizontal {
        handle = handle.on_key_value("left", Fn("split_key"), key_payload);
        key_payload.delta = props.keyboard_step;
        handle = handle.on_key_value("right", Fn("split_key"), key_payload);
    } else {
        handle = handle.on_key_value("up", Fn("split_key"), key_payload);
        key_payload.delta = props.keyboard_step;
        handle = handle.on_key_value("down", Fn("split_key"), key_payload);
    }
    let end_children = [props.end];
    if grip != () { end_children.push(grip); end_style = end_style.relative(); }
    let end = box(end_children).with_key(props.end_key)
        .with_style(ctx.component_style("end", end_style));
    let children = [start, handle, end];
    let root = if horizontal { row(children) } else { column(children) };
    // The panes are separate regions: nothing in one is in a row or a stack with the other.
    root.with_key(root_ref_key).with_style(ctx.component_style("root", root_style)).with_ref(root_ref)
        .audit_allow(["text-edge-misaligned", "row-height-mismatch", "mixed-type-in-row"])
}
