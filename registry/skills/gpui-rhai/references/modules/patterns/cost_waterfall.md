# CostWaterfall

`patterns/cost_waterfall` · export `CostWaterfall` · version 0.1.0. Generated from
[`registry/patterns/cost_waterfall.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/cost_waterfall.rhai); do not edit.

Show a run ledger, with exact monetary rollups and independently controlled view state.

```rhai
import "patterns/cost_waterfall" as cost_waterfall;

cost_waterfall::CostWaterfall(#{key:"run",run:"run-1",steps:[#{id:"chat",name:"Review",kind:"llm",status:"done",cost_micros:42000,tokens_in:1600,tokens_out:370}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `collapsed` | array of string (at most 192) or `()` | — | Controlled set of folded IDs. Omit for local folds. |
| `currency_prefix` | string or `()` | — | Monetary prefix for integer micro-units; defaults to $. No currency conversion is performed. |
| `default_collapsed` | array of string (at most 192) or `()` | — | Initial uncontrolled folded steps. |
| `default_mode` | `"cost"` or `"tokens"` | `"cost"` | Initial uncontrolled measure. |
| `default_open_id` | string or `()` | — | Initial uncontrolled detail ID. |
| `key` | string | required | Stable card instance. |
| `max_height` | integer 96–1800 or `()` | — | Maximum height of the scrollable ledger, excluding its fixed headers. |
| `mode` | `"cost"` or `"tokens"` or `()` | — | Controlled measure. Omit to let the view own it. |
| `on_fold_change` | callback or `()` | — | Receives the proposed folded IDs. |
| `on_mode_change` | callback or `()` | — | Receives a measure proposal. |
| `on_open_change` | callback or `()` | — | Receives the proposed detail selection. |
| `open_id` | string or `()` | — | Controlled detail ID. An empty string means closed; omit for local selection. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `run` | string | required | Ledger identity. Changing it clears local view state. |
| `status` | `"running"` or `"done"` or `"failed"` or `()` | — | Optional Host run state; otherwise derived from the steps. |
| `steps` | array of object (at most 192) | required | A bounded flat tree; each step contributes its own amounts exactly once. |
| `style` | style | — | Style merged over the root part. |
| `task` | string or `()` | — | Optional caption next to the heading. |
| `title` | string or `()` | — | Heading, defaults to Run cost. |

### `steps[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `cost_micros` | integer -1000000000000–1000000000000 or `()` | — | Own cost (negative for credit), in millionths of the displayed unit. Do not pre-sum descendants. |
| `detail` | string or `()` | — | Secondary caption when there is no model. |
| `duration_ms` | integer 0–604800000 or `()` | — | Host-reported duration in milliseconds. |
| `error` | string or `()` | — | Optional literal failure explanation. |
| `icon` | node or `()` | — | Optional decorative mark. |
| `id` | string | required | Nonempty stable step ID, unique across the run. |
| `input` | string or `()` | — | Optional literal input excerpt. |
| `kind` | `"agent"` or `"llm"` or `"tool"` | required | Step class used for color and details. |
| `model` | string or `()` | — | Model name. |
| `name` | string | required | Step label. |
| `output` | string or `()` | — | Optional literal output excerpt. |
| `parent` | string or `()` | — | Parent ID; omit for a root. Siblings retain their order. |
| `status` | `"running"` or `"done"` or `"failed"` | required | Host-reported step state. |
| `tokens_in` | integer 0–1000000000000 or `()` | — | Own input tokens, excluding descendants. |
| `tokens_out` | integer 0–1000000000000 or `()` | — | Own output tokens, excluding descendants. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `fold_change` | `on_fold_change` | object | Proposes folding or unfolding. |
| `mode_change` | `on_mode_change` | object | Proposes the measure. A controlled card waits for Host props. |
| `open_change` | `on_open_change` | object | Proposes the active detail, or an empty ID to close it. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `amount`, `axis`, `bar`, `column_label`, `credit`, `detail`, `detail_header`, `empty`, `error`, `excerpt`, `flag`, `fold`, `glyph`, `head`, `header`, `heading`, `indent`, `ledger_columns`, `live`, `mark`, `metric`, `model`, `name`, `panel`, `root`, `row`, `row_open`, `rule`, `selected`, `shimmer`, `spark`, `stat`, `stat_label`, `stats`, `status`, `status_done`, `status_failed`, `status_pill`, `status_running`, `step_column`, `sub`, `switch`, `switch_button`, `task`, `tick`, `top`, `track`, `viewport`.

## Theme

- Tokens: `accent`, `border`, `cost_waterfall.agent_bg`, `cost_waterfall.agent_edge`, `cost_waterfall.agent_mark`, `cost_waterfall.agent_out`, `cost_waterfall.error_bg`, `cost_waterfall.error_edge`, `cost_waterfall.error_out`, `cost_waterfall.llm_bg`, `cost_waterfall.llm_edge`, `cost_waterfall.llm_mark`, `cost_waterfall.llm_out`, `cost_waterfall.muted`, `cost_waterfall.tool_bg`, `cost_waterfall.tool_edge`, `cost_waterfall.tool_mark`, `cost_waterfall.tool_out`, `cost_waterfall.tray`, `design.name`, `focus_ring`, `radius.lg`, `radius.pill`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.code`

## Dependencies

[`components/spinner`](../components/spinner.md)
