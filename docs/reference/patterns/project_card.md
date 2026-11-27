# ProjectCard

`patterns/project_card` · export `ProjectCard` · version 0.1.0. Generated from
[`registry/patterns/project_card.rhai`](../../../registry/patterns/project_card.rhai); do not edit.

An expandable project summary with a segmented progress ring. State: disclosure, hover/focus, roster filter and picker; task and status snapshots belong to the Host.

```rhai
import "patterns/project_card" as project_card;

project_card::ProjectCard(#{key:"release",project:"r1",revision:0,title:"Ship the release",tasks:[#{id:"test",label:"Test",done:true}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `action_label` | string or `()` | — | Footer action label; hidden unless on_action is supplied. |
| `busy` | bool | `false` | Ignore task/status/action proposals while the Host commits. |
| `default_open` | bool | `false` | Initial uncontrolled disclosure. |
| `due` | string or `()` | — | Ready-to-display due date. |
| `hint` | node or `()` | — | Optional footer content when nothing is highlighted. |
| `key` | string | required | Stable retained card key. |
| `members` | array of object (at most 16) or `()` | — | Optional project roster. |
| `on_action` | callback or `()` | — | Receives a versioned footer action request. |
| `on_open_change` | callback or `()` | — | Receives a disclosure change or proposal. |
| `on_status_change` | callback or `()` | — | Receives a versioned status proposal. |
| `on_task_change` | callback or `()` | — | Receives a versioned task completion proposal. |
| `open` | bool or `()` | — | Controlled disclosure when present. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `project` | string | required | Stable project identity. |
| `revision` | integer ≥ 0 | required | Accepted snapshot revision. |
| `status` | string or `()` | — | Accepted status ID, or the first available option. |
| `statuses` | array of object (at most 8) or `()` | — | Status choices; defaults to To do, In progress, In review and Done. |
| `style` | style | — | Style merged over the root part. |
| `tasks` | array of object (at most 48) | required | Subtasks, in ring and list order. |
| `title` | string | required | Visible project title. |
| `trigger` | `"hover"` or `"click"` | `"hover"` | Open with pointer hover or explicit activation; keyboard always works. |

### `members[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `handle` | `image` handle or `()` | — | Optional prepared image; Avatar handles fallback. |
| `id` | string | required | Unique member ID. |
| `name` | string | required | Display name. |

### `statuses[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `icon` | node or `()` | — | Optional decorative replacement icon. |
| `id` | string | required | Unique status ID. |
| `label` | string | required | Visible status label. |
| `tone` | `"gray"` or `"violet"` or `"amber"` or `"blue"` or `"green"` or `"red"` or `()` | — | Status tone; gray by default. |

### `tasks[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `assignee` | string or `()` | — | Optional member ID, not their display name. |
| `done` | bool or `()` | — | Accepted completion. |
| `id` | string | required | Unique task ID. |
| `label` | string | required | Task label. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Invoke the caller's project action. |
| `open_change` | `on_open_change` | object | Notify or propose disclosure. |
| `status_change` | `on_status_change` | object | Propose replacing the accepted status. |
| `task_change` | `on_task_change` | object | Propose toggling one accepted task. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `assignee`, `body`, `caption`, `card`, `check`, `check_done`, `due`, `empty`, `face`, `face_active`, `foot`, `head`, `label`, `more`, `number`, `option`, `option_selected`, `paper`, `people`, `picker`, `ring`, `root`, `status`, `status_icon`, `status_live`, `status_open`, `status_text`, `sub`, `task`, `task_done`, `task_hot`, `tasks`, `title`, `titles`.

## Theme

- Tokens: `border`, `focus_ring`, `project_card.accent`, `project_card.amber`, `project_card.amber_active`, `project_card.amber_wash`, `project_card.blue`, `project_card.blue_active`, `project_card.blue_wash`, `project_card.card`, `project_card.gray`, `project_card.gray_active`, `project_card.gray_wash`, `project_card.green`, `project_card.green_active`, `project_card.green_wash`, `project_card.muted`, `project_card.option_fill`, `project_card.option_hover`, `project_card.option_ink`, `project_card.option_text`, `project_card.red`, `project_card.red_active`, `project_card.red_wash`, `project_card.ring`, `project_card.ring_hot`, `project_card.tray`, `project_card.violet`, `project_card.violet_active`, `project_card.violet_wash`, `radius.lg`, `radius.pill`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/avatar`](../components/avatar.md), [`components/button`](../components/button.md), [`patterns/agent_tree`](../patterns/agent_tree.md)
