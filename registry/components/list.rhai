/* gpui-rhai
{
  "id": "components/list",
  "export": "List",
  "version": "0.4.3",
  "runtime_api": { "min_inclusive": 3, "max_exclusive": 4 },
  "dependencies": ["components/badge", "components/selectable_list"],
  "capabilities": {},
  "tokens": [
    "table.selection", "surface_hover", "focus_ring", "border", "text_primary",
    "text_muted", "disabled", "metrics.row", "metrics.inset", "spacing.xs", "spacing.sm",
    "typography.body", "typography.caption", "scrollbar.thumb", "scrollbar.thumb_hover", "radius.md"
  ],
  "environment": ["size", "density"]
}
*/
// Controlled virtualized list of keyed rows: the rows Table draws, without columns.
// Example: List(#{ key: "tickets", label: "Tickets", items: items, selection_mode: "single",
//              selected_keys: [current], height: 320.0, on_selection_change: Fn("selected") })
//
// Rows carry metrics.inset, so their text starts on the edge of a bled DataView or Region
// body. With a selection mode the list is one tab stop: Up/Down/Home/End move the selection
// and keep it revealed, Enter emits row_click, and the selected row shows the focus frame
// while the list has focus. Without one, on_context_request alone makes it a tab stop for
// Shift+F10 and the menu key. Items are data, like Table cells: a leading status `badge`
// (#{ text, variant, dot }), the `title`, a muted `secondary` line (beside the title or
// below it) and trailing `meta` such as a time or a count. Badges of different widths push
// the titles apart; `badge_width` reserves one slot on every row so the titles align. The
// rows' behavior is SelectableList's (components/selectable_list), with rows of this content.

import "components/badge" as badge;
import "components/selectable_list" as selectable_list;

define_component(#{
    metadata: #{ id: "components/list", "export": "List", version: "0.4.3",
        runtime_api: #{ min_inclusive: 3, max_exclusive: 4 },
        dependencies: ["components/badge", "components/selectable_list"], capabilities: #{},
        tokens: [
            "table.selection", "surface_hover", "focus_ring", "border", "text_primary",
            "text_muted", "disabled", "metrics.row", "metrics.inset", "spacing.xs", "spacing.sm",
            "typography.body", "typography.caption", "scrollbar.thumb", "scrollbar.thumb_hover", "radius.md"
        ],
        environment: ["size", "density"] },
    schema: #{
        props: #{
            scrollbar: #{ schema: #{ type: "string", allowed: ["auto", "always", "hidden"] }, required: false, sensitive: false,
                "default": #{ type: "string", value: "auto" }, doc: "Shared native viewport scrollbar visibility." },
            key: #{ schema: #{ type: "string" }, required: true, sensitive: false },
            label: #{ schema: #{ type: "string" }, required: true, sensitive: false,
                doc: "Accessible name of the list." },
            items: #{ schema: #{ type: "one_of", variants: [#{ type: "collection" }, #{ type: "array", max_items: 10000, items: #{
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
                    has_children: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                        "default": #{ type: "bool", value: false }, doc: "Shows a logical child disclosure indicator for hierarchical browsing." },
                } } }] }, required: true, sensitive: false, doc: "Rows in display order; a NativeCollection is normalized and navigated in Rust without enumerating it in Rhai." },
            keyboard_navigation: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true }, doc: "Owns the single tab stop and navigation keys. Set false only when a parent such as ColumnBrowser owns keyboard navigation and focus; selected rows still remain revealed." },
            active: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: true }, doc: "Draws the selected cursor as active; false keeps ancestor selection visible without its focus frame." },
            selection_mode: #{ schema: #{ type: "string", allowed: ["none", "single", "multiple"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "none" },
                doc: "`single` selects the clicked row and a second click clears it, `multiple` toggles it; any mode but `none`, or `on_context_request`, makes the list one tab stop." },
            selected_keys: #{ schema: #{ type: "array", max_items: 10000, items: #{ type: "string" } },
                required: false, sensitive: false, "default": #{ type: "array", value: [] },
                doc: "Selected row keys; the caller stores the `selection_change` payload and passes it back." },
            secondary_layout: #{ schema: #{ type: "string", allowed: ["inline", "below"] },
                required: false, sensitive: false, "default": #{ type: "string", value: "inline" },
                doc: "`inline` sets `secondary` beside the title at body size; `below` sets it on a caption line under it." },
            dividers: #{ schema: #{ type: "bool" }, required: false, sensitive: false,
                "default": #{ type: "bool", value: false }, doc: "Draws a 1px border under every row." },
            badge_width: #{ schema: #{ type: "optional", value: #{ type: "one_of", variants: [
                #{ type: "number", exclusive_min: 0.0 }, #{ type: "length" }] } },
                required: false, sensitive: false,
                doc: "Badge slot width reserved on every row so the titles align: logical pixels, or a Length resolved in the list's density; `()` fits each badge." },
            height: #{ schema: #{ type: "optional", value: #{ type: "one_of", variants: [
                #{ type: "number", exclusive_min: 0.0 }, #{ type: "length" }] } },
                required: false, sensitive: false,
                doc: "Viewport height: logical pixels, or a Length such as `theme_length(\"metrics.row\") * 6`; give exactly one of `height` or `fill_height`." },
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
                doc: "Called with #{ key, has_children } on a double click; useful to distinguish browsing selection from leaf activation." },
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
            } }, doc: "Emitted when an enabled row is double-clicked." },
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
            } }, doc: "Emitted on a right press on a row, after selecting it unless the selection holds it, or on Shift+F10 or the menu key: for the selected row with a selection mode, for the list without one." },
        },
        slots: #{
            loading_content: #{ required: false, multiple: false, doc: "Node shown in the list's place while `loading`." },
            empty: #{ required: false, multiple: false,
                doc: "Node shown instead of `empty_text` when `items` is empty." },
        },
        parts: ["root", "row", "row_selected", "badge", "title", "secondary", "meta",
            "cursor", "focus_frame", "loading", "empty", "scrollbar_track", "scrollbar_thumb", "scrollbar_thumb_hover"],
    },
    render: Fn("render_List")
});

