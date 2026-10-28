/* gpui-rhai
{
  "id": "components/tabs",
  "export": "Tabs",
  "version": "0.3.1",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/tab_overflow_menu", "components/icon_button", "components/sliding_panel"],
  "capabilities": {},
  "assets": ["icons/close.svg", "icons/plus.svg"],
  "tokens": [
    "surface_hover", "surface_raised", "focus_ring", "text_primary", "tabs.foreground",
    "tabs.counter", "control.hover", "border", "disabled", "tabbar.background", "tabbar.active",
    "accent", "on_accent", "warning", "on_warning", "danger", "on_danger",
    "metrics.control", "metrics.control_pad", "metrics.icon", "metrics.inset",
    "metrics.label_column", "metrics.counter", "spacing.xxs", "spacing.xs", "spacing.sm",
    "space.related", "space.group", "radius.sm", "radius.md", "radius.lg",
    "typography.control", "typography.counter", "typography.body"
  ],
  "environment": ["size", "density", "corners"]
}
*/
// Controlled tabs with one tab stop; the arrow keys select. Three looks share every
// capability: `track` is a tonal track with a raised thumb, for a compact switcher among
// other controls; `underline` puts the labels on a hairline with a 2px ink bar under the
// selected one, for the sections of a panel or a card; `bar` is a strip of document tabs
// whose selected tab takes the panel's surface (`tabbar.active`) and covers its line.
//
// Example: Tabs(#{ value: "overview", label: "Sections", tabs: tabs, on_change: Fn("changed") })
//
// A tab carries only what the component draws: an icon, the label, a count capsule or a
// tone lamp, a dirty mark and a close button. None of them is a tab stop; the list reaches
// each one: Left/Right (Up/Down when vertical), Home and End select, Delete or Backspace
// closes the selected tab, Alt+Arrow moves it when the tabs are reorderable, and Shift+F10
// or the menu key asks for a context menu. Actions that are not about one tab go in `start`
// and `end`, outside the list; `on_add` draws a "+" after the last tab. Tabs keep their width
// and the strip scrolls when they do not fit, keeping the selected tab revealed. With
// `panel_motion: "slide"` a newly selected panel slides in from the side of the tab it replaces.

import "components/tab_overflow_menu" as tab_overflow_menu;
import "components/icon_button" as icon_button;
import "components/sliding_panel" as sliding_panel;

