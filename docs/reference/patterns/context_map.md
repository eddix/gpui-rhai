# ContextMap

`patterns/context_map` · export `ContextMap` · version 0.1.0. Generated from
[`registry/patterns/context_map.rhai`](../../../registry/patterns/context_map.rhai); do not edit.

Inspect the area occupied by ordered context blocks, including a prior compaction.

```rhai
import "patterns/context_map" as context_map;

context_map::ContextMap(#{key:"map",context:"session",revision:0,limit:100000,blocks:[#{id:"chat",label:"Conversation",kind:"chat",tokens:26000}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string or `()` | — | Accepted agent whose snapshot is supplied. |
| `agents` | array of object (at most 32) or `()` | — | Host-owned agents. |
| `before` | array of object (at most 64) or `()` | — | Optional prior accepted snapshot, supplied with compaction. |
| `blocks` | array of object (at most 64) | required | Ordered accepted blocks. Zero counts do not occupy space. |
| `compaction` | object or `()` | — | Optional explanation of the previous compaction. |
| `compare` | `"before"` or `"after"` or `()` | — | Controlled side. If omitted, comparison is local to the receipt. |
| `context` | string | required | Identity of the accepted context. |
| `key` | string | required | Stable view instance. |
| `limit` | integer 0–9007199254740991 | required | Token allowance when no agent is selected; zero is unknown. |
| `model` | string or `()` | — | Model caption used without an active agent. |
| `model_icon` | node or `()` | — | Decorative model mark. |
| `on_agent_change` | callback or `()` | — | Request a different agent; the Host owns the accepted data. |
| `on_compare` | callback or `()` | — | Receives a comparison request. |
| `on_select` | callback or `()` | — | Receives an inspected block ID, or an empty string for unused capacity. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Accepted revision for Host requests. |
| `show_comparison` | bool | `false` | Reserve a status row even when no receipt exists. |
| `status` | string or `()` | — | Short activity caption, such as Compacting. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Card title. |

### `agents[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Agent identity. |
| `limit` | integer 0–9007199254740991 | required | Agent allowance; zero is unknown. |
| `model` | string or `()` | — | Model caption. |
| `name` | string | required | Display name. |

### `before[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `detail` | string or `()` | — | Inspection caption. |
| `icon` | node or `()` | — | Optional decorative tile mark. |
| `id` | string | required | Stable nonempty business ID. |
| `items` | array of string (at most 64) or `()` | — | App-owned details. |
| `kind` | `"system"` or `"tools"` or `"memory"` or `"file"` or `"chat"` or `"result"` or `"summary"` | required | Semantic palette category. |
| `label` | string | required | Human-readable name. |
| `tokens` | integer 0–9007199254740991 | required | Accepted nonnegative count. |

### `blocks[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `detail` | string or `()` | — | Inspection caption. |
| `icon` | node or `()` | — | Optional decorative tile mark. |
| `id` | string | required | Stable nonempty business ID. |
| `items` | array of string (at most 64) or `()` | — | App-owned details, summarized after twelve entries. |
| `kind` | `"system"` or `"tools"` or `"memory"` or `"file"` or `"chat"` or `"result"` or `"summary"` | required | Semantic palette category. |
| `label` | string | required | Human-readable name. |
| `tokens` | integer 0–9007199254740991 | required | Accepted nonnegative count. |

### `compaction` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `dropped` | array of string (at most 64) or `()` | — | Before IDs omitted. |
| `id` | string | required | Stable compaction receipt identity. |
| `summarized` | array of string (at most 64) or `()` | — | Before IDs folded into summary. |
| `summary` | string or `()` | — | Accepted after block containing the summary. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `agent_change` | `on_agent_change` | object | Apply the agent, blocks and allowance together to accept. |
| `compare` | `on_compare` | object | Requested side of an accepted compaction receipt. |
| `select` | `on_select` | object | Inspection notice. Does not mutate business state. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `compare`, `compare_current`, `empty`, `fate`, `fate_empty`, `field`, `footer`, `free`, `full`, `header`, `mark`, `model`, `number`, `percent`, `root`, `status`, `summary`, `swatch`, `tile`, `tile_attention`, `tile_count`, `tile_label`, `tile_measures`, `tile_name`, `tile_pinned`, `tip_detail`, `tip_fate`, `tip_item`, `tip_items`, `tip_meta`, `tip_title`, `title`, `tooltip`, `used`, `warn`.

## Theme

- Tokens: `accent`, `border`, `context_map.chat_bg`, `context_map.chat_edge`, `context_map.chat_ink`, `context_map.file_bg`, `context_map.file_edge`, `context_map.file_ink`, `context_map.free_edge`, `context_map.free_hatch`, `context_map.memory_bg`, `context_map.memory_edge`, `context_map.memory_ink`, `context_map.model`, `context_map.muted`, `context_map.result_bg`, `context_map.result_edge`, `context_map.result_ink`, `context_map.shine`, `context_map.summary_bg`, `context_map.summary_edge`, `context_map.summary_ink`, `context_map.system_bg`, `context_map.system_edge`, `context_map.system_ink`, `context_map.tools_bg`, `context_map.tools_edge`, `context_map.tools_ink`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/button`](../components/button.md), [`components/select`](../components/select.md), [`components/spinner`](../components/spinner.md), [`patterns/agent_tree`](../patterns/agent_tree.md)
