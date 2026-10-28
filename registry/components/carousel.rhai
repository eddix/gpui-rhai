/* gpui-rhai
{
  "id": "components/carousel",
  "export": "Carousel",
  "version": "0.3.1",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/sliding_panel", "components/icon_button", "components/icon"],
  "capabilities": {},
  "assets": ["icons/chevron_left.svg", "icons/chevron_right.svg"],
  "tokens": [
    "text_primary", "text_muted", "control.border", "surface_raised", "focus_ring",
    "metrics.control", "metrics.icon", "spacing.xxs", "spacing.xs", "space.related",
    "radius.sm", "radius.md", "typography.control"
  ],
  "environment": ["size", "density", "corners"]
}
*/
// A controlled pager that shows one item at a time, for a compact place that steps through
// items of one kind: alerts, cards, previews. The caller owns `index`; the arrows, the dots
// and the keys propose another one. It never turns by itself.
//
// `header` puts the label and the controls in a row above the item; `overlay` lays the
// arrows over the item's sides, shown while the item is hovered or the controls have focus,
// with the position below it. The position is a count (`2 / 5`), dots or nothing.
//
// The controls are one tab stop beside the item, never around it, so keys typed in an item
// stay the item's: Left and Right step, Home and End go to the ends. The arrows and the dots
// are reached from it and are not tab stops. A new item slides in from the side it comes
// from (`SlidingPanel`).
//
// Example: Carousel(#{ key: "alerts", label: "Active alerts", index: current,
//              items: [#{ label: "Disk full", content: disk }, #{ label: "CPU high", content: cpu }],
//              on_change: Fn("shown") })

import "components/sliding_panel" as sliding_panel;
import "components/icon_button" as icon_button;
import "components/icon" as icon;

define_component(#{
    metadata: #{
        id: "components/carousel", "export": "Carousel", version: "0.3.1",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/sliding_panel", "components/icon_button", "components/icon"],
        capabilities: #{},
        tokens: [
            "text_primary", "text_muted", "control.border", "surface_raised", "focus_ring",
            "metrics.control", "metrics.icon", "spacing.xxs", "spacing.xs", "space.related",
            "radius.sm", "radius.md", "typography.control"
        ],
        environment: ["size", "density", "corners"],
        assets: ["icons/chevron_left.svg", "icons/chevron_right.svg"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Stable identity of the carousel, which names its sliding panel and keeps its focus state." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the carousel and its controls; the `header` layout also shows it as the title." },
            index: #{ schema: #{ type: "integer", min: 0 }, required: true, sensitive: false,
                doc: "Position of the shown item, from 0; the caller stores the `change` payload and passes it back." },
            items: #{ schema: #{ type: "array", max_items: 64, items: #{
                type: "object", allow_unknown: false, fields: #{
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Name of the item: its accessible name, its dot's, and part of the controls' value." },
                    content: #{ schema: #{ type: "node" }, required: true, sensitive: false,
                        doc: "The item, owned by the caller; only the shown one renders." },
                },
            } }, required: true, sensitive: false, doc: "The items in order; at least one." },
            variant: #{ schema: #{ type: "string", allowed: ["header", "overlay"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "header" },
                doc: "`header`: the label and the controls in a row above the item. `overlay`: arrows over the item's sides, shown while it is hovered or the controls have focus, and the position below it." },
            indicator: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["count", "dots", "none"] } },
                required: false, sensitive: false,
                doc: "How the position shows: `count` (`2 / 5`), `dots` (one per item, which a press selects) or `none`; unset, `count` for `header` and `dots` for `overlay`." },
            wrap: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Whether stepping past the last item goes to the first and back; without it the arrow at an end is disabled, or left out in `overlay`." },
            label_visible: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Whether the `header` layout shows the label as its title; it stays the accessible name." },
            previous_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Previous" },
                doc: "Accessible name of the previous arrow." },
            next_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Next" },
                doc: "Accessible name of the next arrow." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false, doc: "Overrides the `size` environment for the carousel." },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the proposed `index` when an arrow, a dot or a key steps; without it the controls are inert." },
        },
        state: #{ fields: #{
            focus_controls: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
            focus_arrows: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
        } },
        events: #{
            change: #{ payload: #{ type: "integer" },
                doc: "Emitted when an arrow, a dot or a key proposes another item; the payload is its `index`." },
        },
        slots: #{},
        parts: ["root", "header", "title", "controls", "focus_frame", "previous", "next", "count",
            "dots", "dot", "dot_selected", "stage", "arrows", "viewport", "item"],
    },
    render: Fn("render_Carousel")
});

fn Carousel(props) { render_component("components/carousel", props) }
fn prop_or(props, name, fallback) { if name in props && props[name] != () { props[name] } else { fallback } }

// The index `step` places from `index`, wrapping in a loop; -1 past an end, or with one item.
fn step_index(index, count, step, wrap) {
    if count < 2 { return -1; }
    let next = index + step;
    if wrap { (next + count) % count } else if next < 0 || next >= count { -1 } else { next }
}

