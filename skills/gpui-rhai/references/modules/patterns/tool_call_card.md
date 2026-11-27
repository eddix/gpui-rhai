# ToolCallCard

`patterns/tool_call_card` · export `ToolCallCard` · version 0.1.0. Generated from
[`registry/patterns/tool_call_card.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/tool_call_card.rhai); do not edit.

One read-only tool call with status, arguments, result and optional retry.

```rhai
import "patterns/tool_call_card" as tool_call_card;

tool_call_card::ToolCallCard(#{key:"render",session:"inspect",revision:0,entry:#{id:"call/a",name:"read_document",status:"success",result:#{pages:2}}})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `default_open` | bool | `false` | Initial local disclosure policy. |
| `entry` | object | required | Accepted call snapshot. |
| `icon` | node or `()` | — | Optional custom glyph in place of the built-in icon. |
| `key` | string | required | Stable visual instance. |
| `max_code_height` | integer 64–480 | `220` | Logical-pixel cap for the initial code viewport. |
| `now_ms` | integer 0–9007199254740991 or `()` | — | Optional deterministic current epoch milliseconds; otherwise use the view's clock. |
| `on_copy` | callback or `()` | — | Receives identity and section after a clipboard request, without raw content. |
| `on_open_change` | callback or `()` | — | Disclosure request and local notification. |
| `on_retry` | callback or `()` | — | Versioned retry intent, only for failures. |
| `open` | bool or `()` | — | Controlled disclosure when supplied. |
| `open_on_error` | bool | `true` | Open a failed call until the caller or reader makes an explicit choice. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pending` | bool | `false` | Retry is awaiting Host acknowledgement. |
| `revision` | integer 0–9007199254740991 | required | Accepted snapshot revision. |
| `session` | string | required | Host session identity. |
| `style` | style | — | Style merged over the root part. |

### `entry` fields

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
| `copy` | `on_copy` | object | Clipboard request notification without arguments or result. |
| `open_change` | `on_open_change` | bool | The requested expanded state. |
| `retry` | `on_retry` | object | Request another attempt; no execution takes place in the component. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `announcement`, `body`, `chevron`, `code`, `copy`, `copy_done`, `error`, `error_text`, `failure`, `header`, `header_open`, `icon`, `label`, `more`, `name`, `placeholder`, `retry`, `root`, `section`, `status`, `summary`.

## Theme

- Tokens: `border`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text_muted`, `text_primary`, `tool_call.code`, `tool_call.error`, `tool_call.error_border`, `tool_call.tile`, `typography.body`, `typography.caption`, `typography.code`

## Dependencies

[`components/button`](../components/button.md), [`components/code_viewer`](../components/code_viewer.md), [`components/spinner`](../components/spinner.md), [`patterns/agent_tree`](agent_tree.md)
