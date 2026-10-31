/* gpui-rhai
{
  "id": "components/toast",
  "export": "Toast",
  "version": "0.2.2",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "assets": ["icons/close.svg"],
  "tokens": [
    "surface_raised", "border", "surface_hover", "success", "warning", "danger",
    "text_primary", "text_muted", "metrics.inset", "metrics.icon", "spacing.xxs", "spacing.xs",
    "spacing.sm", "space.related", "radius.xs", "radius.sm", "radius.lg", "typography.body",
    "typography.caption", "focus_ring"
  ],
  "environment": ["density"]
}
*/
// Controlled Toast composition over public Layer, declarative timeout, atoms, and hover events.
// State: caller owns items; runtime timers own one-shot deadline and pause/resume state; the
// stack layout keeps which corner the pointer and keyboard focus are in.
// `layout: "stack"` piles each corner's toasts behind its newest, the last of that corner's
// items: up to two 6px edges peek out and a count marks the pile. Hovering the pile or moving
// keyboard focus into it expands it into a list, newest nearest the corner, under a heading
// with a Clear all button, and holds every countdown in that corner until pointer and focus
// leave; Escape collapses it.
// Example: Toast(#{ key: "toasts", items: items, on_dismiss: Fn("dismissed") })

define_component(#{
    metadata: #{
        id: "components/toast", "export": "Toast", version: "0.2.2",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 }, dependencies: [], capabilities: #{},
        tokens: [
            "surface_raised", "border", "surface_hover", "success", "warning", "danger",
            "text_primary", "text_muted", "metrics.inset", "metrics.icon", "spacing.xxs",
            "spacing.xs", "spacing.sm", "space.related", "radius.xs", "radius.sm", "radius.lg",
            "typography.body", "typography.caption", "focus_ring"
        ],
        environment: ["density"],
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
                doc: "Called with a toast's `id` when its countdown ends, its close button is pressed or Clear all dismisses its stack." },
            on_activate: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with a toast's `id` when it is clicked or Enter is pressed on it; without it a toast is not a button." },
            layout: #{ schema: #{ type: "string", allowed: ["flat", "stack"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "flat" },
                doc: "`flat` lists each corner's toasts in a column; `stack` piles them behind the newest and expands the pile while it is hovered or holds keyboard focus. In `stack` every item joins its corner's pile and counts down: `max_visible` applies to `flat` only." },
            heading: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Notifications" },
                doc: "Heading of an expanded stack, followed by the number of toasts in it; also names the pile." },
            clear_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Clear all" },
                doc: "Label of an expanded stack's button that dismisses every toast in its corner." },
        },
        state: #{ fields: #{
            hovered: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
            focused: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
            collapsed: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        } },
        events: #{
            dismiss: #{ payload: #{ type: "string" },
                doc: "Emitted when a toast's countdown ends, its close button is pressed or Clear all dismisses its stack; the payload is the item's `id`." },
            activate: #{ payload: #{ type: "string" },
                doc: "Emitted when a toast is clicked or Enter is pressed on it while `on_activate` is set; the payload is the item's `id`." },
        },
        slots: #{},
        parts: ["root", "region", "toast", "toast_neutral", "toast_success",
            "toast_warning", "toast_danger", "title", "message", "close", "lamp",
            "stack_edge", "stack_count", "stack_header", "stack_clear", "stack_list"],
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
fn toast_activate(ctx, id) {
    ctx.emit("activate", id);
    handled()
}
fn toast_hover(ctx, payload) {
    if payload.hovered { ctx.pause_timeout(timer_key(payload.value)); }
    else { ctx.resume_timeout(timer_key(payload.value)); }
    handled()
}

// A stack is held while the pointer is over it or keyboard focus is in it: it expands (until
// Escape) and every countdown in its corner pauses. The pointer leaving also lets go of focus
// a press left inside, so a pressed pile does not stay open once the pointer moves away.
fn stack_hover(ctx, payload) {
    let region = payload.value;
    if payload.hovered {
        stack_enter(ctx, "hovered", region);
    } else {
        if ctx.get_state("hovered") == region { ctx.set_state("hovered", ""); }
        if ctx.get_state("focused") == region { ctx.set_state("focused", ""); }
    }
    handled()
}
fn stack_focus(ctx, payload) {
    let region = payload.value;
    if payload.focused { stack_enter(ctx, "focused", region); }
    else if ctx.get_state("focused") == region { ctx.set_state("focused", ""); }
    handled()
}
fn stack_enter(ctx, field, region) {
    ctx.set_state(field, region);
    if ctx.get_state("collapsed") == region { ctx.set_state("collapsed", ""); }
    // A bottom corner's newest toast ends its list: scroll it into view.
    if region.starts_with("bottom") { ctx.scroll_into_view(newest_ref(region)); }
}
fn stack_escape(ctx, region) {
    if !stack_expanded(ctx, region) { return; }
    ctx.set_state("collapsed", region);
    handled()
}
fn stack_clear(ctx, ids) {
    for id in ids {
        ctx.cancel_timeout(timer_key(id));
        ctx.emit("dismiss", id);
    }
    handled()
}
fn stack_held(ctx, region) {
    ctx.get_state("hovered") == region || ctx.get_state("focused") == region
}
fn stack_expanded(ctx, region) {
    stack_held(ctx, region) && ctx.get_state("collapsed") != region
}
fn newest_ref(region) { `newest-${region}` }

