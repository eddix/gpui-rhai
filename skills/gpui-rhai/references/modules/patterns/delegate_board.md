# DelegateBoard

`patterns/delegate_board` · export `DelegateBoard` · version 0.1.0. Generated from
[`registry/patterns/delegate_board.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/delegate_board.rhai); do not edit.

Move work between an unassigned pile and agent queues with one versioned assignment contract. State: keyboard pick, focus handoff, and one automatic attempt per accepted board revision.

```rhai
import "patterns/delegate_board" as delegate_board;

delegate_board::DelegateBoard(#{key:"work",board:"work",revision:0,now_ms:0,agents:[],tasks:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agents` | array of object (at most 16) | required | Agent order also breaks ties in the least-loaded allocation. |
| `board` | string | required | Stable business identity; use a distinct drag namespace for each board in a window. At most 72 UTF-8 bytes. |
| `busy` | bool | `false` | Host is resolving a proposal; interaction stops and accepted tasks stay in place. |
| `currency` | string | `"$"` | Display currency prefix for optional queue costs. |
| `key` | string | required | Stable component instance key. |
| `mode` | `"manual"` or `"auto"` | `"manual"` | The accepted policy; auto proposes a batch for a new board revision while there is available work. |
| `notice` | string or `()` | — | Host acceptance, refusal, or other information. |
| `now_ms` | integer 0–253402300799999 | required | Host epoch milliseconds used for busy-until estimates. |
| `on_assign` | callback or `()` | — | Receives one assignment proposal, containing one move or a batch. |
| `on_mode_change` | callback or `()` | — | Requests a manual/auto mode change; pass the new mode only after acceptance. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Accepted board revision. Advance it when tasks, assignments, agent availability or mode change. |
| `style` | style | — | Style merged over the root part. |
| `suggestion` | object or `()` | — | Optional caller suggestion; highlights the task and previews the agent's new workload, without assigning it. |
| `tasks` | array of object (at most 64) | required | Tasks in accepted queue order. The first task assigned to each agent is its running task. |
| `time_zone` | string | `"UTC"` | Zone used by the locale time formatter. |
| `title` | string or `()` | — | Board heading. |

### `agents[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `accepting` | bool or `()` | — | Defaults to true; false prevents new assignments without clearing the current queue. |
| `avatar` | node or `()` | — | Optional decorative agent mark. |
| `current_minutes` | integer 0–525600 or `()` | — | Remaining minutes for the first queued task; defaults to that task's estimate. |
| `id` | string | required | Unique nonempty stable agent ID. |
| `model` | string or `()` | — | Model or role label. |
| `name` | string | required | Agent name. |
| `rate_cents` | integer 0–100000 or `()` | — | Optional price per minute in hundredths of the displayed currency. |

### `suggestion` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string | required | Proposed agent. |
| `task` | string | required | Task being suggested. |

### `tasks[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string or `()` | — | Accepted agent ID; omit or pass null for unassigned. |
| `estimate` | integer 0–525600 | required | Estimated minutes of work. |
| `id` | string | required | Unique nonempty stable task ID. |
| `title` | string | required | Task caption. |
| `tools` | array of object (at most 3) or `()` | — | Up to three tool marks, with no tool execution. |

### `tasks[].tools[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `icon` | node or `()` | — | Optional decorative tool icon; falls back to a text mark. |
| `label` | string | required | Tool name for the accessibility description. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `assign` | `on_assign` | object | A proposal does not move accepted tasks. Only a Host update changes ownership. |
| `mode_change` | `on_mode_change` | object | Requests a mode change; does not assign work on its own. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `agent`, `agent_name`, `agent_suggested`, `agent_target`, `agents`, `avatar`, `cost`, `count`, `empty`, `estimate`, `footer`, `free`, `head`, `hint`, `identity`, `lanes`, `load`, `mode`, `model`, `notice`, `pick_help`, `pile`, `queue`, `queue_task`, `queued`, `root`, `running`, `section_title`, `status`, `symbol`, `task`, `task_picked`, `task_suggested`, `task_title`, `time`, `title`, `tool`, `tools`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `radius.lg`, `selection`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.success`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/drag_source`](../components/drag_source.md), [`components/drop_zone`](../components/drop_zone.md), [`components/scroll_area`](../components/scroll_area.md), [`components/switch`](../components/switch.md), [`patterns/approval_card`](approval_card.md)
