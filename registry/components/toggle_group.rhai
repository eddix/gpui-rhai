/* gpui-rhai
{
  "id": "components/toggle_group",
  "export": "ToggleGroup",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "border", "selection", "surface_hover", "text_primary", "text_muted", "disabled",
    "focus_ring", "metrics.control", "metrics.control_pad", "radius.md", "radius.lg", "typography.control"
  ],
  "environment": ["size", "density", "corners"]
}
*/
// ToggleGroup holds pressed tool state: equal-height segments joined inside one frame.
//
// Props: key, label, items, values, mode, orientation, allow_empty, size, disabled, on_change.
// Events: change(array<string>).
// State: caller owns values; component owns only the roving active item.
// Parts: root, item.
// Example: ToggleGroup(#{ key: "align", label: "Alignment", values: ["left"], items: items, on_change: Fn("changed") })
//
// Keyboard: one tab stop on the active segment; arrows move focus between enabled
// segments; Enter or Space toggles the focused segment.

define_component(#{
    metadata: #{ id: "components/toggle_group", "export": "ToggleGroup", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "border", "selection", "surface_hover", "text_primary", "text_muted", "disabled",
            "focus_ring", "metrics.control", "metrics.control_pad", "radius.md", "radius.lg", "typography.control"
        ],
        environment: ["size", "density", "corners"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            items: #{ schema: #{ type: "array", max_items: 64, items: #{
                type: "object", allow_unknown: false, fields: #{
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                }
            } }, required: true, sensitive: false },
            values: #{ schema: #{ type: "array", max_items: 64, items: #{ type: "string" } },
                required: true, sensitive: false },
            mode: #{ schema: #{ type: "string", allowed: ["single", "multiple"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "single" } },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false,
                sensitive: false, "default": #{ type: "string", value: "horizontal" } },
            allow_empty: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true } },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false },
            disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{ active: #{ schema: #{ type: "string" },
            "default": #{ type: "string", value: "" } } } },
        events: #{ change: #{ payload: #{ type: "array", max_items: 64, items: #{ type: "string" } } } },
        slots: #{}, parts: ["root", "item"],
    },
    render: Fn("render_ToggleGroup")
});

fn ToggleGroup(props) { render_component("components/toggle_group", props) }
fn item_disabled(item) { "disabled" in item && item.disabled }
fn includes(values, value) {
    let found = false;
    for current in values { if current == value { found = true; } }
    found
}
fn enabled_values(items) {
    let values = [];
    for item in items { if !item_disabled(item) { values.push(item.value); } }
    values
}
fn selected_next(values, value, mode, allow_empty) {
    if mode == "single" {
        if includes(values, value) && allow_empty { [] } else { [value] }
    } else {
        let next = [];
        for current in values { if current != value { next.push(current); } }
        if next.len == values.len { next.push(value); }
        next
    }
}
fn adjacent(values, active, step) {
    if values.len == 0 { return ""; }
    let index = 0;
    for i in 0..values.len { if values[i] == active { index = i; } }
    if step > 0 { values[(index + 1) % values.len] }
    else { values[(index + values.len - 1) % values.len] }
}
fn segment_key(group, value) { `${group}-${value}` }
fn move_action(ctx, target) {
    ctx.set_state("active", target.value);
    // Headless lifecycles have no window to move focus in; the roving state still moves.
    try { ctx.focus(target.key); } catch { }
}
fn press_action(spec, ctx, payload) {
    ctx.set_state("active", spec.value);
    if spec.emit { ctx.emit("change", spec.next); }
}

