/* gpui-rhai
{
  "id": "components/selectable_list",
  "export": "SelectableList",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": [],
  "capabilities": {},
  "tokens": [
    "table.selection", "surface_hover", "focus_ring", "border", "text_muted", "metrics.row",
    "metrics.inset", "spacing.xs", "spacing.sm", "typography.body", "scrollbar.thumb",
    "scrollbar.thumb_hover", "radius.md"
  ],
  "environment": ["size", "density"]
}
*/
// The behavior of List and Table rows with rows the application draws: selection, the one
// tab stop and its keys, the cursor and focus frame, row and context requests, accessibility
// and virtualization stay the list's, and a `row` Node factory draws each row's content.
// Example: SelectableList(#{ key: "tracks", label: "Tracks", items: tracks,
//              selection_mode: "single", selected_keys: state.selected,
//              row: ctx.node_factory(Fn("track_row")), on_selection_change: Fn("selected") })
// fn track_row(ctx, row) { row([cover(row.item), column([text(row.item.title), text(row.item.artist)])]) }
//
// Items are maps with a unique `key` and any fields of the application's own; `disabled`
// mutes a row and the keys skip it, and the field `label_field` names (`label` by default)
// names the row for assistive technology. The factory gets `#{ key, item, index, selected,
// current }`, current being the row the keys act on. Callbacks in what it draws reach the
// application. By default each row is a pressable block like a List row: the selection tint,
// the hover fill and a control's corners, spacing.xs in from the list's sides; `row_chrome:
// false` leaves the row bare for the factory to paint. Rows take their own heights.
//
// List and Table share this module's behavior functions: next_selection, row_frame,
// navigation_keys, context_keys and the handlers they bind.

