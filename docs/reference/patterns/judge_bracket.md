# JudgeBracket

`patterns/judge_bracket` · export `JudgeBracket` · version 0.1.0. Generated from
[`registry/patterns/judge_bracket.rhai`](../../../registry/patterns/judge_bracket.rhai); do not edit.

An ordered elimination bracket, with comparison and a Host-owned champion choice.

```rhai
import "patterns/judge_bracket" as judge_bracket;

judge_bracket::JudgeBracket(#{key:"judge",session:"drafts",contenders:[#{id:"a",name:"A"},#{id:"b",name:"B"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `capacity` | integer 0–16 | `0` | 0 picks the smallest adequate bracket. Explicit capacities 2, 4, 8, and 16 preserve the layout as the field changes. |
| `contenders` | array of object (at most 16) | required | Attempts in seed order. A trailing vacant seed is a true bye; an unresolved match is not. |
| `judge` | string or `()` | — | Display name of the judge. |
| `judging` | string or `()` | — | ID of the ready match the Host is evaluating. |
| `key` | string | required | Stable instance key. |
| `on_pick` | callback or `()` | — | Request use of the validated champion. Does not execute the chosen output. |
| `on_selection_change` | callback or `()` | — | Receives a requested match ID or null for automatic selection. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `pending` | bool | `false` | Host is deciding a Use winner proposal. |
| `revision` | integer ≥ 0 | `0` | Accepted Host revision attached to winner proposals. |
| `selected_match` | string or `()` | — | Optional controlled match selection; explicit null uses the automatic selection. |
| `session` | string | required | Identity of the evaluation. A new session resets local match selection. |
| `style` | style | — | Style merged over the root part. |
| `task` | string or `()` | — | Short evaluation prompt shown when room is available. |
| `title` | string or `()` | — | Heading of this bracket. |
| `verdicts` | map of object | `#{}` | Accepted judge decisions indexed by zero-based round:match. Both ordered entrant IDs must still match for a verdict to count. |

### `contenders[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `icon` | node or `()` | — | Optional decorative entrant icon. |
| `id` | string | required | Unique stable contender ID. |
| `meta` | string or `()` | — | Supporting label shown with the output. |
| `name` | string | required | Display name of the attempt. |
| `output` | string or `()` | — | Read-only draft text for the comparison; never executes. |

### `verdicts.<key>` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `a` | string | required | Expected entrant ID in the first slot. |
| `b` | string | required | Expected entrant ID in the second slot. |
| `reason` | string or `()` | — | Explanation shown only for a verdict still bound to this pairing. |
| `scores` | array of number -1000–1000 (at most 2) | `[]` | Optional displayed scores in entrant order, not used to determine the winner. |
| `winner` | string | required | Winning entrant ID. Must be a or b. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `pick` | `on_pick` | object | Explicit proposal for the current champion; never contains their output or judge's reason. |
| `selection_change` | `on_selection_change` | string or `()` | A selected match, or null for automatic selection. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `answer`, `answer_lost`, `answer_meta`, `answer_name`, `answer_won`, `answers`, `badge`, `bracket`, `button`, `head`, `live`, `mark`, `match`, `match_active`, `name`, `notice`, `output`, `panel`, `panel_head`, `panel_status`, `panel_title`, `reason`, `root`, `round`, `round_active`, `score`, `slot`, `slot_active`, `slot_champion`, `slot_empty`, `slot_lost`, `slot_won`, `summary`, `task`, `tile`, `title`, `waiting`, `wires`.

## Theme

- Tokens: `bracket.blue`, `bracket.card`, `bracket.dash`, `bracket.faint`, `bracket.hint`, `bracket.ink`, `bracket.line`, `bracket.lost`, `bracket.muted`, `bracket.on_solid`, `bracket.path`, `bracket.rail`, `bracket.shine`, `bracket.solid`, `bracket.tag`, `bracket.tray`, `bracket.well`, `bracket.won`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/spinner`](../components/spinner.md)
