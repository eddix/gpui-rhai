# AgentGraph

`patterns/agent_graph` · export `AgentGraph` · version 0.1.0. Generated from
[`registry/patterns/agent_graph.rhai`](../../../registry/patterns/agent_graph.rhai); do not edit.

Requests and results travel across the accepted run's communication graph.

```rhai
import "patterns/agent_graph" as agent_graph;

agent_graph::AgentGraph(#{key:"graph",run:"preview",revision:0,now_ms:0,nodes:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `calls` | array of object (at most 96) or `()` | — | Recent calls in chronological order, newest last. Calls can add temporary links (64 distinct total). |
| `currency` | string or `()` | — | Currency label for costs; defaults to USD. |
| `edges` | array of object (at most 64) or `()` | — | Standing connections. Opposite declarations represent one connection. |
| `heat_window_ms` | integer 1000–3600000 | `45000` | Age at which requests no longer contribute to link heat. |
| `height` | integer 190–1600 or `()` | — | Preferred logical canvas height. May grow to fit dense graphs without overlapping targets. |
| `key` | string | required | Stable presentation key. |
| `nodes` | array of object (at most 24) | required | Up to 24 distinct nodes. Reordering nodes can change their orbit positions, not their callback identity. |
| `notice` | string or `()` | — | Optional Host feedback for a proposed selection. |
| `now_ms` | integer ≥ 0 | required | Host clock for link heat and the calls-per-minute count. |
| `on_select` | callback or `()` | — | Enable Host-owned selection and receive versioned proposals. Omit for local inspection. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Accepted snapshot revision for selection intents. |
| `run` | string | required | Run identity; change this when the call ID namespace is replaced. |
| `selected` | string or `()` | — | Accepted inspected node in controlled mode. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Graph heading. |

### `calls[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `at_ms` | integer ≥ 0 | required | When the call was made, in the Host clock domain. |
| `elapsed_ms` | integer ≥ 0 or `()` | — | Duration of the terminal reply. |
| `from` | string | required | Caller. |
| `id` | string | required | Stable identity of this call in the run; do not reuse. |
| `label` | string or `()` | — | Short context for the call. |
| `status` | `"pending"` or `"ok"` or `"error"` or `()` | — | Pending goes outbound; accepted terminal transitions return along the same edge. |
| `to` | string | required | Receiver. |

### `edges[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `from` | string | required | One endpoint. |
| `to` | string | required | Other endpoint. |

### `nodes[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `cost_micros` | integer 0–1000000000000 or `()` | — | Own cost in millionths of the currency unit. |
| `detail` | string or `()` | — | Model or short description shown in the inspector. |
| `doing` | string or `()` | — | Current activity summary. |
| `icon` | node or `()` | — | Decorative mark; defaults to initials. |
| `id` | string | required | Stable node identity. |
| `kind` | `"agent"` or `"tool"` or `"user"` | required | Agents are on the inner ring; tools and users are on the outer ring. |
| `lead` | bool or `()` | — | Optional single lead agent; defaults to the first agent. |
| `name` | string | required | Full name, also used by assistive technology. |
| `short` | string or `()` | — | Optional visible short label for compact layouts. |
| `status` | `"idle"` or `"working"` or `"waiting"` or `"error"` or `"done"` or `()` | — | Accepted status; defaults to idle. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `select` | `on_select` | object | In controlled mode the inspector changes only when the Host updates selected. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `agent`, `bar`, `call`, `call_end`, `call_label`, `call_peer`, `calls`, `close`, `count`, `detail`, `empty`, `inspect_cost`, `inspector`, `inspector_head`, `inspector_title`, `lead`, `legend`, `legend_label`, `mark`, `node`, `node_label`, `node_neighbor`, `node_quiet`, `node_selected`, `notice`, `orbit`, `panel`, `pulse`, `root`, `stage`, `state`, `stats`, `status`, `status_done`, `status_error`, `status_waiting`, `status_working`, `title`, `tool`, `total`, `user`, `wires`.

## Theme

- Tokens: `accent`, `agent_tree.link`, `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/button`](../components/button.md), [`patterns/approval_card`](../patterns/approval_card.md)