fn variant_color(variant) {
    if variant == "success" || variant == "warning" || variant == "danger" { theme_color(variant) }
    else { () }
}

// A toast is a raised overlay block with a hairline; status toasts carry a square lamp. With
// `on_activate` it is also a button, framed while it has focus.
fn toast_node(ctx, item, dismiss_label, activatable, hover_pause) {
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
                .items_center().justify_center().radius(theme_radius("xs"))
                .border(px(1)).border_color(rgba(0x00000000))
                .text_color(theme_color("text_muted"))
                .hover(style().background(theme_color("surface_hover")).text_color(theme_color("text_primary")))
                .focus(style().border_color(theme_color("focus_ring")))))
            .accessibility_role("button").accessibility_label(dismiss_label)
            .on_click_value(Fn("toast_manual_dismiss"), item.id));
    }
    let children = [row(header).with_style(style().items_center().gap(theme_spacing("sm"))
        .typography("body").font_weight(600))];
    if prop_or(item, "message", "") != "" {
        children.push(text(item.message).with_style(ctx.component_style("message",
            style().typography("body").text_color(theme_color("text_muted")))));
    }
    let card_style = style().width(px(320)).gap(theme_spacing("xxs")).padding(theme_length("metrics.inset"))
        .border(px(1)).border_color(theme_color("border"))
        .radius(theme_radius("lg")).background(theme_color("surface_raised"))
        .text_color(theme_color("text_primary"));
    if activatable { card_style = card_style.focus(style().border_color(theme_color("focus_ring"))); }
    let card = column(children).with_key(item.id)
        .with_style(ctx.component_style(`toast_${variant}`, ctx.component_style("toast", card_style)))
        .accessibility_role(if variant == "danger" { "alert" } else { "status" })
        .accessibility_label(item.title);
    if hover_pause { card = card.on_hover_value(Fn("toast_hover"), item.id); }
    if activatable { card = card.on_click_value(Fn("toast_activate"), item.id); }
    card
}

