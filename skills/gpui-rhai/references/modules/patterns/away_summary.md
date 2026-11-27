# AwaySummary

`patterns/away_summary` · export `AwaySummary` · version 0.1.0. Generated from
[`registry/patterns/away_summary.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/away_summary.rhai); do not edit.

An inline recap of work that changed while someone was away. State: stable per-session ordering and disclosure/focus; the Host owns results and pending work.

```rhai
import "patterns/away_summary" as away_summary;

away_summary::AwaySummary(#{key:"away",session:"shift",revision:0,since_ms:0,now_ms:3600000,items:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `collapsed` | bool or `()` | — | Controlled fold state when present. |
| `default_collapsed` | bool | `false` | Initial local fold state. |
| `items` | array of object (at most 64) | required | Accepted updates; stable keys preserve their initial status group and arrival position. |
| `key` | string | required | Stable card key. |
| `limit` | integer 1–24 | `5` | Initially visible rows. |
| `now_ms` | integer 0–253402300799999 or `()` | — | Supply an accepted clock for deterministic snapshots. The view clock otherwise refreshes every 30 seconds. |
| `on_action` | callback or `()` | — | Receives a versioned row-action proposal. |
| `on_collapsed_change` | callback or `()` | — | Receives a disclosure proposal or notification. The Host may treat collapse as acknowledgement. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pending` | array of string (at most 64) or `()` | — | IDs awaiting a Host response; their actions cannot be pressed again while pending. |
| `revision` | integer ≥ 0 | required | Accepted updates revision. |
| `session` | string | required | New interval ID resets the preserved arrival order. |
| `since_ms` | integer 0–253402300799999 | required | When the person left, in Unix milliseconds. |
| `style` | style | — | Style merged over the root part. |
| `time_style` | `"duration"` or `"clock"` | `"duration"` | Render elapsed time or the departure clock. |
| `time_zone` | string | `"local"` | Zone used for clock display. |
| `title` | string or `()` | — | Visible summary heading. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `action_label` | string or `()` | — | Action override. Defaults to Decide, Retry or Open. |
| `detail` | string or `()` | — | One short line of accepted results. |
| `hide_action` | bool or `()` | — | Suppress this row's action. |
| `id` | string | required | Unique update ID. |
| `kind` | `"needs_you"` or `"failed"` or `"done"` | required | Accepted update status. |
| `title` | string | required | Task label. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Ask the application to act; the UI never invents results. |
| `collapsed_change` | `on_collapsed_change` | object | One versioned fold or acknowledgement operation. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `action_needs`, `card`, `count`, `counts`, `detail`, `divider`, `done`, `dot`, `empty`, `failed`, `foot`, `head`, `head_narrow`, `label`, `mark`, `more`, `needs_you`, `root`, `row`, `rows`, `rule`, `title`, `toggle`, `toggle_folded`, `when`.

## Theme

- Tokens: `away_summary.done`, `away_summary.failed`, `away_summary.needs`, `border`, `focus_ring`, `radius.lg`, `radius.pill`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/button`](../components/button.md), [`patterns/agent_tree`](agent_tree.md)
