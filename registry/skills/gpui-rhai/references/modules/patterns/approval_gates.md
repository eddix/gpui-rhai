# ApprovalGates

`patterns/approval_gates` · export `ApprovalGates` · version 0.1.0. Generated from
[`registry/patterns/approval_gates.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/approval_gates.rhai); do not edit.

ApprovalGates presents the pending gates, blocked steps and accepted history of one run. State: selected gate, disclosures and a revision-checked edit. Nothing executes in the component.

```rhai
import "patterns/approval_gates" as approval_gates;

approval_gates::ApprovalGates(#{key:"review",run:"release/1",gates:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `gates` | array of object (at most 64) | required | All gates; keep accepted receipts in this list to retain history. |
| `key` | string | required | Stable visual identity of the stack. |
| `live` | bool | `false` | Whether the run can receive further gates. |
| `on_decide` | callback or `()` | — | Receives a versioned approval, rejection, edited approval or request for file changes. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `run` | string | required | Host run identity, included in every proposal. |
| `run_state` | `"waiting"` or `"running"` or `"stopped"` or `"done"` or `()` | — | Optional authoritative run label. Otherwise inferred from accepted gates and live. |
| `style` | style | — | Style merged over the root part. |
| `task` | string or `()` | — | Optional task caption. |
| `title` | string or `()` | — | Summary heading. |

### `gates[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `action` | string | required | Plain description of the requested action. |
| `agent` | string | required | Display name of the requester. |
| `at` | string or `()` | — | Caller-formatted time. |
| `busy` | bool or `()` | — | A proposal is waiting for the Host. |
| `context` | array of string (at most 24) or `()` | — | Earlier steps. |
| `diff` | array of object (at most 8) or `()` | — | Neutral, read-only file comparisons. |
| `editable` | bool or `()` | — | Show edit, or ask for file changes, while open. |
| `files` | array of string (at most 32) or `()` | — | Resources touched. |
| `holds` | array of string (at most 24) or `()` | — | Steps waiting on this gate. |
| `id` | string | required | Unique gate ID within the run. |
| `input` | string or `()` | — | Read-only evidence and the initial text for a non-file edit. |
| `kind` | `"tool"` or `"file"` or `"agent"` or `"message"` | required | Evidence type. No tool call is interpreted or invoked. |
| `note` | string or `()` | — | Accepted decision or rejection feedback. |
| `reason` | string | required | Reason the requester supplied. |
| `revision` | integer ≥ 0 | required | Revision of the presented facts and proposed action. |
| `status` | `"open"` or `"approved"` or `"edited"` or `"rejected"` or `"changes_requested"` | required | Accepted phase; only open gates accept new decisions. |

### `gates[].diff[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `after` | string | required | Proposed text provided by the Host. |
| `before` | string | required | Original text provided by the Host. |
| `name` | string | required | Display filename, not read by the component. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `decide` | `on_decide` | object | User proposal, not a result or execution directive. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `agent`, `approve`, `avatar`, `bar`, `body`, `code`, `context`, `date`, `diff`, `edit`, `editor`, `empty`, `extra`, `files`, `front`, `head`, `history`, `hold`, `holds`, `icon`, `kind`, `more`, `note`, `page`, `pager`, `peek`, `peek_far`, `question`, `reason`, `receipt`, `receipt_body`, `reject`, `root`, `section`, `section_title`, `stack`, `stat`, `stat_label`, `stat_value`, `stats`, `status`, `summary`, `task`, `title`.

## Theme

- Tokens: `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/code_viewer`](../components/code_viewer.md), [`components/diff_viewer`](../components/diff_viewer.md), [`components/icon_button`](../components/icon_button.md), [`components/textarea`](../components/textarea.md), [`patterns/approval_card`](approval_card.md)