fn render_ToggleGroup(ctx, props) {
    let mode = if "mode" in props { props.mode } else { "single" };
    if mode == "single" && props.values.len > 1 { throw "single toggle group accepts at most one value"; }
    let allow_empty = if "allow_empty" in props { props.allow_empty } else { true };
    let enabled = enabled_values(props.items);
    let stored_active = ctx.get_state("active");
    let active = if includes(enabled, stored_active) { stored_active }
        else if props.values.len > 0 && includes(enabled, props.values[0]) { props.values[0] }
        else if enabled.len > 0 { enabled[0] } else { "" };
    let disabled = "disabled" in props && props.disabled;
    let vertical = "orientation" in props && props.orientation == "vertical";
    let can_change = "on_change" in props && props.on_change != ();
    let count = props.items.len;

    // A horizontal group is a capsule with round corners; a vertical one rounds like a
    // panel. The outer segments carry the frame's corners, so a pressed fill never
    // pokes past them (clipping is rectangular).
    let corner = theme_radius(if vertical { "lg" } else { "md" });
    let segments = [];
    for index in 0..count {
        let item = props.items[index];
        let pressed = includes(props.values, item.value);
        let unavailable = disabled || item_disabled(item);
        let fill = if pressed { theme_color("selection") } else { rgba(0x00000000) };
        // Segments overlap the 2px group frame by -2px on its sides, so a pressed fill
        // and the focus border sit flush with the outer edge.
        let segment = style()
            .flex_row().items_center().justify_center().self_stretch()
            .min_height(theme_length("metrics.control"))
            .padding_x(theme_length("metrics.control_pad"))
            .border(px(2)).border_color(fill)
            .background(fill)
            .text_color(if pressed { theme_color("text_primary") } else { theme_color("text_muted") })
            .focus(style().border_color(theme_color("focus_ring")))
            .disabled(style().text_color(theme_color("disabled")));
        if vertical {
            segment = segment.margin_x(offset_px(-2));
            if index == 0 {
                segment = segment.margin_top(offset_px(-2)).radius_top_left(corner).radius_top_right(corner);
            }
            if index == count - 1 {
                segment = segment.margin_bottom(offset_px(-2))
                    .radius_bottom_left(corner).radius_bottom_right(corner);
            }
        } else {
            segment = segment.margin_y(offset_px(-2));
            if index == 0 { segment = segment.margin_start(offset_px(-2)).radius_start(corner); }
            if index == count - 1 { segment = segment.margin_end(offset_px(-2)).radius_end(corner); }
        }
        if !pressed && !unavailable {
            segment = segment.hover(style().background(theme_color("surface_hover"))
                .text_color(theme_color("text_primary")));
        }
        let key = segment_key(props.key, item.value);
        let node = row([text(item.label)])
            .with_key(key)
            .with_ref(element_ref(key))
            .with_style(ctx.component_style("item", segment))
            .disabled(unavailable)
            .tab_stop(!unavailable && item.value == active)
            .accessibility_role("button")
            .accessibility_label(item.label)
            .accessibility_pressed(pressed);
        if !unavailable {
            node = node.on_click(Fn("press_action").curry(#{
                value: item.value,
                emit: can_change,
                next: selected_next(props.values, item.value, mode, allow_empty),
            }));
        }
        segments.push(node);
    }

    let frame = style()
        .border(px(2)).border_color(theme_color("border"))
        .radius(corner).clip()
        .typography("control");
    let root = if vertical { column(segments) } else { row(segments) };
    root = root
        .with_style(ctx.component_style("root", frame))
        .disabled(disabled)
        .accessibility_role("group")
        .accessibility_label(props.label)
        .accessibility_orientation(if vertical { "vertical" } else { "horizontal" });
    if "size" in props && props.size != () { root = root.env(#{ size: props.size }); }
    if disabled || enabled.len == 0 { return root; }
    let previous = adjacent(enabled, active, -1);
    let next = adjacent(enabled, active, 1);
    let to_previous = #{ value: previous, key: segment_key(props.key, previous) };
    let to_next = #{ value: next, key: segment_key(props.key, next) };
    if vertical {
        root.on_key_value("up", Fn("move_action"), to_previous)
            .on_key_value("down", Fn("move_action"), to_next)
    } else {
        root.on_key_value("left", Fn("move_action"), to_previous)
            .on_key_value("right", Fn("move_action"), to_next)
    }
}
