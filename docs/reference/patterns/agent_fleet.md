# AgentFleet

`patterns/agent_fleet` · export `AgentFleet` · version 0.1.0. Generated from
[`registry/patterns/agent_fleet.rhai`](../../../registry/patterns/agent_fleet.rhai); do not edit.

A run overview with one expanded lead, a child roster and a task inspector.

```rhai
import "patterns/agent_fleet" as agent_fleet;

agent_fleet::AgentFleet(#{key:"team",run:"preview",revision:0,now_ms:0,agents:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agents` | array of object (at most 64) | required | A flat forest, including all leads and their sub-agents. Parents may follow their children in input order. |
| `busy_agent` | string or `()` | — | Agent whose resolution is still being processed. |
| `controlled_open` | bool | `false` | Use open as the only source of disclosure. When false, the component owns disclosure. |
| `currency` | string or `()` | — | Currency label, defaults to USD. |
| `default_open` | string or `()` | — | Initial local open lead, defaults to first. An empty string starts closed. |
| `highlight` | string or `()` | — | Emphasize this agent. A new highlight reveals its lead in local mode; it never overrides controlled disclosure. |
| `key` | string | required | Stable fleet instance key. |
| `notice` | string or `()` | — | Host feedback, also visible inside an open inspector. |
| `now_ms` | integer ≥ 0 | required | Host clock used by every active agent. |
| `on_open_change` | callback or `()` | — | Receives a versioned disclosure proposal. |
| `on_resolve` | callback or `()` | — | Receives approval or failure-resolution intent. |
| `on_select` | callback or `()` | — | Receives selection of a sub-agent (for example, to open a log). |
| `open` | string or `()` | — | Accepted open lead ID in controlled mode; null means none. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Accepted snapshot revision, independent of clock ticks. |
| `run` | string | required | Business identity of the run. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Fleet heading. |

### `agents[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `ask` | string or `()` | — | Approval evidence for a waiting agent. |
| `avatar` | node or `()` | — | Decorative avatar, defaults to initials. |
| `cost_micros` | integer -1000000000000–1000000000000 or `()` | — | Own cost (or credit if negative) in millionths of the currency unit. |
| `ended_ms` | integer ≥ 0 or `()` | — | End time in the Host clock domain. |
| `error` | string or `()` | — | Failure evidence. |
| `id` | string | required | Stable unique business identity. |
| `model` | string or `()` | — | Optional model caption. |
| `name` | string | required | Agent display name. |
| `parent` | string or `()` | — | Parent agent ID; omit for a lead. |
| `started_ms` | integer ≥ 0 or `()` | — | Start time in the Host clock domain. |
| `state` | `"queued"` or `"running"` or `"waiting"` or `"done"` or `"failed"` or `"stopped"` | required | Accepted execution state. |
| `steps` | array of object (at most 24) or `()` | — | Ordered task history available in the inspector. |
| `task` | string | required | What the agent was assigned. |
| `tools` | array of object (at most 6) or `()` | — | Tools present in the task's summary. |

### `agents[].steps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `elapsed_ms` | integer ≥ 0 or `()` | — | Fixed task duration. |
| `id` | string | required | Unique step ID within this agent. |
| `state` | `"queued"` or `"running"` or `"waiting"` or `"done"` or `"failed"` or `"stopped"` or `()` | — | Defaults to done. |
| `title` | string | required | Task detail. |
| `tool` | string or `()` | — | Optional tool caption. |

### `agents[].tools[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `icon` | node or `()` | — | Decorative tool mark. |
| `label` | string | required | Tool name. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `open_change` | `on_open_change` | object | The Host must accept the requested lead when controlled_open is enabled. |
| `resolve` | `on_resolve` | object | Validate run, revision, agent and state in the Host. A refusal leaves the inspector available. |
| `select` | `on_select` | object | Does not modify the accepted agent state. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `avatar`, `bar`, `body`, `branch`, `branch_empty`, `cost`, `count`, `empty`, `evidence`, `grid`, `head`, `highlight`, `inspect`, `inspect_title`, `lead`, `lead_open`, `lead_quiet`, `links`, `model`, `name`, `notice`, `review_actions`, `root`, `stats`, `status`, `status_done`, `status_failed`, `status_queued`, `status_running`, `status_stopped`, `status_waiting`, `step`, `step_state`, `step_time`, `step_tool`, `steps`, `sub`, `summary`, `summary_value`, `task`, `time`, `title`, `tool`, `tools`, `total`.

## Theme

- Tokens: `accent`, `agent_tree.link`, `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/popover`](../components/popover.md), [`patterns/agent_tree`](../patterns/agent_tree.md), [`patterns/approval_card`](../patterns/approval_card.md)
