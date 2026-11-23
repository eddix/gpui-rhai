# SetupGuide

`components/setup_guide` · export `SetupGuide` · version 0.1.0. Generated from
[`registry/components/setup_guide.rhai`](../../../registry/components/setup_guide.rhai); do not edit.

An onboarding panel with a bounded local disclosure model and Host-owned data.

```rhai
import "components/setup_guide" as setup_guide;

setup_guide::SetupGuide(#{key:"setup",groups:[#{id:"first",title:"First steps",tasks:[#{id:"invite",label:"Invite teammates"}]}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Disables actions and task changes. |
| `groups` | array of object (at most 16) | required | Groups in display order. |
| `key` | string | required | Stable identity of this panel. |
| `on_change` | callback or `()` | — | Complete list of proposed completed task IDs. |
| `on_close` | callback or `()` | — | Shows Close; the Host decides whether to remove the panel. |
| `on_edit` | callback or `()` | — | Shows an Edit action; the Host decides what it opens. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Panel title. |
| `value` | array of string (at most 128) or `()` | — | Controlled completed IDs; local completion is used when omitted. |

### `groups[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Unique nonempty group ID. |
| `tasks` | array of object (at most 32) | required | Sequential tasks. The panel supports up to 128 tasks across its groups. |
| `title` | string | required | Group heading. |

### `groups[].tasks[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Prevents manual completion, for example while the app owns the task. |
| `done` | bool | `false` | Initial local state for a newly admitted task. |
| `id` | string | required | Task ID, unique across the panel. |
| `label` | string | required | Task label and accessible name. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | array of string (at most 128) | Completed task proposal in definition order. A rejected proposal never advances disclosure. |
| `close` | `on_close` | none | Close requested. |
| `edit` | `on_edit` | none | Edit requested. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `body`, `check`, `chevron`, `count`, `digit`, `edit`, `fill`, `group`, `group_done`, `group_header`, `group_open`, `group_title`, `header`, `icon`, `icon_button`, `label`, `label_done`, `label_locked`, `locked_icon`, `mark`, `mark_done`, `mark_locked`, `progress`, `root`, `strike`, `task`, `task_current`, `task_locked`, `tasks`, `title`, `track`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `radius.lg`, `radius.md`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.caption`, `typography.control`
- Environment: `density`

## Dependencies

[`components/accordion`](../components/accordion.md), [`components/button`](../components/button.md), [`components/icon_button`](../components/icon_button.md), [`components/progress_tracker`](../components/progress_tracker.md)
