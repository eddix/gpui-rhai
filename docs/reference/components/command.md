# Command

`components/command` · export `Command` · version 0.2.0. Generated from
[`registry/components/command.rhai`](../../../registry/components/command.rhai); do not edit.

Embeddable keyboard-first command search with deterministic fuzzy ranking.

State: caller owns query and the roving active command.

```rhai
import "components/command" as command;

command::Command(#{ key: "palette", label: "Commands", query: query, active_value: active, items: items, on_query_change: Fn("queried"), on_active_change: Fn("preview"), on_action: Fn("run") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `active_value` | string | required | Highlighted command value; the caller stores the `active_change` payload. Other values show the first enabled match. |
| `autofocus` | bool | `false` | Focuses the search field when the command first mounts. |
| `disabled` | bool | `false` | Disables the whole command: the search field, the rows and the list's keys. |
| `empty_text` | string | `"No commands found"` | Message shown in place of the list when no command matches. |
| `items` | array of object (at most 4096) or native collection | required | Commands to rank: item maps, or a native collection keyed by value with the same fields but no `content` or `action`. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the search field and the command list. |
| `max_visible` | integer 1–24 | `8` | Rows the list shows before it scrolls; group headings count as rows. |
| `on_action` | callback or `()` | — | Called with the command value when a row is clicked or Enter activates the highlighted command. |
| `on_active_change` | callback or `()` | — | Called with the command value to highlight on Up, Down, Home, End or hover; store it as `active_value`. |
| `on_query_change` | callback or `()` | — | Called with the new search text on each edit; store it as `query`. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string | `"Type a command"` | Hint shown in the empty search field. |
| `query` | string | required | Search text; the caller stores the `query_change` payload and passes it back. |
| `style` | style | — | Style merged over the root part. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `action` | string or `()` | — | Registered action dispatched on activation; it also supplies the shortcut legend and can disable the row. |
| `content` | node or `()` | — | Rich node shown in place of the label text; ranking and the accessible name still use `label`. |
| `disabled` | bool | `false` | Dims the command; it cannot be activated and keyboard navigation skips it. |
| `group` | string or `()` | — | Heading the command is listed under; groups keep the source order of their first match. |
| `keywords` | array of string (at most 32) | `[]` | Extra terms matched against the query; a keyword match ranks just below the same label match. |
| `label` | string | required | Non-empty text ranked against the query; the row shows it unless `content` is set. |
| `shortcut` | string or `()` | — | Key legend shown at the row end, written like `cmd-k`; overrides the binding of `action`. |
| `value` | string | required | Unique, non-empty id reported by `active_change` and `action`. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | string | Emitted when a command is clicked or activated with Enter, after its bound action; the payload is its value. |
| `active_change` | `on_active_change` | string | Emitted when a key or hover moves the highlight; the payload is the newly highlighted value. |
| `query_change` | `on_query_change` | string | Emitted on each edit of the search field; the payload is the new query. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `empty`, `group`, `indicator_bar`, `item`, `item_active`, `label`, `list`, `root`, `search`, `shortcut`.

## Theme

- Tokens: `accent`, `border`, `disabled`, `metrics.inset`, `metrics.row`, `radius.lg`, `selection`, `spacing.sm`, `spacing.xs`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.label`
- Environment: `corners`, `density`

## Dependencies

[`components/input`](../components/input.md), [`components/kbd`](../components/kbd.md)
