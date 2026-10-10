# CodeViewer

`components/code_viewer` · export `CodeViewer` · version 0.2.0. Generated from
[`registry/components/code_viewer.rhai`](../../../registry/components/code_viewer.rhai); do not edit.

Native, virtualized, selectable read-only source document surface.

```rhai
import "components/code_viewer" as code_viewer;

code_viewer::CodeViewer(#{ key: "source", source: rhai_source, label: "Rhai source", language: "rhai" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `file_name` | string or `()` | — | File name whose extension picks the syntax when `language` is not set. |
| `key` | string | required | Stable viewer identity; keeps scroll, selection and search across renders, also when an inline `source` changes. |
| `label` | string | `"Code"` | Accessible name of the document surface. |
| `language` | string or `()` | — | Syntax to highlight, such as `"rhai"`; takes precedence over `file_name`, and unknown names show plain text. |
| `on_location_activate` | callback or `()` | — | Called with `#{ line, column }` when the text is double-clicked or Enter is pressed in it. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `show_line_numbers` | bool | `true` | Shows 1-based line numbers in the gutter; with `false` the gutter stays, empty. |
| `source` | string or native document | required | Text to show: a string, or a Host-owned `NativeTextDocument` from `ctx.get_native_text_document`. |
| `style` | style | — | Style merged over the root part. |
| `tab_size` | integer 1–16 | `4` | Columns between tab stops when tabs are expanded for display. |
| `wrap` | `"none"` or `"viewport"` or `"column"` | `"none"` | `none` scrolls long lines sideways, `viewport` wraps them at the viewer width, `column` at `wrap_column`. |
| `wrap_column` | integer 20–500 | `100` | Display column at which lines wrap when `wrap` is `column`. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `location_activate` | `on_location_activate` | object | Emitted on a double-click or Enter in the text; the payload is the one-based source location. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `error`, `gutter`, `line`, `loading`, `root`, `search`, `text`.

## Theme

- Tokens: `surface`, `surface_raised`
