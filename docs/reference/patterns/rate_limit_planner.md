# RateLimitPlanner

`patterns/rate_limit_planner` · export `RateLimitPlanner` · version 0.1.0. Generated from
[`registry/patterns/rate_limit_planner.rhai`](../../../registry/patterns/rate_limit_planner.rhai); do not edit.

A 24-hour load forecast with native drag previews and reversible Host proposals.

```rhai
import "patterns/rate_limit_planner" as rate_limit_planner;

rate_limit_planner::RateLimitPlanner(#{key:"plan",session:"sync",limit:40000,runs:[#{id:"sync",name:"Sync index",at:480,peak:20000}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actual` | array of number or `()` (at most 1440) | `[]` | Minute readings from the window start. Null gaps carry the previous reading; no unmeasured past is drawn. |
| `buffer` | number 0.05–1 | `0.8` | Fraction of the hard limit used as the warning line, not a change to the limit. |
| `checkpoint` | object or `()` | — | Optional marker and load summary at a weekday or weekend checkpoint. |
| `controlled` | bool | `true` | When false, runs seed a local plan for this session. |
| `day` | `"weekdays"` or `"weekend"` | `"weekdays"` | Day category used to select recurring runs for this forecast. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `limit` | integer 1–1000000000 | required | Hard shared limit in tokens per minute; excess contributes to overload bands. |
| `now` | integer 0–1439 or `()` | — | Wall-clock minute now. Earlier offsets within this 24-hour window cannot be changed. |
| `on_runs_change` | callback or `()` | — | Receives a complete proposed plan plus its session, revision and operation; the SDK never schedules those runs. |
| `on_selected_change` | callback or `()` | — | Receives the ID of the run chosen for inspection. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pending` | bool | `false` | Disables plan mutations while a controlled Host decides. |
| `revision` | integer ≥ 0 | `0` | Version of the Host's accepted inputs and schedule. |
| `runs` | array of object (at most 64) | required | Ordered forecast runs. IDs remain stable when reshuffled. |
| `selected` | string or `()` | — | Optional run ID for the detailed curve; omission uses the local selection. |
| `session` | string | required | Identity of this plan; changing it resets local selection and editing. |
| `start` | integer 0–1439 | `360` | Minute after midnight at the start of the rolling 24-hour plot. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Overrides the visible plan heading. |

### `checkpoint` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `at` | integer 0–1439 | required | Checkpoint minute after midnight, in the same wall-clock day as the runs. |
| `label` | string | required | Human-readable checkpoint caption used next to the reported time. |

### `runs[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `at` | integer 0–1439 | required | Minute after midnight in the planner's declared wall-clock day. |
| `days` | `"daily"` or `"weekdays"` or `"weekend"` or `()` | — | Selects which kind of day includes this run in its forecast; omission means daily. |
| `half_life` | integer 1–1440 or `()` | — | Minutes of decay for one half-life; defaults to 45. |
| `id` | string | required | Unique run key; retain it when moving or editing the run. |
| `name` | string | required | Caption used in the chart, editor and selected-run summary. |
| `peak` | integer 0–1000000000 | required | Peak tokens per minute. |
| `rise` | integer 1–720 or `()` | — | Minutes to reach the peak; defaults to 30. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `runs_change` | `on_runs_change` | object | One complete plan proposal. Applies no external schedule. |
| `selected_change` | `on_selected_change` | string | Selected run ID for the detailed blue curve. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `add`, `axis`, `badge`, `bar`, `bar_hatch`, `bar_locked`, `bar_selected`, `bar_sheen`, `bar_text`, `button`, `dot`, `editor`, `empty`, `field`, `fit`, `flag`, `foot`, `head`, `icon`, `lane`, `lane_selected`, `mark`, `name`, `peak`, `plan_fill`, `plot`, `plot_hatch`, `plot_row`, `root`, `row_tools`, `scale`, `selected_fill`, `start`, `summary`, `tick`, `tip`, `title`, `verdict`.

## Theme

- Tokens: `focus_ring`, `metrics.rate_plan_sheen`, `radius.lg`, `radius.sm`, `rate_plan.accent`, `rate_plan.actual`, `rate_plan.amber`, `rate_plan.blue`, `rate_plan.button`, `rate_plan.card`, `rate_plan.cyan`, `rate_plan.danger`, `rate_plan.danger_bg`, `rate_plan.green`, `rate_plan.guide`, `rate_plan.hatch`, `rate_plan.hatch_bg`, `rate_plan.ink`, `rate_plan.line`, `rate_plan.muted`, `rate_plan.okay`, `rate_plan.okay_bg`, `rate_plan.on_button`, `rate_plan.pink`, `rate_plan.popup`, `rate_plan.purple`, `rate_plan.stroke`, `rate_plan.tag`, `rate_plan.tag_over`, `rate_plan.teal`, `rate_plan.warm`, `rate_plan.warm_bg`, `rate_plan.warn`, `rate_plan.wash`, `space.spacing_unit`, `space.unit`, `text_primary`, `typography.body`, `typography.caption`, `typography.metadata`

## Dependencies

[`components/input`](../components/input.md), [`components/number_input`](../components/number_input.md), [`components/time_input`](../components/time_input.md)
