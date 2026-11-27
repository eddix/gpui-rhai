# AgentTrail

`patterns/agent_trail` · export `AgentTrail` · version 0.1.0. Generated from
[`registry/patterns/agent_trail.rhai`](../../../registry/patterns/agent_trail.rhai); do not edit.

Follow execution and drill into recent substeps without transferring execution to the UI.

```rhai
import "patterns/agent_trail" as agent_trail;

agent_trail::AgentTrail(#{key:"trail",run:"preview",revision:0,now_ms:0,steps:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string or `()` | — | Agent or model caption. |
| `agent_icon` | node or `()` | — | Decorative agent icon in the heading. |
| `busy_step` | string or `()` | — | Waiting step whose proposal the Host is resolving. |
| `expanded` | array of string (at most 32) or `()` | — | Controlled expanded step IDs. Omit to expand running steps by default and allow local overrides. |
| `key` | string | required | Stable trail instance key. |
| `notice` | string or `()` | — | Host feedback; a refusal does not alter the accepted phase. |
| `now_ms` | integer ≥ 0 | required | Host clock shared by all running steps. |
| `on_decision` | callback or `()` | — | Receives a versioned decision for a waiting step. |
| `on_expand` | callback or `()` | — | Receives expansion intent in display order. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Accepted revision, independent of clock ticks. |
| `run` | string | required | Identity of the run represented by this snapshot. |
| `steps` | array of object (at most 32) | required | Ordered steps from the accepted snapshot; maximum 256 substeps in total. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Optional task title; omit to use the trail without a header. |

### `steps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `ask` | string or `()` | — | Evidence or question in the waiting step's approval disclosure. |
| `detail` | string or `()` | — | Current activity, outcome, or short step summary. |
| `elapsed_ms` | integer ≥ 0 or `()` | — | Fixed duration of a completed step. |
| `id` | string | required | Unique stable step ID. |
| `requires_approval` | bool or `()` | — | Show an upcoming approval before a pending step. |
| `started_ms` | integer ≥ 0 or `()` | — | Start time in the Host clock domain. |
| `state` | `"done"` or `"running"` or `"pending"` or `"waiting"` or `"failed"` or `"declined"` | required | Accepted execution state. |
| `substeps` | array of object (at most 48) or `()` | — | Ordered child progress; downsampled for initial folded display, never for business state. |
| `title` | string | required | Step title. |
| `tool` | string or `()` | — | Tool caption, also exposed when the caller supplies a logo. |
| `tool_icon` | node or `()` | — | Decorative logo for the step. |

### `steps[].substeps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `detail` | string or `()` | — | File, count or other short detail. |
| `id` | string | required | Stable ID, unique within this step. |
| `state` | `"done"` or `"running"` or `"pending"` or `()` | — | Default done. |
| `title` | string | required | Substep caption. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `decision` | `on_decision` | object | The Host must check the run, revision and waiting step. The component does not approve or start work locally. |
| `expand` | `on_expand` | object | Adopt the list to accept controlled disclosure. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `agent`, `agent_icon`, `ask`, `body`, `clock`, `count`, `declined`, `detail`, `empty`, `entry`, `failed`, `fold`, `head`, `name`, `node`, `notice`, `pending`, `pulse`, `question`, `rail`, `root`, `row`, `stage`, `status`, `sub`, `subdetail`, `sublist`, `subnode`, `subnode_active`, `subtitle`, `title`, `tool`, `waiting`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/button`](../components/button.md), [`components/spinner`](../components/spinner.md), [`patterns/approval_card`](../patterns/approval_card.md)
