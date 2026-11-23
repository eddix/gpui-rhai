# AgentLanes

`patterns/agent_lanes` · export `AgentLanes` · version 0.1.0. Generated from
[`registry/patterns/agent_lanes.rhai`](../../../registry/patterns/agent_lanes.rhai); do not edit.

A clipped, interactive time window with non-overlapping agent tasks and presented hand-offs. State: locally selected and focused task. Only the Host changes tasks or accepts a decision.

```rhai
import "patterns/agent_lanes" as agent_lanes;

agent_lanes::AgentLanes(#{key:"activity",timeline:"build",revision:0,now_ms:0,agents:[],tasks:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agents` | array of object (at most 24) | required | Lanes in supplied order. |
| `allow_open` | bool | `false` | Show a Host navigation proposal in the inspector. |
| `highlight` | string or `()` | — | Optional highlighted agent. Other lane tasks remain available. |
| `key` | string | required | Stable instance key. |
| `notice` | string or `()` | — | A Host response displayed without changing local task state. |
| `now_ms` | integer ≥ 0 | required | Current time supplied by the Host. |
| `on_action` | callback or `()` | — | Receives approve, reject, or open proposals for the accepted revision. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Accepted revision; change it on business updates, not clock ticks. |
| `span_ms` | integer 1000–604800000 | `1800000` | Width of the visible history window up to now. |
| `style` | style | — | Style merged over the root part. |
| `tasks` | array of object (at most 128) | required | Tasks on the shared timeline. Overlapping tasks occupy different tracks without changing their time. |
| `timeline` | string | required | Business identity of the activity stream. |
| `title` | string or `()` | — | Heading above the lanes. |

### `agents[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `avatar` | node or `()` | — | Optional decorative mark; defaults to a text mark. |
| `id` | string | required | Unique stable lane ID. |
| `model` | string or `()` | — | Optional model or role. |
| `name` | string | required | Agent name. |

### `tasks[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string | required | One declared lane ID. |
| `ask` | string or `()` | — | Decision presented for a waiting task. |
| `busy` | bool or `()` | — | Disables waiting-task proposals until the Host decides. |
| `end_ms` | integer ≥ 0 or `()` | — | An explicit end after or equal to start. Required for done or failed; omitted ongoing tasks reach now. |
| `handoff_to` | string or `()` | — | Successor ID. Drawn only when both ends are in the displayed window. |
| `id` | string | required | Unique stable task ID. |
| `start_ms` | integer ≥ 0 | required | Start on the Host clock. |
| `status` | `"running"` or `"waiting"` or `"done"` or `"failed"` | required | Accepted activity state. |
| `summary` | array of string (at most 8) or `()` | — | Read-only observations shown in the task inspector. |
| `title` | string | required | Task name on the timeline and in inspection. |
| `tools` | array of object (at most 3) or `()` | — | Up to three tool marks. |

### `tasks[].tools[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `icon` | node or `()` | — | Optional decorative tool mark; no tool is invoked. |
| `label` | string | required | Tool name included in the accessible description. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Validate the task and revision in the Host before accepting a proposal. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `agent`, `agent_name`, `avatar`, `axis`, `body`, `card`, `duration`, `empty`, `head`, `inspector`, `inspector_title`, `lane`, `marker`, `model`, `notice`, `paint`, `pressure`, `question`, `root`, `stage`, `summary`, `task`, `task_done`, `task_failed`, `task_open`, `task_running`, `task_title`, `task_waiting`, `tick`, `title`, `tool`, `track`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `selection`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`patterns/approval_card`](../patterns/approval_card.md)