define_component(#{
    metadata: #{ id: "components/selectable_list", "export": "SelectableList", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: [], capabilities: #{},
        tokens: [
            "table.selection", "surface_hover", "focus_ring", "border", "text_muted", "metrics.row",
            "metrics.inset", "spacing.xs", "spacing.sm", "typography.body", "scrollbar.thumb",
            "scrollbar.thumb_hover", "radius.md"
        ],
        environment: ["size", "density"] },
    schema: #{
        props: #{
            scrollbar: #{ schema: #{ type: "string", allowed: ["auto", "always", "hidden"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "auto" }, doc: "Shared native viewport scrollbar visibility." },
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the list." },
            items: #{ schema: #{ type: "one_of", variants: [#{ type: "collection" },
                #{ type: "array", max_items: 10000, items: #{ type: "ui_value" } }] }, required: true, sensitive: false,
                doc: "Rows in display order: maps with a unique `key`, an optional `disabled` bool and the application's own fields; a NativeCollection is navigated in Rust without enumerating it in Rhai." },
            row: #{ schema: #{ type: "callback" }, required: true, sensitive: false,
                doc: "Node factory bound with ctx.node_factory(Fn(...)), called as Fn(ctx, row) for each realized row; row is #{ key, item, index, selected, current }. Its nodes' callbacks reach the scope that bound it." },
            label_field: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "label" },
                doc: "Item field that names each row for assistive technology; a row without it is named by its key." },
            row_chrome: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true },
                doc: "Draws each row as a List row draws: the selection tint, the hover fill, a control's corners and the content inset. False leaves the row bare; the factory paints the selected state from `row.selected`." },
            estimated_row_height: #{ schema: #{ type: "number", exclusive_min: 0.0 }, required: false, sensitive: false,
                "default": #{ type: "float", value: 48.0 },
                doc: "Height the virtual collection assumes for rows it has not measured, in logical pixels." },
            keyboard_navigation: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true }, doc: "Owns the single tab stop and navigation keys." },
            active: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true }, doc: "Draws the selected cursor as active; false keeps the selection visible without its focus frame." },
            selection_mode: #{ schema: #{ type: "string", allowed: ["none", "single", "multiple"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "none" },
                doc: "`single` selects the clicked row and a second click clears it, `multiple` toggles it; any mode but `none`, or `on_context_request`, makes the list one tab stop." },
            selected_keys: #{ schema: #{ type: "array", max_items: 10000, items: #{ type: "string" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Selected row keys; the caller stores the `selection_change` payload and passes it back." },
            height: #{ schema: #{ type: "optional", value: #{ type: "one_of", variants: [
                #{ type: "number", exclusive_min: 0.0 }, #{ type: "length" }] } },
                required: false, sensitive: false,
                doc: "Viewport height: logical pixels, or a Length; give exactly one of `height` or `fill_height`." },
            fill_height: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Grows into the free height of a flex parent instead of a fixed `height`; give exactly one of the two." },
            loading: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false },
                doc: "Shows `loading_content` in the list's place instead of the rows." },
            loading_content: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Node shown while `loading`; `()` shows a muted line with the localized `collection.loading` message (\"Loading…\")." },
            empty_text: #{ schema: #{ type: "string" }, required: false, sensitive: false,
                "default": #{ type: "string", value: "" },
                doc: "Muted text shown when `items` is empty, unless `empty` is given. Empty, the localized `common.no_items` message, `No items` without one." },
            empty: #{ schema: #{ type: "optional", value: #{ type: "node" } }, required: false, sensitive: false,
                doc: "Node shown instead of `empty_text` when `items` is empty." },
            on_selection_change: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the next selected keys when a row is clicked, Up, Down, Home or End is pressed, or a context request selects a row." },
            on_row_click: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with the row key when an enabled row is clicked, or Enter is pressed on the selected row." },
            on_row_activate: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with #{ key, has_children } on a double click." },
            on_context_request: #{ schema: #{ type: "optional", value: #{ type: "callback" } }, required: false, sensitive: false,
                doc: "Called with `#{ key, anchor, source }` when a row is right-pressed or Shift+F10 or the menu key is pressed; without a selection mode it makes the list a tab stop for those keys." },
        },
        state: #{ fields: #{} },
        events: #{
            selection_change: #{ payload: #{ type: "array", max_items: 10000, items: #{ type: "string" } },
                doc: "Emitted when a click, Up, Down, Home, End or a context request selects rows; the payload is the next selection." },
            row_click: #{ payload: #{ type: "string" },
                doc: "Emitted when an enabled row is clicked or Enter is pressed on the selected row; the payload is the row key." },
            row_activate: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
                has_children: #{ schema: #{ type: "bool" }, required: true, sensitive: false },
            } }, doc: "Emitted when an enabled row is double-clicked; `has_children` is the item's field, false without it." },
            context_request: #{ payload: #{ type: "object", allow_unknown: false, fields: #{
                key: #{ schema: #{ type: "optional", value: #{ type: "string" } }, required: true, sensitive: false,
                    doc: "Row the menu is for; `()` from the keyboard when no row is selected or without a selection mode, a menu for the whole list." },
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
                    doc: "Where to open a Menu: the pointer, or the selected row's bounds (the list's when no row is selected or it is not laid out)." },
                source: #{ schema: #{ type: "string", allowed: ["pointer", "keyboard"] }, required: true, sensitive: false,
                    doc: "`pointer` for a right press, `keyboard` for Shift+F10 or the menu key." },
            } }, doc: "Emitted on a right press on a row, after selecting it unless the selection holds it, or on Shift+F10 or the menu key." },
        },
        slots: #{
            loading_content: #{ required: false, multiple: false, doc: "Node shown in the list's place while `loading`." },
            empty: #{ required: false, multiple: false, doc: "Node shown instead of `empty_text` when `items` is empty." },
        },
        parts: ["root", "row", "row_selected", "cursor", "focus_frame", "loading", "empty",
            "scrollbar_track", "scrollbar_thumb", "scrollbar_thumb_hover"],
    },
    render: Fn("render_SelectableList")
});

// The text prop `name`, or the localized `key` message (`fallback` without one) while it
// is empty or unset.
fn localized_prop(ctx, props, name, key, fallback) {
    let value = if name in props && props[name] != () { props[name] } else { "" };
    if value == "" { ctx.t_or(key, fallback) } else { value }
}

