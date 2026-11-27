# AgentQueue

`patterns/agent_queue` · export `AgentQueue` · version 0.1.0. Generated from
[`registry/patterns/agent_queue.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/agent_queue.rhai); do not edit.

Worker dispatch, interactive job inspection and queue depth on a shared Host clock. State: locally pinned job and keyboard cursor only. Job actions are proposals.

```rhai
import "patterns/agent_queue" as agent_queue;

agent_queue::AgentQueue(#{key:"pool",queue:"pool",revision:0,now_ms:0,jobs:[],workers:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `allow_cancel` | bool | `false` | Allow waiting/running jobs to propose cancellation. |
| `allow_promote` | bool | `false` | Allow a waiting job to propose moving to the front. |
| `allow_retry` | bool | `false` | Allow a failed job to propose retrying if attempts remain. |
| `backpressure` | integer 0–1000000 or `()` | — | Optional line; the Waiting statistic is marked only when depth is above it. |
| `busy` | bool | `false` | Temporarily disables commands, not inspection. |
| `default_open_id` | string or `()` | — | Initially inspected job in local mode. |
| `jobs` | array of object (at most 128) | required | Live jobs and retained history. Retain jobs overlapping the chart window to reconstruct complete depth. |
| `key` | string | required | Stable instance key. |
| `max_workers` | integer 0–32 | `16` | Highest proposed concurrency. |
| `min_workers` | integer 0–32 | `1` | Lowest proposed concurrency. |
| `name` | string or `()` | — | Optional queue or provider name. |
| `notice` | string or `()` | — | Host feedback shown without changing accepted state. |
| `now_ms` | integer ≥ 0 | required | Host clock shared with job and rate-limit time stamps. |
| `on_action` | callback or `()` | — | Receives a revisioned proposal. The component does not run jobs. |
| `on_open_change` | callback or `()` | — | Inspection change; in controlled mode the Host accepts it by updating open_id. |
| `open_id` | string or `()` | — | Controlled inspection. Pass an empty string for closed, or omit for local inspection. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `paused` | bool | `false` | Accepted pause; does not change running jobs or manufacture a rate limit. |
| `queue` | string | required | Business queue identifier, included in proposals. |
| `rate_limits` | array of object (at most 32) or `()` | — | Throttle history; overlaps are painted once and subtracted once from estimated progress. |
| `revision` | integer ≥ 0 | required | Accepted queue revision. Advance it on business changes, not clock ticks. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Optional heading. |
| `totals` | object or `()` | — | Omit to count the retained terminal records instead. |
| `window_ms` | integer 1000–604800000 | `120000` | Width of the depth chart's time window. |
| `workers` | array of object (at most 32) | required | Accepted slots in presentation order. A running slot can hold at most one job. |

### `jobs[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `attempt` | integer 1–999 or `()` | — | Attempt number, initially one. |
| `detail` | string or `()` | — | Job evidence or a short explanation. |
| `ended_ms` | integer ≥ 0 or `()` | — | Required for a terminal job; absent for active jobs. |
| `enqueued_ms` | integer ≥ 0 | required | When this attempt entered the queue. |
| `error` | string or `()` | — | Reported failure, shown within inspection. |
| `estimate_ms` | integer ≥ 1 or `()` | — | Estimate used when progress is absent. Finished runs do not extend the waiting interval. |
| `id` | string | required | Unique stable job ID. |
| `kind` | `"llm"` or `"tool"` or `"agent"` or `"human"` | required | Type of work, used for labels and the job mark. |
| `max_attempts` | integer 1–999 or `()` | — | Maximum attempts. Disables Retry once exhausted. |
| `name` | string | required | Job caption. |
| `priority` | `"high"` or `"normal"` or `"low"` or `()` | — | Queue priority; ties are chronological, then in caller order. |
| `priority_order` | integer or `()` | — | Optional explicit order within a priority, before unranked jobs. Does not rewrite enqueue time. |
| `progress` | number 0–1 or `()` | — | Authoritative running progress. |
| `started_ms` | integer ≥ 0 or `()` | — | Optional start time; required for running jobs, absent for queued jobs. |
| `status` | `"queued"` or `"running"` or `"done"` or `"failed"` or `"cancelled"` | required | Accepted job state. |
| `worker` | string or `()` | — | Running job's accepted slot. Omit if it is running outside the declared slots. |

### `rate_limits[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `from` | integer ≥ 0 | required | Inclusive rate limit start. |
| `label` | string or `()` | — | Optional reason/provider detail. |
| `until_ms` | integer ≥ 1 | required | Exclusive rate limit end, after from. |

### `totals` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `done` | integer ≥ 0 | — | Accepted lifetime success count. |
| `failed` | integer ≥ 0 | — | Accepted lifetime failure count. |

### `workers[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `detail` | string or `()` | — | Optional caption below the slot name. |
| `id` | string | required | Stable and unique slot identity. |
| `name` | string | required | Slot name. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | A proposal does not change queue state. Check the exact revision before accepting. |
| `open_change` | `on_open_change` | object | Inspection proposal. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `axis`, `bin`, `bin_failed`, `bins`, `card`, `chart`, `chart_title`, `controls`, `depth_fill`, `detail`, `detail_action`, `detail_meta`, `detail_title`, `empty`, `error`, `flow`, `head`, `job`, `job_name`, `job_open`, `job_side`, `mark`, `metadata`, `notice`, `plot`, `priority`, `progress`, `root`, `scale`, `section`, `section_count`, `stat`, `stat_label`, `stat_value`, `state`, `stats`, `title`, `waiting`, `worker`, `worker_detail`, `worker_group`, `worker_name`, `worker_state`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `selection`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text.warning`, `text_muted`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/progress`](../components/progress.md), [`components/scroll_area`](../components/scroll_area.md), [`patterns/approval_card`](approval_card.md)
