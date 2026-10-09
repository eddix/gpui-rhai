# Sortable

`components/sortable` · export `Sortable` · version 0.2.0. Generated from
[`registry/components/sortable.rhai`](../../../registry/components/sortable.rhai); do not edit.

Controlled keyed ordering with native pointer targets and keyboard moves. State: order is caller-owned; native state contains only one transient drag. Keyboard: focus a handle, then Option/Alt+Arrow, Home, or End.

```rhai
import "components/sortable" as sortable;

sortable::Sortable(#{ key:"queue", label:"Queue", items:items, on_reorder:Fn("reordered") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `direction` | `"vertical"` or `"horizontal"` or `"grid"` | `"vertical"` | Lays rows out as a column, a scrolling row or a grid of `grid_columns`; `virtual_data` needs `vertical`. |
| `disabled` | bool | `false` | Disables dragging and keyboard moves for every row. |
| `estimated_row_height` | number > 0 | `40.0` | Row height the virtual list assumes before rows are measured, in logical pixels. |
| `fill_height` | bool | `false` | Lets the virtual list fill its parent's height; give exactly one of `height` and `fill_height`. |
| `gap` | length or `()` | — | Space between rows, `spacing.xs` when unset; ignored with `virtual_data`. |
| `grid_columns` | integer 1–64 | `3` | Number of columns when `direction` is `grid`. |
| `height` | number > 0 or `()` | — | Viewport height of the virtual list, in logical pixels; give exactly one of `height` and `fill_height`. |
| `items` | array of object (at most 512) or `()` | — | Rows in their current order, owned by the caller; give exactly one of `items` and `virtual_data`. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the list. |
| `label_key` | string | `"label"` | Field of a `virtual_data` row shown as its text and used as its accessible name. |
| `on_reorder` | callback or `()` | — | Called with the `reorder` payload when a row is dropped or moved with Option/Alt and an arrow, Home or End. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `threshold` | number 0–64 | `4.0` | Pointer travel in logical pixels before a press becomes a drag. |
| `virtual_data` | array of map of any value (at most 10000) or native collection or `()` | — | Virtualized rows: maps with a string `key`, a `label_key` field and an optional `disabled`. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `content` | node | required | Node shown beside the drag handle. |
| `disabled` | bool | `false` | Keeps the row from being dragged or moved by key; rows can still drop beside it. |
| `key` | string | required | Identity of the row, unique in the list; `reorder` reports it. |
| `label` | string | required | Accessible name of the row; its handle reads `Reorder <label>`. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `reorder` | `on_reorder` | object | Emitted when a row is dropped or moved by key; the payload names the move, and the caller reorders its rows. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `handle`, `handle_icon`, `item`, `item_disabled`, `root`.

## Theme

- Tokens: `border`, `focus_ring`, `metrics.control`, `metrics.inset`, `metrics.row`, `spacing.xs`, `surface`, `text_muted`, `typography.body`
- Environment: `density`