fn pick(target, ctx, payload) { ctx.emit("change", target); }
fn pick_value(ctx, value) { ctx.emit("change", value); }
fn key_pick(ctx, value) { ctx.emit("change", value); handled() }
// Overlay arrows show while the controls or an arrow holds focus.
fn focus_change(ctx, payload) { ctx.set_state(`focus_${payload.value}`, payload.focused); }

// The keys of the controls: Left and Right step, Home and End go to the ends.
fn carousel_keys(node, props, cfg) {
    if !cfg.selects { return node; }
    let previous = step_index(props.index, cfg.count, -1, cfg.wrap);
    let next = step_index(props.index, cfg.count, 1, cfg.wrap);
    if previous >= 0 { node = node.on_key_value("left", Fn("key_pick"), previous); }
    if next >= 0 { node = node.on_key_value("right", Fn("key_pick"), next); }
    node.on_key_value("home", Fn("key_pick"), 0).on_key_value("end", Fn("key_pick"), cfg.count - 1)
}

// A previous (`step` -1) or next arrow; reached from the controls, not a tab stop itself.
fn arrow(ctx, props, cfg, step) {
    let target = step_index(props.index, cfg.count, step, cfg.wrap);
    let back = step < 0;
    let part = if back { "previous" } else { "next" };
    let glyph = icon::Icon(#{
        source: asset(if back { "app/icons/chevron_left" } else { "app/icons/chevron_right" }),
        rtl_source: asset(if back { "app/icons/chevron_right" } else { "app/icons/chevron_left" }) });
    let config = #{ key: part, icon: glyph,
        label: if back { prop_or(props, "previous_label", "Previous") } else { prop_or(props, "next_label", "Next") },
        variant: if cfg.overlay { "outline" } else { "ghost" },
        disabled: target < 0 || !cfg.selects };
    if target >= 0 && cfg.selects { config.on_click = Fn("pick").curry(target); }
    let edge = style().flex_shrink(false);
    // Over the item an arrow needs a surface of its own.
    if cfg.overlay { edge = edge.background(theme_color("surface_raised")); }
    icon_button::IconButton(config).tab_stop(false).with_style(ctx.component_style(part, edge))
}

// The position as `2 / 5` in tabular figures. The widest position is laid out unpainted at
// zero height under it, as Button does with its loading text, so the box keeps one width at
// every index and the arrows beside it never move.
fn count_node(ctx, props, cfg) {
    column([
        text(`${props.index + 1} / ${cfg.count}`),
        text(`${cfg.count} / ${cfg.count}`).with_style(style().height(px(0)).invisible())
            .accessibility_role("presentation"),
    ]).with_key("count").with_style(ctx.component_style("count", style().items_center()
        .whitespace_nowrap().typography("control").font_feature("tnum", 1)
        .text_color(theme_color("text_muted"))))
}

// One dot per item in an icon-sized target; a press on another dot selects its item.
fn dots_node(ctx, props, cfg) {
    let dots = [];
    for index in 0..cfg.count {
        let selected = index == props.index;
        // The 6px square is the status-lamp constant; the selected dot stretches to an icon's
        // width, so the position does not rest on color alone.
        let mark = style().width(if selected { theme_length("metrics.icon") } else { px(6) }).height(px(6))
            .radius(theme_radius("sm"))
            .background(if selected { theme_color("text_primary") } else { theme_color("control.border") });
        if !selected { mark = mark.group_hover(style().background(theme_color("text_muted"))); }
        let resolved = ctx.component_style("dot", mark);
        if selected { resolved = ctx.component_style("dot_selected", resolved); }
        let dot = row([row([]).with_key("mark").with_style(resolved)]).with_key(`dot:${index}`)
            .tab_stop(false).accessibility_role("button").accessibility_label(props.items[index].label)
            .with_style(style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
                .flex_shrink(false).items_center().justify_center().hover(style()));
        if selected { dot = dot.accessibility_current("true"); }
        else if cfg.selects { dot = dot.on_click_value(Fn("pick_value"), index); }
        dots.push(dot);
    }
    // The targets touch: the space between the dots is their own.
    row(dots).with_key("dots").with_style(ctx.component_style("dots", style().items_center()))
}

// The controls' focus frame, painted last, so it shows over the arrows and dots it surrounds.
fn focus_frame(ctx) {
    row([]).with_key("focus_frame").with_style(ctx.component_style("focus_frame", style()
        .absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0)).radius(theme_radius("md"))
        .border(px(2)).border_color(rgba(0x00000000))
        .group_focus(style().border_color(theme_color("focus_ring")))))
}

