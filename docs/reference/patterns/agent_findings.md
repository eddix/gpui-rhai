# AgentFindings

`patterns/agent_findings` · export `AgentFindings` · version 0.1.0. Generated from
[`registry/patterns/agent_findings.rhai`](../../../registry/patterns/agent_findings.rhai); do not edit.

Present a team of checking agents, live progress and a bounded newest-first feed of findings.

```rhai
import "patterns/agent_findings" as agent_findings;

agent_findings::AgentFindings(#{key:"scan",run:"check/1",revision:0,now_ms:0,agents:[],findings:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `agents` | array of object (at most 24) | required | Up to 24 agents in the Host's presentation order. |
| `findings` | array of object (at most 256) | required | Up to 256 accepted findings for the feed and counts. |
| `key` | string | required | Stable retained surface key. |
| `now_ms` | integer 0–253402300799999 or `()` | — | Accepted epoch millisecond clock. If omitted a view-owned five-second wall-clock refresh runs only when the shown feed has timed findings. |
| `on_open` | callback or `()` | — | Receives a versioned open request. Omit to show a read-only feed. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | required | Revision of the accepted agents and findings snapshot. |
| `rows` | integer 1–12 | `4` | Maximum active rows in the newest-first feed; totals count the full input. |
| `run` | string | required | Stable business identity of the verification run. Replace it to start a new run. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Heading; defaults to Verification. |

### `agents[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `avatar` | node or `()` | — | Optional decorative logo or a picture; omit for initials. |
| `avatar_image` | `image` handle or `()` | — | Native avatar image with initials on load failure. Cannot be combined with avatar. |
| `done` | integer 0–1000000000 | required | Completed checks; display caps at total. |
| `id` | string | required | Unique agent ID within the run. |
| `model` | string or `()` | — | Optional agent role or model. |
| `name` | string | required | Displayed and accessible agent name. |
| `status` | `"running"` or `"done"` or `"failed"` or `()` | — | Accepted phase; defaults to done once the planned count is reached. |
| `step` | string or `()` | — | Current activity, when running. |
| `total` | integer 0–1000000000 | required | Planned checks. |

### `findings[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string | required | Owning agent ID. Findings remain visible after their agent leaves; an empty ID denotes an unassigned finding. |
| `at_ms` | integer 0–253402300799999 or `()` | — | Finding epoch milliseconds. Missing times sort last, ties retain Host input order. |
| `id` | string | required | Unique stable finding ID; keep it through ledger updates. |
| `location` | string or `()` | — | Optional source location or application-specific reference. |
| `severity` | `"critical"` or `"warning"` or `"note"` | required | Severity shape and text. |
| `title` | string | required | Visible and accessible title. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `open` | `on_open` | object | Emitted only on activation of a displayed finding; the Host opens the correct detail. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `agent`, `agents`, `author`, `avatar`, `avatar_logo`, `bar`, `chip`, `chip_label`, `completion`, `count`, `counter`, `counts`, `critical`, `dim`, `done`, `empty`, `failed`, `faint`, `feed`, `feed_clip`, `feed_content`, `feed_head`, `finding`, `finding_body`, `finding_title`, `glow`, `indicator`, `label`, `location`, `meta`, `name`, `note`, `root`, `running`, `selected`, `separator`, `severity`, `sheen`, `spark`, `step`, `time`, `title`, `total`, `totals`, `track`, `warning`, `who`.

## Theme

- Tokens: `accent`, `agent_findings.critical`, `agent_findings.faint`, `agent_findings.heading_case`, `agent_findings.note`, `agent_findings.quiet`, `agent_findings.warning`, `border`, `focus_ring`, `selection`, `space.unit`, `surface_hover`, `surface_raised`, `text_primary`, `typography.body`, `typography.caption`, `typography.code`

## Dependencies

[`components/avatar`](../components/avatar.md), [`components/progress`](../components/progress.md), [`patterns/agent_tree`](../patterns/agent_tree.md), [`patterns/approval_card`](../patterns/approval_card.md)
