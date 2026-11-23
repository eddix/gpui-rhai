# DecisionTree

`patterns/decision_tree` · export `DecisionTree` · version 0.1.0. Generated from
[`registry/patterns/decision_tree.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/decision_tree.rhai); do not edit.

A geometry-backed tree of versioned choices. Only a Host receipt resolves a request. State: visible request, focused/pointed branch, expanded branch and custom draft.

```rhai
import "patterns/decision_tree" as decision_tree;

decision_tree::DecisionTree(#{key:"plan",decisions:[#{id:"plan",revision:0,agent:"Planner",question:"Continue?",options:[#{id:"go",label:"Proceed"}]}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `allow_other` | bool | `true` | Offer the user a custom instruction branch. |
| `decisions` | array of object (at most 32) | required | Requests in Host presentation order; remove a finished request when its receipt no longer needs to be shown. |
| `key` | string | required | Stable tree instance key. |
| `on_choose` | callback or `()` | — | Receives one versioned proposal when a leaf or custom instruction is submitted. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Optional heading above the pending queue. |

### `decisions[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `agent` | string | required | Who is asking. |
| `busy` | bool or `()` | — | Host is considering a proposal; choices are not available. |
| `context` | string or `()` | — | Context below the question. |
| `id` | string | required | Unique request ID. |
| `model` | string or `()` | — | Optional model name. |
| `notice` | string or `()` | — | Non-final Host feedback, such as a declined proposal. |
| `options` | any value | required | One to eight unique #{id,label,effects?,recommended?,children?} options. Each effect is plain text or an array of text and #{text,note,source?} spans. A branch may contain one level of choices; option IDs are unique across the request. |
| `question` | string | required | Question at the root of this decision. |
| `receipt` | object or `()` | — | Only an accepted Host receipt locks the choice and shows the final outcome. |
| `revision` | integer ≥ 0 | required | Version on which the user's answer is based. |
| `risk` | `"low"` or `"medium"` or `"high"` or `()` | — | Optional assessment of this decision's risk. |

### `decisions[].receipt` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `message` | string | required | Accepted result returned by the Host. |
| `option` | string | required | Chosen leaf option ID, or empty if a custom instruction was accepted. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `choose` | `on_choose` | object | Versioned answer proposal. A proposal does not change accepted facts. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `active`, `agent`, `avatar`, `board`, `branch`, `branches`, `choice`, `connector`, `context`, `count`, `editor`, `effect`, `empty`, `flag`, `flag_label`, `flag_tip`, `head`, `icon`, `label`, `leaf`, `model`, `muted`, `notice`, `number`, `pager`, `question`, `receipt`, `recommended`, `risk`, `risk_high`, `risk_low`, `risk_medium`, `root`, `source`, `subchoices`, `text`, `title`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `radius.lg`, `selection`, `space.spacing_unit`, `space.unit`, `spacing.md`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/hover_card`](../components/hover_card.md), [`components/icon_button`](../components/icon_button.md), [`components/textarea`](../components/textarea.md), [`patterns/approval_card`](approval_card.md), [`patterns/decision_card`](decision_card.md)
