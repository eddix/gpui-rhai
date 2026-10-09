/* gpui-rhai
{
  "id": "components/toast",
  "export": "Toast",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/close.svg"],
  "tokens": [
    "surface_raised", "border", "surface_hover", "success", "warning", "danger",
    "text_primary", "text_muted", "metrics.inset", "metrics.icon", "spacing.xxs", "spacing.sm",
    "space.related", "radius.lg", "typography.body"
  ],
  "environment": ["density", "corners"]
}
*/
// Controlled Toast composition over public Layer, declarative timeout, atoms, and hover events.
// State: caller owns items; runtime timers own one-shot deadline and pause/resume state.
// Example: Toast(#{ key: "toasts", items: items, on_dismiss: Fn("dismissed") })

define_component(#{
    metadata: #{
        id: "components/toast", "export": "Toast", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_raised", "border", "surface_hover", "success", "warning", "danger",
            "text_primary", "text_muted", "metrics.inset", "metrics.icon", "spacing.xxs",
            "spacing.sm", "space.related", "radius.lg", "typography.body"
        ],
        environment: ["density", "corners"],
        assets: ["icons/close.svg"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Identity of the toast stack; the ids of its four corner layers derive from it." },
            max_visible: #{ schema: #{ type: "integer", min: 1, max: 10 }, required: false, sensitive: false,
                "default": #{ type: "integer", value: 3 },
                doc: "How many items, counted from the start of `items` across all regions, show at once; the rest wait without a countdown." },
            dismiss_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Dismiss notification" },
                doc: "Accessible name of each toast's close button." },
            items: #{ schema: #{ type: "array", max_items: 100, items: #{
                type: "object", allow_unknown: false, fields: #{
                    id: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Identity of the toast, passed to `on_dismiss`; unique and not blank." },
                    title: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Bold first line and accessible name; not blank." },
                    message: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                        "default": #{ type: "string", value: "" },
                        doc: "Muted text under the title; empty shows none." },
                    variant: #{ schema: #{ type: "string", allowed: ["neutral", "success", "warning", "danger"] }, required: false, sensitive: false,
                        "default": #{ type: "string", value: "neutral" },
                        doc: "Tone of the toast: a status variant adds a colored square lamp, and `danger` is announced as an alert." },
                    region: #{ schema: #{ type: "string", allowed: ["top_left", "top_right", "bottom_left", "bottom_right"] }, required: false, sensitive: false,
                        "default": #{ type: "string", value: "top_right" },
                        doc: "Window corner the toast stacks in." },
                    duration_ms: #{ schema: #{ type: "integer", min: 1, max: 86400000 }, required: false, sensitive: false,
                        "default": #{ type: "integer", value: 5000 },
                        doc: "Milliseconds from showing until the toast asks to be dismissed; hovering it pauses the countdown." },
                    paused: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Holds the countdown while `true`; the toast stays until it is closed or `paused` turns `false`." },
                    dismissible: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: true },
                        doc: "Whether the toast has a close button." },
                },
            } }, required: true, sensitive: false,
                doc: "The notifications in display order; the caller removes an item when it is dismissed." },
            on_dismiss: #{ schema: #{ type: "callback" }, required: true, sensitive: false,
                doc: "Called with a toast's `id` when its countdown ends or its close button is pressed." },
        },
        state: #{ fields: #{} },
        events: #{ dismiss: #{ payload: #{ type: "string" },
            doc: "Emitted when a toast's countdown ends or its close button is pressed; the payload is the item's `id`." } },
        slots: #{},
        parts: ["root", "region", "toast", "toast_neutral", "toast_success",
            "toast_warning", "toast_danger", "title", "message", "close", "lamp"],
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
    if variant == "success" || variant == "warning" || variant == "danger" { theme_color(variant) }
    else { () }
}

// A toast is a raised overlay block with a hairline; status toasts carry a square lamp.
fn toast_node(ctx, item, dismiss_label) {
    let variant = prop_or(item, "variant", "neutral");
    let header = [];
    let lamp = variant_color(variant);
    if lamp != () {
        header.push(row([]).with_style(ctx.component_style("lamp", style()
            .width(px(6)).height(px(6)).background(lamp))));
    }
    header.push(text(item.title).with_style(ctx.component_style("title", style().flex_grow())));
    if prop_or(item, "dismissible", true) {
        header.push(row([image_source(asset("app/icons/close"))
                .with_style(style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon")))])
            .with_key(`${item.id}-close`)
            .with_style(ctx.component_style("close", style()
                .items_center().justify_center()
                .text_color(theme_color("text_muted"))
                .hover(style().background(theme_color("surface_hover")).text_color(theme_color("text_primary")))))
            .accessibility_role("button").accessibility_label(dismiss_label)
            .on_click_value(Fn("toast_manual_dismiss"), item.id));
    }
    let children = [row(header).with_style(style().items_center().gap(theme_spacing("sm"))
        .typography("body").font_weight(600))];
    if prop_or(item, "message", "") != "" {
        children.push(text(item.message).with_style(ctx.component_style("message",
            style().typography("body").text_color(theme_color("text_muted")))));
    }
    column(children).with_key(item.id)
        .with_style(ctx.component_style(`toast_${variant}`,
            ctx.component_style("toast",
                    style().width(px(320)).gap(theme_spacing("xxs")).padding(theme_length("metrics.inset"))
                    .border(px(1)).border_color(theme_color("border"))
                    .radius(theme_radius("lg")).background(theme_color("surface_raised"))
                    .text_color(theme_color("text_primary")))))
        .on_hover_value(Fn("toast_hover"), item.id)
        .accessibility_role(if variant == "danger" { "alert" } else { "status" })
        .accessibility_label(item.title)
}

fn region_layer(ctx, key, placement, children) {
    if children.len == 0 { return (); }
    layer(column(children).with_key(`${key}-column`)
        .with_style(ctx.component_style("region", style().width(px(320)).gap(theme_length("space.related")))),
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
        let node = toast_node(ctx, item, prop_or(props, "dismiss_label", "Dismiss notification"));
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
        let node = region_layer(ctx, spec.key, spec.placement, spec.children);
        if node != () { layers.push(node); }
    }
    box(layers).with_key(props.key)
        .with_style(ctx.component_style("root", style()))
}
