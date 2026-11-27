# AgentInbox

`patterns/agent_inbox` · export `AgentInbox` · version 0.1.0. Generated from
[`registry/patterns/agent_inbox.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/agent_inbox.rhai); do not edit.

One agent-task inbox in board and list form. Actions never mutate accepted jobs. State: local layout, focused task and disclosure. Timing is supplied by the Host.

```rhai
import "patterns/agent_inbox" as agent_inbox;

agent_inbox::AgentInbox(#{key:"activity",inbox:"release",revision:0,now_ms:0,tasks:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `allow_open` | bool | `false` | Show an Open proposal in running and done details. |
| `busy` | bool | `false` | Disables all task proposals; inspection and layout stay usable. |
| `default_view` | `"board"` or `"list"` | `"board"` | Initial local layout. |
| `highlight` | string or `()` | — | An accepted task to emphasize. Other tasks stay usable. |
| `inbox` | string | required | Business identity of the inbox. |
| `key` | string | required | Stable instance key. |
| `lanes` | array of `"needs"` or `"running"` or `"done"` (at most 3) or `()` | — | Limit the visible lanes, in their canonical order; omit to show all. |
| `new_label` | string or `()` | — | Optional caption for a Host new-task proposal. |
| `notice` | string or `()` | — | Accepted Host feedback. |
| `now_ms` | integer ≥ 0 | required | One Host clock for all live tasks. |
| `on_action` | callback or `()` | — | Receives a revisioned task proposal. |
| `on_view_change` | callback or `()` | — | Receives a layout choice. A controlled caller publishes the accepted layout. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Accepted revision, advanced on task changes, not time ticks. |
| `style` | style | — | Style merged over the root part. |
| `tasks` | array of object (at most 96) | required | Accepted tasks in order within their lane. Waiting and failed tasks share Needs you. |
| `title` | string or `()` | — | Heading above the lanes. |
| `view` | `"board"` or `"list"` or `()` | — | Controlled layout. Omit for local layout state. |

### `tasks[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string | required | Name of the assigned agent or model. |
| `busy` | bool or `()` | — | Disables this task's actions without changing its accepted state. |
| `description` | string or `()` | — | Summary shown on the card and expanded on inspection. |
| `elapsed_ms` | integer ≥ 0 or `()` | — | Fixed completed or failed duration, never advanced by the live clock. |
| `id` | string | required | Unique stable task ID. |
| `sources` | integer ≥ 0 or `()` | — | Number of sources read. |
| `started_ms` | integer ≥ 0 or `()` | — | Start of the waiting or running phase on the Host clock. |
| `status` | `"waiting"` or `"running"` or `"done"` or `"failed"` | required | Accepted task state. |
| `steps` | array of object (at most 24) or `()` | — | Task evidence in chronological order; its four most recent steps are shown. |
| `title` | string | required | Task name; the title is not an identity. |
| `tools` | integer ≥ 0 or `()` | — | Number of tool calls to report. |

### `tasks[].steps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable business ID within the task. |
| `label` | string | required | Step summary. |
| `time` | string or `()` | — | Optional display time supplied by the Host. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Validate against the current task, status, and revision before applying. |
| `view_change` | `on_view_change` | object | Only changes the layout, never the task data. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `agent`, `body`, `card`, `card_highlight`, `card_mark`, `card_open`, `cards`, `controls`, `count`, `crumb`, `description`, `empty`, `foot`, `head`, `lane`, `lane_head`, `lane_mark`, `lane_title`, `lanes`, `main`, `main_list`, `metric`, `metrics`, `notice`, `option`, `option_active`, `root`, `state`, `state_done`, `state_failed`, `state_running`, `state_waiting`, `step`, `step_time`, `steps`, `summary`, `switcher`, `task_title`, `title`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `selection`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`patterns/approval_card`](approval_card.md)
