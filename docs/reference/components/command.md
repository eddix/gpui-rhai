# Command

`components/command` · export `Command` · version 0.4.3. Generated from
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
| `appearance` | `"panel"` or `"embedded"` | `"panel"` | panel owns its tray/card chrome; embedded lets a surrounding Dialog or host panel own those edges, without changing the input or row identity. |
| `autofocus` | bool | `false` | Focuses the search field when the command first mounts. |
| `disabled` | bool | `false` | Disables the whole command: the search field, the rows and the list's keys. |
| `empty_text` | string | `""` | Message shown in place of the list when no command matches. Empty, the localized `command.empty` message, `No commands found` without one. |
| `footer` | node or `()` | — | Optional footer content in place of the default navigation/Enter hints; no shortcut is registered. |
| `footer_visible` | bool | `true` | Shows the keyboard-hint footer, or the caller's footer node. Hiding it does not change navigation or bindings. |
| `items` | array of object (at most 4096) or native collection | required | Commands to rank: item maps, or a native collection keyed by value with the same textual fields and logical asset IDs for icon/logo, but no `content`, UI nodes or per-row `action`. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the search field and the command list. |
| `max_visible` | integer 1–24 | `8` | Maximum viewport capacity in command-row units; group headings use their natural measured height. |
| `narrow_at` | number 0–16384 | `480.0` | At or below this committed card width in logical pixels, use taller rows and hide visual hints/keycaps. This observes the actual card, not the window; initial unmeasured layout uses wide presentation. |
| `on_action` | callback or `()` | — | Called with the command value when a row is clicked or Enter activates the highlighted command. |
| `on_active_change` | callback or `()` | — | Called with the command value to highlight on Up, Down, Home, End or hover; store it as `active_value`. |
| `on_query_change` | callback or `()` | — | Called with the new search text on each edit; store it as `query`. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string | `""` | Hint shown in the empty search field. Empty, the localized `command.placeholder` message, `Type a command` without one. |
| `query` | string | required | Search text; the caller stores the `query_change` payload and passes it back. |
| `scrollbar` | `"auto"` or `"always"` or `"hidden"` | `"auto"` | Shared command-list scrollbar visibility; the search field keeps keyboard focus. |
| `style` | style | — | Style merged over the root part. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `action` | string or `()` | — | Registered action dispatched on activation; its enabled state and shortcut are inherited. |
| `content` | node or `()` | — | Rich visual label; ranking and accessible name still use label. |
| `disabled` | bool | `false` | Dimmed and excluded from activation and navigation. |
| `group` | string or `()` | — | Heading; groups preserve first matching source order. |
| `hint` | string or `()` | — | Inline secondary text, also searched with keyword precedence. |
| `icon` | node or string or `()` | — | Non-interactive glyph node or logical asset ID; the leading slot is reserved when empty. |
| `keywords` | array of string (at most 32) | `[]` | Additional search terms with keyword precedence. |
| `label` | string | required | Required searchable/accessibility text; content may replace its visual label. |
| `logo` | node or string or `()` | — | Non-interactive logo node or logical asset ID on a tile; takes precedence over icon. |
| `shortcut` | string or `()` | — | Row-end shortcut legend overriding a registered action binding. |
| `value` | string | required | Unique non-empty value reported by active_change and action. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | string | Emitted when a command is clicked or activated with Enter, after its bound action; the payload is its value. |
| `active_change` | `on_active_change` | string | Emitted when a key or hover moves the highlight; the payload is the newly highlighted value. |
| `query_change` | `on_query_change` | string | Emitted on each edit of the search field; the payload is the new query. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `footer` | no | no | Optional keyboard hints or application-owned status. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `card`, `card_embedded`, `empty`, `footer`, `footer_hint`, `group`, `hint`, `icon`, `icon_slot`, `icon_slot_active`, `image_fallback`, `item`, `item_active`, `item_narrow`, `label`, `list`, `list_viewport`, `logo`, `logo_slot`, `root`, `root_embedded`, `scrollbar_thumb`, `scrollbar_thumb_hover`, `scrollbar_track`, `search`, `search_hint`, `search_hint_icon`, `search_hint_text`, `search_icon`, `search_input`, `search_placeholder`, `shortcut`, `shortcut_active`, `text`.

## Theme

- Tokens: `border`, `command.enter_asset`, `command.enter_label`, `command.group_case`, `command.narrow_footer`, `command.search_asset`, `disabled`, `metrics.command_narrow_row`, `metrics.command_row`, `metrics.inset`, `radius.lg`, `radius.md`, `scrollbar.thumb`, `scrollbar.thumb_hover`, `selection`, `spacing.sm`, `spacing.xs`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.label`
- Environment: `density`, `size`

## Dependencies

[`components/input`](../components/input.md), [`components/kbd`](../components/kbd.md)
