# Table

`components/table` · export `Table` · version 0.2.0. Generated from
[`registry/components/table.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/table.rhai); do not edit.

Data-backed Table composed entirely from public Box/Text/virtual_collection APIs. Use either a positive logical-pixel height or fill_height for flex layouts.

State: application values are controlled; keyed native width overrides retain drag previews.

```rhai
import "components/table" as table;

table::Table(#{ key: "users", label: "Users", row_key: "id", rows: rows,
    columns: columns, height: 420 })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `collapsed_groups` | array of string (at most 100000) | `[]` | Group values whose rows are hidden; needs `group_by`, and the caller toggles the `group_toggle` payload in it. |
| `columns` | array of object (at most 256) | required | Columns in display order; 1 to 256 with unique keys. |
| `empty` | node or `()` | — | Node shown instead of `empty_text` when no row is left to show. |
| `empty_text` | string | `"No results"` | Muted text shown when no row is left after filtering and paging, unless `empty` is given. |
| `estimated_row_height` | number > 0 | `32.0` | Row height in logical pixels the virtual body assumes before rows are measured; rows draw at `metrics.row`. |
| `fill_height` | bool | `false` | Grows into the free height of a flex parent instead of a fixed `height`; give exactly one of the two. |
| `group_by` | string or `()` | — | Column key whose values, non-empty strings, group the rows under headers in first-seen order. |
| `height` | number > 0 or `()` | — | Height of the rows area below the header in logical pixels; give exactly one of `height` or `fill_height`. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the table; rows are announced as `<label> row <n>`. |
| `loading` | bool | `false` | Shows `loading_content` below the header instead of the rows. |
| `loading_content` | node or `()` | — | Node shown while `loading`; `()` shows a muted "Loading…" line. |
| `on_column_resize` | callback or `()` | — | Called with `#{ key, width }` when a resize ends; with it the caller owns widths and writes `width` into the column. |
| `on_context_request` | callback or `()` | — | Called with `#{ key, column, anchor, source }` on a context request; open a Menu at `anchor`, in window coordinates. |
| `on_group_toggle` | callback or `()` | — | Called with the group value when a group header is clicked; without it group headers are not clickable. |
| `on_row_click` | callback or `()` | — | Called with the row key when a row is clicked, or Enter is pressed on the selected row. |
| `on_selection_change` | callback or `()` | — | Called with the next selected keys when a row is clicked, Up, Down, Home or End is pressed, or a context request selects a row. |
| `on_sort_change` | callback or `()` | — | Called with the next `sort`, or `()` after descending, when a sortable header is clicked. |
| `page` | integer ≥ 1 | `1` | One-based page shown when `page_size` is set, counted after `query` filters the rows. |
| `page_size` | integer ≥ 1 or `()` | — | Rows per page; `()` shows every row that matches `query`. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `query` | string | `""` | Case-insensitive substring filter over `search_fields`; a non-empty query needs `search_fields`. |
| `resizable_columns` | bool | `false` | Makes columns resizable: drag a header edge, double-click it to fit the content, or press Left/Right on its handle. |
| `row_key` | string | required | Row field whose value, as a string, identifies the row in `selected_keys` and events. |
| `rows` | array of map of any value (at most 100000) or native collection | required | Row maps keyed by field, or a NativeCollection that sorts, groups and filters in Rust; cells show values as strings. |
| `search_fields` | array of string (at most 256) | `[]` | Row fields `query` searches; each is `row_key` or a declared column key. |
| `selected_keys` | array of string (at most 100000) | `[]` | Selected row keys; the caller stores the `selection_change` payload and passes it back. |
| `selection_mode` | `"none"` or `"single"` or `"multiple"` | `"none"` | `single` selects the clicked row and a second click clears it, `multiple` toggles it; any mode but `none` makes the table one tab stop. |
| `sort` | object or `()` | — | Current sort; the caller stores the `sort_change` payload. A NativeCollection is sorted by it; array rows keep the order given. |
| `sticky_group_headers` | bool | `true` | Keeps the current group header pinned at the top of the rows while they scroll. |
| `striped` | bool | `false` | Tints every second displayed row with `surface_raised`. |
| `style` | style | — | Style merged over the root part. |

