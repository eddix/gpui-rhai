/* gpui-rhai
{
  "id": "components/list",
  "export": "List",
  "version": "0.2.0",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/badge"],
  "capabilities": {},
  "tokens": [
    "table.selection", "surface_hover", "accent", "focus_ring", "border", "text_primary",
    "text_muted", "disabled", "metrics.row", "metrics.inset", "spacing.xs", "spacing.sm",
    "typography.body", "typography.caption"
  ],
  "environment": ["density"]
}
*/
// Controlled virtualized list of keyed rows: the rows Table draws, without columns.
// Example: List(#{ key: "tickets", label: "Tickets", items: items, selection_mode: "single",
//              selected_keys: [current], height: 320.0, on_selection_change: Fn("selected") })
//
// Rows carry metrics.inset, so their text starts on the edge of a bled DataView or Region
// body. With a selection mode the list is one tab stop: Up/Down/Home/End move the selection
// and keep it revealed, Enter emits row_click, and the selected row shows the focus frame
// while the list has focus. Items are data, like Table cells: a leading status `badge`
// (#{ text, variant, dot }), the `title`, a muted `secondary` line (beside the title or
// below it) and trailing `meta` such as a time or a count. Badges of different widths push
// the titles apart; `badge_width` reserves one slot on every row so the titles align.

import "components/badge" as badge;

define_component(#{
    metadata: #{ id: "components/list", "export": "List", version: "0.2.0",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/badge"], capabilities: #{},
        tokens: [
            "table.selection", "surface_hover", "accent", "focus_ring", "border", "text_primary",
            "text_muted", "disabled", "metrics.row", "metrics.inset", "spacing.xs", "spacing.sm",
            "typography.body", "typography.caption"
        ],
        environment: ["density"] },
    schema: #{
        props: #{
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the list." },
            items: #{ schema: #{ type: "array", max_items: 10000, items: #{
                type: "object", allow_unknown: false, fields: #{
                    key: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Unique row key, used in `selected_keys` and every event." },
                    title: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                        doc: "Main row text, on one line truncated with an ellipsis." },
                    secondary: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Muted text beside the title or below it, as `secondary_layout` sets." },
                    meta: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: false, sensitive: false,
                        doc: "Trailing muted text such as a time or a count, in tabular figures." },
                    badge: #{ schema: #{ type: "optional", value: #{ type: "object", allow_unknown: false, fields: #{
                        text: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                            doc: "Badge label." },
                        variant: #{ schema: #{ type: "string",
                            allowed: ["neutral", "accent", "success", "warning", "danger"] },
                            required: false, sensitive: false, "default": #{ type: "string", value: "neutral" },
                            doc: "Status tone of the badge." },
                        dot: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                            "default": #{ type: "bool", value: true },
                            doc: "Shows the badge's status lamp before its text." },
                    } } }, required: false, sensitive: false,
                        doc: "Leading status Badge, before the title; see `badge_width` to align titles." },
                    disabled: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false },
                        doc: "Mutes the row; it ignores clicks and the keyboard skips it." },
                } } }, required: true, sensitive: false, doc: "Rows in display order." },
            selection_mode: #{ schema: #{ type: "string", allowed: ["none", "single", "multiple"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "none" },
                doc: "`single` selects the clicked row and a second click clears it, `multiple` toggles it; any mode but `none` makes the list one tab stop." },
            selected_keys: #{ schema: #{ type: "array", max_items: 10000, items: #{ type: "string" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Selected row keys; the caller stores the `selection_change` payload and passes it back." },
            secondary_layout: #{ schema: #{ type: "string", allowed: ["inline", "below"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "inline" },
                doc: "`inline` sets `secondary` beside the title at body size; `below` sets it on a caption line under it." },
            dividers: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false }, doc: "Draws a 1px border under every row." },
            badge_width: #{ schema: #{ type: "optional", value: #{ type: "number", exclusive_min: 0.0 } },
                required: false, sensitive: false,
                doc: "Badge slot width in logical pixels, reserved on every row so the titles align; `()` fits each badge." },
            height: #{ schema: #{ type: "optional", value: #{ type: "number", exclusive_min: 0.0 } },
                required: false, sensitive: false,
                doc: "Viewport height in logical pixels; give exactly one of `height` or `fill_height`." },
            fill_height: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Grows into the free height of a flex parent instead of a fixed `height`; give exactly one of the two." },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "No items" },
                doc: "Muted text shown when `items` is empty, unless `empty` is given." },
            empty: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Node shown instead of `empty_text` when `items` is empty." },
            on_selection_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the next selected keys when a row is clicked, Up, Down, Home or End is pressed, or a context request selects a row." },
            on_row_click: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the row key when an enabled row is clicked, or Enter is pressed on the selected row." },
            on_context_request: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `#{ key, anchor, source }` when a row is right-pressed or Shift+F10 or the menu key is pressed." },
        },
        state: #{ fields: #{} },
        events: #{
            selection_change: #{ payload: #{ type: "array", max_items: 10000, items: #{ type: "string" } },
                doc: "Emitted when a click, Up, Down, Home, End or a context request selects rows; the payload is the next selection." },
            row_click: #{ payload: #{ type: "string" },
                doc: "Emitted when an enabled row is clicked or Enter is pressed on the selected row; the payload is the row key." },
            context_request: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: true, sensitive: false,
                    doc: "Row the menu is for; `()` from the keyboard when no row is selected." },
                anchor: #{ schema: #{ type: "object", allow_unknown: false, fields: #{
                    x: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                        doc: "Left edge in window coordinates, in logical pixels." },
                    y: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                        doc: "Top edge in window coordinates, in logical pixels." },
                    width: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                        doc: "Width in logical pixels; 0 for a pointer request." },
                    height: #{ schema: #{ type: "number" }, required: true, sensitive: false,
                        doc: "Height in logical pixels; 0 for a pointer request." },
                } }, required: true, sensitive: false,
                    doc: "Where to open a Menu: the pointer, or the selected row's bounds (the list's when the row is not laid out)." },
                source: #{ schema: #{ type: "string", allowed: ["pointer", "keyboard"] }, required: true, sensitive: false,
                    doc: "`pointer` for a right press, `keyboard` for Shift+F10 or the menu key." },
            } }, doc: "Emitted on a right press on a row, after selecting it unless the selection holds it, or on Shift+F10 or the menu key with a selection mode." },
        },
        slots: #{ empty: #{ required: false, multiple: false,
            doc: "Node shown instead of `empty_text` when `items` is empty." } },
        parts: ["root", "row", "row_selected", "badge", "title", "secondary", "meta", "indicator_bar",
            "cursor", "empty"],
    },
    render: Fn("render_List")
});

