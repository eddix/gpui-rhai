# List

`components/list` · export `List` · version 0.2.0. Generated from
[`registry/components/list.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/list.rhai); do not edit.

Controlled virtualized list of keyed rows: the rows Table draws, without columns.

Rows carry metrics.inset, so their text starts on the edge of a bled DataView or Region body. With a selection mode the list is one tab stop: Up/Down/Home/End move the selection and keep it revealed, Enter emits row_click, and the selected row shows the focus frame while the list has focus. Items are data, like Table cells: a leading status `badge` (#{ text, variant, dot }), the `title`, a muted `secondary` line (beside the title or below it) and trailing `meta` such as a time or a count. Badges of different widths push the titles apart; `badge_width` reserves one slot on every row so the titles align.

```rhai
import "components/list" as list;

list::List(#{ key: "tickets", label: "Tickets", items: items, selection_mode: "single",
    selected_keys: [current], height: 320.0, on_selection_change: Fn("selected") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `badge_width` | number > 0 or `()` | — | Badge slot width in logical pixels, reserved on every row so the titles align; `()` fits each badge. |
| `dividers` | bool | `false` | Draws a 1px border under every row. |
| `empty` | node or `()` | — | Node shown instead of `empty_text` when `items` is empty. |
| `empty_text` | string | `"No items"` | Muted text shown when `items` is empty, unless `empty` is given. |
| `fill_height` | bool | `false` | Grows into the free height of a flex parent instead of a fixed `height`; give exactly one of the two. |
| `height` | number > 0 or `()` | — | Viewport height in logical pixels; give exactly one of `height` or `fill_height`. |
| `items` | array of object (at most 10000) | required | Rows in display order. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the list. |
| `on_context_request` | callback or `()` | — | Called with `#{ key, anchor, source }` when a row is right-pressed or Shift+F10 or the menu key is pressed. |
| `on_row_click` | callback or `()` | — | Called with the row key when an enabled row is clicked, or Enter is pressed on the selected row. |
| `on_selection_change` | callback or `()` | — | Called with the next selected keys when a row is clicked, Up, Down, Home or End is pressed, or a context request selects a row. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `secondary_layout` | `"inline"` or `"below"` | `"inline"` | `inline` sets `secondary` beside the title at body size; `below` sets it on a caption line under it. |
| `selected_keys` | array of string (at most 10000) | `[]` | Selected row keys; the caller stores the `selection_change` payload and passes it back. |
| `selection_mode` | `"none"` or `"single"` or `"multiple"` | `"none"` | `single` selects the clicked row and a second click clears it, `multiple` toggles it; any mode but `none` makes the list one tab stop. |
| `style` | style | — | Style merged over the root part. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `badge` | object or `()` | — | Leading status Badge, before the title; see `badge_width` to align titles. |
| `disabled` | bool | `false` | Mutes the row; it ignores clicks and the keyboard skips it. |
| `key` | string | required | Unique row key, used in `selected_keys` and every event. |
| `meta` | string or `()` | — | Trailing muted text such as a time or a count, in tabular figures. |
| `secondary` | string or `()` | — | Muted text beside the title or below it, as `secondary_layout` sets. |
| `title` | string | required | Main row text, on one line truncated with an ellipsis. |

### `items[].badge` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `dot` | bool | `true` | Shows the badge's status lamp before its text. |
| `text` | string | required | Badge label. |
| `variant` | `"neutral"` or `"accent"` or `"success"` or `"warning"` or `"danger"` | `"neutral"` | Status tone of the badge. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `context_request` | `on_context_request` | object | Emitted on a right press on a row, after selecting it unless the selection holds it, or on Shift+F10 or the menu key with a selection mode. |
| `row_click` | `on_row_click` | string | Emitted when an enabled row is clicked or Enter is pressed on the selected row; the payload is the row key. |
| `selection_change` | `on_selection_change` | array of string (at most 10000) | Emitted when a click, Up, Down, Home, End or a context request selects rows; the payload is the next selection. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `empty` | no | no | Node shown instead of `empty_text` when `items` is empty. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `badge`, `cursor`, `empty`, `indicator_bar`, `meta`, `root`, `row`, `row_selected`, `secondary`, `title`.

## Theme

- Tokens: `accent`, `border`, `disabled`, `focus_ring`, `metrics.inset`, `metrics.row`, `spacing.sm`, `spacing.xs`, `surface_hover`, `table.selection`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/badge`](badge.md)
