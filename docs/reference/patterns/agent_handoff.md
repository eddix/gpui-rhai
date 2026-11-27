# AgentHandoff

`patterns/agent_handoff` · export `AgentHandoff` · version 0.1.0. Generated from
[`registry/patterns/agent_handoff.rhai`](../../../registry/patterns/agent_handoff.rhai); do not edit.

Inspect relay evidence, restore omitted material, or propose assigning a task. State: inspection and the keyboard carry. The Host owns all transfers and assignments.

```rhai
import "patterns/agent_handoff" as agent_handoff;

agent_handoff::AgentHandoff(#{key:"relay",relay:"release",revision:0,agents:[],tasks:[],handoffs:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agents` | array of object (at most 16) | required | Which agents appear in the relay, in order. |
| `busy` | bool | `false` | Suspend assignment and restoration while the Host resolves a proposal. |
| `handoffs` | array of object (at most 64) | required | Accepted records, oldest first. The latest is shown first on each edge; history stays inspectable. |
| `key` | string | required | Stable instance key. |
| `notice` | string or `()` | — | Accepted Host notice or refusal. |
| `on_assign` | callback or `()` | — | Versioned task-assignment proposal. The Host updates the accepted task. |
| `on_restore` | callback or `()` | — | Versioned proposal to restore one omitted item in one transfer. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `relay` | string | required | Business relay identity, at most 72 UTF-8 bytes. Use a distinct drag namespace in a window. |
| `revision` | integer ≥ 0 | required | Version of the accepted relay snapshot. |
| `style` | style | — | Style merged over the root part. |
| `tasks` | array of object (at most 64) | required | Tasks keyed independently of their title. A historical transfer may outlive its current task record. |
| `title` | string or `()` | — | Relay heading. |

### `agents[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `accepting` | bool or `()` | — | Defaults to true; false prevents assignment to this destination. |
| `avatar` | node or `()` | — | Optional decorative avatar. |
| `cost_cents` | integer 0–9000000000000000 or `()` | — | Accepted usage cost in hundredths of a dollar. |
| `description` | string or `()` | — | Short role description. |
| `id` | string | required | Unique stable agent ID. |
| `model` | string or `()` | — | Optional model or role caption. |
| `name` | string | required | Display name. |

### `handoffs[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `from` | string | required | Sender ID. |
| `id` | string | required | Unique transfer ID. A newer ID on an edge replays its delivery motion. |
| `items` | array of object (at most 24) | required | Display what arrived and what was omitted, not file contents. |
| `task` | string | required | Stable task ID, not its title. |
| `title` | string or `()` | — | Historical caption to use if a task has been pruned. |
| `to` | string | required | Recipient ID. |

### `handoffs[].items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Item identity within this transfer. |
| `kind` | `"file"` or `"source"` or `"fact"` or `"decision"` or `"note"` or `()` | — | Kind of evidence. |
| `label` | string | required | Display caption. |
| `lost` | bool or `()` | — | True if it did not reach the recipient. |
| `why` | string or `()` | — | Reason shown when inspecting omitted material. |

### `tasks[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string or `()` | — | Accepted current or last owner. |
| `description` | string or `()` | — | Read-only task evidence. |
| `id` | string | required | Unique task ID; display titles are not identities. |
| `note` | string or `()` | — | Short progress or outcome. |
| `status` | `"todo"` or `"working"` or `"done"` or `"warning"` | required | Accepted task state. |
| `title` | string | required | Task name. |
| `tools` | array of object (at most 8) or `()` | — | Tool evidence, never invoked by the component. |

### `tasks[].tools[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `detail` | string or `()` | — | Optional trace detail. |
| `id` | string | required | Stable ID of a displayed tool. |
| `logo` | node or `()` | — | Optional decorative icon. |
| `name` | string | required | Display name or purpose. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `assign` | `on_assign` | object | Requests an assignment, without changing the relay automatically. |
| `restore` | `on_restore` | object | Requests restoration of the exact material; does not mutate the receipt or open any file. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `agent`, `agent_desc`, `agent_target`, `avatar`, `board`, `current`, `empty`, `handle`, `head`, `inspector`, `inspector_title`, `item`, `item_kind`, `item_lost`, `items`, `latest`, `line`, `lost`, `model`, `name`, `notice`, `packet`, `packet_meta`, `packet_title`, `pair`, `pair_count`, `pair_head`, `pair_name`, `pulse`, `reason`, `root`, `status`, `summary`, `task`, `task_list`, `task_title`, `title`, `wires`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `selection`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/drag_source`](../components/drag_source.md), [`components/drop_zone`](../components/drop_zone.md), [`components/scroll_area`](../components/scroll_area.md), [`patterns/approval_card`](../patterns/approval_card.md)