define_component(#{
    metadata: #{
        id: "components/tabs", "export": "Tabs", version: "0.3.1",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/tab_overflow_menu", "components/icon_button", "components/sliding_panel"],
        capabilities: #{},
        tokens: [
            "surface_hover", "surface_raised", "focus_ring", "text_primary", "tabs.foreground",
            "tabs.counter", "control.hover", "border", "disabled", "tabbar.background", "tabbar.active",
            "accent", "on_accent", "warning", "on_warning", "danger", "on_danger",
            "metrics.control", "metrics.control_pad", "metrics.icon", "metrics.inset",
            "metrics.label_column", "metrics.counter", "spacing.xxs", "spacing.xs", "spacing.sm",
            "space.related", "space.group", "radius.sm", "radius.md", "radius.lg",
            "typography.control", "typography.counter", "typography.body"
        ],
        environment: ["size", "density", "corners"],
        assets: ["icons/close.svg", "icons/plus.svg"],
    },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                doc: "Stable identity of the tabs; required with `reorderable`, `overflow_menu` or a sliding `panel_motion`, whose drag list, menu and sliding panel it names." },
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "The `value` of the selected tab, or one no tab has (such as `\"\"`) for none; the caller stores the `change` payload and passes it back." },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the tab list, and of the group that holds the list and the panel." },
            variant: #{ schema: #{ type: "string", allowed: ["track", "underline", "bar"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "track" },
                doc: "`track` is a tonal track with a raised thumb; `underline` puts the labels on a hairline and marks the selected one with a 2px ink bar; `bar` is a strip of document tabs whose selected tab joins the panel under it. `underline` and `bar` are horizontal only." },
            orientation: #{ schema: #{ type: "string", allowed: ["horizontal", "vertical"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "horizontal" },
                doc: "Direction of the list: `horizontal` is a row moved with Left/Right, `vertical` a column beside the panel moved with Up/Down." },
            layout: #{ schema: #{ type: "string", allowed: ["content", "equal"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "content" },
                doc: "Tab widths of a horizontal list: `content` fits each label and scrolls when the tabs do not fit, `equal` fills the width and shares it equally." },
            motion_key: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "Shared-layout id, at most 128 characters, that slides the selected indicator between tabs; empty turns it off." },
            tabs: #{ schema: #{ type: "array", max_items: 256, items: #{
                type: "object", allow_unknown: false, fields: #{
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Unique business identity of the tab, including the empty string; matched against `value` and reported unchanged by every callback." },
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Tab text and accessible name; past twice `metrics.label_column` the label truncates." },
                    icon: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Node drawn at icon size before the label." },
                    label_visible: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: true },
                        doc: "Whether the label text shows; a tab that hides it needs an `icon` and keeps the label as its name." },
                    count: #{ schema: #{ type: "optional", value: #{ type: "integer", min: 0 } }, required: false, sensitive: false,
                        doc: "Number shown in a capsule after the label, `99+` past 99; zero shows too. The full number joins the accessible name." },
                    tone: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["accent", "warning", "danger"] } }, required: false, sensitive: false,
                        doc: "Fills the count capsule with the tone; without a `count` it shows a 6px lamp after the label." },
                    tone_label: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Accessible text for the tone, such as `Has updates`; it joins the accessible name, since a lamp has no text." },
                    closable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Whether the tab has a close button, shown while selected or hovered; a middle press closes it, and Delete closes it while selected." },
                    dirty: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Marks unsaved changes with a square; on a closable tab it turns into the close button under the pointer." },
                    content: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                        doc: "Caller-owned panel. Eagerly constructed components keep Rhai state/effects even when omitted (ADR 0019); keep_mounted also preserves native scroll/editing state." },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Whether the tab ignores the pointer; the arrow keys skip it." },
                },
            } }, required: true, sensitive: false, doc: "The tabs in list order; empty is a strip with no tabs." },
            panel: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Whether the selected tab's `content` renders; `false` renders only the strip, for view switchers." },
            keep_mounted: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Retains all supplied panels, including native scrolling, caret/selection/undo and focus identity. Hidden panels have no layout, input or presented accessibility. Business effects keep running; use ctx.is_visible() for an explicit application pause policy." },
            panel_motion: #{ schema: #{ type: "string", allowed: ["none", "slide"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "none" },
                doc: "`slide` brings a newly selected panel in from the side of the tab it replaces, a short way as it fades in (from above or below when vertical); the panel it replaces is gone at once, and the Host's motion policy settles it. `none` switches at once. `slide` needs `key`." },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false, doc: "Overrides the `size` environment for the tabs." },
            start: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Nodes before the list (above it when vertical), outside it: each keeps its own tab stop." },
            end: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Nodes at the far end of the strip, after the add button and the overflow menu." },
            overflow_menu: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Whether a menu button lists every tab; picking one selects it. Needs `key`." },
            menu_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "All tabs" },
                doc: "Accessible name of the overflow menu button and its menu." },
            close_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Close" },
                doc: "Accessible name of a close button, followed by the tab's label." },
            add_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Add tab" },
                doc: "Accessible name of the add button." },
            reorderable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Whether tabs move by dragging or Alt+Arrow; moves are reported through `on_reorder`. Needs `key`." },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with a tab's `value` when it is clicked, reached with an arrow key, Home or End, or picked in the overflow menu; without it the tabs do not select." },
            on_close: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `#{ value, next }` when a close button, a middle press or Delete closes a tab; `next` is the tab to select instead (see `close_successor`). The caller removes the tab." },
            on_add: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Draws a \"+\" button after the last tab, pinned at the strip's end when the tabs overflow; called when it is pressed." },
            on_reorder: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `#{ value, anchor, placement }` when a tab is dropped or moved by key; the caller reorders `tabs`." },
            on_context_request: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `#{ value, anchor, source }` on a right press, Shift+F10 or the menu key; without it no menu is asked for." },
        },
        state: #{ fields: #{} },
        events: #{
            change: #{ payload: #{ type: "string" },
                doc: "Emitted when a tab is clicked, reached by key or picked in the overflow menu; the payload is its `value`." },
            close: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "The `value` of the tab to close." },
                next: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "The enabled tab after it, else the one before it, else `\"\"`." },
            } }, doc: "Emitted when a close button, a middle press or Delete closes a tab." },
            add: #{ payload: #{ type: "null" }, doc: "Emitted when the add button is pressed." },
            context_request: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "The `value` of the tab the menu is for." },
                anchor: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                    x: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                        doc: "Left edge of the anchor in window coordinates, in logical pixels." },
                    y: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                        doc: "Top edge of the anchor in window coordinates, in logical pixels." },
                    width: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                        doc: "Width of the anchor in logical pixels; zero for a pointer request." },
                    height: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                        doc: "Height of the anchor in logical pixels; zero for a pointer request." },
                } }, required: true, sensitive: false,
                    doc: "Where to open the menu: the pointer position, or the selected tab's bounds for a keyboard request." },
                source: #{ schema: #{ type: "string", allowed: ["pointer", "keyboard"] }, required: true, sensitive: false,
                    doc: "How the menu was asked for: `pointer` for a right press, `keyboard` for Shift+F10 or the menu key." },
            } }, doc: "Emitted on a right press over a tab, or Shift+F10 or the menu key on the selected tab." },
            reorder: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                value: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "The `value` of the moved tab." },
                anchor: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                    doc: "The `value` of the tab it lands beside." },
                placement: #{ schema: #{ type: "string", allowed: ["before", "after"] }, required: true, sensitive: false,
                    doc: "Whether the moved tab goes `before` or `after` the anchor tab." },
            } }, doc: "Emitted when a tab is dropped in a new place or moved with Alt+Arrow." },
        },
        slots: #{
            start: #{ required: false, multiple: true, doc: "Nodes before the list." },
            end: #{ required: false, multiple: true, doc: "Nodes at the far end of the strip." },
        },
        parts: ["root", "strip", "list", "line", "indicator", "focus_frame", "tab", "tab_selected",
            "label", "icon", "counter", "lamp", "slot", "close", "dirty", "add", "menu", "start", "end",
            "panel"],
        effects: ["reveal", "reveal_order"],
    },
    render: Fn("render_Tabs")
});

