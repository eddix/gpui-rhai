# ContextWindow

`patterns/context_window` · export `ContextWindow` · version 0.1.0. Generated from
[`registry/patterns/context_window.rhai`](../../../registry/patterns/context_window.rhai); do not edit.

A token allocation field with Host-controlled removal and compaction.

```rhai
import "patterns/context_window" as context_window;

context_window::ContextWindow(#{key:"context",context:"chat",revision:0,limit:100000,parts:[#{id:"chat",label:"Conversation",tokens:23000}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `action_label` | string or `()` | — | Optional footer action label. |
| `cells` | integer 1–384 | `216` | Number of cells in the decorative field. |
| `columns` | integer 8–48 | `36` | Wide layout columns. Narrow layouts use fewer. |
| `compacted` | object or `()` | — | Show compacted until the accepted part grows again. |
| `context` | string | required | Stable context identity. |
| `key` | string | required | Stable visual instance. |
| `limit` | integer 0–9007199254740991 | required | Accepted token allowance; zero means unavailable. |
| `model` | string or `()` | — | Optional model tag. |
| `on_action` | callback or `()` | — | Versioned footer action intent. |
| `on_request` | callback or `()` | — | Versioned remove or compact proposal, not a mutation. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `parts` | array of object (at most 32) | required | Parts in the fill order, with distinct stable IDs. |
| `pending` | object or `()` | — | Authoritative acknowledgement state. Does not change the counts. |
| `revision` | integer 0–9007199254740991 | required | Accepted capacity revision. |
| `saved_tokens` | integer 0–9007199254740991 or `()` | — | Optional accepted result of the previous compaction. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Optional card heading. |
| `warn_at` | number 0–0.95 | `0.8` | Warning fraction of the allowance. |

### `compacted` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `part` | string | required | Recently compacted part ID. |
| `tokens` | integer 0–9007199254740991 | required | Its accepted post-compaction size. |

### `parts[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `compact` | bool | `false` | Expose compaction if the caller handles it. |
| `id` | string | required | Stable part ID. |
| `ink` | string or `()` | — | Optional theme color token. |
| `items` | array of object (at most 64) or `()` | — | Optional item breakdown. If supplied, its sum supersedes tokens. |
| `label` | string | required | Part caption. |
| `locked` | bool | `false` | Prevent removal and compaction. |
| `tokens` | integer 0–9007199254740991 or `()` | — | Aggregate without items. |

### `parts[].items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable item ID within its part. |
| `label` | string | required | Attached item label. |
| `tokens` | integer 0–9007199254740991 | required | Accepted item token count. |

### `pending` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `item` | string or `()` | — | Target item for removal, omitted for compaction. |
| `kind` | `"remove"` or `"compact"` | required | Operation awaiting acceptance. |
| `part` | string | required | Target part. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Footer action request. |
| `request` | `on_request` | object | This component never mutates the accepted input. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `action_icon`, `active`, `amount`, `arrow`, `big`, `card`, `compact`, `done`, `dot`, `empty`, `field`, `figure`, `footer`, `full`, `head`, `item`, `item_name`, `item_value`, `items`, `main`, `model`, `name`, `note`, `of`, `part`, `parts`, `pill`, `remove`, `root`, `saved`, `spark`, `tile`, `tile_active`, `title`, `warn`.

## Theme

- Tokens: `border`, `context_window.c1`, `context_window.c2`, `context_window.c3`, `context_window.c4`, `context_window.c5`, `context_window.c6`, `context_window.free`, `context_window.full_bg`, `context_window.muted`, `context_window.saved`, `context_window.shine`, `context_window.warn`, `context_window.warn_bg`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.display`

## Dependencies

[`components/button`](../components/button.md), [`components/spinner`](../components/spinner.md), [`patterns/agent_tree`](../patterns/agent_tree.md), [`patterns/compact_breakdown`](../patterns/compact_breakdown.md)
