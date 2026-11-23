# UsageMeter

`patterns/usage_meter` · export `UsageMeter` · version 0.1.0. Generated from
[`registry/patterns/usage_meter.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/usage_meter.rhai); do not edit.

Present accepted usage, period history and capped or unlimited plan limits.

```rhai
import "patterns/usage_meter" as usage_meter;

usage_meter::UsageMeter(#{key:"usage",series:"account",revision:0,total:1000,used:[#{id:"api",label:"API",value:240}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `busy` | bool | `false` | Suppress repeated proposals without changing the selected period. |
| `daily` | array of object (at most 120) or `()` | — | Chronological sparkline data. No data is fetched by this component. |
| `key` | string | required | Stable component key. |
| `limits` | array of object (at most 12) or `()` | — | Additional capped or unlimited resources, in caller order. |
| `now_ms` | integer 0–253402300799999 or `()` | — | Inject an accepted clock for reproducible rendering. Otherwise the view clock refreshes hourly when a reset exists. |
| `on_action` | callback or `()` | — | Receives a versioned upgrade or top-up request; no purchase is initiated by this component. |
| `on_period_change` | callback or `()` | — | Receives a period proposal; the application accepts or refuses it. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `period` | string or `()` | — | Current accepted period; required with period choices. |
| `periods` | array of object (at most 6) or `()` | — | Optional controlled period choices. |
| `plan` | string or `()` | — | Optional plan name near the reset date. |
| `reset_at_ms` | integer 0–253402300799999 or `()` | — | Reset instant in Unix milliseconds. |
| `revision` | integer ≥ 0 | required | Revision of the accepted usage snapshot. |
| `series` | string | required | Identity of the account or usage series. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Heading. Defaults to Usage. |
| `top_up_label` | string or `()` | — | Action label near and at the limit. |
| `total` | number 0–1000000000000 | required | Accepted finite nonnegative capacity. Usage above zero with zero capacity is over the limit. |
| `unit` | string or `()` | — | Unit of the main quota. Defaults to credits. |
| `upgrade_label` | string or `()` | — | Action label below the warning threshold. |
| `used` | array of object (at most 32) | required | At most 32 usage categories. The Host accepts a complete snapshot, never partial period previews. |
| `warn_at` | number 0.01–1 | `0.8` | Warn at or above this fraction of quota, below the limit. |

### `daily[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Unique stable day identity, not a display label. |
| `label` | string | required | Day label for hover, keyboard focus and accessibility. |
| `value` | number 0–1000000000000 or `()` | — | Omit for a future day. A present zero is an observed day. |

### `limits[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `decimals` | integer 0–6 or `()` | — | Maximum displayed fraction digits. Defaults to 1 for fractional amounts. |
| `id` | string | required | Unique quota key. |
| `label` | string | required | Quota label. |
| `total` | number 0–1000000000000 or `()` | — | Omit for an unlimited resource; zero is a real finite capacity. |
| `unit` | string or `()` | — | Optional quota-specific unit. |
| `used` | number 0–1000000000000 | required | Accepted usage for this limit. |

### `periods[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Unique period ID. |
| `label` | string | required | Visible period label. |

### `used[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `color` | integer 0–16777215 or `()` | — | Custom RGB24 paint. Defaults to a stable palette slot. |
| `id` | string | required | Stable category identity across periods. |
| `label` | string | required | Visible category label. |
| `value` | number 0–1000000000000 | required | Accepted amount from this category. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | An explicit upgrade or top-up request, not a purchase or quota mutation. |
| `period_change` | `on_period_change` | object | Ask the Host for a different period. Keep all prior values until the Host publishes the new snapshot. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `action_strong`, `amount`, `bar`, `daily`, `daily_header`, `day_active`, `day_fill`, `day_future`, `dot`, `dot_limit`, `empty`, `figure`, `footer`, `footer_cozy`, `head`, `head_narrow`, `hint`, `hint_amount`, `hint_name`, `hint_unit`, `legend`, `legend_label`, `limit`, `limit_fill`, `limit_full`, `limit_head`, `limit_readout`, `limit_readout_cozy`, `limit_track`, `limit_unlimited`, `limit_value`, `limit_warn`, `limits`, `muted`, `panel`, `panel_cozy`, `pct`, `pct_limit`, `pct_value`, `pct_warn`, `period`, `period_active`, `period_fill`, `period_tight`, `periods`, `periods_cozy`, `plan`, `readout`, `root`, `row`, `row_active`, `segment`, `segment_cut`, `segment_start`, `share`, `spark`, `spark_dense`, `spark_few`, `status`, `strong`, `swatch`, `title`, `total`.

## Theme

- Tokens: `accent`, `border`, `compact_breakdown.c1`, `compact_breakdown.c2`, `compact_breakdown.c3`, `compact_breakdown.c4`, `compact_breakdown.c5`, `focus_ring`, `radius.lg`, `radius.pill`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.metadata`, `usage_meter.accent`, `usage_meter.fill`, `usage_meter.full`, `usage_meter.full_text`, `usage_meter.spark`, `usage_meter.track`, `usage_meter.warn`, `usage_meter.warn_text`

## Dependencies

[`components/button`](../components/button.md), [`patterns/agent_tree`](agent_tree.md), [`patterns/compact_breakdown`](compact_breakdown.md)