fn Tabs(props) { render_component("components/tabs", props) }
fn prop_or(props, name, fallback) { if name in props && props[name] != () { props[name] } else { fallback } }
fn tab_flag(tab, name) { if name in tab { tab[name] } else { false } }
fn tab_value_or(tab, name) { if name in tab { tab[name] } else { () } }

// The enabled tab after `value`, else the one before it, else "": what to select when
// `value` closes. Exported for callers that close a tab from outside the strip.
fn close_successor(tabs, value) {
    let at = -1;
    for index in 0..tabs.len { if tabs[index].value == value { at = index; } }
    if at < 0 { return ""; }
    for index in (at + 1)..tabs.len { if !tab_flag(tabs[index], "disabled") { return tabs[index].value; } }
    let index = at - 1;
    while index >= 0 {
        if !tab_flag(tabs[index], "disabled") { return tabs[index].value; }
        index -= 1;
    }
    ""
}

// The enabled tab `step` places from `value`, wrapping; `value` itself when none is enabled.
fn step_tab(tabs, value, step) {
    if tabs.len == 0 { return value; }
    let current = 0;
    for index in 0..tabs.len { if tabs[index].value == value { current = index; } }
    for offset in 1..(tabs.len + 1) {
        let index = if step > 0 { (current + offset) % tabs.len }
            else { (current + tabs.len - (offset % tabs.len)) % tabs.len };
        if !tab_flag(tabs[index], "disabled") { return tabs[index].value; }
    }
    value
}

