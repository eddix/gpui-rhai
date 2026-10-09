# CommandDialog

`components/command_dialog` · export `CommandDialog` · version 0.2.0. Generated from
[`registry/components/command_dialog.rhai`](../../../registry/components/command_dialog.rhai); do not edit.

Controlled modal presentation of Command without registering a global shortcut.

The command fills the panel edge to edge: the panel has no padding, its search line spans the panel, and a visible title sits on the rows' inset above it.

State: caller owns open, query, and active_value.

```rhai
import "components/command_dialog" as command_dialog;

command_dialog::CommandDialog(#{ key: "palette", open: open, query: query, active_value: active, label: "Commands", items: items, on_open_change: Fn("opened"), on_query_change: Fn("queried"), on_active_change: Fn("preview"), on_action: Fn("run") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `active_value` | string | required | Highlighted command value; the caller stores the `active_change` payload. Other values show the first enabled match. |
| `close_on_action` | bool | `true` | Follows each `action` with `open_change(false)`, so activating a command closes the dialog. |
| `command_part_styles` | map of style | `#{}` | Part styles for the inner Command, such as `search`, `item` and `item_active`. |
| `dialog_part_styles` | map of style | `#{}` | Part styles for the inner Dialog, such as `panel` and `title`, merged over the edge-to-edge layout. |
| `disabled` | bool | `false` | Disables the search field and the list's arrow, Home, End and Enter keys. |
| `empty_text` | string | `"No commands found"` | Message shown in place of the list when no command matches. |
| `items` | array of object (at most 4096) or native collection | required | Commands to rank: item maps, or a native collection keyed by value with the same fields but no `action`. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Dialog title, also the accessible name of the dialog, the search field and the list. |
| `max_visible` | integer 1–24 | `8` | Rows the list shows before it scrolls; group headings count as rows. |
| `on_action` | callback or `()` | — | Called with the command value when a row is clicked or Enter activates the highlighted command. |
| `on_active_change` | callback or `()` | — | Called with the command value to highlight on Up, Down, Home, End or hover; store it as `active_value`. |
| `on_escape` | callback or `()` | — | Called on Escape; when set, Escape no longer closes the dialog, so the caller can close it or go back a level. |
| `on_open_change` | callback or `()` | — | Called with `false` when Escape, an outside click or `close_on_action` closes the dialog; store it as `open`. |
| `on_query_change` | callback or `()` | — | Called with the new search text on each edit; store it as `query`. |
| `open` | bool | required | Whether the dialog is shown; the caller stores the `open_change` payload and passes it back. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string | `"Type a command"` | Hint shown in the empty search field. |
| `query` | string | required | Search text; the caller stores the `query_change` payload and passes it back. |
| `style` | style | — | Style merged over the root part. |
| `title_visible` | bool | `true` | Shows `label` as a title above the search; when hidden, the label still names the dialog. |
| `width` | length or `()` | — | Panel width in place of the dialog's 420 px; the overlay still caps it at 90% of the window. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `action` | string or `()` | — | Registered action dispatched on activation; it also supplies the shortcut legend and can disable the row. |
| `disabled` | bool | `false` | Dims the command; it cannot be activated and keyboard navigation skips it. |
| `group` | string or `()` | — | Heading the command is listed under; groups keep the source order of their first match. |
| `keywords` | array of string (at most 32) | `[]` | Extra terms matched against the query; a keyword match ranks just below the same label match. |
| `label` | string | required | Non-empty text ranked against the query and shown on the row. |
| `shortcut` | string or `()` | — | Key legend shown at the row end, written like `cmd-k`; overrides the binding of `action`. |
| `value` | string | required | Unique, non-empty id reported by `active_change` and `action`. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | string | Emitted when a command is clicked or activated with Enter; the payload is its value. |
| `active_change` | `on_active_change` | string | Emitted when a key or hover moves the highlight; the payload is the newly highlighted value. |
| `escape` | `on_escape` | none | Emitted on Escape inside the command while `on_escape` is set; the payload is `()`. |
| `open_change` | `on_open_change` | bool | Emitted when Escape, an outside click or `close_on_action` closes the dialog; the payload is `false`. |
| `query_change` | `on_query_change` | string | Emitted on each edit of the search field; the payload is the new query. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `command`, `dialog`, `root`.

## Theme

- Tokens: `metrics.inset`
- Environment: `density`

## Dependencies

[`components/command`](../components/command.md), [`components/dialog`](../components/dialog.md)