// The text prop `name`, or the localized `key` message (`fallback` without one) while it
// is empty or unset.
fn localized_prop(ctx, props, name, key, fallback) {
    let value = if name in props && props[name] != () { props[name] } else { "" };
    if value == "" { ctx.t_or(key, fallback) } else { value }
}

fn List(props) { render_component("components/list", props) }
fn prop_or(props, name, fallback) { if name in props { props[name] } else { fallback } }
fn field_or(item, name, fallback) { if name in item && item[name] != () { item[name] } else { fallback } }
fn is_rtl(ctx) { let rtl = false; try { rtl = ctx.text_direction() == "rtl"; } catch { } rtl }
// `height` takes logical pixels or a Length.
fn list_length(value) { if type_of(value) == "Length" { value } else { px(value) } }

fn render_list_row(active, below, dividers, badge_slot, context, total, mode, selected, focus, ctx, payload) {
    let item = payload.item;
    item.active = active;
    let children = [];
    let badge = if item.badge == () { () } else {
        badge::Badge(#{ key: `${payload.key}-badge`, text: item.badge.text,
            variant: item.badge.variant, dot: item.badge.dot, size: "sm" })
            .with_key(`${payload.key}-badge`)
            .with_style(ctx.component_style("badge", style().flex_shrink(false)))
    };
    if type_of(badge_slot) == "UiNode" {
        // A Length slot: a zero-height spacer of that width with the badge under it.
        children.push(column(if badge == () { [badge_slot] } else { [badge_slot, badge] })
            .with_key(`${payload.key}-slot`).with_style(style().flex_shrink(false).items_start()));
    } else if badge_slot != () {
        children.push(row(if badge == () { [] } else { [badge] }).with_key(`${payload.key}-slot`)
            .with_style(style().width(px(badge_slot)).flex_shrink(false).items_center()));
    } else if badge != () { children.push(badge); }
    let title = text(item.title).with_key(`${payload.key}-title`)
        .with_style(ctx.component_style("title", style().min_width(px(0)).whitespace_nowrap()
            .text_ellipsis().typography("body")
            .text_color(if item.disabled { theme_color("disabled") } else { theme_color("text_primary") })));
    let label = item.title;
    // On the selection tint the supporting text takes the title's color: text_muted is
    // tuned for the surfaces, not for the tint.
    let supporting = if item.selected && !item.disabled { theme_color("text_primary") }
        else { theme_color("text_muted") };
    if item.secondary != () {
        label = `${label}, ${item.secondary}`;
        // Beside the title the secondary text keeps the body size, so the row shares one
        // line box; below it, it is a caption line.
        let secondary = text(item.secondary).with_key(`${payload.key}-secondary`)
            .with_style(ctx.component_style("secondary", style().min_width(px(0)).whitespace_nowrap()
                .text_ellipsis().typography(if below { "caption" } else { "body" })
                .text_color(supporting)));
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
                .typography("body").font_feature("tnum", 1).text_color(supporting))));
    }
    if item.has_children {
        children.push(text(if is_rtl(ctx) { "‹" } else { "›" })
            .with_style(style().flex_shrink(false).text_color(theme_color("text_muted"))));
    }
    selectable_list::row_frame(ctx, item, payload, children, label, #{ dividers: dividers, below: below,
        chrome: true, context: context, mode: mode, selected: selected, focus: focus, total: total })
}

