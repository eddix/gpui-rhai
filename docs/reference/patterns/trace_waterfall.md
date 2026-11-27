# TraceWaterfall

`patterns/trace_waterfall` · export `TraceWaterfall` · version 0.1.0. Generated from
[`registry/patterns/trace_waterfall.rhai`](../../../registry/patterns/trace_waterfall.rhai); do not edit.

Time-based execution tree with exact rollups, a shared clock and span details.

```rhai
import "patterns/trace_waterfall" as trace_waterfall;

trace_waterfall::TraceWaterfall(#{key:"run",session:"preview",spans:[#{id:"root",name:"Review",kind:"agent",status:"done",start_ms:0,end_ms:820}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `collapsed` | array of string (at most 128) or `()` | — | Controlled collapsed span IDs when supplied. |
| `currency` | string | `"USD"` | Currency code for integer cost_micros; not a conversion. |
| `default_collapsed` | array of string (at most 128) | `[]` | Initial set of folded spans when disclosure is local. |
| `default_open_id` | string or `()` | — | Initial inspected span when inspector selection is local. |
| `key` | string | required | Stable identity of this instance among its siblings; keeps its state across renders. |
| `mark_slowest` | bool | `true` | Marks the longest finished model or tool call even when folding changes visible rows. |
| `max_height` | integer 96–1800 | `460` | Scrollable tree viewport height cap in logical pixels. |
| `min_range_ms` | integer 1000–31536000000 | `10000` | Minimum displayed elapsed-time range; longer spans can extend it. |
| `now_ms` | integer 0–31536000000 or `()` | — | Deterministic supplied elapsed time; takes precedence over started_at_ms. |
| `on_collapsed_change` | callback or `()` | — | Receives the requested fold plus the complete proposed collapsed ID list and run identity. |
| `on_open_change` | callback or `()` | — | Receives a span ID or null with the current session and revision; private span text is omitted. |
| `open_id` | string or `()` | — | Controlled inspector ID. An explicit null closes it. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | `0` | Accepted Host revision carried in interaction events. |
| `session` | string | required | Identity of this run; changing it resets local folding and selection. |
| `spans` | array of object (at most 128) | required | Bounded flat tree. Parent may occur before or after its children. Siblings retain input order. |
| `started_at_ms` | integer 0–9007199254740991 or `()` | — | Epoch time for the wall-clock live mode. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Overrides the trace heading; it does not name or identify a span. |

### `spans[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `cost_micros` | integer 0–1000000000000 or `()` | — | This span's own cost; descendants are added to their parents. |
| `detail` | string or `()` | — | Optional private explanatory text for this span. |
| `end_ms` | integer 0–31536000000 or `()` | — | Elapsed milliseconds at completion. Missing running ends follow the clock. |
| `error` | string or `()` | — | Optional private failure detail; the status field determines whether the span is failed. |
| `icon` | string or `()` | — | Short custom mark replacing the kind swatch. |
| `id` | string | required | Unique span ID, stable across timing updates, folds and inspection. |
| `input` | string or `()` | — | Optional input text displayed only for inspection, not replay. |
| `kind` | `"agent"` or `"subagent"` or `"llm"` or `"tool"` | required | Span category; model and tool calls contribute to call and slowest-call reporting. |
| `model` | string or `()` | — | Optional model label shown with the span metrics. |
| `name` | string | required | Visible span name in the tree and inspector. |
| `output` | string or `()` | — | Optional read-only output text supplied by the Host. |
| `parent` | string or `()` | — | Parent ID; exactly one span has no parent. |
| `start_ms` | integer 0–31536000000 | required | Elapsed milliseconds since the run began. |
| `status` | `"running"` or `"done"` or `"failed"` | required | Host-reported lifecycle status of this span. |
| `tokens_in` | integer 0–1000000000000 or `()` | — | Input tokens attributable to this span itself; do not include its descendants here. |
| `tokens_out` | integer 0–1000000000000 or `()` | — | Output tokens attributable to this span itself; do not include its descendants here. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `collapsed_change` | `on_collapsed_change` | object | Disclosure proposal with the new complete collapsed ID list. |
| `open_change` | `on_open_change` | object | Requested span ID or null; never includes private span text. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `axis`, `axis_caption`, `axis_label`, `bar`, `bar_narrow`, `card`, `chevron`, `count`, `detail_close`, `detail_error`, `detail_head`, `detail_stat_label`, `detail_stat_value`, `detail_stats`, `detail_subtitle`, `detail_title`, `details`, `document`, `documents`, `empty`, `error_glyph`, `fragment`, `grid`, `guide`, `guides`, `head`, `head_done`, `head_error`, `head_live`, `head_status`, `indicator`, `lane`, `mark`, `name`, `names`, `now`, `now_dot`, `readout`, `readout_tag`, `root`, `row`, `row_open`, `rows`, `rule`, `shine`, `slow`, `span_label_flow`, `spark`, `stat`, `stat_error`, `stat_label`, `stat_label_tiny`, `stat_rule`, `stat_value`, `stat_value_narrow`, `stats`, `stats_narrow`, `status`, `subtitle`, `task`, `tick`, `tile`, `title`.

## Theme

- Tokens: `design.name`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `text_primary`, `trace.accent`, `trace.agent_bg`, `trace.agent_edge`, `trace.agent_ink`, `trace.agent_mark`, `trace.card`, `trace.done_bg`, `trace.done_ink`, `trace.error_bg`, `trace.error_edge`, `trace.error_ink`, `trace.error_mark`, `trace.grid`, `trace.guide`, `trace.heading_case`, `trace.hover`, `trace.live_bg`, `trace.live_ink`, `trace.llm_bg`, `trace.llm_edge`, `trace.llm_ink`, `trace.llm_mark`, `trace.muted`, `trace.now`, `trace.selected`, `trace.slow_bg`, `trace.slow_ink`, `trace.text`, `trace.tool_bg`, `trace.tool_edge`, `trace.tool_ink`, `trace.tool_mark`, `trace.tray`, `trace.well`, `typography.body`, `typography.caption`, `typography.code`, `typography.label`

## Dependencies

[`components/icon_button`](../components/icon_button.md), [`components/spinner`](../components/spinner.md)
