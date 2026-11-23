# ParallelAttempts

`patterns/parallel_attempts` · export `ParallelAttempts` · version 0.1.0. Generated from
[`registry/patterns/parallel_attempts.rhai`](../../../registry/patterns/parallel_attempts.rhai); do not edit.

Compare independent attempts without letting a visual winner change Host state.

```rhai
import "patterns/parallel_attempts" as parallel_attempts;

parallel_attempts::ParallelAttempts(#{key:"race",run:"trial-7",attempts:[#{id:"test",name:"Test",status:"done",score:92}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `attempts` | array of object (at most 24) | required | Attempts in visual and stable tie-break order. |
| `currency_prefix` | string or `()` | — | Visible prefix; defaults to $. |
| `key` | string | required | Stable card key. |
| `on_pick` | callback or `()` | — | Selection notification or proposal. This never runs an attempt. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `picked` | string or `()` | — | Controlled accepted ID; empty string explicitly selects none. Omit for local selection and automatic best after settlement. |
| `revision` | integer ≥ 0 | `0` | Snapshot revision echoed by selection proposals. |
| `run` | string | required | Attempt set identity. A new run clears any previous local pick. |
| `style` | style | — | Style merged over the root part. |
| `task` | string or `()` | — | Shared task description. |
| `title` | string or `()` | — | Defaults to Best of N. |

### `attempts[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `approach` | string or `()` | — | Optional method or strategy. |
| `cost_micros` | integer -1000000000000–1000000000000 or `()` | — | Signed integer micro-units of currency; missing has no row cost label. |
| `duration_ms` | integer 0–1000000000000 or `()` | — | Elapsed milliseconds supplied by the Host. |
| `icon` | node or `()` | — | Decorative node constrained to the model tile. |
| `id` | string | required | Stable ID unique within the run. |
| `model` | string or `()` | — | Name beside the attempt. |
| `name` | string | required | Short visible attempt name. |
| `note` | string or `()` | — | Outcome once settled. |
| `progress` | number 0–1 or `()` | — | Accepted fraction; failed keeps its last supplied fraction. |
| `score` | number 0–100 or `()` | — | Finished result score. The first done attempt wins a tie. |
| `status` | `"running"` or `"done"` or `"failed"` | required | Settled includes both done and failed. |
| `step` | string or `()` | — | Current step while running. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `pick` | `on_pick` | object | Requests use of a settled successful attempt by its business ID. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `approach`, `best`, `card`, `card_narrow`, `empty`, `end`, `end_narrow`, `fill`, `glow`, `header`, `header_narrow`, `icon`, `line`, `line_narrow`, `mark`, `meta`, `meta_slot`, `model`, `name`, `panel`, `pick`, `pick_on`, `picked`, `quiet`, `root`, `score`, `score_failed`, `score_running`, `sheen`, `stats`, `stats_value`, `summary`, `summary_narrow`, `task`, `title`, `track`, `track_narrow`, `who`.

## Theme

- Tokens: `accent`, `border`, `design.name`, `focus_ring`, `parallel_attempts.action_hover`, `parallel_attempts.best_bg`, `parallel_attempts.best_edge`, `parallel_attempts.done_bg`, `parallel_attempts.done_edge`, `parallel_attempts.done_ink`, `parallel_attempts.failed_bg`, `parallel_attempts.failed_edge`, `parallel_attempts.failed_ink`, `parallel_attempts.icon`, `parallel_attempts.ink`, `parallel_attempts.mark`, `parallel_attempts.muted`, `parallel_attempts.running_bg`, `parallel_attempts.running_edge`, `parallel_attempts.running_ink`, `parallel_attempts.shine`, `parallel_attempts.strong`, `parallel_attempts.text`, `parallel_attempts.track`, `parallel_attempts.tray`, `parallel_attempts.wash`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.initials`, `typography.metadata`
