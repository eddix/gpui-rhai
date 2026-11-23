# RunningNow

`patterns/running_now` · export `RunningNow` · version 0.1.0. Generated from
[`registry/patterns/running_now.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/running_now.rhai); do not edit.

A quiet trigger and a live, nonmodal task list. Task state is Host-owned.

```rhai
import "patterns/running_now" as running_now;

running_now::RunningNow(#{key:"run-chip",board:"jobs",revision:0,tasks:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `align` | `"start"` or `"end"` | `"end"` | Logical popover alignment at the trigger edge. |
| `board` | string | required | Stable business board identity, included in action proposals. |
| `default_open` | bool | `false` | Initial disclosure in local mode. |
| `key` | string | required | Stable trigger instance and overlay key. |
| `on_action` | callback or `()` | — | Receives a versioned pause or resume request; task state is not changed automatically. |
| `on_open_all` | callback or `()` | — | Optional action on the footer to show the full task list. |
| `on_open_change` | callback or `()` | — | Receives the requested disclosure boolean. |
| `on_open_task` | callback or `()` | — | Receives the selected task identity. |
| `open` | bool or `()` | — | Controlled disclosure. The Host accepts changes through on_open_change. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pending` | array of string (at most 64) | `[]` | Task IDs whose mutations are awaiting Host acknowledgement. Duplicate pause/resume requests are suppressed. |
| `revision` | integer ≥ 0 | required | Accepted task snapshot revision. |
| `style` | style | — | Style merged over the root part. |
| `tasks` | array of object (at most 64) | required | Task rows in the caller's stable display order. |
| `title` | string or `()` | — | Optional localized name of the task list. |

### `tasks[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `done` | integer 0–1000000 | required | Completed steps, clamped to total. |
| `id` | string | required | Stable task ID; unique within the board. |
| `status` | `"running"` or `"paused"` or `"needs_you"` or `"done"` | required | Accepted task status. |
| `step` | string | `""` | Current step or what the task is waiting for. |
| `title` | string | required | Two-line row's first label. |
| `total` | integer 0–1000000 | required | Expected number of steps; zero means the number of steps is unknown. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Propose a task transition; the caller decides whether it takes effect. |
| `open_all` | `on_open_all` | object | Open the full task list. |
| `open_change` | `on_open_change` | bool | Requested disclosure state. |
| `open_task` | `on_open_task` | object | Open an individual task by business identity. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `action_label`, `action_text`, `announcement`, `badge`, `body`, `caption`, `chevron`, `count`, `done_mark`, `empty`, `finished`, `footer`, `heading`, `label`, `list`, `main`, `name`, `panel`, `part`, `ring`, `root`, `row`, `separator`, `step`, `trigger`, `trigger_caption`, `trigger_open`.

## Theme

- Tokens: `border`, `focus_ring`, `radius.lg`, `radius.pill`, `radius.sm`, `running_now.amber`, `running_now.badge`, `running_now.badge_text`, `running_now.blue`, `running_now.faint`, `running_now.orange`, `running_now.teal`, `running_now.trigger_ink`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`patterns/agent_tree`](agent_tree.md)