fn select_tab(ctx, value) { ctx.emit("change", value); }
fn key_select(ctx, value) { ctx.emit("change", value); handled() }
fn close_tab(ctx, payload) { ctx.emit("close", payload); handled() }
fn add_tab(ctx, payload) { ctx.emit("add", ()); }
fn drag_reorder(ctx, payload) {
    ctx.emit("reorder", #{ value: payload.source_key, anchor: payload.anchor_key,
        placement: payload.placement });
}
fn key_reorder(ctx, payload) { ctx.emit("reorder", payload); handled() }
// A middle press closes a closable tab; a right press asks for a context menu.
fn tab_pointer(target, ctx, event) {
    if event.button == "middle" && target.close != () {
        ctx.emit("close", target.close);
        return event_response().prevent_default().stop();
    }
    if event.button == "right" && target.context {
        ctx.emit("context_request", #{ value: target.value, source: "pointer",
            anchor: #{ x: event.window.x, y: event.window.y, width: 0.0, height: 0.0 } });
        return event_response().prevent_default().stop();
    }
    propagate()
}
// Shift+F10 or the menu key: the selected tab's bounds are the anchor.
fn key_context(ctx, value) {
    let bounds = ctx.element_bounds("selected-tab");
    let anchor = if bounds == () { #{ x: 0.0, y: 0.0, width: 0.0, height: 0.0 } }
        else { #{ x: bounds.visual.x, y: bounds.visual.y, width: bounds.visual.width,
            height: bounds.visual.height } };
    ctx.emit("context_request", #{ value: value, anchor: anchor, source: "keyboard" });
    handled()
}
fn menu_pick(ctx, value) { ctx.emit("change", value); }
// Keep the selected tab in view after the selection changes.
fn reveal_start(ctx, deps) { ctx.scroll_into_view("selected-tab"); }
fn reveal_stop(ctx, deps) {}
// After the caller reorders, adds or removes tabs, keep the selected tab in view if it was in
// view: the bounds are still the last frame's. A strip scrolled away from it by hand stays.
fn reveal_order_start(ctx, deps) {
    let tab = ctx.element_bounds("selected-tab");
    let list = ctx.element_bounds("tab-list");
    if tab == () || list == () { ctx.scroll_into_view("selected-tab"); return; }
    let seen = if deps.vertical {
        tab.visual.y < list.visual.y + list.visual.height && tab.visual.y + tab.visual.height > list.visual.y
    } else {
        tab.visual.x < list.visual.x + list.visual.width && tab.visual.x + tab.visual.width > list.visual.x
    };
    if seen { ctx.scroll_into_view("selected-tab"); }
}

// The text a count capsule shows.
fn count_text(count) { if count > 99 { "99+" } else { `${count}` } }

// The count capsule or the tone lamp after the label, if the tab has either.
fn tab_marker(ctx, tab) {
    let count = tab_value_or(tab, "count");
    let tone = tab_value_or(tab, "tone");
    if count != () {
        // A marker: metrics.counter is the control line height, so the capsule is as tall as
        // the label, and its corners follow the corner style (square, subtle or capsule).
        let fill = if tone == () { theme_color("tabs.counter") } else { theme_color(tone) };
        let ink = if tone == () { theme_color("text_primary") } else { theme_color(`on_${tone}`) };
        let capsule = style().height(theme_length("metrics.counter")).min_width(theme_length("metrics.counter"))
            .padding_x(theme_spacing("xs")).flex_shrink(false).items_center().justify_center()
            .radius(theme_radius("sm")).background(fill)
            .typography("counter").font_feature("tnum", 1).text_color(ink);
        return row([text(count_text(count)).with_style(style().whitespace_nowrap())])
            .with_key("counter").with_style(ctx.component_style("counter", capsule));
    }
    if tone != () {
        // The 6px square is the status-lamp constant.
        return row([]).with_key("lamp").with_style(ctx.component_style("lamp", style()
            .width(px(6)).height(px(6)).flex_shrink(false).background(theme_color(tone))));
    }
    ()
}

// The close button over the slot, and the dirty mark under it. The close button shows while
// the tab is selected (unless it is dirty) or hovered; the mark shows until the tab is hovered.
fn tab_slot(ctx, props, cfg, tab, selected, close) {
    let closable = tab_flag(tab, "closable");
    let dirty = tab_flag(tab, "dirty");
    let children = [];
    if dirty {
        let mark = style().width(px(6)).height(px(6)).background(theme_color("text_primary"));
        if closable { mark = mark.group_hover(style().opacity(0.0)); }
        children.push(row([]).with_key("dirty").with_style(ctx.component_style("dirty", mark)));
    }
    if closable {
        let shown = selected && !dirty;
        let button = style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
            .items_center().justify_center().radius(theme_radius("md"))
            .opacity(if shown { 1.0 } else { 0.0 })
            .group_hover(style().opacity(1.0))
            // The track is surface_hover already, so its close button hovers a step darker.
            .hover(style().background(theme_color(if cfg.variant == "track" { "control.hover" } else { "surface_hover" })))
            // Above a reorderable tab's drag source, so a press here closes the tab.
            .occlude();
        children.push(row([image_source(asset("app/icons/close"))
            .with_style(style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon")))])
            .with_key("close")
            .with_style(ctx.component_style("close", button))
            .tab_stop(false)
            .accessibility_role("button")
            .accessibility_label(`${prop_or(props, "close_label", "Close")} ${tab.label}`)
            .on_click_value(Fn("close_tab"), close));
    }
    // A fixed square, so the mark, the button and nothing keep the tab's width.
    row(children).with_key("slot").with_style(ctx.component_style("slot", style().relative()
        .width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
        .flex_shrink(false).items_center().justify_center()))
}

// The native drag source over a reorderable tab: a release before the threshold is a tap,
// and the press leaves focus on the list.
fn drag_overlay(cfg, tabs, index, item_ref) {
    let tab = tabs[index];
    gpui_rhai::SortableItemPrimitive(#{ key: `${cfg.key}-${tab.value}-sort`,
        list_id: cfg.key, collection_id: cfg.scope, item_key: tab.value,
        previous_key: if index > 0 { tabs[index - 1].value } else { () },
        next_key: if index + 1 < tabs.len { tabs[index + 1].value } else { () },
        first_key: tabs[0].value, last_key: tabs[tabs.len - 1].value,
        direction: if cfg.vertical { "vertical" } else { "horizontal" }, threshold: 4.0,
        disabled: false, item_ref: item_ref, take_focus: false, tap: true,
        on_reorder: Fn("drag_reorder"), on_tap: Fn("select_tab") })
        .with_key("drag").with_ref(item_ref)
        .with_style(style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0)))
        .tab_stop(false)
}

// The look of one slot. Weight stays 400 so widths never jitter; a disabled tab takes the
// disabled ink and ignores hover.
fn tab_style(cfg, tab, selected, slotted, on_edge) {
    let label_visible = if "label_visible" in tab { tab.label_visible } else { true };
    let base = style().relative().items_center().gap(theme_spacing("xs")).typography("control")
        .text_color(if selected { theme_color("text_primary") } else { theme_color("tabs.foreground") })
        .hover(style().text_color(theme_color("text_primary")))
        .disabled(style().text_color(theme_color("disabled")));
    if cfg.vertical {
        return base.width(relative(1.0)).min_height(theme_length("metrics.control"))
            .padding_x(theme_length("metrics.control_pad")).justify_start();
    }
    if cfg.layout == "equal" {
        base = base.flex_basis(relative(0)).flex_grow().min_width(px(0)).justify_center();
    } else {
        // Tabs keep their width, up to two label columns, and the strip scrolls.
        base = base.flex_shrink(false).max_width(theme_length("metrics.label_column") * 2.0);
    }
    if cfg.variant == "bar" {
        // Side hairlines are reserved on every tab and drawn on the selected one, which also
        // takes the panel's surface and covers the strip's line. The strip is a rectangle
        // with no margin, so the selected tab stays square in every corner style; a first tab
        // on the strip's start edge draws no start hairline.
        return base.padding_start(theme_length("metrics.inset"))
            .padding_end(if slotted { theme_spacing("xs") } else { theme_length("metrics.inset") })
            .border_start(if on_edge { px(0) } else { px(1) }).border_end(px(1))
            .border_color(if selected { theme_color("border") } else { rgba(0x00000000) })
            .background(if selected { theme_color("tabbar.active") } else { rgba(0x00000000) });
    }
    if cfg.variant == "underline" {
        // Just room for the focus frame beside the label, so the first label sits
        // spacing.xs off the content edge of the panel or card under it.
        return base.self_stretch().justify_center().padding_x(theme_spacing("xs"));
    }
    // Track slots fill the track's inner height.
    base = base.self_stretch().justify_center();
    if !label_visible && cfg.layout != "equal" { base.padding_x(theme_spacing("sm")) }
    else { base.padding_x(theme_length("metrics.control_pad")) }
}

// The selected tab's decoration, painted under its content: the track's raised thumb, which
// carries the list's focus, or the underline's bar.
fn selection_marks(ctx, cfg) {
    let marks = [];
    if cfg.variant == "track" {
        let thumb = style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
            .radius(cfg.corner).background(theme_color("surface_raised"))
            .border(px(2)).border_color(theme_color("surface_raised"))
            .group_focus(style().border_color(theme_color("focus_ring")));
        marks.push(row([]).with_key("indicator").with_style(ctx.component_style("indicator", thumb)));
    } else if cfg.variant == "underline" {
        let bar = style().absolute().left(theme_spacing("xs")).right(theme_spacing("xs"))
            .bottom(px(0)).height(px(2)).background(theme_color("text_primary"));
        marks.push(row([]).with_key("indicator").with_style(ctx.component_style("indicator", bar)));
    }
    if cfg.motion_key != "" {
        for index in 0..marks.len {
            marks[index] = marks[index].shared_layout(cfg.motion_key, "indicator")
                .layout_motion(ctx.motion_duration("fast"), ctx.motion_easing("standard"));
        }
    }
    marks
}

// The focus frame over the selected underline or bar tab, painted last: the track's thumb
// carries its own.
fn focus_frame(ctx, cfg) {
    let frame = row([]).with_key("focus_frame").with_style(ctx.component_style("focus_frame", style()
        .absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
        .border(px(2)).border_color(rgba(0x00000000))
        .group_focus(style().border_color(theme_color("focus_ring")))));
    if cfg.motion_key != "" {
        frame = frame.shared_layout(cfg.motion_key, "focus_frame")
            .layout_motion(ctx.motion_duration("fast"), ctx.motion_easing("standard"));
    }
    frame
}

fn tab_node(ctx, props, cfg, tabs, index) {
    let tab = tabs[index];
    let selected = tab.value == props.value;
    let disabled = tab_flag(tab, "disabled");
    let closable = tab_flag(tab, "closable");
    let slotted = closable || tab_flag(tab, "dirty");
    let label_visible = if "label_visible" in tab { tab.label_visible } else { true };
    let has_icon = "icon" in tab && tab.icon != ();
    if !label_visible && !has_icon { throw `Tabs item ${tab.value} hides its label without an icon`; }
    let close = if closable { #{ value: tab.value, next: close_successor(tabs, tab.value) } } else { () };

    let children = if selected { selection_marks(ctx, cfg) } else { [] };
    let content = [];
    if has_icon {
        content.push(tab.icon.with_style(ctx.component_style("icon",
            style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
                .flex_shrink(false))));
    }
    if label_visible {
        content.push(text(tab.label).with_key("label").with_style(ctx.component_style("label",
            style().min_width(px(0)).whitespace_nowrap().text_ellipsis())));
    } else {
        content.push(text(" ").with_style(style().width(px(0)).overflow_hidden()));
    }
    let marker = tab_marker(ctx, tab);
    if marker != () { content.push(marker); }
    // Above the thumb, and shrinking with the slot so a long label truncates.
    children.push(row(content).with_key("content").with_style(style().relative().items_center()
        .justify_center().gap(theme_spacing("xs")).min_width(px(0))));
    let item_ref = if cfg.reorderable && !disabled { element_ref(`tab-${tab.value}`) } else { () };
    if item_ref != () { children.push(drag_overlay(cfg, tabs, index, item_ref)); }
    if slotted { children.push(tab_slot(ctx, props, cfg, tab, selected, close)); }
    if selected && cfg.variant != "track" { children.push(focus_frame(ctx, cfg)); }

    let name = tab.label;
    if "count" in tab && tab.count != () { name = `${name}, ${tab.count}`; }
    if "tone_label" in tab && tab.tone_label != () && "tone" in tab && tab.tone != () {
        name = `${name}, ${tab.tone_label}`;
    }
    let on_edge = index == 0 && cfg.start_len == 0;
    let resolved = ctx.component_style("tab", tab_style(cfg, tab, selected, slotted, on_edge));
    if selected { resolved = ctx.component_style("tab_selected", resolved); }
    // Business values may be empty. Prefixing is injective and supplies a non-empty node key
    // without changing the controlled value or callback payload.
    let node = row(children).with_key(`tab:${tab.value}`).with_style(resolved)
        .disabled(disabled).tab_stop(false)
        .accessibility_role("tab").accessibility_label(name).accessibility_selected(selected);
    if selected { node = node.with_ref(element_ref("selected-tab")); }
    if !disabled {
        if close != () || cfg.context {
            node = node.on("pointer_down", Fn("tab_pointer").curry(#{ value: tab.value,
                close: close, context: cfg.context }));
        }
        if cfg.selects && !cfg.reorderable { node = node.on_click_value(Fn("select_tab"), tab.value); }
    }
    node
}

// The list's keys: selection, and closing, moving and the context menu of the selected tab.
fn list_keys(list, props, cfg, tabs) {
    let enabled = [];
    for tab in tabs { if !tab_flag(tab, "disabled") { enabled.push(tab.value); } }
    if enabled.len == 0 { return list; }
    let back = if cfg.vertical { "up" } else { "left" };
    let forth = if cfg.vertical { "down" } else { "right" };
    if cfg.selects {
        list = list.on_key_value(back, Fn("key_select"), step_tab(tabs, props.value, -1))
            .on_key_value(forth, Fn("key_select"), step_tab(tabs, props.value, 1))
            .on_key_value("home", Fn("key_select"), enabled[0])
            .on_key_value("end", Fn("key_select"), enabled[enabled.len - 1]);
    }
    let at = -1;
    for index in 0..enabled.len { if enabled[index] == props.value { at = index; } }
    if at < 0 { return list; }
    let selected = ();
    for tab in tabs { if tab.value == props.value { selected = tab; } }
    if tab_flag(selected, "closable") {
        let close = #{ value: props.value, next: close_successor(tabs, props.value) };
        list = list.on_key_value("delete", Fn("close_tab"), close)
            .on_key_value("backspace", Fn("close_tab"), close);
    }
    if cfg.reorderable {
        if at > 0 {
            list = list.on_key_value(`alt+${back}`, Fn("key_reorder"),
                #{ value: props.value, anchor: enabled[at - 1], placement: "before" });
        }
        if at + 1 < enabled.len {
            list = list.on_key_value(`alt+${forth}`, Fn("key_reorder"),
                #{ value: props.value, anchor: enabled[at + 1], placement: "after" });
        }
    }
    if cfg.context {
        list = list.on_key_value("shift+f10", Fn("key_context"), props.value)
            .on_key_value("menu", Fn("key_context"), props.value);
    }
    list
}

// The list of every tab, behind a chevron after the list. The menu's open state lives in
// `TabOverflowMenu`, so the tabs themselves stay stateless and need no key.
fn overflow_menu(ctx, props, cfg, tabs) {
    let items = [];
    for tab in tabs {
        items.push(#{ value: tab.value, label: tab.label, disabled: tab_flag(tab, "disabled") });
    }
    let edge = style().flex_shrink(false);
    // The bar keeps its separators: a hairline between the strip and the menu.
    if cfg.variant == "bar" { edge = edge.border_start(px(1)).border_color(theme_color("border")); }
    row([tab_overflow_menu::TabOverflowMenu(#{ key: `${cfg.key}-menu`,
        label: prop_or(props, "menu_label", "All tabs"), value: props.value, items: items,
        on_pick: Fn("menu_pick") })])
        .with_key("menu").with_style(ctx.component_style("menu", edge))
}

// Nodes beside the list, in a row (a column when vertical) that keeps its size. The space
// goes toward the list only, so on an unfilled strip the first control sits on the content
// edge of the panel under it; the filled bar insets its slots on both sides, like a toolbar.
fn slot_group(ctx, cfg, nodes, part) {
    let group = if cfg.vertical { column(nodes) } else { row(nodes) };
    let space = theme_spacing("xs");
    let slot = style().flex_shrink(false).items_center().gap(space);
    slot = if cfg.variant == "bar" { slot.padding_x(space) }
        else if cfg.vertical { if part == "start" { slot.padding_bottom(space) } else { slot.padding_top(space) } }
        else if part == "start" { slot.padding_end(space) } else { slot.padding_start(space) };
    group.with_key(part).with_style(ctx.component_style(part, slot))
}

fn render_Tabs(ctx, props) {
    let tabs = props.tabs;
    let variant = prop_or(props, "variant", "track");
    let vertical = prop_or(props, "orientation", "horizontal") == "vertical";
    if vertical && variant != "track" {
        throw `Tabs variant ${variant} supports horizontal orientation only`;
    }
    let layout = if vertical { "content" } else { prop_or(props, "layout", "content") };
    let motion_key = prop_or(props, "motion_key", "");
    if motion_key.len > 128 { throw "Tabs motion_key must be at most 128 characters"; }
    let reorderable = prop_or(props, "reorderable", false);
    let has_menu = prop_or(props, "overflow_menu", false) && tabs.len > 0;
    let key = prop_or(props, "key", "");
    let slide = prop_or(props, "panel_motion", "none") == "slide";
    if (reorderable || prop_or(props, "overflow_menu", false) || slide) && key == "" {
        throw "Tabs needs a key with reorderable, overflow_menu or a sliding panel_motion";
    }
    let values = [];
    for tab in tabs {
        if values.contains(tab.value) { throw "Tabs values must be unique"; }
        values.push(tab.value);
    }
    let start = prop_or(props, "start", []);
    let end = prop_or(props, "end", []);
    let has_add = "on_add" in props && props.on_add != ();
    let list_ref = element_ref("tab-list");
    let cfg = #{ variant: variant, vertical: vertical, layout: layout, motion_key: motion_key,
        key: key, scope: list_ref.scope, reorderable: reorderable, start_len: start.len,
        // A horizontal track is a capsule with round corners; a vertical one rounds like a panel.
        corner: theme_radius(if vertical { "lg" } else { "md" }),
        selects: "on_change" in props && props.on_change != (),
        context: "on_context_request" in props && props.on_context_request != () };

    let headers = [];
    let panels = [];
    let show_panel = prop_or(props, "panel", true);
    let keep_mounted = prop_or(props, "keep_mounted", false);
    let named = false;
    for index in 0..tabs.len {
        let tab = tabs[index];
        let selected = tab.value == props.value;
        if selected { named = true; }
        headers.push(tab_node(ctx, props, cfg, tabs, index));
        if show_panel && (selected || keep_mounted) && "content" in tab && tab.content != () {
            let panel_style = style().typography("body").flex_grow().flex_basis(relative(0))
                .min_width(px(0)).min_height(px(0));
            panels.push(column([tab.content]).with_key(`panel:${tab.value}`)
                .retained_page(selected).tab_stop(false)
                .accessibility_role("tabpanel").accessibility_label(tab.label)
                .with_style(ctx.component_style("panel", panel_style)));
        }
    }

    // The list scrolls along its axis; content-width tabs keep their width.
    let list = if vertical { column(headers) } else { row(headers) };
    let list_style = style().relative().min_width(px(0)).focus(style());
    if variant == "track" {
        // One continuous tonal track holding the slots 2px in.
        list_style = list_style.padding(theme_spacing("xxs")).gap(theme_spacing("xxs"))
            .background(theme_color("surface_hover")).radius(cfg.corner);
    } else if variant == "underline" {
        // Underlined labels sit about control_pad apart.
        list_style = list_style.gap(theme_length("metrics.control_pad") * 0.5);
    }
    if vertical {
        list_style = list_style.min_width(theme_length("metrics.label_column")).min_height(px(0))
            .overflow_y_scroll();
    } else {
        list_style = list_style.min_height(theme_length("metrics.control")).overflow_x_scroll();
    }
    // The list always sits in the strip, so the strip, which may be the component's root
    // beside other keyless Tabs, carries no fixed key and the list's ref has one.
    list = list.with_key("tabs").with_ref(list_ref)
        .with_style(ctx.component_style("list", list_style))
        .accessibility_role("tablist").accessibility_label(props.label)
        .accessibility_value(props.value);
    if !vertical { list = list.translate_wheel(); }
    let keyboard = cfg.selects || reorderable || cfg.context;
    for tab in tabs { if tab_flag(tab, "closable") { keyboard = true; } }
    if keyboard && tabs.len > 0 { list = list_keys(list.tab_stop(true), props, cfg, tabs); }
    // Only a selected tab carries the ref; without one the effects are not declared, so they
    // start again when a tab comes back. A reorder that keeps the selection can carry the
    // selected tab out of view, so the order has its own effect.
    if named {
        effect("reveal", [props.value], Fn("reveal_start"), Fn("reveal_stop"));
        effect("reveal_order", #{ order: values, vertical: vertical },
            Fn("reveal_order_start"), Fn("reveal_stop"));
    }

    // The strip: the line under everything, the start slot, the list, the add button, then
    // the menu and the end slot at the far end.
    let line = variant == "underline" || variant == "bar";
    let children = [];
    if line {
        // Painted first, under the list, so the selected tab covers it. A border on the
        // scrolling list would clip the tab instead.
        children.push(row([]).with_key("line").with_style(ctx.component_style("line", style()
            .absolute().left(px(0)).right(px(0)).bottom(px(0)).height(px(1))
            .background(theme_color("border")))));
    }
    if start.len > 0 { children.push(slot_group(ctx, cfg, start, "start")); }
    // An equal list takes the room the slots leave.
    if layout == "equal" { list = list.with_style(style().flex_grow()); }
    children.push(list);
    if has_add {
        children.push(icon_button::IconButton(#{ key: "add",
            icon: image_source(asset("app/icons/plus")), label: prop_or(props, "add_label", "Add tab"),
            on_click: Fn("add_tab") }).with_style(ctx.component_style("add", style().flex_shrink(false))));
    }
    if has_menu || end.len > 0 {
        // The add button follows the last tab; the rest goes to the far end.
        if layout != "equal" { children.push(box([]).with_key("spacer").with_style(style().flex_grow())); }
        if has_menu { children.push(overflow_menu(ctx, props, cfg, tabs)); }
        if end.len > 0 { children.push(slot_group(ctx, cfg, end, "end")); }
    }
    // Children stretch across the strip, so slots and buttons share the list's extent.
    let strip_style = style().relative().min_width(px(0));
    if vertical {
        strip_style = strip_style.min_height(px(0));
    } else if variant == "bar" {
        strip_style = strip_style.height(theme_length("metrics.control"))
            .background(theme_color("tabbar.background"));
    }
    strip_style = strip_style.width(if layout == "equal" && !vertical { relative(1.0) } else { auto() });
    let strip = (if vertical { column(children) } else { row(children) })
        .with_style(ctx.component_style("strip", strip_style));

    let size_env = "size" in props && props.size != ();
    if panels.len == 0 {
        if size_env { strip = strip.env(#{ size: props.size }); }
        return strip;
    }
    // Only a strip inside our own panel root reserves its main-axis size; a standalone
    // switcher must still shrink and scroll in a Toolbar. A horizontal track fits its tabs; an
    // underline or bar strip spans the panel, so its line rules the width. A vertical strip
    // spans the panel's height, and its track fits the tabs and scrolls when they overflow.
    let strip_layout = style().flex_shrink(false);
    if variant == "track" && !vertical && layout == "content" { strip_layout = strip_layout.self_start(); }
    strip = strip.with_style(strip_layout);
    // The bar's selected tab joins the panel: no gap, and the panel takes its surface. The
    // underline's hairline already separates the panel, so it sits only `related` below.
    let gap = if variant == "bar" { px(0) } else if variant == "underline" { theme_length("space.related") }
        else { theme_length("space.group") };
    let content_style = style().flex_grow().flex_basis(relative(0)).min_width(px(0)).min_height(px(0));
    if variant == "bar" { content_style = content_style.background(theme_color("tabbar.active")); }
    let content = column(panels).with_key("panels").with_style(content_style);
    if slide {
        // The sliding panel keeps the surface and the size; only the panels move over it.
        content = sliding_panel::SlidingPanel(#{ key: `${key}-panel`, value: props.value, order: values,
            orientation: if vertical { "vertical" } else { "horizontal" },
            content: column(panels).with_key("panels").with_style(style().flex_grow().min_width(px(0))
                .min_height(px(0))) })
            .with_style(content_style);
    }
    let root = if vertical { row([strip, content]) } else { column([strip, content]) };
    root = root.with_style(ctx.component_style("root", style().min_width(px(0)).min_height(px(0))
        .gap(gap)))
        .accessibility_role("group").accessibility_label(props.label);
    if size_env { root.env(#{ size: props.size }) } else { root }
}