fn render_List(ctx, props) {
    let fill_height = prop_or(props, "fill_height", false);
    let has_height = "height" in props && props.height != ();
    if has_height == fill_height { throw "List requires exactly one of height or fill_height"; }
    let mode = prop_or(props, "selection_mode", "none");
    let selected = prop_or(props, "selected_keys", []);
    let keyboard = prop_or(props, "keyboard_navigation", true);
    let below = prop_or(props, "secondary_layout", "inline") == "below";
    // A virtual collection takes only a pixel height: under a Length height the rows fill a
    // column of that height instead.
    let length_height = has_height && type_of(props.height) == "Length";
    let size = if fill_height { style().flex_grow().min_height(px(0)) } else { style().height(list_length(props.height)) };
    if prop_or(props, "loading", false) {
        let loading = if "loading_content" in props && props.loading_content != () {
            props.loading_content
        } else { text(ctx.t_or("collection.loading", "Loading…")) };
        return selectable_list::list_state(ctx, props, size, "loading", loading);
    }
    if props.items.len == 0 {
        let empty = if "empty" in props && props.empty != () { props.empty }
            else { text(localized_prop(ctx, props, "empty_text", "common.no_items", "No items")) };
        return selectable_list::list_state(ctx, props, size, "empty", empty);
    }
    // The rows stay linear in the items: each carries its selected flag, and the selection
    // reaches the row renderer once (a click builds the next selection then), so the data
    // does not grow with rows times selected keys.
    let data = native_list_view(props.items, if mode == "none" { [] } else { selected });
    let collection = #{ key: `${props.key}-rows`, label: props.label, data: data,
        estimated_height: if below { 48.0 } else { 32.0 }, overdraw_pixels: 128.0,
        alignment: "top", follow_tail: false };
    let fills = fill_height || length_height;
    if fills { collection.fill_height = true; } else { collection.height = props.height; }
    let neighbors = ();
    if mode != "none" {
        neighbors = native_list_neighbors(data, selected);
        if neighbors.current != () { collection.reveal_key = neighbors.current; }
    }
    let context = "on_context_request" in props && props.on_context_request != ();
    // A row renderer's curry is plain data, which a Length is not: a Length badge width
    // reaches the rows as a spacer node of that width.
    let badge_slot = prop_or(props, "badge_width", ());
    if type_of(badge_slot) == "Length" {
        badge_slot = row([]).with_style(style().width(badge_slot));
    }
    // The list is a tab stop when it takes keys; a row click hands the focus back to it.
    let focus = if keyboard && (mode == "none" && context || mode != "none" && neighbors.first != ()) {
        `${props.key}-focus`
    } else { () };
    let rows = selectable_list::collection_scrollbars(ctx, props, virtual_collection(collection,
        Fn("render_list_row").curry(prop_or(props, "active", true)).curry(below).curry(prop_or(props, "dividers", false))
            .curry(badge_slot).curry(context).curry(data.len)
            .curry(mode).curry(selected).curry(focus)));
    selectable_list::list_root(ctx, props, rows, #{ mode: mode, keyboard: keyboard, context: context,
        fills: fills, length_height: length_height, size: size, focus: focus, neighbors: neighbors,
        collection: collection.key })
}
