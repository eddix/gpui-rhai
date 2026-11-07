# SelectableList

`components/selectable_list` · export `SelectableList` · version 0.4.3. Generated from
[`registry/components/selectable_list.rhai`](../../../registry/components/selectable_list.rhai); do not edit.

The behavior of List and Table rows with rows the application draws: selection, the one tab stop and its keys, the cursor and focus frame, row and context requests, accessibility and virtualization stay the list's, and a `row` Node factory draws each row's content.

Items are maps with a unique `key` and any fields of the application's own; `disabled` mutes a row and the keys skip it, and the field `label_field` names (`label` by default) names the row for assistive technology. The factory gets `#{ key, item, index, selected, current }`, current being the row the keys act on. Callbacks in what it draws reach the application. By default each row is a pressable block like a List row: the selection tint, the hover fill and a control's corners, spacing.xs in from the list's sides; `row_chrome: false` leaves the row bare for the factory to paint. Rows take their own heights.

List and Table share this module's behavior functions: next_selection, row_frame, navigation_keys, context_keys and the handlers they bind.

```rhai
import "components/selectable_list" as selectable_list;

selectable_list::SelectableList(#{ key: "tracks", label: "Tracks", items: tracks,
                 selection_mode: "single", selected_keys: state.selected,
                 row: ctx.node_factory(Fn("track_row")), on_selection_change: Fn("selected") })
    fn track_row(ctx, row) { row([cover(row.item), column([text(row.item.title), text(row.item.artist)])]) }
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `active` | bool | `true` | Draws the selected cursor as active; false keeps the selection visible without its focus frame. |
| `empty` | node or `()` | — | Node shown instead of `empty_text` when `items` is empty. |
| `empty_text` | string | `""` | Muted text shown when `items` is empty, unless `empty` is given. Empty, the localized `common.no_items` message, `No items` without one. |
| `estimated_row_height` | number > 0 | `48.0` | Height the virtual collection assumes for rows it has not measured, in logical pixels. |
| `fill_height` | bool | `false` | Grows into the free height of a flex parent instead of a fixed `height`; give exactly one of the two. |
| `height` | number > 0 or length or `()` | — | Viewport height: logical pixels, or a Length; give exactly one of `height` or `fill_height`. |
| `items` | native collection or array of any value (at most 10000) | required | Rows in display order: maps with a unique `key`, an optional `disabled` bool and the application's own fields; a NativeCollection is navigated in Rust without enumerating it in Rhai. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `keyboard_navigation` | bool | `true` | Owns the single tab stop and navigation keys. |
| `label` | string | required | Accessible name of the list. |
| `label_field` | string | `"label"` | Item field that names each row for assistive technology; a row without it is named by its key. |
| `loading` | bool | `false` | Shows `loading_content` in the list's place instead of the rows. |
| `loading_content` | node or `()` | — | Node shown while `loading`; `()` shows a muted line with the localized `collection.loading` message ("Loading…"). |
| `on_context_request` | callback or `()` | — | Called with `#{ key, anchor, source }` when a row is right-pressed or Shift+F10 or the menu key is pressed; without a selection mode it makes the list a tab stop for those keys. |
| `on_row_activate` | callback or `()` | — | Called with #{ key, has_children } on a double click. |
| `on_row_click` | callback or `()` | — | Called with the row key when an enabled row is clicked, or Enter is pressed on the selected row. |
| `on_selection_change` | callback or `()` | — | Called with the next selected keys when a row is clicked, Up, Down, Home or End is pressed, or a context request selects a row. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `row` | callback | required | Node factory bound with ctx.node_factory(Fn(...)), called as Fn(ctx, row) for each realized row; row is #{ key, item, index, selected, current }. Its nodes' callbacks reach the scope that bound it. |
| `row_chrome` | bool | `true` | Draws each row as a List row draws: the selection tint, the hover fill, a control's corners and the content inset. False leaves the row bare; the factory paints the selected state from `row.selected`. |
| `scrollbar` | `"auto"` or `"always"` or `"hidden"` | `"auto"` | Shared native viewport scrollbar visibility. |
| `selected_keys` | array of string (at most 10000) | `[]` | Selected row keys; the caller stores the `selection_change` payload and passes it back. |
| `selection_mode` | `"none"` or `"single"` or `"multiple"` | `"none"` | `single` selects the clicked row and a second click clears it, `multiple` toggles it; any mode but `none`, or `on_context_request`, makes the list one tab stop. |
| `style` | style | — | Style merged over the root part. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `context_request` | `on_context_request` | object | Emitted on a right press on a row, after selecting it unless the selection holds it, or on Shift+F10 or the menu key. |
| `row_activate` | `on_row_activate` | object | Emitted when an enabled row is double-clicked; `has_children` is the item's field, false without it. |
| `row_click` | `on_row_click` | string | Emitted when an enabled row is clicked or Enter is pressed on the selected row; the payload is the row key. |
| `selection_change` | `on_selection_change` | array of string (at most 10000) | Emitted when a click, Up, Down, Home, End or a context request selects rows; the payload is the next selection. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `empty` | no | no | Node shown instead of `empty_text` when `items` is empty. |
| `loading_content` | no | no | Node shown in the list's place while `loading`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `cursor`, `empty`, `focus_frame`, `loading`, `root`, `row`, `row_selected`, `scrollbar_thumb`, `scrollbar_thumb_hover`, `scrollbar_track`.

## Theme

- Tokens: `border`, `focus_ring`, `metrics.inset`, `metrics.row`, `radius.md`, `scrollbar.thumb`, `scrollbar.thumb_hover`, `spacing.sm`, `spacing.xs`, `surface_hover`, `table.selection`, `text_muted`, `typography.body`
- Environment: `density`, `size`
