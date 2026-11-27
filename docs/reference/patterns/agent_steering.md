# AgentSteering

`patterns/agent_steering` · export `AgentSteering` · version 0.1.0. Generated from
[`registry/patterns/agent_steering.rhai`](../../../registry/patterns/agent_steering.rhai); do not edit.

Send versioned steering intent without editing the Host's accepted plan or queue.

```rhai
import "patterns/agent_steering" as agent_steering;

agent_steering::AgentSteering(#{key:"steer",run:"preview",revision:0,now_ms:0,title:"Preview",steps:[],queue:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of `"queue"` or `"interrupt"` or `"remove"` or `"pause"` or `"resume"` (at most 5) or `()` | — | Restrict visible proposals, or omit for every valid action. |
| `agent` | string or `()` | — | Agent or model name. |
| `busy` | bool | `false` | Disable new actions while a proposal is processing. The draft can still be edited. |
| `compose` | bool | `true` | Hide the editor when the application already provides one. |
| `draft` | string or `()` | — | Use a string for a controlled draft, including an empty string. Omit for local edits. |
| `ended_ms` | integer ≥ 0 or `()` | — | Fixed endpoint for a started completed run. |
| `highlight` | array of string (at most 32) or `()` | — | Spotlight accepted step IDs. |
| `key` | string | required | Stable instance key. |
| `notice` | string or `()` | — | Host feedback about a proposal. |
| `now_ms` | integer ≥ 0 | required | Shared Host clock. |
| `on_action` | callback or `()` | — | Receives a request to queue, apply, remove, pause or resume. |
| `on_draft_change` | callback or `()` | — | A controlled editor receives run and the full changed text. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `paused_at_ms` | integer ≥ 0 or `()` | — | Authoritative freeze point while paused. |
| `paused_ms` | integer ≥ 0 | `0` | Completed pause intervals. |
| `placeholder` | string or `()` | — | Hint for the steering editor. |
| `queue` | array of object (at most 24) | required | Accepted waiting notes, oldest first. |
| `receipt` | object or `()` | — | Only a matching accepted receipt clears an unchanged local draft. Controlled drafts are never cleared here. |
| `revision` | integer ≥ 0 | required | Accepted revision, independent from typing or the clock. |
| `run` | string | required | Business run identity. |
| `started_ms` | integer ≥ 0 or `()` | — | Run clock start. |
| `status` | `"running"` or `"paused"` or `"done"` | `"running"` | Accepted run state. |
| `steps` | array of object (at most 32) | required | Accepted plan order. |
| `style` | style | — | Style merged over the root part. |
| `title` | string | required | Work being steered. |

### `queue[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable queued note ID, unique across the entire steering surface. |
| `text` | string | required | Note waiting to be applied. |

### `receipt` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `accepted` | bool | required | Whether the Host accepted it. |
| `proposal` | string | required | Exact proposal identity reported by on_action. |

### `steps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `elapsed_ms` | integer ≥ 0 or `()` | — | Accepted fixed time for a completed step. |
| `id` | string | required | Stable step identity. |
| `notes` | array of object (at most 24) or `()` | — | Notes the Host accepted into this step. |
| `paused_ms` | integer ≥ 0 or `()` | — | Pause time already completed during this step. |
| `started_ms` | integer ≥ 0 or `()` | — | Clock start for the active step. |
| `status` | `"done"` or `"active"` or `"pending"` or `()` | — | Default pending; at most one active step. |
| `title` | string | required | Step caption. |

### `steps[].notes[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Same note ID when it moves from the queue into the step. |
| `picked` | `"next"` or `"now"` or `()` | — | Whether picked up at a step boundary or applied immediately. |
| `text` | string | required | Accepted note text. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Validate version, active step and queued note before updating accepted data. The SDK never executes a note. |
| `draft_change` | `on_draft_change` | object | Controlled draft edit, not a request to steer. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `agent`, `card`, `clock`, `compose`, `empty`, `field`, `header`, `heading`, `hint`, `hints`, `keycap`, `marker`, `marker_active`, `marker_done`, `note`, `note_applied`, `note_arrow`, `note_next`, `note_now`, `note_tag`, `note_text`, `notes`, `notice`, `plan`, `queue`, `queue_actions`, `queue_title`, `queued`, `rail`, `root`, `send`, `state`, `state_done`, `state_paused`, `state_running`, `steer`, `step`, `step_active`, `step_clock`, `step_done`, `step_lead`, `step_quiet`, `step_title`, `title`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.success`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.metadata`

## Dependencies

[`components/button`](../components/button.md), [`components/spinner`](../components/spinner.md), [`components/textarea`](../components/textarea.md), [`patterns/agent_monitor`](../patterns/agent_monitor.md), [`patterns/agent_tree`](../patterns/agent_tree.md), [`patterns/approval_card`](../patterns/approval_card.md)
