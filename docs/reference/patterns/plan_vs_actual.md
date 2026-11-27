# PlanVsActual

`patterns/plan_vs_actual` · export `PlanVsActual` · version 0.1.0. Generated from
[`registry/patterns/plan_vs_actual.rhai`](../../../registry/patterns/plan_vs_actual.rhai); do not edit.

Compare a dependency plan to observed work without starting or modifying tasks.

```rhai
import "patterns/plan_vs_actual" as plan_vs_actual;

plan_vs_actual::PlanVsActual(#{key:"run",run:"pricing",now_ms:18000,steps:[#{id:"layout",title:"Build layout",planned_ms:14000,start_ms:0}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `currency` | string or `()` | — | Display prefix, defaults to $. |
| `key` | string | required | Stable instance key. |
| `now_ms` | integer 0–31536000000 | required | Host-owned elapsed milliseconds; no script timer is created. |
| `on_inspect` | callback or `()` | — | Called for a deliberate pin or unpin, not passive hover. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pinned` | string or `()` | — | Controlled inspection ID, or empty to leave unpinned. |
| `revision` | integer ≥ 0 | `0` | Accepted revision returned with inspection proposals. |
| `run` | string | required | Identity of a single run. Changing it clears ephemeral attention. |
| `steps` | array of object (at most 96) | required | Accepted snapshot in display order; the steps form an acyclic dependency graph. |
| `style` | style | — | Style merged over the root part. |
| `task` | string or `()` | — | Optional shared task label. |
| `title` | string or `()` | — | Defaults to Plan vs actual. |

### `steps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `depends_on` | array of string (at most 32) | `[]` | Distinct prerequisite step IDs. Input does not have to be topologically ordered. |
| `end_ms` | integer 0–31536000000 or `()` | — | Recorded finish. When only the finish is known, the display infers a non-negative start from the planned duration. |
| `icon` | node or `()` | — | Optional decorative mark beside the step. |
| `id` | string | required | Unique stable step ID. |
| `planned_micros` | integer 0–1000000000000 or `()` | — | Optional budget in currency micro-units. A known zero is not missing. |
| `planned_ms` | integer 0–31536000000 | required | Planned step duration. |
| `spent_micros` | integer -1000000000000–1000000000000 or `()` | — | Observed signed cost in currency micro-units. |
| `start_ms` | integer 0–31536000000 or `()` | — | Actual start, relative to run start. Omit for waiting steps. |
| `title` | string | required | Visible step name. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `inspect` | `on_inspect` | object | Inspection proposal. Empty ID clears the pin. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actual`, `axis`, `axis_label`, `delta`, `detail`, `detail_line`, `detail_title`, `duration`, `empty`, `finish_tag`, `forecast`, `guide`, `head`, `head_narrow`, `legend`, `legend_mark`, `legend_narrow`, `mark`, `metric`, `metric_label`, `metric_value`, `now_dot`, `over`, `plan`, `rail`, `root`, `row`, `row_dim`, `row_hot`, `scaffold`, `sheen`, `state`, `step`, `step_icon`, `task`, `tick`, `title`, `totals`, `totals_narrow`, `track`, `verdict`, `wire`, `wire_dot`.

## Theme

- Tokens: `border`, `focus_ring`, `plan_vs_actual.actual`, `plan_vs_actual.actual_edge`, `plan_vs_actual.actual_mark`, `plan_vs_actual.ahead`, `plan_vs_actual.ahead_fill`, `plan_vs_actual.ahead_gap`, `plan_vs_actual.card`, `plan_vs_actual.forecast`, `plan_vs_actual.forecast_edge`, `plan_vs_actual.ghost`, `plan_vs_actual.grid`, `plan_vs_actual.late`, `plan_vs_actual.late_edge`, `plan_vs_actual.late_fill`, `plan_vs_actual.late_gap`, `plan_vs_actual.link`, `plan_vs_actual.live`, `plan_vs_actual.live_line`, `plan_vs_actual.muted`, `plan_vs_actual.quiet`, `plan_vs_actual.text`, `plan_vs_actual.wash`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
