# ToolCallGroup

`patterns/tool_call_group` · export `ToolCallGroup` · version 0.1.0. Generated from
[`registry/patterns/tool_call_group.rhai`](../../../registry/patterns/tool_call_group.rhai); do not edit.

A compact summary and lazy disclosure for several read-only tool calls.

```rhai
import "patterns/tool_call_group" as tool_call_group;

tool_call_group::ToolCallGroup(#{key:"tools",session:"inspect",revision:0,calls:[#{id:"a",name:"read_document",status:"running"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `calls` | array of object (at most 16) | required | Accepted calls in render order; IDs must be unique. |
| `default_open` | bool | `false` | Initial local disclosure. |
| `duration_ms` | integer 0–9007199254740991 or `()` | — | Explicit accepted group duration; otherwise use the sum of known completed durations. |
| `key` | string | required | Stable group identity. |
| `label` | string or `()` | — | Alternative completed group caption. |
| `now_ms` | integer 0–9007199254740991 or `()` | — | Inject clock time for deterministic previews; otherwise the group runs one view-clock timer. |
| `on_copy` | callback or `()` | — | Forwards the copied section and IDs, never raw text. |
| `on_open_change` | callback or `()` | — | Group disclosure request. |
| `on_retry` | callback or `()` | — | Forwards the child call's retry identity without executing it. |
| `open` | bool or `()` | — | Controlled disclosure when supplied. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pending` | array of string (at most 16) | `[]` | Accepted IDs whose retry is awaiting acknowledgement. |
| `revision` | integer 0–9007199254740991 | required | Accepted batch revision. |
| `running_label` | string or `()` | — | Alternative running caption. |
| `session` | string | required | Session ID carried by all contained call intents. |
| `style` | style | — | Style merged over the root part. |

### `calls[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `arguments` | any value or `()` | — | Arguments as structured JSON values, JSON text, or plain text. |
| `duration_ms` | integer 0–9007199254740991 or `()` | — | Accepted final duration in milliseconds. |
| `error` | string or `()` | — | Host-supplied failure message. |
| `icon` | `"tool"` or `"search"` or `"database"` or `"file"` or `"web"` or `"terminal"` or `()` | — | Built-in glyph name. |
| `id` | string | required | Stable call identity within this session. |
| `name` | string | required | Tool name. Presentation only; the component never executes it. |
| `result` | any value or `()` | — | Result as structured JSON values, JSON text, or plain text. |
| `started_ms` | integer 0–9007199254740991 or `()` | — | Epoch time in milliseconds for a running duration. |
| `status` | `"running"` or `"success"` or `"error"` | required | Accepted execution status. |
| `summary` | string or `()` | — | Short optional summary. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `copy` | `on_copy` | object | Child clipboard request notification. |
| `open_change` | `on_open_change` | bool | Group disclosure intent. |
| `retry` | `on_retry` | object | Versioned retry request from a child card. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `announcement`, `arrow`, `elapsed`, `empty`, `failed`, `header`, `icons`, `list`, `mark`, `root`, `title`.

## Theme

- Tokens: `border`, `focus_ring`, `radius.lg`, `radius.pill`, `space.spacing_unit`, `space.unit`, `surface_hover`, `text.danger`, `text_muted`, `text_primary`, `tool_call.tile`, `typography.body`, `typography.caption`

## Dependencies

[`patterns/agent_tree`](../patterns/agent_tree.md), [`patterns/tool_call_card`](../patterns/tool_call_card.md)
