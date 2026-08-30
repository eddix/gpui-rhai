/* gpui-rhai
{
  "id": "components/toast",
  "export": "Toast",
  "version": "0.1.0",
  "runtime_api": { "min_inclusive": 1, "max_exclusive": 2 },
  "dependencies": [],
  "capabilities": {}
}
*/
// Controlled Toast composition over public Layer, declarative timeout, atoms, and hover events.
// Props: key, items, max_visible, on_dismiss. Events: dismiss(string).
// State: caller owns items; runtime timers own one-shot deadline and pause/resume state.
// Example: Toast(#{ key: "toasts", items: items, on_dismiss: Fn("dismissed") })

define_component(#{
    metadata: #{
        id: "components/toast", "export": "Toast", version: "0.1.0",
        runtime_api: #{ min_inclusive: 1, max_exclusive: 2 }, dependencies: [], capabilities: #{},
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            max_visible: #{ schema: #{ type: "integer", min: 1, max: 10 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 3 } },
            items: #{ schema: #{ type: "array", max_items: 100, items: #{
                type: "object", allow_unknown: false, fields: #{
                    id: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    title: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    message: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                        "default": #{ type: "string", value: "" } },
                    variant: #{ schema: #{ type: "string", allowed: ["neutral", "success", "warning", "danger"] }, required: false, sensitive: false,
                        "default": #{ type: "string", value: "neutral" } },
                    region: #{ schema: #{ type: "string", allowed: ["top_left", "top_right", "bottom_left", "bottom_right"] }, required: false, sensitive: false,
                        "default": #{ type: "string", value: "top_right" } },
                    duration_ms: #{ schema: #{ type: "integer", min: 1, max: 86400000 }, required: false, sensitive: false,
                        "default": #{ type: "integer", value: 5000 } },
                    paused: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                    dismissible: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: true } },
                },
            } }, required: true, sensitive: false },
            on_dismiss: #{ schema: #{ type: "callback" }, required: true, sensitive: false },
        },
        state: #{ fields: #{} },
        events: #{ dismiss: #{ payload: #{ type: "string" } } },
        slots: #{},
        parts: ["root", "region", "toast", "toast_neutral", "toast_success",
            "toast_warning", "toast_danger", "title", "message", "close"],
    },
    render: Fn("render_Toast")
});

fn Toast(props) { render_component("components/toast", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn timer_key(id) { `toast:${id}` }

fn toast_timeout(ctx, id) { ctx.emit("dismiss", id); }
fn toast_manual_dismiss(ctx, id) {
    ctx.cancel_timeout(timer_key(id));
    ctx.emit("dismiss", id);
    handled()
}
fn toast_hover(ctx, payload) {
    if payload.hovered { ctx.pause_timeout(timer_key(payload.value)); }
    else { ctx.resume_timeout(timer_key(payload.value)); }
    handled()
}

fn variant_color(variant) {
    if variant == "success" { theme_color("success") }
    else if variant == "warning" { theme_color("warning") }
    else if variant == "danger" { theme_color("danger") }
    else { theme_color("border") }
}

fn toast_node(props, item) {
    let variant = prop_or(item, "variant", "neutral");
    let header = [text(item.title)
        .with_style(component_style(props, "title", style()))];
    if prop_or(item, "dismissible", true) {
        header.push(text("×").with_key(`${item.id}-close`)
            .with_style(component_style(props, "close", style().padding_x(px(4))))
            .accessibility_role("button").accessibility_label("Dismiss")
            .on_click_value(Fn("toast_manual_dismiss"), item.id));
    }
    let children = [row(header).with_style(style().items_center().justify_between())];
    if prop_or(item, "message", "") != "" {
        children.push(text(item.message).with_style(component_style(props, "message",
            style().text_color(theme_color("text_muted")))));
    }
    column(children).with_key(item.id)
        .with_style(component_style(props, `toast_${variant}`,
            component_style(props, "toast",
                style().width(px(320)).gap(px(4)).padding(px(12))
                    .border(px(1)).border_color(variant_color(variant))
                    .radius(theme_radius("md")).background(theme_color("surface_raised"))
                    .text_color(theme_color("text_primary")))))
        .on_hover_value(Fn("toast_hover"), item.id)
        .accessibility_role(if variant == "danger" { "alert" } else { "status" })
        .accessibility_label(item.title)
}

fn region_layer(props, key, placement, children) {
    if children.len == 0 { return (); }
    layer(column(children).with_key(`${key}-column`)
        .with_style(component_style(props, "region", style().width(px(320)).gap(px(8)))),
        #{ id: key, placement: placement, inset: 12, priority: 9000 })
}

fn render_Toast(ctx, props) {
    let ids = #{};
    for item in props.items {
        let id = item.id;
        id.trim();
        let title = item.title;
        title.trim();
        if id == "" { throw "Toast item ID cannot be empty"; }
        if title == "" { throw `Toast item '${item.id}' title cannot be empty`; }
        if item.id in ids { throw `Toast item ID '${item.id}' is duplicated`; }
        ids[item.id] = true;
    }

    let top_left = [];
    let top_right = [];
    let bottom_left = [];
    let bottom_right = [];
    let visible = props.items.len;
    if visible > prop_or(props, "max_visible", 3) { visible = prop_or(props, "max_visible", 3); }
    for index in 0..visible {
        let item = props.items[index];
        timeout(timer_key(item.id), prop_or(item, "duration_ms", 5000),
            prop_or(item, "paused", false), Fn("toast_timeout"), item.id);
        let node = toast_node(props, item);
        let region = prop_or(item, "region", "top_right");
        if region == "top_left" { top_left.push(node); }
        else if region == "bottom_left" { bottom_left.push(node); }
        else if region == "bottom_right" { bottom_right.push(node); }
        else { top_right.push(node); }
    }
    let layers = [];
    for spec in [
        #{ key: `${props.key}-top-left`, placement: "top_left", children: top_left },
        #{ key: `${props.key}-top-right`, placement: "top_right", children: top_right },
        #{ key: `${props.key}-bottom-left`, placement: "bottom_left", children: bottom_left },
        #{ key: `${props.key}-bottom-right`, placement: "bottom_right", children: bottom_right },
    ] {
        let node = region_layer(props, spec.key, spec.placement, spec.children);
        if node != () { layers.push(node); }
    }
    box(layers).with_key(props.key)
        .with_style(component_style(props, "root", style()))
}
