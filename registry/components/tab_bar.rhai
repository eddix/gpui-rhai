/* gpui-rhai
{
  "id": "components/tab_bar",
  "export": "TabBar",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/menu"],
  "capabilities": {},
  "assets": ["icons/close.svg", "icons/chevron_down.svg"],
  "tokens": [
    "tabbar.background", "tabbar.active", "border", "focus_ring", "text_primary",
    "tabs.foreground", "disabled", "surface_hover", "metrics.control", "metrics.inset",
    "metrics.icon", "metrics.label_column", "spacing.xs", "radius.md", "typography.control"
  ],
  "environment": ["size", "density", "corners"]
}
*/
// Controlled document tabs that belong to the panel under them: the selected tab takes the
// panel's surface (`tabbar.active`) and covers the bar's bottom line, so the panel's extent
// says what the tabs switch. Tabs keep their width and the strip scrolls sideways when they
// do not fit; the selected tab stays revealed.
//
// Props: key, label, value, tabs[{value,label,icon?,closable?,dirty?,disabled?}], size,
//        start/end (nodes beside the strip), overflow_menu, menu_label, close_label,
//        reorderable, on_change, on_close, on_context_request, on_reorder.
// Events: change(string), close(string), context_request(#{ value, anchor, source }),
//         reorder(#{ value, anchor, placement }).
// Parts: root, line, start, list, tab, tab_selected, icon, label, slot, close, dirty, cursor,
//        menu, end.
// Example: TabBar(#{ key: "docs", label: "Open files", value: current, tabs: tabs,
//              on_change: Fn("opened"), on_close: Fn("closed") })
//
// Keyboard: the strip is one tab stop; Left/Right/Home/End move a cursor (the 2px focus
// frame) and Enter or Space selects it, so moving does not switch panels on every step.
// Shift+F10 or the menu key asks for a context menu for the cursor tab; Alt+Left/Right moves
// it when the tabs are reorderable. A closable tab shows its close button while it is
// selected or hovered, and closes on a middle press; a dirty tab shows a square mark that
// turns into the close button under the pointer. Reorderable tabs are dragged anywhere on
// the tab.

import "components/menu" as menu;

define_component(#{
    metadata: #{ id: "components/tab_bar", "export": "TabBar", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/menu"], capabilities: #{},
        tokens: [
            "tabbar.background", "tabbar.active", "border", "focus_ring", "text_primary",
            "tabs.foreground", "disabled", "surface_hover", "metrics.control", "metrics.inset",
            "metrics.icon", "metrics.label_column", "spacing.xs", "radius.md", "typography.control"
        ],
        environment: ["size", "density", "corners"],
        assets: ["icons/close.svg", "icons/chevron_down.svg"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            tabs: #{ schema: #{ type: "array", max_items: 256, items: #{
                type: "object", allow_unknown: false, fields: #{
                    value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    label: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                    icon: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false },
                    closable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                    dirty: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false } },
                } } }, required: true, sensitive: false },
            size: #{ schema: #{ type: "optional", value: #{ type: "string", allowed: ["xs", "sm", "md", "lg"] } },
                required: false, sensitive: false },
            start: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            end: #{ schema: #{ type: "array", max_items: 8, items: #{ type: "node" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] } },
            overflow_menu: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            menu_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "All tabs" } },
            close_label: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "Close" } },
            reorderable: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false } },
            on_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_close: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_context_request: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
            on_reorder: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false },
        },
        state: #{ fields: #{
            cursor: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
            cursor_base: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
            menu_open: #{ schema: #{ type: "bool" }, "default": #{ type: "bool", value: false } },
            menu_active: #{ schema: #{ type: "string" }, "default": #{ type: "string", value: "" } },
        } },
        events: #{
            change: #{ payload: #{ type: "string" } },
            close: #{ payload: #{ type: "string" } },
            context_request: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                anchor: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                    x: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                    y: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                    width: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                    height: #{ schema: #{ type: "number" }, required: true, sensitive: false },
                } }, required: true, sensitive: false },
                source: #{ schema: #{ type: "string", allowed: ["pointer", "keyboard"] }, required: true, sensitive: false },
            } } },
            reorder: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                value: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                anchor: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                placement: #{ schema: #{ type: "string", allowed: ["before", "after"] }, required: true, sensitive: false },
            } } },
        },
        slots: #{
            start: #{ required: false, multiple: true },
            end: #{ required: false, multiple: true },
        },
        parts: ["root", "line", "start", "list", "tab", "tab_selected", "icon", "label", "slot",
            "close", "dirty", "cursor", "menu", "end"],
        effects: ["reveal"],
    },
    render: Fn("render_TabBar")
});

