# AgentMonitor

`patterns/agent_monitor` · export `AgentMonitor` · version 0.1.0. Generated from
[`registry/patterns/agent_monitor.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/agent_monitor.rhai); do not edit.

A single accepted run, its activity clock, and recent steps with native inspection.

```rhai
import "patterns/agent_monitor" as agent_monitor;

agent_monitor::AgentMonitor(#{key:"monitor",run:"preview",revision:0,now_ms:0,state:"idle",steps:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of `"pause"` or `"resume"` or `"cancel"` or `"allow"` or `"decline"` or `"retry"` or `"open_log"` (at most 7) or `()` | — | Enable a subset of actions. Omit for all valid actions when there is an on_action callback. |
| `ask` | object or `()` | — | Approved, versioned evidence from the Host, not an executable command. |
| `busy` | bool | `false` | Disable run mutations while the Host is resolving a proposal. |
| `cost_micros` | integer -1000000000000–1000000000000 or `()` | — | Run cost or credit in millionths of the currency unit. |
| `currency` | string or `()` | — | Currency caption; defaults to USD. |
| `current` | object or `()` | — | Override only the activity well; not a new running step. |
| `default_folded` | bool | `false` | Begin in the compact bar. |
| `ended_ms` | integer ≥ 0 or `()` | — | Fixed endpoint for a started terminal run. |
| `error` | string or `()` | — | Short failure evidence. |
| `key` | string | required | Stable monitor instance key. |
| `mark` | node or `()` | — | Decorative agent avatar or logo. |
| `name` | string or `()` | — | Agent name in the folded bar and open monitor. |
| `notice` | string or `()` | — | Host feedback; a rejection leaves the accepted state unchanged. |
| `now_ms` | integer ≥ 0 | required | Authoritative Host clock, never a script timer. |
| `on_action` | callback or `()` | — | Receives proposals, not optimistic runtime state changes. |
| `outcome` | object or `()` | — | Summary displayed when finished. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `paused_at_ms` | integer ≥ 0 or `()` | — | Fixed endpoint while paused. |
| `paused_ms` | integer ≥ 0 | `0` | Completed pause intervals to subtract, excluding the current pause. |
| `revision` | integer ≥ 0 | required | Accepted business revision, independent of clock ticks. |
| `role` | string or `()` | — | Agent role or model caption. |
| `run` | string | required | Business run ID; a retry may publish a new ID. |
| `started_ms` | integer ≥ 0 or `()` | — | Start in the Host clock domain. |
| `state` | `"idle"` or `"running"` or `"waiting"` or `"paused"` or `"finished"` or `"failed"` or `"cancelled"` | required | Accepted run state. |
| `steps` | array of object (at most 64) | required | Steps in chronological display order. At most one may be running. |
| `style` | style | — | Style merged over the root part. |

### `ask` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `command` | string or `()` | — | Exact command or input displayed as literal evidence. |
| `icon` | node or `()` | — | Tool mark for the request. |
| `id` | string | required | Stable request ID required in Allow/Decline proposals. |
| `title` | string | required | Approval question. |

### `current` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `icon` | node or `()` | — | Decorative tool override. |
| `id` | string | required | Stable presentation identity of the current caption. |
| `kind` | `"read"` or `"search"` or `"edit"` or `"run"` or `"think"` or `"web"` or `()` | — | Tile kind. |
| `label` | string | required | Current work caption, independent from the history. |

### `outcome` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `added` | integer ≥ 0 or `()` | — | Added lines. |
| `files` | integer ≥ 0 or `()` | — | Touched files. |
| `note` | string or `()` | — | Outcome caption. |
| `removed` | integer ≥ 0 or `()` | — | Removed lines. |

### `steps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `detail` | string or `()` | — | Step description shown in the inspector. |
| `elapsed_ms` | integer ≥ 0 or `()` | — | Fixed accepted elapsed time of a terminal step. |
| `icon` | node or `()` | — | Optional override for the tool tile. |
| `id` | string | required | Stable step ID. The history row and its duration segment share it. |
| `kind` | `"read"` or `"search"` or `"edit"` or `"run"` or `"think"` or `"web"` or `()` | — | Tool kind used for its decorative tile. |
| `label` | string | required | Short action caption. |
| `output` | string or `()` | — | Literal text shown without evaluating or executing it. |
| `paused_ms` | integer ≥ 0 or `()` | — | Completed pause intervals affecting this step. |
| `started_ms` | integer ≥ 0 or `()` | — | Host start of a running step. |
| `state` | `"queued"` or `"running"` or `"done"` or `"failed"` | required | Accepted step state. |
| `tool` | string or `()` | — | Tool or system caption. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Host validates run, revision, step and request before applying a transition. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `agent`, `bar`, `bar_folded`, `card`, `card_folded`, `clock`, `command`, `cost`, `counter`, `details`, `details_kind`, `details_output`, `details_title`, `fold`, `footer`, `history`, `history_content`, `history_hot`, `history_name`, `history_row`, `history_time`, `mark`, `name`, `notice`, `outcome`, `outcome_add`, `outcome_remove`, `role`, `root`, `segment`, `segment_done`, `segment_failed`, `segment_hot`, `segment_running`, `segment_waiting`, `status`, `status_failed`, `status_finished`, `status_paused`, `status_running`, `status_waiting`, `steer`, `strip`, `tool`, `tool_live`, `tools`, `well`, `well_body`, `well_failed`, `well_label`, `well_waiting`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.code`

## Dependencies

[`components/button`](../components/button.md), [`components/popover`](../components/popover.md), [`components/spinner`](../components/spinner.md), [`patterns/agent_tree`](agent_tree.md), [`patterns/approval_card`](approval_card.md)
