# AgentRecap

`patterns/agent_recap` · export `AgentRecap` · version 0.1.0. Generated from
[`registry/patterns/agent_recap.rhai`](../../../registry/patterns/agent_recap.rhai); do not edit.

Replay an accepted interval of agent activity and send versioned event commands to the Host.

```rhai
import "patterns/agent_recap" as agent_recap;

agent_recap::AgentRecap(#{key:"night",run:"night/1",revision:0,since_ms:0,until_ms:3600000,events:[]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `board` | bool | `true` | Show the three lanes after the activity plot. |
| `events` | array of object (at most 72) | required | Up to 72 events, sortable by epoch time and stable ID. |
| `key` | string | required | Stable surface key. |
| `limit` | integer 1–8 | `3` | Maximum visible cards in each lane. Summary and plot always include the whole interval. |
| `notice` | string or `()` | — | Optional Host response, presented as a semantic status. |
| `on_action` | callback or `()` | — | Receives primary, review, undo or redo proposals; the application owns all business outcomes. |
| `on_inspect` | callback or `()` | — | Receives the versioned inspected event ID (empty means inspection ended). |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `replay_ms` | integer 0–8000 | `1800` | Decorative replay length; reduced and disabled motion show the whole snapshot immediately. |
| `replay_serial` | integer ≥ 0 | `0` | Increment to explicitly replay the current series. Resizing, theme and ordinary revision changes do not restart it. |
| `revision` | integer ≥ 0 | required | Revision of accepted events and outcomes. |
| `run` | string | required | Identity of the event snapshot series. |
| `since_ms` | integer 0–253402300799999 | required | First epoch millisecond in the replay interval. |
| `style` | style | — | Style merged over the root part. |
| `time_zone` | string | `"UTC"` | Display zone for epoch timestamps, not an offset for curve positions. |
| `title` | string or `()` | — | Recap title. |
| `until_ms` | integer 1–253402300799999 | required | Last epoch millisecond; must be later than since_ms. |

### `events[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `action` | string or `()` | — | Custom primary label for a needs event. |
| `at_ms` | integer 0–253402300799999 | required | Event epoch milliseconds. Events outside the supplied interval do not appear. |
| `can_undo` | bool or `()` | — | Set false to omit Undo for an irreversible outcome. |
| `count` | integer 0–1000000000 or `()` | — | Completed work quantity. Affects peak height, not the event count. |
| `detail` | string or `()` | — | Secondary context, or failure explanation. |
| `done_label` | string or `()` | — | Custom accepted outcome label for a needs event. |
| `id` | string | required | Unique business key; stable across revisions. |
| `kind` | `"needs"` or `"failed"` or `"done"` | required | Original lane. Successful commands do not change a card's lane or key. |
| `logo` | node or `()` | — | Decorative mark; defaults to an initial. |
| `state` | `"open"` or `"pending"` or `"resolved"` or `"undone"` or `()` | — | Host-accepted state. For an originally done event use open or undone. Pending does not accept another command. |
| `title` | string | required | Event's accessible title. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `action` | `on_action` | object | Proposal against an accepted revision. |
| `inspect` | `on_inspect` | object | Local dot/card inspection changed. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `board`, `card`, `card_head`, `card_hot`, `card_quiet`, `card_title`, `curve_fill`, `detail`, `dot`, `empty`, `failure`, `footer`, `head`, `identity`, `lane`, `lane_count`, `lane_head`, `lane_label`, `mark`, `meta`, `notice`, `panel`, `pill`, `plot`, `primary`, `quantity`, `quiet`, `root`, `since`, `stack`, `status`, `summary`, `summary_count`, `summary_item`, `summary_label`, `time`, `times`, `tip`, `title`, `undo_state`, `working`.

## Theme

- Tokens: `agent_recap.area`, `agent_recap.cursor`, `agent_recap.dot`, `agent_recap.failed`, `agent_recap.grid`, `agent_recap.needs`, `agent_recap.stroke`, `border`, `button.primary_bg`, `button.primary_fg`, `focus_ring`, `space.unit`, `surface_hover`, `surface_raised`, `text.success`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/button`](../components/button.md), [`components/spinner`](../components/spinner.md), [`patterns/agent_tree`](../patterns/agent_tree.md), [`patterns/approval_card`](../patterns/approval_card.md)
