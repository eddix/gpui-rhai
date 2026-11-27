# AgentCheckpoints

`patterns/agent_checkpoints` · export `AgentCheckpoints` · version 0.1.0. Generated from
[`registry/patterns/agent_checkpoints.rhai`](../../../registry/patterns/agent_checkpoints.rhai); do not edit.

An immutable chronological history with a controlled head. Rewind is a proposal.

```rhai
import "patterns/agent_checkpoints" as agent_checkpoints;

agent_checkpoints::AgentCheckpoints(#{key:"history",run:"preview",revision:0,checkpoints:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `busy` | bool | `false` | Keep history navigable, but disable rewind while the Host resolves a proposal. |
| `checkpoints` | array of object (at most 96) | required | Up to 96 checkpoints, oldest first. Parents must precede children. |
| `currency` | string or `()` | — | Short ISO currency label for cents; defaults to USD. |
| `head` | string or `()` | — | Existing checkpoint ID. Omit for the last checkpoint. |
| `height` | length or `()` | — | Maximum height of the scrollable history; default 456px. |
| `key` | string | required | Stable history surface key. |
| `notice` | string or `()` | — | Host feedback for a pending, accepted or declined proposal. |
| `on_rewind` | callback or `()` | — | Receives a revision- and head-bound proposal to start a new attempt. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Accepted snapshot revision. |
| `run` | string | required | Business identity of the run. |
| `style` | style | — | Style merged over the root part. |
| `task` | string or `()` | — | Short task caption. |
| `title` | string or `()` | — | History heading. |

### `checkpoints[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `cost_cents` | integer 0–1000000000 or `()` | — | Cost in hundredths of the display currency. |
| `elapsed_ms` | integer ≥ 0 or `()` | — | Elapsed time from the start of its attempt. |
| `files` | integer ≥ 0 or `()` | — | Number of changed files. |
| `icon` | node or `()` | — | Optional decorative step icon. |
| `id` | string | required | Unique stable checkpoint ID. |
| `parent` | string or `()` | — | ID of an earlier checkpoint; omit for an independent root. |
| `status` | `"done"` or `"running"` or `"failed"` or `"stopped"` or `()` | — | Accepted status; defaults to done. |
| `title` | string | required | Checkpoint title; does not identify the checkpoint. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `rewind` | `on_rewind` | object | Validate run, revision, checkpoint and head in the Host before branching. No history is modified by the component. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `attempt`, `body`, `content`, `current`, `empty`, `failed`, `head`, `icon`, `marker`, `marker_archived`, `marker_current`, `marker_live`, `meta`, `notice`, `rail`, `rewind`, `root`, `row`, `row_archived`, `row_current`, `stage`, `summary`, `task`, `title`, `title_text`, `track`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/button`](../components/button.md), [`patterns/approval_card`](../patterns/approval_card.md)