// The one tab stop, beside the item and never around it.
fn controls_node(ctx, props, cfg, children) {
    children.push(focus_frame(ctx));
    let shown = props.items[props.index].label;
    let layout = style().relative().flex_shrink(false).items_center().gap(theme_spacing("xxs")).focus(style());
    // A header row and a count or dots are a control tall; overlay controls with no position
    // take no room, and the arrows they reveal show their focus.
    layout = if cfg.overlay && cfg.indicator == "none" { layout.self_center() }
        else if cfg.overlay { layout.self_center().min_height(theme_length("metrics.control")) }
        else { layout.min_height(theme_length("metrics.control")) };
    let node = row(children).with_key("controls").tab_stop(true)
        .accessibility_role("group").accessibility_label(props.label)
        .accessibility_value(`${shown}, ${props.index + 1} / ${cfg.count}`)
        .with_style(ctx.component_style("controls", layout));
    if cfg.overlay { node = node.on_focus_within_value(Fn("focus_change"), "controls"); }
    carousel_keys(node, props, cfg)
}

fn indicator_node(ctx, props, cfg) {
    if cfg.indicator == "count" { count_node(ctx, props, cfg) }
    else if cfg.indicator == "dots" { dots_node(ctx, props, cfg) }
    else { () }
}

// Arrows over the item's sides, shown while it is hovered or the controls have focus, and the
// controls below it.
fn overlay_layout(ctx, props, cfg, viewport, indicator) {
    // An arrow at an end is left out; the empty box keeps the other one on its side.
    let sides = [];
    for step in [-1, 1] {
        let target = step_index(props.index, cfg.count, step, cfg.wrap);
        sides.push(if target >= 0 && cfg.selects { arrow(ctx, props, cfg, step) }
            else { box([]).with_key(if step < 0 { "previous" } else { "next" }) });
    }
    let visible = ctx.get_state("focus_controls") || ctx.get_state("focus_arrows");
    let layer = style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
        .items_center().justify_between().padding_x(theme_spacing("xs"))
        .opacity(if visible { 1.0 } else { 0.0 }).group_hover(style().opacity(1.0));
    let arrows = row(sides).with_key("arrows").on_focus_within_value(Fn("focus_change"), "arrows")
        .with_style(ctx.component_style("arrows", layer));
    // The stage is the hover group the arrows show for.
    let stage = column([viewport, carousel_keys(arrows, props, cfg)]).with_key("stage")
        .with_style(ctx.component_style("stage", style().relative().min_width(px(0)).hover(style())));
    column([stage, controls_node(ctx, props, cfg, if indicator == () { [] } else { [indicator] })])
}

// The title and the controls in a row above the item.
fn header_layout(ctx, props, cfg, viewport, indicator) {
    let title = if prop_or(props, "label_visible", true) {
        text(props.label).with_key("title").with_style(ctx.component_style("title", style()
            .flex_grow().min_width(px(0)).whitespace_nowrap().text_ellipsis().typography("control")
            .text_color(theme_color("text_primary"))))
    } else { box([]).with_key("title").with_style(style().flex_grow()) };
    let controls = [arrow(ctx, props, cfg, -1)];
    if indicator != () { controls.push(indicator); }
    controls.push(arrow(ctx, props, cfg, 1));
    // The header leads the item as a heading does, so its space is its own padding rather
    // than a gap: the item's spacing answers to the space around the carousel.
    let header = row([title, controls_node(ctx, props, cfg, controls)]).with_key("header")
        .with_style(ctx.component_style("header", style().items_center().min_width(px(0))
            .gap(theme_length("space.related")).padding_bottom(theme_length("space.related"))));
    column([header, viewport])
}

fn render_Carousel(ctx, props) {
    let count = props.items.len;
    if count == 0 { throw "Carousel needs at least one item"; }
    if props.index >= count { throw `Carousel index ${props.index} is past its ${count} items`; }
    let variant = prop_or(props, "variant", "header");
    let overlay = variant == "overlay";
    let cfg = #{ count: count, overlay: overlay, wrap: prop_or(props, "wrap", false),
        indicator: prop_or(props, "indicator", if overlay { "dots" } else { "count" }),
        selects: "on_change" in props && props.on_change != () };

    // Only the shown item renders; a new one slides in from the side it comes from.
    let item = props.items[props.index];
    let shown = column([item.content]).with_key(`item:${props.index}`)
        .accessibility_role("group").accessibility_label(item.label)
        .with_style(ctx.component_style("item", style().min_width(px(0))));
    let order = [];
    for index in 0..count { order.push(`${index}`); }
    let viewport = sliding_panel::SlidingPanel(#{ key: `${props.key}-items`, value: `${props.index}`,
        order: order, wrap: cfg.wrap, content: shown })
        .with_style(ctx.component_style("viewport", style().min_width(px(0))));
    let indicator = indicator_node(ctx, props, cfg);

    let root = if overlay { overlay_layout(ctx, props, cfg, viewport, indicator) }
        else { header_layout(ctx, props, cfg, viewport, indicator) };
    root = root.with_style(ctx.component_style("root", style().min_width(px(0))))
        .accessibility_role("group").accessibility_label(props.label);
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
