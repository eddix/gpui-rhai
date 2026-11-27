# AgentPlan

`patterns/agent_plan` · export `AgentPlan` · version 0.1.0. Generated from
[`registry/patterns/agent_plan.rhai`](../../../registry/patterns/agent_plan.rhai); do not edit.

Review and edit an ordered plan, then present the Host's execution and recovery. State: an uncommitted text editor only; step order and phase are Host inputs.

```rhai
import "patterns/agent_plan" as agent_plan;

agent_plan::AgentPlan(#{key:"run",plan:"release",revision:0,now_ms:0,steps:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `allow_cancel` | bool | `false` | Show a Cancel proposal in draft, processing or failure. |
| `busy` | bool | `false` | Host is resolving a proposal; local editor text remains visible. |
| `estimate` | string or `()` | — | Estimated duration in the draft header. |
| `key` | string | required | Stable instance key. |
| `notice` | string or `()` | — | Host feedback from accepting or rejecting proposals. |
| `now_ms` | integer ≥ 0 | required | A single Host clock for active step durations. |
| `on_action` | callback or `()` | — | Receives versioned edits, run, cancel and retry proposals. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `plan` | string | required | Business identity of the plan. |
| `revision` | integer ≥ 0 | required | Current accepted revision. Advance when the plan changes, not on clock ticks. |
| `status` | `"draft"` or `"running"` or `"failed"` or `"done"` or `"cancelled"` | `"draft"` | Accepted plan phase. |
| `steps` | array of object (at most 48) | required | Steps in the current plan order. Task identity, not array index, owns edit/retry actions. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Goal heading. |

### `steps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `detail` | string or `()` | — | Short step explanation. |
| `elapsed_ms` | integer ≥ 0 or `()` | — | Accepted fixed duration of a terminal step. |
| `error` | string or `()` | — | Reason for failure. |
| `id` | string | required | Unique stable step ID. |
| `started_ms` | integer ≥ 0 or `()` | — | Host start of the active step. |
| `status` | `"pending"` or `"active"` or `"done"` or `"failed"` or `"skipped"` or `()` | — | Accepted step status; default pending. |
| `title` | string | required | Step title. |
| `tool` | string or `()` | — | Name of the tool, for display only. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Validate the plan, revision and identities in the Host. This component does not run tools. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `controls`, `detail`, `draft`, `editor`, `editor_title`, `elapsed`, `empty`, `error`, `estimate`, `field`, `field_label`, `footer`, `head`, `marker`, `marker_active`, `marker_done`, `marker_failed`, `marker_skipped`, `notice`, `number`, `rail`, `retry`, `root`, `sort_item`, `status`, `step`, `step_body`, `step_head`, `step_title`, `summary`, `title`, `tool`, `workspace`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.code`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/input`](../components/input.md), [`components/sortable`](../components/sortable.md), [`components/spinner`](../components/spinner.md), [`patterns/approval_card`](../patterns/approval_card.md)
