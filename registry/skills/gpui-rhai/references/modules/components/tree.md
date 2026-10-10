# Tree

`components/tree` · export `Tree` · version 0.2.0. Generated from
[`registry/components/tree.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/tree.rhai); do not edit.

Controlled virtualized hierarchical outline over the shared Rust projection.

```rhai
import "components/tree" as tree;

tree::Tree(#{key:"files",label:"Files",items:items,expanded:[],selected_keys:[],height:320})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `active_key` | string or `()` | — | Keyboard cursor row, kept in view; the caller stores the `active_change` payload. Without one the cursor is on the first enabled row. |
| `disabled` | bool | `false` | Disables the whole tree: it leaves the tab order and its rows ignore clicks. |
| `estimated_row_height` | number > 0 | `32.0` | Row height in logical pixels the virtual list assumes before rows are measured; rows draw at `metrics.row`. |
| `expanded` | array of string (at most 10000) | required | Keys of expanded nodes, each present in `items`; the caller stores the `expanded_change` payload. |
| `fill_height` | bool | `false` | Fills the height the parent gives instead of a fixed `height`; give exactly one of the two. |
| `height` | number > 0 or `()` | — | Viewport height in logical pixels; give exactly one of `height` or `fill_height`. |
| `items` | array of object (at most 10000) | required | Outline nodes as one flat list linked by `parent`; siblings keep their order in this list. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the tree. |
| `on_active_change` | callback or `()` | — | Called with the new cursor key when a row is clicked or the arrow keys, Home or End move the cursor. |
| `on_expanded_change` | callback or `()` | — | Called with the next `expanded` keys when a disclosure is clicked or Left/Right toggles a row. |
| `on_selection_change` | callback or `()` | — | Called with the next selected keys when a row is clicked or Enter is pressed, unless `selection_mode` is `none`. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `selected_keys` | array of string (at most 10000) | required | Selected node keys; the caller stores the `selection_change` payload and passes it back. |
| `selection_mode` | `"none"` or `"single"` or `"multiple"` | `"single"` | `single` selects the clicked row and a second click keeps it selected, `multiple` toggles it in the selection, `none` only moves the cursor. |
| `style` | style | — | Style merged over the root part. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Mutes the row; it cannot be clicked, toggled or reached with the keyboard. |
| `key` | string | required | Unique node key, used in `expanded`, `selected_keys`, `active_key` and every event. |
| `label` | string | required | Row text, on one line truncated with an ellipsis; also the row's accessible name. |
| `loading` | bool | `false` | Shows a trailing ellipsis while the node's children load. |
| `parent` | string or `()` | — | Key of the parent node, which must be in `items`; `()` puts the node at the root. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `active_change` | `on_active_change` | string or `()` | Emitted when a click or Up, Down, Home, End, Left or Right moves the cursor; the payload is the new cursor key. |
| `expanded_change` | `on_expanded_change` | array of string (at most 10000) | Emitted when a disclosure is clicked or Left/Right collapses or expands the cursor row; the payload is the next `expanded` list. |
| `selection_change` | `on_selection_change` | array of string (at most 10000) | Emitted when a row is clicked or Enter is pressed, unless `selection_mode` is `none`; the payload is the next selection. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `cursor`, `disclosure`, `indicator_bar`, `label`, `loading`, `root`, `row`, `row_active`, `row_selected`.

## Theme

- Tokens: `accent`, `disabled`, `focus_ring`, `metrics.icon`, `metrics.inset`, `metrics.row`, `selection`, `spacing.lg`, `spacing.xs`, `surface_hover`, `text_muted`, `text_primary`, `typography.body`
- Environment: `density`
