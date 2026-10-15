# ColumnBrowser

`components/column_browser` · export `ColumnBrowser` · version 0.2.2. Generated from
[`registry/components/column_browser.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/column_browser.rhai); do not edit.

Controlled Miller columns. Data loading, cancellation, request revisions and dropping descendants belong to the caller, not this presentational component.

```rhai
import "components/column_browser" as column_browser;

column_browser::ColumnBrowser(#{ key: "files", label: "Files", columns: [
        #{ key: "root", title: "Root", items: [#{ key: "config", title: "Configuration", has_children: true }] }
    ] })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `column_width` | number > 0 or length | `240.0` | Default nonshrinking column width; the strip overflows horizontally in narrow windows. |
| `columns` | array of object (at most 128) | required | Controlled column chain. After selection the caller replaces descendants and guards late loading results by path/request revision. |
| `detail` | node or `()` | — | Optional caller-owned trailing detail pane, using column_width. |
| `fill_height` | bool | `false` | Fill the bounded flex parent's remaining height. |
| `height` | number > 0 or length or `()` | — | Total browser height; give height or fill_height, not both. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the browser. |
| `on_activate` | callback or `()` | — | Called with #{ column, key, path } on Enter or double click of a leaf. |
| `on_retry` | callback or `()` | — | Called with the failed column key; retry and loading remain caller-owned. |
| `on_select` | callback or `()` | — | Called with #{ column, key, path }; the caller owns updating the chain. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `path` | array of string (at most 128) or `()` | — | Optional controlled selected path; when present it supplies each column's selected key instead of selected. No longer than columns. |
| `path_separator` | string | `"/"` | Separator used for display and clipboard text. |
| `show_path` | bool | `true` | Show the selectable path, level breadcrumbs and Copy path action. |
| `style` | style | — | Style merged over the root part. |

### `columns[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `empty_text` | string | `"No items"` | Text for a loaded empty column. |
| `error` | string or `()` | — | Caller-owned error; loading takes precedence, then error, then items. |
| `items` | native collection or array of object (at most 10000) | required | List row grammar plus has_children; a NativeCollection stays native, including keyboard target lookup. |
| `key` | string | required | Stable unique column identity; every event names this key. |
| `loading` | bool | `false` | Caller-owned loading state for this column alone. |
| `selected` | string or `()` | — | Controlled selected key. A disappeared selection is not replaced or emitted during render. |
| `title` | string | required | Column heading and listbox name. |
| `width` | number > 0 or length or `()` | — | Column width in pixels or Length, overriding column_width; columns never shrink to fit. |

### `columns[].items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `activate` | `on_activate` | object | Leaf activation, distinct from selection. |
| `retry` | `on_retry` | string | Retry requested for a column; the browser starts no work. |
| `select` | `on_select` | object | Proposal selecting a row and truncating the path at its level. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `detail` | no | no | Caller-owned detail pane after the last column. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `column`, `detail`, `focus_frame`, `heading`, `path`, `path_text`, `root`, `status`, `strip`.

## Theme

- Tokens: `border`, `focus_ring`, `metrics.inset`, `metrics.row`, `spacing.sm`, `spacing.xs`, `text_muted`, `text_primary`, `typography.body`, `typography.label`
- Environment: `density`

## Dependencies

[`components/button`](button.md), [`components/list`](list.md)
