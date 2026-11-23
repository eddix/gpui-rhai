# ToolAccessMap

`patterns/tool_access_map` · export `ToolAccessMap` · version 0.1.0. Generated from
[`registry/patterns/tool_access_map.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/tool_access_map.rhai); do not edit.

Inspect a Host's authorized tool snapshot and recorded calls; never invokes a tool.

```rhai
import "patterns/tool_access_map" as tool_access_map;

tool_access_map::ToolAccessMap(#{key:"map",session:"build",agents:[#{id:"review",name:"Reviewer",status:"done",tools:[#{id:"diff",name:"Diff"}]}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string or `()` | — | Controlled visible agent. An absent ID falls back to the first agent. |
| `agents` | array of object (at most 12) | required | An ordered, bounded run snapshot. |
| `default_agent` | string or `()` | — | Initial selection in local mode. |
| `key` | string | required | Stable visual instance. |
| `on_agent_change` | callback or `()` | — | The requested agent and current run revision. |
| `on_replay` | callback or `()` | — | An explicit replay request, not tool execution. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `replay_pending` | bool | `false` | Disable replay controls while the Host decides a proposal. |
| `revision` | integer ≥ 0 | `0` | Accepted snapshot revision used to scope editable input and replay. |
| `session` | string | required | Host identity of the current run. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Defaults to Tool access. |
| `tools` | array of object (at most 16) or `()` | — | Optional complete catalog; otherwise ordered by first appearance. |

### `agents[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Unique stable agent ID used to scope tool selection and replay proposals. |
| `model` | string or `()` | — | Optional model caption shown with the agent. |
| `name` | string | required | Agent name shown in the picker and access graph. |
| `status` | `"queued"` or `"running"` or `"waiting"` or `"done"` or `"failed"` | required | Host-reported agent status; controls presentation, not its authorization. |
| `task` | string or `()` | — | Private description of the work assigned to this agent. |
| `tools` | array of object (at most 16) | required | Tools this agent can access; unlisted tools are drawn as blocked. |

### `agents[].tools[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `calls` | array of object (at most 24) | — | Up to 24 calls, oldest first; IDs unique per tool. |
| `description` | string or `()` | — | Agent-specific explanation of the tool, falling back to the catalog caption. |
| `icon` | string or `()` | — | Short decorative text mark for the tool, not a file path or SVG. |
| `id` | string | required | Stable tool identifier; matches the same tool in the optional catalog. |
| `name` | string or `()` | — | Agent-specific display name, falling back to the catalog entry. |

### `agents[].tools[].calls[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `at_ms` | integer 0–9007199254740991 or `()` | — | Run-relative start time in milliseconds, rendered as elapsed minutes and seconds. |
| `duration_ms` | integer 0–9007199254740991 or `()` | — | Elapsed call duration in milliseconds, supplied by the Host. |
| `error` | string or `()` | — | Host error text for the failed result; not an SDK exception. |
| `id` | string | required | Call identity, unique within this agent and tool's call history. |
| `input` | any value | required | Supplied argument snapshot; shown as text or structured data and never executed by the component. |
| `output` | any value or `()` | — | Optional read-only result snapshot displayed in the call inspector. |
| `replayed` | bool | — | Marks an accepted call as a replay; setting it does not replay the tool. |
| `status` | `"running"` or `"done"` or `"failed"` | required | Accepted state of this invocation, used for pending, output or failure presentation. |
| `summary` | string or `()` | — | Short private caption for the call list, separate from input or output. |

### `tools[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `description` | string or `()` | — | Default description unless the agent supplies one. |
| `icon` | string or `()` | — | Default decorative text mark for a catalog tool. |
| `id` | string | required | Unique tool ID shared with the corresponding per-agent entries. |
| `name` | string | required | Default user-facing tool name, including on a blocked tile. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `agent_change` | `on_agent_change` | object | Selection intent for the active run. |
| `replay` | `on_replay` | object | Human-triggered request with the selected call ID and validated input. Authorization and execution remain with the Host. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `agent`, `agent_status`, `agents`, `avatar`, `back`, `badge`, `blocked`, `blocked_content`, `body`, `button`, `call`, `call_active`, `calls`, `code`, `detail`, `editor`, `empty`, `head`, `invalid`, `layout`, `live`, `mark`, `model`, `panel`, `panel_desc`, `panel_head`, `person`, `primary`, `root`, `section`, `stage`, `stats`, `tab`, `tab_active`, `tab_avatar`, `task`, `tile`, `title`, `tool`, `tool_count`, `tool_info`, `tool_name`, `wire`.

## Theme

- Tokens: `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `text_primary`, `tool_access.card`, `tool_access.danger`, `tool_access.danger_fill`, `tool_access.fill`, `tool_access.glint`, `tool_access.ink`, `tool_access.live`, `tool_access.live_fill`, `tool_access.muted`, `tool_access.on_solid`, `tool_access.rail`, `tool_access.solid`, `tool_access.success`, `tool_access.success_fill`, `tool_access.text`, `tool_access.tray`, `tool_access.used`, `tool_access.warning`, `tool_access.warning_fill`, `tool_access.well`, `typography.body`, `typography.caption`, `typography.code`, `typography.initials`, `typography.metadata`

## Dependencies

[`components/code_viewer`](../components/code_viewer.md), [`components/spinner`](../components/spinner.md), [`components/textarea`](../components/textarea.md), [`patterns/agent_tree`](agent_tree.md)