fn TabBar(props) { render_component("components/tab_bar", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn tab_flag(tab, name) { if name in tab { tab[name] } else { false } }

// Selecting a tab moves the cursor with it.
fn select_tab(ctx, value) {
    ctx.set_state("cursor", value);
    ctx.set_state("cursor_base", value);
    ctx.emit("change", value);
}
fn tab_click(ctx, value) { select_tab(ctx, value); }
fn tab_tap(ctx, value) { select_tab(ctx, value); }
fn tab_activate(ctx, value) { select_tab(ctx, value); handled() }
fn tab_close(ctx, value) { ctx.emit("close", value); handled() }
fn cursor_move(ctx, payload) {
    ctx.set_state("cursor", payload.cursor);
    ctx.set_state("cursor_base", payload.base);
    handled()
}
fn tab_reorder(ctx, payload) {
    ctx.emit("reorder", #{ value: payload.source_key, anchor: payload.anchor_key,
        placement: payload.placement });
}
fn key_reorder(ctx, payload) { ctx.emit("reorder", payload); handled() }
// A middle press closes a closable tab; a right press asks for a context menu.
fn tab_pointer(target, ctx, event) {
    if event.button == "middle" && target.closable {
        ctx.emit("close", target.value);
        return event_response().prevent_default().stop();
    }
    if event.button == "right" && target.context {
        ctx.emit("context_request", #{ value: target.value, source: "pointer",
            anchor: #{ x: event.window.x, y: event.window.y, width: 0.0, height: 0.0 } });
        return event_response().prevent_default().stop();
    }
    propagate()
}
// Shift+F10 or the menu key: the cursor tab's bounds are the anchor.
fn key_context(ctx, value) {
    let bounds = ctx.element_bounds("cursor-tab");
    let anchor = if bounds == () { #{ x: 0.0, y: 0.0, width: 0.0, height: 0.0 } }
        else { #{ x: bounds.visual.x, y: bounds.visual.y, width: bounds.visual.width,
            height: bounds.visual.height } };
    ctx.emit("context_request", #{ value: value, anchor: anchor, source: "keyboard" });
    handled()
}
fn menu_open_change(ctx, open) { ctx.set_state("menu_open", open); }
fn menu_active_change(ctx, value) { ctx.set_state("menu_active", value); }
fn menu_pick(ctx, value) { ctx.set_state("menu_open", false); select_tab(ctx, value); }
// Keep the cursor tab, which follows the selection, in view after either moves.
fn reveal_start(ctx, deps) { ctx.scroll_into_view("cursor-tab"); }
fn reveal_stop(ctx, deps) {}

// The close button over the slot, and the dirty mark under it. The close button shows while
// the tab is selected (unless it is dirty) or hovered; the mark shows until the tab is hovered.
fn tab_slot(ctx, props, tab, selected) {
    let closable = tab_flag(tab, "closable");
    let dirty = tab_flag(tab, "dirty");
    let children = [];
    if dirty {
        // The 6px mark is the status-lamp constant.
        let mark = style().width(px(6)).height(px(6)).background(theme_color("text_primary"));
        if closable { mark = mark.group_hover(style().opacity(0.0)); }
        children.push(row([]).with_key("dirty").with_style(ctx.component_style("dirty", mark)));
    }
    if closable {
        let shown = selected && !dirty;
        let close = style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
            .items_center().justify_center().radius(theme_radius("md"))
            .opacity(if shown { 1.0 } else { 0.0 })
            .group_hover(style().opacity(1.0))
            .hover(style().background(theme_color("surface_hover")))
            // Above a reorderable tab's drag source, so a press here closes the tab.
            .occlude();
        // A row, so the centering applies (a box has no flex direction).
        children.push(row([image_source(asset("app/icons/close"))
            .with_style(style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon")))])
            .with_key("close")
            .with_style(ctx.component_style("close", close))
            .tab_stop(false)
            .accessibility_role("button")
            .accessibility_label(`${prop_or(props, "close_label", "Close")} ${tab.label}`)
            .on_click_value(Fn("tab_close"), tab.value));
    }
    // A fixed square, so the mark, the button and nothing keep the tab's width.
    row(children).with_key("slot").with_style(ctx.component_style("slot", style().relative()
        .width(theme_length("metrics.icon")).height(theme_length("metrics.icon"))
        .padding(theme_spacing("xs")).flex_shrink(false).items_center().justify_center()))
}

// Side hairlines are reserved on every tab and drawn on the selected one, which also takes
// the panel's surface and covers the bar's bottom line. The bar is a rectangle with no
// margin, so the selected tab stays square in every corner style. A first tab on the bar's
// start edge draws no start hairline: the edge (or a separator beside the bar) is its edge.
fn tab_style(selected, disabled, slotted, on_edge) {
    let ink = if disabled { theme_color("disabled") }
        else if selected { theme_color("text_primary") } else { theme_color("tabs.foreground") };
    let hover_ink = if disabled { theme_color("disabled") } else { theme_color("text_primary") };
    let tab = style().relative().flex_shrink(false).min_width(px(0))
        .max_width(theme_length("metrics.label_column") * 2.0)
        .items_center().gap(theme_spacing("xs"))
        .padding_start(theme_length("metrics.inset"))
        .padding_end(if slotted { theme_spacing("xs") } else { theme_length("metrics.inset") });
    tab = tab.border_start(if on_edge { px(0) } else { px(1) }).border_end(px(1))
        .border_color(if selected { theme_color("border") } else { rgba(0x00000000) })
        .background(if selected { theme_color("tabbar.active") } else { rgba(0x00000000) })
        .typography("control").text_color(ink).hover(style().text_color(hover_ink));
    tab
}

// The cursor frame over the cursor tab, shown while the strip has focus.
fn cursor_frame(ctx) {
    row([]).with_key("cursor")
        .with_style(ctx.component_style("cursor", style().absolute()
            .left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
            .border(px(2)).border_color(rgba(0x00000000))
            .group_focus(style().border_color(theme_color("focus_ring")))))
}

// The native drag source over a reorderable tab: a release before the threshold is a tap,
// and the press leaves focus on the strip.
fn drag_overlay(list_key, scope, tabs, index, item_ref) {
    let tab = tabs[index];
    gpui_rhai::SortableItemPrimitive(#{ key: `${list_key}-${tab.value}-sort`,
        list_id: list_key, collection_id: scope, item_key: tab.value,
        previous_key: if index > 0 { tabs[index - 1].value } else { () },
        next_key: if index + 1 < tabs.len { tabs[index + 1].value } else { () },
        first_key: tabs[0].value, last_key: tabs[tabs.len - 1].value,
        direction: "horizontal", threshold: 4.0, disabled: false, item_ref: item_ref,
        take_focus: false, tap: true, on_reorder: Fn("tab_reorder"), on_tap: Fn("tab_tap") })
        .with_key("drag").with_ref(item_ref)
        .with_style(style().absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0)))
        .tab_stop(false)
}

fn render_TabBar(ctx, props) {
    let tabs = props.tabs;
    let reorderable = prop_or(props, "reorderable", false);
    let context = "on_context_request" in props && props.on_context_request != ();
    let enabled = [];
    for tab in tabs { if !tab_flag(tab, "disabled") { enabled.push(tab.value); } }
    // The cursor follows the selection until the keys move it, and returns to it whenever
    // the selection changes from outside.
    let stored = ctx.get_state("cursor");
    let cursor = if ctx.get_state("cursor_base") == props.value && stored in enabled { stored }
        else { props.value };
    let list_ref = element_ref(`${props.key}-tabs`);
    let cursor_ref = element_ref("cursor-tab");
    let nodes = [];
    for index in 0..tabs.len {
        let tab = tabs[index];
        let selected = tab.value == props.value;
        let disabled = tab_flag(tab, "disabled");
        let slotted = tab_flag(tab, "closable") || tab_flag(tab, "dirty");
        let children = [];
        if "icon" in tab && tab.icon != () {
            children.push(tab.icon.with_style(ctx.component_style("icon", style()
                .width(theme_length("metrics.icon")).height(theme_length("metrics.icon")).flex_shrink(false))));
        }
        children.push(text(tab.label).with_key("label").with_style(ctx.component_style("label", style()
            .min_width(px(0)).whitespace_nowrap().text_ellipsis())));
        let item_ref = if reorderable && !disabled { element_ref(`${props.key}-tab-${tab.value}`) } else { () };
        if item_ref != () { children.push(drag_overlay(props.key, list_ref.scope, tabs, index, item_ref)); }
        if slotted { children.push(tab_slot(ctx, props, tab, selected)); }
        if tab.value == cursor { children.push(cursor_frame(ctx)); }
        let on_edge = index == 0 && prop_or(props, "start", []).len == 0;
        let resolved = ctx.component_style("tab", tab_style(selected, disabled, slotted, on_edge));
        if selected { resolved = ctx.component_style("tab_selected", resolved); }
        let node = row(children).with_key(tab.value).with_style(resolved)
            .disabled(disabled).tab_stop(false)
            .accessibility_role("tab").accessibility_label(tab.label).accessibility_selected(selected);
        // The cursor tab carries the ref the reveal effect scrolls to.
        if tab.value == cursor { node = node.with_ref(cursor_ref); }
        if !disabled {
            node = node.on("pointer_down", Fn("tab_pointer").curry(#{ value: tab.value,
                closable: tab_flag(tab, "closable"), context: context }));
            if !reorderable { node = node.on_click_value(Fn("tab_click"), tab.value); }
        }
        nodes.push(node);
    }
    let list = row(nodes).with_key("tabs").with_ref(list_ref)
        .with_style(ctx.component_style("list", style().flex_grow().min_width(px(0))
            .overflow_x_scroll().focus(style())))
        .translate_wheel()
        .accessibility_role("tablist").accessibility_label(props.label);
    if enabled.len > 0 {
        let at = 0;
        for index in 0..enabled.len { if enabled[index] == cursor { at = index; } }
        let previous = enabled[if at > 0 { at - 1 } else { 0 }];
        let next = enabled[if at + 1 < enabled.len { at + 1 } else { at }];
        let base = props.value;
        list = list.tab_stop(true)
            .on_key_value("left", Fn("cursor_move"), #{ cursor: previous, base: base })
            .on_key_value("right", Fn("cursor_move"), #{ cursor: next, base: base })
            .on_key_value("home", Fn("cursor_move"), #{ cursor: enabled[0], base: base })
            .on_key_value("end", Fn("cursor_move"), #{ cursor: enabled[enabled.len - 1], base: base })
            .on_key_value("enter", Fn("tab_activate"), cursor)
            .on_key_value("space", Fn("tab_activate"), cursor);
        if context {
            list = list.on_key_value("shift+f10", Fn("key_context"), cursor)
                .on_key_value("menu", Fn("key_context"), cursor);
        }
        if reorderable {
            if at > 0 {
                list = list.on_key_value("alt+left", Fn("key_reorder"),
                    #{ value: cursor, anchor: enabled[at - 1], placement: "before" });
            }
            if at + 1 < enabled.len {
                list = list.on_key_value("alt+right", Fn("key_reorder"),
                    #{ value: cursor, anchor: enabled[at + 1], placement: "after" });
            }
        }
    }
    effect("reveal", [props.value, cursor], Fn("reveal_start"), Fn("reveal_stop"));

    // The bar's bottom line, painted first so the selected tab covers it.
    let children = [row([]).with_key("line").with_style(ctx.component_style("line", style()
        .absolute().left(px(0)).right(px(0)).bottom(px(0)).height(px(1))
        .background(theme_color("border"))))];
    let start = prop_or(props, "start", []);
    if start.len > 0 {
        children.push(row(start).with_key("start").with_style(ctx.component_style("start", style()
            .flex_shrink(false).items_center().gap(theme_spacing("xs"))
            .padding_x(theme_spacing("xs")))));
    }
    children.push(list);
    if prop_or(props, "overflow_menu", false) && tabs.len > 0 {
        let items = [];
        for tab in tabs {
            items.push(#{ kind: "item", value: tab.value, label: tab.label,
                disabled: tab_flag(tab, "disabled") });
        }
        let menu_label = prop_or(props, "menu_label", "All tabs");
        let trigger = row([image_source(asset("app/icons/chevron_down"))
            .with_style(style().width(theme_length("metrics.icon")).height(theme_length("metrics.icon")))])
            // A definite height: the Menu root around the trigger has none to fill (#128).
            .with_style(style().width(theme_length("metrics.control")).height(theme_length("metrics.control"))
                .items_center().justify_center().text_color(theme_color("tabs.foreground"))
                .hover(style().background(theme_color("surface_hover")).text_color(theme_color("text_primary"))))
            .accessibility_role("button").accessibility_label(menu_label);
        let active = ctx.get_state("menu_active");
        children.push(row([menu::Menu(#{ key: `${props.key}-menu`, label: menu_label, trigger: trigger,
            open: ctx.get_state("menu_open"), active_value: if active == "" { props.value } else { active },
            items: items, on_open_change: Fn("menu_open_change"),
            on_active_change: Fn("menu_active_change"), on_action: Fn("menu_pick") })])
            .with_key("menu").with_style(ctx.component_style("menu", style().flex_shrink(false)
                .border_left(px(1)).border_color(theme_color("border")))));
    }
    let end = prop_or(props, "end", []);
    if end.len > 0 {
        children.push(row(end).with_key("end").with_style(ctx.component_style("end", style()
            .flex_shrink(false).items_center().gap(theme_spacing("xs"))
            .padding_x(theme_spacing("xs")))));
    }
    let root = row(children).with_key(props.key)
        .with_style(ctx.component_style("root", style().relative().min_width(px(0))
            .height(theme_length("metrics.control")).background(theme_color("tabbar.background"))));
    if "size" in props && props.size != () { root.env(#{ size: props.size }) } else { root }
}
