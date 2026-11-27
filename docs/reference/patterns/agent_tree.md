# AgentTree

`patterns/agent_tree` · export `AgentTree` · version 0.1.0. Generated from
[`registry/patterns/agent_tree.rhai`](../../../registry/patterns/agent_tree.rhai); do not edit.

A dispatch hierarchy with separate visual, focus and accepted run state.

```rhai
import "patterns/agent_tree" as agent_tree;

agent_tree::AgentTree(#{key:"team",run:"preview",revision:0,now_ms:0,agents:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agents` | array of object (at most 64) | required | Every agent in the run, including folded nodes. Siblings respect input order. |
| `busy_agent` | string or `()` | — | Agent with an unresolved Host response; disables its resolution buttons. |
| `collapsed` | array of string (at most 64) or `()` | — | Controlled collapsed branch IDs. Omit to let the component own disclosure. |
| `currency` | string or `()` | — | Currency label for costs; defaults to USD. |
| `default_collapsed` | array of string (at most 64) or `()` | — | Initial collapsed branch IDs when disclosure is local. |
| `key` | string | required | Stable component key. |
| `notice` | string or `()` | — | Feedback about the latest proposal, also visible in an open review panel. |
| `now_ms` | integer ≥ 0 | required | One shared clock for all active agents. |
| `on_collapse` | callback or `()` | — | Receives the proposed collapsed snapshot. |
| `on_resolve` | callback or `()` | — | Receives approval and failure-resolution proposals. |
| `on_select` | callback or `()` | — | Requests selecting an agent, for example to show its log. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Accepted business revision independent of clock ticks. |
| `root` | string or `()` | — | Agent at the root. If omitted, infer the unique non-supervisor root. |
| `run` | string | required | Identity of the accepted run. |
| `selected` | string or `()` | — | Host-selected agent. Its descendants are distinguished from unrelated branches. |
| `style` | style | — | Style merged over the root part. |
| `supervisor` | string or `()` | — | Optional peer watching the root, with no parent; not included in the root's subtree cost. |
| `title` | string or `()` | — | Heading for this dispatch tree. |
| `top` | node or `()` | — | Optional run summary above the cards. |

### `agents[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `ask` | string or `()` | — | What is waiting for a decision. |
| `avatar` | node or `()` | — | Decorative avatar; defaults to initials. |
| `cost_micros` | integer 0–1000000000000 or `()` | — | Agent's own cost in millionths of the currency unit. |
| `ended_ms` | integer ≥ 0 or `()` | — | Fixed clock reading when work ended. |
| `error` | string or `()` | — | Failure evidence. |
| `extra` | node or `()` | — | Optional details inside the card. |
| `id` | string | required | Unique stable business identity. |
| `model` | string or `()` | — | Model caption. |
| `name` | string | required | Display name. |
| `parent` | string or `()` | — | ID of the dispatcher; input order need not be topological. |
| `started_ms` | integer ≥ 0 or `()` | — | Clock reading when work started. |
| `state` | `"queued"` or `"running"` or `"waiting"` or `"done"` or `"failed"` or `"stopped"` | required | Accepted execution status. |
| `task` | string | required | Assigned task. |
| `tools` | array of object (at most 6) or `()` | — | Compact tool marks with named accessibility descriptions. |

### `agents[].tools[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `icon` | node or `()` | — | Optional decorative tool icon. |
| `label` | string | required | Tool name. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `collapse` | `on_collapse` | object | Apply this list to accept a controlled fold. |
| `resolve` | `on_resolve` | object | Does not change, retry or dismiss work until the Host accepts. |
| `select` | `on_select` | object | Apply selected to update the highlighted branch. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `top` | no | no | Optional summary above the dispatch tree. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `avatar`, `card`, `cell`, `cost`, `count`, `empty`, `fold`, `grid`, `head`, `model`, `name`, `notice`, `outside`, `rail`, `related`, `review`, `review_actions`, `review_detail`, `review_name`, `review_viewport`, `root`, `running_count`, `selected`, `stage`, `stats`, `status`, `status_done`, `status_failed`, `status_queued`, `status_running`, `status_stopped`, `status_waiting`, `summary_button`, `summary_failed`, `summary_waiting`, `supervisor`, `task`, `time`, `title`, `tool`, `tools`, `total`.

## Theme

- Tokens: `accent`, `agent_tree.link`, `agent_tree.link_muted`, `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/button`](../components/button.md), [`components/popover`](../components/popover.md), [`components/spinner`](../components/spinner.md), [`patterns/approval_card`](../patterns/approval_card.md)
