# FanOutGrid

`patterns/fan_out_grid` · export `FanOutGrid` · version 0.1.0. Generated from
[`registry/patterns/fan_out_grid.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/fan_out_grid.rhai); do not edit.

Inspect a dense field of tasks and propose retrying the failed subset.

```rhai
import "patterns/fan_out_grid" as fan_out_grid;

fan_out_grid::FanOutGrid(#{key:"workers",run:"batch-4",tasks:[#{id:"docs",label:"Index docs",status:"running"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `currency_prefix` | string or `()` | — | Visible currency prefix; defaults to $. |
| `icon` | node or `()` | — | Optional decorative title icon. |
| `job` | string or `()` | — | Description shared by the tasks. |
| `key` | string | required | Stable instance key. |
| `now_ms` | integer 0–1000000000000 | `0` | Host-owned elapsed time; no script timer is started. |
| `on_retry` | callback or `()` | — | If supplied, enables Retry failed when the snapshot contains errors. |
| `on_select` | callback or `()` | — | Callback for pin and unpin proposals. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pinned` | string or `()` | — | Controlled inspection ID; empty unpins. Omit for a locally controlled pin. |
| `revision` | integer ≥ 0 | `0` | Accepted Host revision echoed in proposals. |
| `run` | string | required | Job identity. A new run clears local attention and selection. |
| `style` | style | — | Style merged over the root part. |
| `tasks` | array of object (at most 1024) | required | Up to 1,024 tasks, in visual order. |
| `title` | string or `()` | — | Defaults to Fan-out. |
| `workers` | integer 1–1024 or `()` | — | Optional concurrency for the ETA. Defaults to at least one active worker. |

### `tasks[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `attempts` | integer 1–1000000 or `()` | — | Number of tries already made. |
| `cost_micros` | integer -1000000000000–1000000000000 or `()` | — | Signed exact currency micro-units. |
| `end_ms` | integer 0–1000000000000 or `()` | — | Accepted finish time, no earlier than start_ms. |
| `error` | string or `()` | — | Error detail when failed or retrying. |
| `id` | string | required | Stable, unique task ID. |
| `label` | string | required | Task name. |
| `result` | string or `()` | — | Result detail when done. |
| `start_ms` | integer 0–1000000000000 or `()` | — | Relative to the job start. |
| `status` | `"queued"` or `"running"` or `"retrying"` or `"done"` or `"failed"` | required | Accepted task status. |
| `worker` | string or `()` | — | Worker name or label. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `retry` | `on_retry` | object | Propose retrying exactly the failed task IDs in snapshot order. |
| `select` | `on_select` | object | Select an exact business ID, or an empty string to clear it. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `body`, `body_compact`, `cell`, `cell_attention`, `cell_compact`, `cell_dim`, `cell_pinned`, `empty`, `field`, `filter`, `filter_count`, `filter_dim`, `filter_empty`, `filter_on`, `footer`, `header`, `job`, `legend`, `legend_compact`, `mark`, `metric`, `panel`, `panel_compact`, `retry`, `root`, `sheen`, `summary`, `summary_compact`, `swatch`, `terminal`, `tip`, `tip_error`, `tip_meta`, `tip_result`, `tip_status`, `tip_title`, `title`.

## Theme

- Tokens: `border`, `fan_out_grid.done_bg`, `fan_out_grid.done_edge`, `fan_out_grid.done_ink`, `fan_out_grid.failed_bg`, `fan_out_grid.failed_edge`, `fan_out_grid.failed_ink`, `fan_out_grid.ink`, `fan_out_grid.mark`, `fan_out_grid.muted`, `fan_out_grid.pin`, `fan_out_grid.queued_bg`, `fan_out_grid.queued_edge`, `fan_out_grid.queued_ink`, `fan_out_grid.retrying_bg`, `fan_out_grid.retrying_edge`, `fan_out_grid.retrying_ink`, `fan_out_grid.running_bg`, `fan_out_grid.running_edge`, `fan_out_grid.running_ink`, `fan_out_grid.shine`, `fan_out_grid.tray`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.metadata`
