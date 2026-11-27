# CompactBreakdown

`patterns/compact_breakdown` · export `CompactBreakdown` · version 0.1.0. Generated from
[`registry/patterns/compact_breakdown.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/compact_breakdown.rhai); do not edit.

A total split into stable parts, with ranked detail and a controlled period.

```rhai
import "patterns/compact_breakdown" as compact_breakdown;

compact_breakdown::CompactBreakdown(#{key:"cost",series:"usage",revision:0,items:[#{id:"api",label:"API calls",value:84.0}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `busy` | bool | `false` | Ignore new period proposals while preserving keyboard focus. |
| `decimals` | integer 0–6 | `0` | Maximum fraction digits for amounts. |
| `delta` | number -1000000–1000000 or `()` | — | Signed percent change measured against the previous period. |
| `empty_label` | string or `()` | — | Description when no categories are available. |
| `footer` | node or `()` | — | Optional custom footer in place of the largest category. |
| `items` | array of object (at most 32) | required | Accepted categories, not the result of a pending period proposal. |
| `key` | string | required | Stable component key. |
| `on_period_change` | callback or `()` | — | Handle a period proposal without mutating the accepted snapshot in the component. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `period` | string or `()` | — | Host-accepted period, required when choices are provided. |
| `periods` | array of object (at most 6) or `()` | — | Optional period switch choices. |
| `prefix` | string | `""` | Optional number prefix, e.g. a currency symbol; at most 64 UTF-8 bytes. |
| `revision` | integer ≥ 0 | required | Revision of the accepted snapshot. |
| `series` | string | required | Stable category namespace. Change it when starting an unrelated breakdown. |
| `show_footer` | bool | `true` | Show the largest category or the supplied footer. |
| `style` | style | — | Style merged over the root part. |
| `suffix` | string | `""` | Optional unit suffix; at most 64 UTF-8 bytes. |
| `title` | string or `()` | — | Heading. Defaults to Breakdown. |
| `total_label` | string or `()` | — | Caption when no category is being inspected. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `color` | integer 0–16777215 or `()` | — | Optional RGB24 override for this category. |
| `id` | string | required | Unique category ID stable across accepted periods. |
| `label` | string | required | Human-readable category label. |
| `value` | number 0–1000000000000 | required | Finite nonnegative amount. |

### `periods[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Unique period ID. |
| `label` | string | required | Visible period label. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `period_change` | `on_period_change` | object | A proposal; accept by passing new period and data, or keep the original snapshot. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `amount`, `bar`, `caption`, `delta`, `delta_down`, `delta_up`, `dot`, `empty`, `figure`, `footer`, `footer_cozy`, `footer_text`, `head`, `head_narrow`, `list`, `name`, `panel`, `panel_cozy`, `period`, `period_active`, `period_fill`, `period_tight`, `periods`, `periods_cozy`, `root`, `row`, `row_active`, `segment`, `segment_end`, `segment_start`, `share`, `title`, `total`.

## Theme

- Tokens: `accent`, `border`, `compact_breakdown.c1`, `compact_breakdown.c2`, `compact_breakdown.c3`, `compact_breakdown.c4`, `compact_breakdown.c5`, `compact_breakdown.card`, `compact_breakdown.down`, `compact_breakdown.down_bg`, `compact_breakdown.up`, `compact_breakdown.up_bg`, `danger`, `design.name`, `focus_ring`, `radius.lg`, `radius.pill`, `radius.sm`, `space.spacing_unit`, `space.unit`, `success`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.metadata`

## Dependencies

[`patterns/agent_tree`](agent_tree.md)