fn region_layer(ctx, key, placement, children) {
    if children.len == 0 { return (); }
    layer(column(children).with_key(`${key}-column`)
        .with_style(ctx.component_style("region", style().width(px(320)).gap(theme_length("space.related")))),
        #{ id: key, placement: placement, inset: 12, priority: 9000 })
}

// One corner's pile. Every toast stays in one keyed list, collapsed or expanded, so a toast or
// close button keeps focus while the pile opens; collapsed, only the newest shows.
fn stack_layer(ctx, props, key, region, items) {
    if items.len == 0 { return (); }
    let bottom = region.starts_with("bottom");
    let expanded = stack_expanded(ctx, region);
    let count = items.len;
    let newest = count - 1;
    let dismiss_label = prop_or(props, "dismiss_label", "Dismiss notification");
    let activatable = "on_activate" in props && props.on_activate != ();
    let heading = prop_or(props, "heading", "Notifications");
    // Nearest the corner: a bottom corner lists oldest to newest, a top corner the reverse.
    let ordered = [];
    let ids = [];
    for index in 0..count {
        ordered.push(if bottom { items[index] } else { items[newest - index] });
        ids.push(items[index].id);
    }
    let cards = [];
    for index in 0..count {
        let card = toast_node(ctx, ordered[index], dismiss_label, activatable, false);
        if index == (if bottom { newest } else { 0 }) {
            card = card.with_ref(element_ref(newest_ref(region)));
        } else if !expanded {
            card = card.with_style(style().hidden());
        }
        cards.push(card);
    }
    let list_style = style().flex_col().gap(theme_length("space.related"));
    if expanded { list_style = list_style.max_height(px(480)).overflow_y_scroll(); }
    let list = column(cards).with_key(`${key}-list`)
        .with_style(ctx.component_style("stack_list", list_style));

    let children = [];
    // Up to two edges peek out 6px behind the newest toast, farther and narrower first so the
    // nearer one covers it; each runs 8px under the next, past any `radius.lg`.
    let peek = if expanded { 0 } else if count > 2 { 2 } else { count - 1 };
    let depth = peek;
    while depth > 0 {
        let edge_style = style().absolute().left(px(8 * depth)).right(px(8 * depth)).height(px(14))
            .border(px(1)).border_color(theme_color("border")).radius(theme_radius("lg"))
            .background(theme_color("surface_raised"));
        edge_style = if bottom { edge_style.top(px(6 * (peek - depth))) }
            else { edge_style.bottom(px(6 * (peek - depth))) };
        children.push(box([]).with_key(`${key}-edge-${depth}`)
            .with_style(ctx.component_style("stack_edge", edge_style)));
        depth -= 1;
    }
    let header = ();
    if expanded {
        let clear_label = prop_or(props, "clear_label", "Clear all");
        let clear = row([text(clear_label)]).with_key(`${key}-clear`)
            .with_style(ctx.component_style("stack_clear", style()
                .items_center().padding_x(theme_spacing("xs")).radius(theme_radius("xs"))
                .border(px(1)).border_color(rgba(0x00000000))
                .hover(style().background(theme_color("surface_hover")).text_color(theme_color("text_primary")))
                .focus(style().border_color(theme_color("focus_ring")))))
            .accessibility_role("button").accessibility_label(clear_label)
            .on_click_value(Fn("stack_clear"), ids);
        header = row([text(heading), text(`${count}`),
                box([]).with_style(style().flex_grow()), clear])
            .with_key(`${key}-header`)
            .with_style(ctx.component_style("stack_header", style()
                .items_center().gap(theme_spacing("xs"))
                .typography("caption").text_color(theme_color("text_muted"))));
    }
    // Collapsed, the edges take the room a spacer keeps beside the newest toast.
    let peek_room = if peek > 0 { box([]).with_key(`${key}-peek`).with_style(style().height(px(6 * peek))) }
        else { () };
    let before = if bottom { [peek_room, header] } else { [] };
    let after = if bottom { [] } else { [header, peek_room] };
    for node in before { if node != () { children.push(node); } }
    children.push(list);
    for node in after { if node != () { children.push(node); } }
    // The count marks the newest toast's top end corner.
    if !expanded && count > 1 {
        let count_style = style().absolute().inset_end(px(8)).padding_x(theme_spacing("xs"))
            .radius(theme_radius("sm")).background(theme_color("border"))
            .text_color(theme_color("text_muted")).typography("caption");
        count_style = count_style.top(offset_px(if bottom { 6 * peek - 8 } else { -8 }));
        children.push(row([text(`${count}`)]).with_key(`${key}-count`)
            .with_style(ctx.component_style("stack_count", count_style)));
    }
    let pile_style = style().width(px(320)).relative();
    if expanded { pile_style = pile_style.gap(theme_length("space.related")); }
    let pile = column(children).with_key(`${key}-column`)
        .with_style(ctx.component_style("region", pile_style))
        .accessibility_role("group").accessibility_label(`${heading} ${count}`)
        .on_hover_value(Fn("stack_hover"), region)
        .on_focus_within_value(Fn("stack_focus"), region)
        .on_key_value("escape", Fn("stack_escape"), region);
    layer(pile, #{ id: key, placement: region, inset: 12, priority: 9000 })
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

    let stack = prop_or(props, "layout", "flat") == "stack";
    let activatable = "on_activate" in props && props.on_activate != ();
    let regions = #{ top_left: [], top_right: [], bottom_left: [], bottom_right: [] };
    // A flat layout shows the first `max_visible` items; a stack holds every item.
    let visible = props.items.len;
    if !stack && visible > prop_or(props, "max_visible", 3) { visible = prop_or(props, "max_visible", 3); }
    for index in 0..visible {
        let item = props.items[index];
        let region = prop_or(item, "region", "top_right");
        let paused = prop_or(item, "paused", false) || stack && stack_held(ctx, region);
        timeout(timer_key(item.id), prop_or(item, "duration_ms", 5000), paused,
            Fn("toast_timeout"), item.id);
        if stack {
            regions[region].push(item);
        } else {
            regions[region].push(toast_node(ctx, item,
                prop_or(props, "dismiss_label", "Dismiss notification"), activatable, true));
        }
    }
    let layers = [];
    for spec in [
        #{ key: `${props.key}-top-left`, region: "top_left" },
        #{ key: `${props.key}-top-right`, region: "top_right" },
        #{ key: `${props.key}-bottom-left`, region: "bottom_left" },
        #{ key: `${props.key}-bottom-right`, region: "bottom_right" },
    ] {
        let node = if stack { stack_layer(ctx, props, spec.key, spec.region, regions[spec.region]) }
            else { region_layer(ctx, spec.key, spec.region, regions[spec.region]) };
        if node != () { layers.push(node); }
    }
    box(layers).with_key(props.key)
        .with_style(ctx.component_style("root", style()))
}