### `columns[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `adornments` | array of object (at most 4) | `[]` | Status badges drawn in the cell from row fields; a cell whose text repeats a badge shows only the badge. |
| `align` | `"start"` or `"center"` or `"end"` or `()` | — | Cell alignment; `()` aligns to `start`, or to `end` in a `numeric` column. |
| `key` | string | required | Row field the column shows; unique among the columns. |
| `max_width` | number > 0, ≤ 16384 or `()` | — | Largest width in logical pixels a resize may reach, at least `min_width`; `()` leaves it open. |
| `min_width` | number > 0, ≤ 16384 or `()` | — | Smallest width in logical pixels a resize may reach; `()` means 48. |
| `numeric` | bool | `false` | Sets the cells in tabular figures and aligns them to the end unless `align` is given. |
| `resizable` | bool or `()` | — | Turns resizing on or off for this column; `()` follows `resizable_columns`. |
| `sortable` | bool | `false` | Makes the header clickable; clicks cycle ascending, descending and unsorted through `sort_change`. |
| `title` | string | required | Header text, shown in capitals in the label voice. |
| `typography` | string or `()` | — | Typography role of the column's cells, such as `code` for identifiers that must tell 0 from O. |
| `width` | object | required | Width before any resize; store the `column_resize` payload's width here when `on_column_resize` is set. |

### `columns[].adornments[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `dot` | bool | `false` | Shows the badge's status lamp before its text. |
| `text_key` | string | required | Row field whose value is the badge text; an empty or missing value drops the badge. |
| `variant` | `"neutral"` or `"accent"` or `"success"` or `"warning"` or `"danger"` | `"neutral"` | Tone of the badge when `variant_key` does not give one. |
| `variant_key` | string or `()` | — | Row field holding the tone per row; it overrides `variant` and must name a valid tone. |

### `columns[].width` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `kind` | `"fixed"` or `"percent"` or `"flex"` | required | `fixed` in logical pixels, `percent` of the viewport width, or `flex` for a share of the space left. |
| `value` | number > 0 | required | Logical pixels up to 16384, a percentage up to 100, or a flex weight, as `kind` says. |

### `sort` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `direction` | `"ascending"` or `"descending"` | required | Sort direction, shown by the header icon. |
| `key` | string | required | Key of the sorted column. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `column_resize` | `on_column_resize` | object | Emitted when a header-edge drag ends, a double-click fits the column, or Left/Right on a handle steps it 8px; the payload is the new width. |
| `context_request` | `on_context_request` | object | Emitted on a right press on a cell, after selecting its row unless the selection holds it, or Shift+F10 or the menu key on the current row. |
| `group_toggle` | `on_group_toggle` | string | Emitted when a group header is clicked, only with `on_group_toggle` set; the payload is the group value. |
| `row_click` | `on_row_click` | string | Emitted when a row is clicked or Enter is pressed on the selected row; the payload is the row key. |
| `selection_change` | `on_selection_change` | array of string (at most 100000) | Emitted when a click, Up, Down, Home, End or a context request selects rows; the payload is the next selection. |
| `sort_change` | `on_sort_change` | object or `()` | Emitted when a sortable header is clicked; the payload is the next `sort`, or `()` after descending. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `empty` | no | no | Node shown instead of `empty_text` when no row is left to show. |
| `loading_content` | no | no | Node shown below the header while `loading`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `body`, `cursor`, `empty`, `group_count`, `group_header`, `group_indicator`, `group_label`, `header`, `header_cell`, `indicator_bar`, `loading`, `resize_handle`, `root`, `row`, `row_selected`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `metrics.icon`, `metrics.inset`, `metrics.row`, `selection`, `spacing.md`, `spacing.sm`, `spacing.xs`, `surface_hover`, `surface_raised`, `table.selection`, `text_muted`, `text_primary`, `typography.body`, `typography.label`
- Environment: `density`

## Dependencies

[`components/badge`](badge.md)
