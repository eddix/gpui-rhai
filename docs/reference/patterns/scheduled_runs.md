# ScheduledRuns

`patterns/scheduled_runs` · export `ScheduledRuns` · version 0.1.0. Generated from
[`registry/patterns/scheduled_runs.rhai`](../../../registry/patterns/scheduled_runs.rhai); do not edit.

A capacity-checked schedule board. The caller owns all accepted assignments.

```rhai
import "patterns/scheduled_runs" as scheduled_runs;

scheduled_runs::ScheduledRuns(#{key:"week",board:"jobs",revision:0,mode:"manual",days:[],periods:[],tasks:[],assignments:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agents` | array of object (at most 12) | `[]` | Optional agent filter choices. |
| `assignments` | array of object (at most 224) | required | Accepted placements. Each booked task must be represented completely. |
| `board` | string | required | Business board identity. |
| `current` | object or `()` | — | Current session. Earlier bookings remain inspectable but cannot be picked up or receive placements. |
| `days` | array of object (at most 7) | required | Day columns in chronological order. |
| `key` | string | required | Stable component instance key. |
| `minimum` | integer 1–1000000 | `15` | Minimum capacity of a split part (one-part tasks may be smaller). |
| `mode` | `"auto"` or `"manual"` | required | Accepted mode. Changing to manual removes only automatic placements when the Host accepts the proposal. |
| `note` | string or `()` | — | Secondary heading, such as the session duration. |
| `on_propose` | callback or `()` | — | Receives a complete proposed next assignment set and revision. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pending` | bool | `false` | Suppress further proposals while the Host is deciding. |
| `periods` | array of object (at most 8) | required | Sessions in chronological order for every day. |
| `revision` | integer 0–2147483647 | required | Accepted schedule snapshot revision. |
| `style` | style | — | Style merged over the root part. |
| `tasks` | array of object (at most 64) | required | Tasks, in the desired backlog order. |
| `title` | string or `()` | — | Heading. |

### `agents[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable agent ID. |
| `label` | string | required | Name in the filter. |
| `mark` | string or `()` | — | Optional short avatar initials. |

### `assignments[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `day` | string | required | Day ID. |
| `id` | string | required | Stable booked piece ID. |
| `ordinal` | integer 1–56 | required | One-based part number within its task. |
| `origin` | `"booked"` or `"manual"` or `"auto"` | required | Accepted assignment source. |
| `period` | string | required | Session ID. |
| `task` | string | required | Task ID. |
| `units` | integer 1–1000000 | required | Booked usage. |

### `current` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `day` | string | required | Current day ID. |
| `period` | string | required | Current session ID. |

### `days[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `date` | string or `()` | — | Optional date label. |
| `id` | string | required | Stable day ID. |
| `label` | string | required | Day name. |

### `periods[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable session ID within a day. |
| `label` | string | required | Session start label. |
| `limit` | integer 1–1000000 | required | Capacity in whole usage units. |

### `tasks[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string or `()` | — | Optional agent ID. |
| `detail` | string or `()` | — | Task detail. |
| `id` | string | required | Stable task ID. |
| `stats` | array of object (at most 2) or `()` | — | Up to two compact metrics, with full labels in details. |
| `title` | string | required | Task title. |
| `units` | integer 1–1000000 | required | Total usage to allocate. |

### `tasks[].stats[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `label` | string | required | Name read in details. |
| `value` | string | required | Displayed quantity. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `propose` | `on_propose` | object | The candidate assignment set; publish with a new revision only after acceptance. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `agent`, `agent_active`, `agent_mark`, `agent_name`, `bar`, `bar_fill`, `bar_fill_past`, `blocked`, `cards`, `cell`, `columns`, `counter`, `current`, `day`, `day_caption`, `day_current`, `day_date`, `detail`, `detail_line`, `detail_name`, `empty`, `footer`, `grip`, `heading`, `mode`, `mode_active`, `mode_track`, `note`, `panel`, `past`, `root`, `stack`, `stack_count`, `stack_head`, `stack_header`, `subdued`, `target`, `task`, `task_dim`, `task_mark`, `task_mark_box`, `task_meta`, `task_metrics`, `task_part`, `task_stat`, `task_title`, `time`, `title`, `tools`, `usage`, `usage_badge`, `usage_label`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `radius.lg`, `radius.pill`, `radius.sm`, `scheduled_runs.capacity`, `scheduled_runs.card`, `scheduled_runs.current`, `scheduled_runs.drop`, `scheduled_runs.faint`, `scheduled_runs.hatch`, `scheduled_runs.ink`, `scheduled_runs.lane`, `scheduled_runs.muted`, `scheduled_runs.text`, `scheduled_runs.track`, `scheduled_runs.well`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.label`, `typography.metadata`

## Dependencies

[`components/button`](../components/button.md), [`components/drag_source`](../components/drag_source.md), [`components/drop_zone`](../components/drop_zone.md), [`components/radio_group`](../components/radio_group.md), [`patterns/agent_tree`](../patterns/agent_tree.md)
