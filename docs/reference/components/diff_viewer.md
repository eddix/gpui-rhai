# DiffViewer

`components/diff_viewer` · export `DiffViewer` · version 0.2.0. Generated from
[`registry/components/diff_viewer.rhai`](../../../registry/components/diff_viewer.rhai); do not edit.

Neutral two-way, read-only source comparison over the native document surface.

```rhai
import "components/diff_viewer" as diff_viewer;

diff_viewer::DiffViewer(#{ key: "servers", left: #{ source: a, label: "Server A" }, right: #{ source: b, label: "Server B" }, mode: "split" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `context_lines` | integer 0–100 or `"all"` | `3` | Unchanged lines kept around each hunk; longer equal runs fold, and `"all"` never folds. |
| `key` | string | required | Stable viewer identity; keeps scroll, folds, selection and search across renders. |
| `left` | object | required | Document on the left in split mode; left and right carry no old or new meaning. |
| `mode` | `"unified"` or `"split"` | `"unified"` | `unified` interleaves both sides in one column; `split` aligns them side by side with one scroll position. |
| `on_location_activate` | callback or `()` | — | Called with `#{ side, line, column }` when a side's text is double-clicked or Enter is pressed in it. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `right` | object | required | Document on the right in split mode. |
| `show_line_numbers` | bool | `true` | Shows the line-number gutters. |
| `style` | style | — | Style merged over the root part. |
| `tab_size` | integer 1–16 | `4` | Columns between tab stops when tabs are expanded for display. |
| `whitespace` | `"exact"` or `"ignore_changes"` or `"ignore_all"` | `"exact"` | `exact` compares lines as written, `ignore_changes` collapses runs of whitespace, `ignore_all` drops all whitespace. |
| `wrap` | `"none"` or `"viewport"` or `"column"` | `"none"` | `none` scrolls long lines sideways, `viewport` wraps them at the pane width, `column` at `wrap_column`. |
| `wrap_column` | integer 20–500 | `100` | Display column at which lines wrap when `wrap` is `column`. |

### `left` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `file_name` | string or `()` | — | File name whose extension picks this side's syntax when `language` is not set. |
| `label` | string | required | Name of this side in the header, the accessible name and copied patches. |
| `language` | string or `()` | — | Syntax to highlight this side with; takes precedence over `file_name`. |
| `source` | string or native document | required | Text of this side: a string, or a Host-owned `NativeTextDocument`. |

### `right` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `file_name` | string or `()` | — | File name whose extension picks this side's syntax when `language` is not set. |
| `label` | string | required | Name of this side in the header, the accessible name and copied patches. |
| `language` | string or `()` | — | Syntax to highlight this side with; takes precedence over `file_name`. |
| `source` | string or native document | required | Text of this side: a string, or a Host-owned `NativeTextDocument`. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `location_activate` | `on_location_activate` | object | Emitted on a double-click or Enter in either side's text; the payload is the side and its one-based location. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `error`, `fold`, `gutter`, `header`, `line`, `loading`, `root`, `search`, `status`, `text`.

## Theme

- Tokens: `surface`
