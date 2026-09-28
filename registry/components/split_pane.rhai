/* gpui-rhai
{
  "id": "components/split_pane",
  "export": "SplitPane",
  "version": "0.1.7",
  "runtime_api": { "min_inclusive": 2, "max_exclusive": 3 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled, nestable two-panel split layout with a native drag hot lane.
//
// `size` is the controlled start-panel ratio. Pointer moves update only a
// native signal; `on_resize` fires once when a drag ends or a keyboard step is
// requested. If the Host rejects that value, the preview returns to `size`.
// Props: key, label, orientation, size, panel nodes/keys, constraints, collapse, disabled.
// Events: resize(number) proposes the next controlled start-panel ratio once.
// State: only native pointer preview; application ratio and collapse remain controlled.
// Parts: root, start, handle, end.
// Example: SplitPane(#{ key:"main", label:"Resize panes", size:0.35,
//     start:navigation, end:content, on_resize:Fn("resized") })

define_component(#{
    metadata: #{ id: "components/split_pane", "export": "SplitPane", version: "0.1.7",
        runtime_api: #{ min_inclusive: 2, max_exclusive: 3 }, dependencies: [], capabilities: #{} },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] },
                required: false, sensitive: false,
                "default": #{ type: "string", value: "horizontal" } },
            size: #{ schema: #{ type: "number", min: 0.0, max: 1.0 }, required: true, sensitive: false },
            min_start: #{ schema: #{ type: "number", min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 120.0 } },
            min_end: #{ schema: #{ type: "number", min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 120.0 } },
            max_start: #{ schema: #{ type: "number", min: 0.0, max: 16384.0 }, required: false,
                sensitive: false, "default": #{ type: "float", value: 16384.0 } },
            start_key: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "start" } },
            end_key: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "end" } },
            start: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            end: #{ schema: #{ type: "node" }, required: true, sensitive: false },
            start_collapsed: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            end_collapsed: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            keyboard_step: #{ schema: #{ type: "number", exclusive_min: 0.0, max: 512.0 },
                required: false, sensitive: false, "default": #{ type: "float", value: 8.0 } },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_resize: #{ schema: #{ type: "optional", value: #{ type: "callback" } },
                required: false, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{ resize: #{ payload: #{ type: "number", min: 0.0, max: 1.0 } } },
        slots: #{}, parts: ["root", "start", "handle", "end"],
    },
    render: Fn("render_SplitPane")
});

fn SplitPane(props) { render_component("components/split_pane", props) }
fn split_prop(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn split_commit(ctx, value) { ctx.emit("resize", value); }

fn split_key(ctx, payload) {
    let group = ctx.element_bounds(payload.group_ref);
    if group == () { return; }
    let group_size = if payload.orientation == "horizontal" { group.layout.width } else { group.layout.height };
    let start_size = payload.source_ratio * group_size;
    if group_size <= 0.0 { return; }
    // The renderer maps horizontal keys to logical start/end in RTL, so the
    // same signed delta remains spatially correct without requiring a locale.
    let delta = payload.delta;
    let maximum = if payload.max_start < group_size - payload.min_end {
        payload.max_start
    } else { group_size - payload.min_end };
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
    let root_style = style().width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0));
    if props.start_collapsed {
        return box([props.end.with_key(props.end_key).with_style(ctx.component_style("end",
            style().width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0))))])
            .with_key(root_ref_key).with_style(ctx.component_style("root", root_style)).with_ref(root_ref);
    }
    if props.end_collapsed {
        return box([props.start.with_key(props.start_key).with_style(ctx.component_style("start",
            style().width(relative(1.0)).height(relative(1.0)).min_width(px(0)).min_height(px(0))))])
            .with_key(root_ref_key).with_style(ctx.component_style("root", root_style)).with_ref(root_ref);
    }

    let preview = optional_float_signal("start-size");
    let start_style = style().min_width(px(0)).min_height(px(0)).flex_shrink(false);
    let end_style = style().min_width(px(0)).min_height(px(0)).flex_grow();
    let handle_style = style().flex_shrink(false)
        .focus(style().background(theme_color("selection")));
    if horizontal {
        start_style = start_style.width(relative(props.size)).height(relative(1.0));
        end_style = end_style.height(relative(1.0));
        handle_style = handle_style.width(px(8)).height(relative(1.0)).cursor_resize_x();
    } else {
        start_style = start_style.height(relative(props.size)).width(relative(1.0));
        end_style = end_style.width(relative(1.0));
        handle_style = handle_style.height(px(8)).width(relative(1.0)).cursor_resize_y();
    }
    let start = props.start.with_key(start_ref_key)
        .with_style(ctx.component_style("start", start_style))
        .with_ref(start_ref)
        .bind_signal(if horizontal { "width_override" } else { "height_override" }, preview);
    let resize_props = #{ key: `${props.key}-handle`, orientation: orientation,
        source_ratio: props.size, min_start: props.min_start, min_end: props.min_end,
        max_start: props.max_start, disabled: props.disabled,
        signal: preview, group_ref: root_ref, start_ref: start_ref };
    if "on_resize" in props && props.on_resize != () { resize_props.on_resize = Fn("split_commit"); }
    let key_payload = #{ orientation: orientation, group_ref: root_ref_key,
        start_ref: start_ref_key, min_start: props.min_start, min_end: props.min_end,
        max_start: props.max_start, source_ratio: props.size, delta: -props.keyboard_step };
    let handle = gpui_rhai::SplitResizePrimitive(resize_props)
        .with_key(`${props.key}-separator`)
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
    let end = props.end.with_key(props.end_key)
        .with_style(ctx.component_style("end", end_style));
    let children = [start, handle, end];
    if horizontal {
        row(children).with_key(root_ref_key).with_style(ctx.component_style("root", root_style)).with_ref(root_ref)
    } else {
        column(children).with_key(root_ref_key).with_style(ctx.component_style("root", root_style)).with_ref(root_ref)
    }
}
