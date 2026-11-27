# DecisionCard

`patterns/decision_card` · export `DecisionCard` · version 0.1.0. Generated from
[`registry/patterns/decision_card.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/decision_card.rhai); do not edit.

DecisionCard pages through Host-owned choices, visual picks and approvals. State: current page, collapse, per-revision draft and native option focus.

```rhai
import "patterns/decision_card" as decision_card;

decision_card::DecisionCard(#{key:"review",decisions:[#{id:"release",revision:0,question:"Continue?"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `decisions` | array of object (at most 48) | required | Open decisions in presentation order. The Host removes an item after accepting an answer. |
| `initial_folded` | bool | `false` | Initially show only the waiting summary. |
| `key` | string | required | Stable presentation identity. |
| `on_answer` | callback or `()` | — | Receives a versioned answer proposal. |
| `on_later` | callback or `()` | — | Receives the request hidden by Later; not an answer. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `reveal` | integer ≥ 0 | `0` | Changing this Host signal unfolds the card and focuses the currently offered options. |
| `style` | style | — | Style merged over the root part. |

### `decisions[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `busy` | bool or `()` | — | This item is awaiting an accepted answer; navigation stays enabled. |
| `facts` | array of object (at most 8) or `()` | — | Read-only facts about the decision. |
| `id` | string | required | Request ID; unique among open decisions. |
| `kind` | `"decision"` or `"pick"` or `"approval"` or `()` | — | Question layout; defaults to decision rows. |
| `notice` | string or `()` | — | Acknowledge or explain a rejected proposal. |
| `options` | array of object (at most 5) or `()` | — | Options for decision and pick; approval has built-in Approve and Decline. |
| `question` | string | required | The question shown in the decision card. |
| `revision` | integer ≥ 0 | required | Expected revision when proposing an answer. |
| `task` | string or `()` | — | Task label in the header. |

### `decisions[].facts[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `label` | string | required | Evidence label. |
| `value` | string | required | Evidence value displayed without taking action. |

### `decisions[].options[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `blocks` | any value | — | Up to 16 wireframe blocks: #{x,y,width,height,tone?} in 120×80 integer coordinates; tone is ink, mid or soft. |
| `id` | string | required | Stable option ID. |
| `preview` | node or `()` | — | Decorative content in a pick tile; never owns a second set of controls. |
| `recommended` | bool or `()` | — | Marks the initially focused option without answering it. |
| `summary` | string or `()` | — | Short description, also exposed on the option. |
| `title` | string | required | Visible and accessible option name. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `answer` | `on_answer` | object | Versioned proposal; rejection does not remove the item or clear its draft. |
| `later` | `on_later` | object | Presentation preference; does not resolve a decision. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `badge`, `card`, `count`, `editor`, `empty`, `fact`, `fact_label`, `fact_value`, `facts`, `head`, `icon`, `later`, `notice`, `number`, `option`, `option_title`, `options`, `own`, `page`, `pager`, `preview`, `question`, `recommended`, `root`, `send`, `show`, `summary`, `task`, `tile`.

## Theme

- Tokens: `border`, `focus_ring`, `selection`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/icon_button`](../components/icon_button.md), [`components/input`](../components/input.md), [`patterns/approval_card`](approval_card.md)