fn SelectableList(props) { render_component("components/selectable_list", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn collection_scrollbars(ctx, props, node) {
    node.with_part_style("scrollbar_track", ctx.component_style("scrollbar_track", style().background(rgba(0x00000000))))
        .with_part_style("scrollbar_thumb", ctx.component_style("scrollbar_thumb", style().background(theme_color("scrollbar.thumb"))))
        .with_part_style("scrollbar_thumb_hover", ctx.component_style("scrollbar_thumb_hover", style().background(theme_color("scrollbar.thumb_hover"))))
        .scrollbars("hidden", prop_or(props, "scrollbar", "auto"))
}
// `height` takes logical pixels or a Length.
fn list_length(value) { if type_of(value) == "Length" { value } else { px(value) } }

// ---- shared behavior: List, Table and SelectableList --------------------------------------

// The selection a click on the row `key` leads to, or () without a selection mode.
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

// A click builds the next selection for the clicked row alone, from the selection the list
// was rendered with, so the rows carry only their selected flag. The press focused the row;
// `focus` (the list's ref key, or `()` when the list takes no keys) hands the focus back to
// the list, which owns the keys and shows the cursor frame.
fn row_action(mode, selected, focus, ctx, key) {
    ctx.emit("row_click", key);
    let selection = next_selection(mode, selected, key);
    if selection != () { ctx.emit("selection_change", selection); }
    if focus != () { try { ctx.focus(focus); } catch { } }
}
fn row_pointer(mode, selected, focus, key, has_children, ctx, event) {
    row_action(mode, selected, focus, ctx, key);
    if type_of(event) == "map" && "click_count" in event && event.click_count >= 2 {
        ctx.emit("row_activate", #{ key: key, has_children: has_children });
    }
    handled()
}
// Keyboard navigation moves the selection and keeps the row revealed.
fn keyboard_select(ctx, key) { ctx.emit("selection_change", [key]); handled() }
fn keyboard_activate(ctx, key) { ctx.emit("row_click", key); handled() }
// A right press asks for a context menu for the row; the row is selected first unless the
// selection already holds it. A Table cell's target also names its column.
fn row_context(target, ctx, event) {
    if event.button != "right" { return propagate(); }
    if target.select != () { ctx.emit("selection_change", target.select); }
    let request = #{ key: target.key, source: "pointer",
        anchor: #{ x: event.window.x, y: event.window.y, width: 0.0, height: 0.0 } };
    if "column" in target { request.column = target.column; }
    ctx.emit("context_request", request);
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
    let request = #{ key: target.key, anchor: anchor, source: "keyboard" };
    if "column" in target { request.column = target.column; }
    ctx.emit("context_request", request);
    handled()
}

// The handler a Table row binds to its click: row_action for the row's key.
fn row_click_handler(mode, selected, focus) { Fn("row_action").curry(mode).curry(selected).curry(focus) }
// The handler a row or a cell binds to its pointer press for the context request `target`.
fn row_context_handler(target) { Fn("row_context").curry(target) }

// Up, Down, Home and End move the selection to the neighbors; Enter clicks the current row.
fn navigation_keys(node, neighbors) {
    node = node
        .on_key_value("up", Fn("keyboard_select"), neighbors.previous)
        .on_key_value("down", Fn("keyboard_select"), neighbors.next)
        .on_key_value("home", Fn("keyboard_select"), neighbors.first)
        .on_key_value("end", Fn("keyboard_select"), neighbors.last);
    if neighbors.current != () { node.on_key_value("enter", Fn("keyboard_activate"), neighbors.current) } else { node }
}
// Shift+F10 and the menu key ask for a context menu for `target`.
fn context_keys(node, target) {
    node.on_key_value("shift+f10", Fn("keyboard_context"), target)
        .on_key_value("menu", Fn("keyboard_context"), target)
}

// A pressable row around `children`: the selection tint, the keyboard cursor, the hover fill,
// the row's accessibility and its pointer handlers. `opts`: dividers, below (the row takes its
// content's height from metrics.row up), chrome, context, mode, selected, focus, total.
fn row_frame(ctx, item, payload, children, label, opts) {
    // Selection is the table selection tint; while the list has keyboard focus the selected
    // row also shows the focus frame. Rows between dividers fill the band edge to edge, like
    // Table rows; without dividers a row is a pressable block with a control's corners,
    // spacing.xs in from the list's sides so the corners stay clear of its edge.
    let dividers = opts.dividers;
    let corner = if dividers || !opts.chrome { px(0) } else { theme_radius("md") };
    // The gap spaces the row's own pieces; the cursor below is out of flow.
    let spaced = children.len > 1;
    if item.selected {
        let cursor = style()
            .absolute().left(px(0)).right(px(0)).top(px(0)).bottom(px(0)).radius(corner)
            .border(px(2)).border_color(rgba(0x00000000))
            .group_focus(style().border_color(theme_color("focus_ring")));
        if item.active { children.push(row([]).with_style(ctx.component_style("cursor", cursor))); }
    }
    let base = style().relative().items_center();
    if opts.chrome {
        if spaced { base = base.gap(theme_spacing("sm")); }
        base = base.padding_x(theme_length("metrics.inset")).radius(corner)
            .background(if item.selected { theme_color("table.selection") } else { rgba(0x00000000) });
        base = if dividers { base.width(relative(1.0)) } else { base.margin_x(theme_spacing("xs")) };
        base = if opts.below { base.padding_y(theme_spacing("xs")).min_height(theme_length("metrics.row")) }
            else { base.height(theme_length("metrics.row")) };
        if dividers { base = base.border_bottom(px(1)).border_color(theme_color("border")); }
        if !item.selected && !item.disabled {
            base = base.hover(style().background(theme_color("surface_hover")));
        }
    }
    let node = row(children).with_key(payload.key);
    node = node
        .with_style(ctx.component_style(if item.selected { "row_selected" } else { "row" }, base))
        .disabled(item.disabled)
        .accessibility_role(if opts.mode != "none" { "option" } else { "listitem" })
        .accessibility_label(label)
        .accessibility_selected(item.selected)
        .accessibility_position_in_set(payload.index + 1)
        .accessibility_size_of_set(opts.total);
    if item.disabled { return node.tab_stop(false); }
    if opts.context {
        let select = if opts.mode == "none" || item.selected { () } else { [payload.key] };
        node = node.on("pointer_down", Fn("row_context").curry(#{ key: payload.key, select: select }));
    }
    // A row is a pointer target only: the list is the one tab stop.
    node.on("click", Fn("row_pointer").curry(opts.mode).curry(opts.selected).curry(opts.focus)
        .curry(payload.key).curry(item.has_children))
        .tab_stop(false)
}

// The loading and empty states take the list's place and size, in muted body text.
fn list_state(ctx, props, size, part, content) {
    column([content]).with_key(props.key)
        .with_style(ctx.component_style(part, size.min_width(px(0))
            .padding_x(theme_length("metrics.inset")).padding_y(theme_spacing("sm"))
            .typography("body").text_color(theme_color("text_muted"))))
        .accessibility_role(if prop_or(props, "selection_mode", "none") == "none" { "list" } else { "listbox" }).accessibility_label(props.label)
}

// The list around its virtual rows: the one tab stop and its keys, or the 2px container
// frame for a list that takes only the context keys. `list` holds the list's own settings:
// mode, keyboard, context, fills, length_height, size, focus, neighbors, collection.
fn list_root(ctx, props, rows, list) {
    let mode = list.mode;
    let neighbors = list.neighbors;
    if list.keyboard && mode == "none" && list.context {
        // Without a selection mode there is no current row: the list is a tab stop for the
        // keyboard request alone, which is for the whole list. A wrapper holds that focus and
        // draws the 2px container frame over the list's edge while it does.
        let target = #{ key: (), collection: list.collection, index: () };
        let frame = row([]).with_style(ctx.component_style("focus_frame", style().absolute()
            .left(px(0)).right(px(0)).top(px(0)).bottom(px(0))
            .border(px(2)).border_color(rgba(0x00000000))
            .group_focus(style().border_color(theme_color("focus_ring")))));
        let fill = if list.fills { style().flex_grow().min_height(px(0)) }
            else { style().height(px(props.height)) };
        return context_keys(column([rows.with_style(fill.min_width(px(0))), frame]).with_key(props.key)
            .with_ref(element_ref(list.focus))
            .with_style(ctx.component_style("root", list.size.min_width(px(0)).relative().focus(style())))
            .accessibility_role("list").accessibility_label(props.label)
            .tab_stop(true), target);
    }
    let root = if list.length_height {
        column([rows.with_style(style().flex_grow().min_height(px(0)).min_width(px(0)))])
    } else { rows };
    root = root.with_key(props.key)
        .with_style(ctx.component_style("root", if list.keyboard { list.size.min_width(px(0)).focus(style()) } else { list.size.min_width(px(0)) }))
        .accessibility_role(if mode == "none" { "list" } else { "listbox" })
        .accessibility_label(props.label);
    if mode == "none" || !list.keyboard { return root; }
    if neighbors.first == () { return root; }
    root = navigation_keys(root.with_ref(element_ref(list.focus)).tab_stop(true), neighbors);
    if list.context {
        let target = #{ key: neighbors.current, collection: list.collection, index: neighbors.current_index };
        root = context_keys(root, target);
    }
    root
}

// ---- SelectableList -------------------------------------------------------------------------

fn render_selectable_row(factory, chrome, label_field, current, active, context, total, mode, selected, focus, ctx, payload) {
    let item = payload.item;
    item.active = active;
    if !("has_children" in item) || type_of(item.has_children) != "bool" { item.has_children = false; }
    let content = ctx.render_node(factory, #{ key: payload.key, item: payload.item, index: payload.index,
        selected: item.selected, current: payload.key == current });
    let label = if label_field in item && type_of(item[label_field]) == "string" { item[label_field] }
        else { payload.key };
    let children = [content.with_style(style().flex_grow().min_width(px(0)))];
    row_frame(ctx, item, payload, children, label, #{ dividers: false, below: true, chrome: chrome,
        context: context, mode: mode, selected: selected, focus: focus, total: total })
}

fn render_SelectableList(ctx, props) {
    let fill_height = prop_or(props, "fill_height", false);
    let has_height = "height" in props && props.height != ();
    if has_height == fill_height { throw "SelectableList requires exactly one of height or fill_height"; }
    let mode = prop_or(props, "selection_mode", "none");
    let selected = prop_or(props, "selected_keys", []);
    let keyboard = prop_or(props, "keyboard_navigation", true);
    // A virtual collection takes only a pixel height: under a Length height the rows fill a
    // column of that height instead.
    let length_height = has_height && type_of(props.height) == "Length";
    let size = if fill_height { style().flex_grow().min_height(px(0)) } else { style().height(list_length(props.height)) };
    if prop_or(props, "loading", false) {
        let loading = if "loading_content" in props && props.loading_content != () {
            props.loading_content
        } else { text(ctx.t_or("collection.loading", "Loading…")) };
        return list_state(ctx, props, size, "loading", loading);
    }
    if props.items.len == 0 {
        let empty = if "empty" in props && props.empty != () { props.empty }
            else { text(localized_prop(ctx, props, "empty_text", "common.no_items", "No items")) };
        return list_state(ctx, props, size, "empty", empty);
    }
    let data = native_selection_view(props.items, if mode == "none" { [] } else { selected });
    let collection = #{ key: `${props.key}-rows`, label: props.label, data: data,
        estimated_height: prop_or(props, "estimated_row_height", 48.0), overdraw_pixels: 128.0,
        alignment: "top", follow_tail: false };
    let fills = fill_height || length_height;
    if fills { collection.fill_height = true; } else { collection.height = props.height; }
    let neighbors = ();
    if mode != "none" {
        neighbors = native_selection_neighbors(props.items, selected);
        if neighbors.current != () { collection.reveal_key = neighbors.current; }
    }
    let context = "on_context_request" in props && props.on_context_request != ();
    // The list is a tab stop when it takes keys; a row click hands the focus back to it.
    let focus = if keyboard && (mode == "none" && context || mode != "none" && neighbors.first != ()) {
        `${props.key}-focus`
    } else { () };
    let current = if neighbors == () { () } else { neighbors.current };
    let rows = collection_scrollbars(ctx, props, virtual_collection(collection,
        Fn("render_selectable_row").curry(props.row).curry(prop_or(props, "row_chrome", true))
            .curry(prop_or(props, "label_field", "label")).curry(current)
            .curry(prop_or(props, "active", true)).curry(context).curry(data.len)
            .curry(mode).curry(selected).curry(focus)));
    list_root(ctx, props, rows, #{ mode: mode, keyboard: keyboard, context: context, fills: fills,
        length_height: length_height, size: size, focus: focus, neighbors: neighbors,
        collection: collection.key })
}
