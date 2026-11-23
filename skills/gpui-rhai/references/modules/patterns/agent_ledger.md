# AgentLedger

`patterns/agent_ledger` · export `AgentLedger` · version 0.1.0. Generated from
[`registry/patterns/agent_ledger.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/agent_ledger.rhai); do not edit.

Describe the changes an agent made and preview atomic undo or redo batches.

```rhai
import "patterns/agent_ledger" as agent_ledger;

agent_ledger::AgentLedger(#{key:"ledger",run:"snapshot/1",revision:0,entries:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string or `()` | — | Agent who made the changes. |
| `busy` | bool | `false` | Block more requests while the Host settles a transaction; retain command focus. |
| `entries` | array of object (at most 64) | required | An accepted ledger of up to 64 changes. The display sorts newest first. |
| `key` | string | required | Stable retained surface key. |
| `notice` | string or `()` | — | Host progress, acceptance or refusal announced as status. |
| `now_ms` | integer 0–253402300799999 or `()` | — | Accepted epoch millisecond clock. Omit for a view-owned fifteen-second relative time refresh. |
| `on_action` | callback or `()` | — | Receives one versioned Undo or Redo request. Without it the ledger is read-only. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `point_at` | string or `()` | — | Optional Undo preview target for a walkthrough. Local action hover and focus take precedence. |
| `revision` | integer ≥ 0 | required | Revision of the Host-accepted entries and undo batches. |
| `run` | string | required | Identity of this ledger series; replace it to start a new ledger. |
| `show_actions` | bool | `false` | Always show commands instead of waiting for hover or focus. |
| `style` | style | — | Style merged over the root part. |
| `time_zone` | string | `"UTC"` | Zone for absolute date labels. |
| `title` | string or `()` | — | Heading, defaulting to Ledger. |
| `undone_batches` | array of object (at most 64) or `()` | — | Host-accepted undo batches. Redo restores exactly one such batch. |

### `entries[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `action` | string | required | What was changed. |
| `at_ms` | integer 0–253402300799999 | required | Epoch milliseconds of the original change. |
| `change` | object or `()` | — | Before/after pair; takes precedence over detail. |
| `depends_on` | array of string (at most 8) or `()` | — | Existing change IDs that this one builds on; the dependency graph must be acyclic. |
| `detail` | string or `()` | — | Single secondary line when change is omitted. |
| `icon` | node or `()` | — | Decorative system mark; defaults to initials. |
| `id` | string | required | Unique stable change ID. |
| `reversible` | bool or `()` | — | False for an irreversible change. |
| `system` | string | required | Application or system where it happened. |
| `target` | string or `()` | — | Object of the change. |

### `entries[].change` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `after` | string | required | Accepted new value. |
| `before` | string | required | Previous value. |

### `undone_batches[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `changes` | array of string (at most 64) | required | The reversible changes actually undone together, including the root. No ID may be in two batches. |
| `root` | string | required | The original Undo command's change ID. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | The Host validates the revision and executes or refuses the proposed batch atomically. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `after`, `agent`, `also`, `arrow`, `bar`, `before`, `before_line`, `change`, `command`, `command_busy`, `command_hot`, `command_narrow`, `command_text`, `detail`, `divider`, `empty`, `end`, `end_narrow`, `flag`, `flag_also`, `flag_kept`, `headline`, `icon`, `icon_undone`, `kept`, `locked`, `main`, `mark`, `notice`, `panel`, `preview`, `quiet`, `root`, `row`, `strike`, `summary`, `target`, `time`, `title`, `total`, `undone_time`, `undone_total`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/avatar`](../components/avatar.md), [`patterns/agent_tree`](agent_tree.md), [`patterns/approval_card`](approval_card.md)
