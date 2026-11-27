# RunControls

`patterns/run_controls` · export `RunControls` · version 0.1.0. Generated from
[`registry/patterns/run_controls.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/run_controls.rhai); do not edit.

RunControls presents a Host-owned run transcript, queue and draft editor. State: the Host owns run identity, lifecycle, draft, queue, mode and policy.

```rhai
import "patterns/run_controls" as run_controls;

run_controls::RunControls(#{key:"run",run_id:"job/7",draft:"",status:"idle"})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agent_id` | string | `"assistant"` | Stable seed for the decorative agent mark. |
| `agent_name` | string or `()` | — | Display name. Omit for a localized fallback. |
| `answer` | string | `""` | Accepted literal response as it streams; up to 96000 UTF-8 bytes. |
| `context` | number 0–1 or `()` | — | Optional share of the context window already used. |
| `disabled` | bool | `false` | Disable editing and all actionable controls. |
| `draft` | string | required | Accepted editable text. The Host clears it after accepting a submission. |
| `elapsed_ms` | integer 0–9000000000000 or `()` | — | Application-measured duration of completed work. |
| `key` | string | required | Stable workspace identity. The editor survives run status changes. |
| `log_height` | integer 80–1200 | `252` | Fixed native transcript viewport height in logical pixels; the composer remains independently sized. |
| `mode` | `"chat"` or `"act"` | `"chat"` | Accepted workspace mode. Chat disables permission choices. This property never authorizes work. |
| `on_change` | callback or `()` | — | Draft edited; receives #{run,text}. |
| `on_dismiss` | callback or `()` | — | Dismiss the current run; receives run_id. |
| `on_mode_change` | callback or `()` | — | A proposal; the Host accepts or rejects the workspace mode. |
| `on_permission_change` | callback or `()` | — | A proposal; only the Host can authorize work. |
| `on_queue` | callback or `()` | — | Queue an input; receives #{run,text}. |
| `on_resume` | callback or `()` | — | Resume stopped or failed work; receives run_id. |
| `on_retry` | callback or `()` | — | Retry the current run; receives run_id. |
| `on_send` | callback or `()` | — | Send a new prompt; receives #{run,text}. |
| `on_steer` | callback or `()` | — | Steer the current run; receives #{run,text}. |
| `on_stop` | callback or `()` | — | Stop or Escape; receives run_id. |
| `on_unqueue` | callback or `()` | — | Remove request with the current run and queue ID. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `permission` | `"ask"` or `"edits"` or `"full"` | `"ask"` | Accepted, application-owned policy. Proposals never grant access or execute anything. |
| `placeholder` | string or `()` | — | Hint for the native editor. |
| `prompt` | string or `()` | — | Accepted prompt of the current transcript. |
| `queue` | array of object (at most 64) | `[]` | Accepted queue in dispatch order. Removing or dispatching remains Host-owned. |
| `run_id` | string | required | Current Host run identity (or empty before the first run); echoed in run events. |
| `status` | `"idle"` or `"running"` or `"stopped"` or `"error"` or `"done"` | `"idle"` | Accepted run status; never advanced by the control. |
| `steers` | array of object (at most 64) | `[]` | Accepted notes in chronological order of nondecreasing answer offset. |
| `style` | style | — | Style merged over the root part. |

### `queue[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable, unique Host-owned queue ID. |
| `text` | string | required | Queued draft shown without markup interpretation. |

### `steers[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `at` | integer ≥ 0 | required | Unicode scalar offset in the accepted answer, not a byte offset. |
| `id` | string | required | Stable, unique accepted note ID. |
| `text` | string | required | Accepted steering note shown as literal text. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | object | Draft edited; receives #{run,text}. |
| `dismiss` | `on_dismiss` | string | Dismiss the current run; receives run_id. |
| `mode_change` | `on_mode_change` | string | A proposal; the Host accepts or rejects the workspace mode. |
| `permission_change` | `on_permission_change` | string | A proposal; only the Host can authorize work. |
| `queue` | `on_queue` | object | Queue an input; receives #{run,text}. |
| `resume` | `on_resume` | string | Resume stopped or failed work; receives run_id. |
| `retry` | `on_retry` | string | Retry the current run; receives run_id. |
| `send` | `on_send` | object | Send a new prompt; receives #{run,text}. |
| `steer` | `on_steer` | object | Steer the current run; receives #{run,text}. |
| `stop` | `on_stop` | string | Stop or Escape; receives run_id. |
| `unqueue` | `on_unqueue` | object | Remove request with the current run and queue ID. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `after`, `agent`, `agent_cell`, `agent_mark`, `answer`, `composer`, `context`, `dismiss`, `editor`, `empty`, `icon`, `jump`, `log`, `modes`, `name`, `note`, `permission`, `permission_alert`, `permission_hint`, `permission_item`, `permission_panel`, `permission_selected`, `permission_trigger`, `prompt`, `queue`, `queued`, `queued_index`, `remove`, `resume`, `retry`, `root`, `send`, `status`, `steer`, `stop`, `tools`, `write`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `metrics.run_prompt_tip`, `radius.lg`, `radius.sm`, `run.mark`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/icon_button`](../components/icon_button.md), [`components/textarea`](../components/textarea.md), [`components/thinking_loader`](../components/thinking_loader.md), [`components/toggle_group`](../components/toggle_group.md)
