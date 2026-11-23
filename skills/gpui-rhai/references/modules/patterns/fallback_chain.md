# FallbackChain

`patterns/fallback_chain` · export `FallbackChain` · version 0.1.0. Generated from
[`registry/patterns/fallback_chain.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/fallback_chain.rhai); do not edit.

Inspect attempts in a host-owned fallback chain, without making requests.

```rhai
import "patterns/fallback_chain" as fallback_chain;

fallback_chain::FallbackChain(#{key:"route",run:"sample",now_ms:5200,attempts:[#{id:"mini",model:"Atlas Mini",status:"trying",started_ms:0}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `attempts` | array of object (at most 24) | required | Attempts in the order in which they appear. |
| `currency` | string or `()` | — | Currency prefix, defaults to $. |
| `key` | string | required | Stable instance key. |
| `now_ms` | integer 0–1000000000000 | `0` | Host-owned elapsed time. No timer or status transition is inferred from the wall clock. |
| `on_retry` | callback or `()` | — | Enables a Retry proposal on a terminal all-failed chain. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `request` | object or `()` | — | Optional request details. |
| `response` | object or `()` | — | Optional response details. |
| `revision` | integer ≥ 0 | `0` | Accepted Host snapshot revision. |
| `route` | string or `()` | — | Route name on the request node. |
| `run` | string | required | Business identity of the chain. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Text above the route; defaults to Request. |

### `attempts[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `cost_micros` | integer -1000000000000–1000000000000 or `()` | — | Signed accepted charge in currency micro-units. |
| `details` | array of object (at most 16) | `[]` | Optional preview details. |
| `duration_ms` | integer 0–1000000000000 or `()` | — | Recorded time for an answered or failed attempt. |
| `id` | string | required | Stable, unique attempt ID. |
| `logo` | node or `()` | — | Decorative mark, otherwise initials. |
| `model` | string | required | Model or route name. |
| `note` | string or `()` | — | Short note on the answered card. |
| `provider` | string or `()` | — | Provider shown on wider cards and in details. |
| `provider_logo` | node or `()` | — | Decorative mark for the provider preview. |
| `reason` | string or `()` | — | Short failure reason. |
| `region` | string or `()` | — | Optional region for the preview. |
| `started_ms` | integer 0–1000000000000 or `()` | — | Trying attempt start on the same clock as now_ms. |
| `status` | `"waiting"` or `"trying"` or `"failed"` or `"answered"` or `"skipped"` | `"waiting"` | Host-owned state. At most one current attempt or answer. |
| `timeout_ms` | integer 1–1000000000000 | `10000` | Positive timeout scale. Reaching it fills the bar but does not fail a task. |

### `attempts[].details[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `label` | string | required | Detail field label. |
| `value` | string or node | required | Plain text or a rich SDK node. |

### `request` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `details` | array of object (at most 16) | `[]` | Optional preview details. |
| `logo` | node or `()` | — | Decorative endpoint mark. |
| `name` | string or `()` | — | Endpoint name. |
| `text` | string or `()` | — | Preview of the request text. |

### `request.details[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `label` | string | required | Detail field label. |
| `value` | string or node | required | Plain text or a rich SDK node. |

### `response` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `details` | array of object (at most 16) | `[]` | Optional preview details. |
| `logo` | node or `()` | — | Decorative endpoint mark. |
| `name` | string or `()` | — | Endpoint name. |
| `text` | string or `()` | — | Preview of the response text. |

### `response.details[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `label` | string | required | Detail field label. |
| `value` | string or node | required | Plain text or a rich SDK node. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `retry` | `on_retry` | object | Requests a retry of the accepted failed chain without changing its status. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `attempt`, `attempt_answered`, `attempt_skipped`, `attempt_status`, `attempt_trying`, `attempts`, `attempts_tree`, `badge`, `block`, `blocks`, `cost`, `detail_label`, `detail_rows`, `detail_value`, `dot`, `empty`, `end_answered`, `end_failed`, `end_wait`, `endpoint`, `endpoint_icon`, `flow`, `grid`, `head`, `logo`, `model`, `node_sub`, `node_title`, `number`, `popup`, `popup_sub`, `popup_title`, `provider`, `pulse`, `quote`, `retry`, `root`, `route`, `state`, `sum`, `timing`, `title`, `totals`, `tree`, `wires`.

## Theme

- Tokens: `border`, `fallback_chain.active`, `fallback_chain.active_edge`, `fallback_chain.active_fill`, `fallback_chain.area`, `fallback_chain.card`, `fallback_chain.error`, `fallback_chain.error_edge`, `fallback_chain.error_fill`, `fallback_chain.fill`, `fallback_chain.grid`, `fallback_chain.idle`, `fallback_chain.ink`, `fallback_chain.muted`, `fallback_chain.ok`, `fallback_chain.ok_edge`, `fallback_chain.ok_fill`, `fallback_chain.popover`, `fallback_chain.quiet`, `fallback_chain.rail`, `fallback_chain.text`, `fallback_chain.tray`, `fallback_chain.wait`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.initials`, `typography.metadata`

## Dependencies

[`components/hover_card`](../components/hover_card.md)