fn List(props) { render_component("components/list", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn field_or(item, name, fallback) { if name in item && item[name] != () { item[name] } else { fallback } }

fn row_action(ctx, payload) {
    ctx.emit("row_click", payload.key);
    if payload.selection != () { ctx.emit("selection_change", payload.selection); }
}
// Keyboard navigation moves the selection and keeps the row revealed, as in Table.
fn keyboard_select(ctx, key) { ctx.emit("selection_change", [key]); handled() }
fn keyboard_activate(ctx, key) { ctx.emit("row_click", key); handled() }
// A right press asks for a context menu for the row; the row is selected first unless the
// selection already holds it.
fn row_context(target, ctx, event) {
    if event.button != "right" { return propagate(); }
    if target.select != () { ctx.emit("selection_change", target.select); }
    ctx.emit("context_request", #{ key: target.key, source: "pointer",
        anchor: #{ x: event.window.x, y: event.window.y, width: 0.0, height: 0.0 } });
    event_response().prevent_default().stop()
}
// Shift+F10 or the menu key: the current row's bounds are the anchor, the list's when the row
// is not laid out or there is none.
fn keyboard_context(ctx, target) {
    let bounds = if target.index != () { ctx.virtual_item_bounds(target.collection, target.index) }
        else { () };
    if bounds == () { bounds = ctx.event_target_bounds(); }
    let anchor = if bounds == () { #{ x: 0.0, y: 0.0, width: 0.0, height: 0.0 } }
        else { #{ x: bounds.x, y: bounds.y, width: bounds.width, height: bounds.height } };
    ctx.emit("context_request", #{ key: target.key, anchor: anchor, source: "keyboard" });
    handled()
}

// The selection a click on this row leads to, or () without a selection mode.
fn next_selection(mode, selected, key) {
    let is_selected = key in selected;
    if mode == "none" { () }
    else if mode == "single" { if is_selected { [] } else { [key] } }
    else {
        let result = [];
        for candidate in selected { if candidate != key { result.push(candidate); } }
        if !is_selected { result.push(key); }
        result
    }
}

// Keyboard targets among the enabled rows: the current row is the first selected one; Up and
// Down move one row and stop at the ends, and go to the first row when none is current.
fn row_neighbors(keys, selected) {
    if keys.len == 0 { return #{ first: (), last: (), previous: (), next: (), current: (), index: () }; }
    let current = -1;
    for index in 0..keys.len { if current < 0 && keys[index] in selected { current = index; } }
    let previous = if current <= 0 { 0 } else { current - 1 };
    let next = if current < 0 { 0 } else if current + 1 < keys.len { current + 1 } else { current };
    #{ first: keys[0], last: keys[keys.len - 1], previous: keys[previous], next: keys[next],
        current: if current >= 0 { keys[current] } else { () } }
}

fn render_list_row(below, dividers, badge_width, context, total, ctx, payload) {
    let item = payload.item;
    let children = [];
    let badge = if item.badge == () { () } else {
        badge::Badge(#{ key: `${payload.key}-badge`, text: item.badge.text,
            variant: item.badge.variant, dot: item.badge.dot, size: "sm" })
            .with_key(`${payload.key}-badge`)
            .with_style(ctx.component_style("badge", style().flex_shrink(false)))
    };
    if badge_width != () {
        children.push(row(if badge == () { [] } else { [badge] }).with_key(`${payload.key}-slot`)
            .with_style(style().width(px(badge_width)).flex_shrink(false).items_center()));
    } else if badge != () { children.push(badge); }
    let title = text(item.title).with_key(`${payload.key}-title`)
        .with_style(ctx.component_style("title", style().min_width(px(0)).whitespace_nowrap()
            .text_ellipsis().typography("body")
            .text_color(if item.disabled { theme_color("disabled") } else { theme_color("text_primary") })));
    let label = item.title;
    if item.secondary != () {
        label = `${label}, ${item.secondary}`;
        // Beside the title the secondary text keeps the body size, so the row shares one
        // line box; below it, it is a caption line.
        let secondary = text(item.secondary).with_key(`${payload.key}-secondary`)
            .with_style(ctx.component_style("secondary", style().min_width(px(0)).whitespace_nowrap()
                .text_ellipsis().typography(if below { "caption" } else { "body" })
                .text_color(theme_color("text_muted"))));
        if below {
            children.push(column([title, secondary]).with_key(`${payload.key}-text`)
                .with_style(style().flex_grow().min_width(px(0))));
        } else {
            children.push(row([title.with_style(style().flex_shrink(false).max_width(relative(0.7))),
                secondary]).with_key(`${payload.key}-text`)
                // The secondary text belongs to the title: closer than the row's own gap.
                .with_style(style().flex_grow().min_width(px(0)).items_center().gap(theme_spacing("xs"))));
        }
    } else {
        children.push(row([title]).with_key(`${payload.key}-text`)
            .with_style(style().flex_grow().min_width(px(0)).items_center()));
    }
    if item.meta != () {
        label = `${label}, ${item.meta}`;
        children.push(text(item.meta).with_key(`${payload.key}-meta`)
            .with_style(ctx.component_style("meta", style().flex_shrink(false).whitespace_nowrap()
                .typography("body").font_feature("tnum", 1).text_color(theme_color("text_muted")))));
    }
    // Selection is the table selection tint plus the 2px indicator bar on the row start;
    // while the list has keyboard focus the selected row also shows the focus frame.
    if item.selected {
        children.push(row([]).with_style(ctx.component_style("indicator_bar", style()
            .absolute().inset_start(px(0)).top(px(0)).bottom(px(0))
            .width(px(2)).background(theme_color("accent")))));
        children.push(row([]).with_style(ctx.component_style("cursor", style()
            .absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
            .border(px(2)).border_color(rgba(0x00000000))
            .group_focus(style().border_color(theme_color("focus_ring"))))));
    }
    let base = style().relative().width(relative(1.0)).items_center().gap(theme_spacing("sm"))
        .padding_x(theme_length("metrics.inset"))
        .background(if item.selected { theme_color("table.selection") } else { rgba(0x00000000) });
    base = if below { base.padding_y(theme_spacing("xs")).min_height(theme_length("metrics.row")) }
        else { base.height(theme_length("metrics.row")) };
    if dividers { base = base.border_bottom(px(1)).border_color(theme_color("border")); }
    if !item.selected && !item.disabled {
        base = base.hover(style().background(theme_color("surface_hover")));
    }
    let node = row(children).with_key(payload.key)
        .with_style(ctx.component_style(if item.selected { "row_selected" } else { "row" }, base))
        .disabled(item.disabled)
        .accessibility_role(if item.selectable { "option" } else { "listitem" })
        .accessibility_label(label)
        .accessibility_selected(item.selected)
        .accessibility_position_in_set(payload.index + 1)
        .accessibility_size_of_set(total);
    if item.disabled { return node.tab_stop(false); }
    if context {
        let select = if item.selection == () || item.selected { () } else { [payload.key] };
        node = node.on("pointer_down", Fn("row_context").curry(#{ key: payload.key, select: select }));
    }
    // A row is a pointer target only: the list is the one tab stop.
    node.on_click_value(Fn("row_action"), #{ key: payload.key, selection: item.selection })
        .tab_stop(false)
}

fn render_List(ctx, props) {
    let fill_height = prop_or(props, "fill_height", false);
    let has_height = "height" in props && props.height != ();
    if has_height == fill_height { throw "List requires exactly one of height or fill_height"; }
    let mode = prop_or(props, "selection_mode", "none");
    let selected = prop_or(props, "selected_keys", []);
    let below = prop_or(props, "secondary_layout", "inline") == "below";
    let size = if fill_height { style().flex_grow().min_height(px(0)) } else { style().height(px(props.height)) };
    if props.items.len == 0 {
        let empty = if "empty" in props && props.empty != () { props.empty }
            else { text(prop_or(props, "empty_text", "No items")) };
        return column([empty]).with_key(props.key)
            .with_style(ctx.component_style("empty", size.min_width(px(0))
                .padding_x(theme_length("metrics.inset")).padding_y(theme_spacing("sm"))
                .typography("body").text_color(theme_color("text_muted"))))
            .accessibility_role("list").accessibility_label(props.label);
    }
    let data = [];
    let enabled = [];
    for item in props.items {
        let disabled = field_or(item, "disabled", false);
        let selectable = mode != "none";
        data.push(#{ key: item.key, title: item.title, secondary: field_or(item, "secondary", ()),
            meta: field_or(item, "meta", ()),
            badge: if "badge" in item && item.badge != () {
                #{ text: item.badge.text, variant: field_or(item.badge, "variant", "neutral"),
                    dot: field_or(item.badge, "dot", true) }
            } else { () },
            disabled: disabled, selectable: selectable,
            selected: selectable && item.key in selected,
            selection: if disabled { () } else { next_selection(mode, selected, item.key) } });
        if !disabled { enabled.push(item.key); }
    }
    let collection = #{ key: `${props.key}-rows`, label: props.label, data: data,
        estimated_height: if below { 48.0 } else { 32.0 }, overdraw_pixels: 128.0,
        alignment: "top", follow_tail: false };
    if fill_height { collection.fill_height = true; } else { collection.height = props.height; }
    let neighbors = ();
    if mode != "none" {
        neighbors = row_neighbors(enabled, selected);
        if neighbors.current != () { collection.reveal_key = neighbors.current; }
    }
    let context = "on_context_request" in props && props.on_context_request != ();
    let root = virtual_collection(collection,
        Fn("render_list_row").curry(below).curry(prop_or(props, "dividers", false))
            .curry(prop_or(props, "badge_width", ())).curry(context).curry(data.len))
        .with_key(props.key)
        .with_style(ctx.component_style("root", size.min_width(px(0)).focus(style())))
        .accessibility_role(if mode == "none" { "list" } else { "listbox" })
        .accessibility_label(props.label);
    if neighbors == () || neighbors.first == () { return root; }
    root = root.tab_stop(true)
        .on_key_value("up", Fn("keyboard_select"), neighbors.previous)
        .on_key_value("down", Fn("keyboard_select"), neighbors.next)
        .on_key_value("home", Fn("keyboard_select"), neighbors.first)
        .on_key_value("end", Fn("keyboard_select"), neighbors.last);
    if neighbors.current != () {
        root = root.on_key_value("enter", Fn("keyboard_activate"), neighbors.current);
    }
    if context {
        let index = ();
        for position in 0..data.len { if index == () && data[position].key == neighbors.current { index = position; } }
        let target = #{ key: neighbors.current, collection: collection.key, index: index };
        root = root.on_key_value("shift+f10", Fn("keyboard_context"), target)
            .on_key_value("menu", Fn("keyboard_context"), target);
    }
    root
}
