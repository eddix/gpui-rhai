# DecisionInbox

`patterns/decision_inbox` · export `DecisionInbox` · version 0.1.0. Generated from
[`registry/patterns/decision_inbox.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/decision_inbox.rhai); do not edit.

DecisionInbox is a queue of decisions with two quick responses and accepted receipts. State: local disclosure only; the Host owns the phase, answer and receipt lifetime.

```rhai
import "patterns/decision_inbox" as decision_inbox;

decision_inbox::DecisionInbox(#{key:"incoming",decisions:[#{id:"reply",revision:0,title:"Reply to the request",primary:"Send",secondary:"Wait"}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `auto_open_next` | bool | `true` | Open the next folded pending item after an accepted receipt arrives. |
| `decisions` | array of object (at most 64) | required | Ordered open and briefly retained resolved decisions. Host removal shortens the queue. |
| `key` | string | required | Stable presentation identity. |
| `max_height` | length or `()` | — | Positive absolute bound for the native viewport, default 440px. |
| `on_answer` | callback or `()` | — | Receives a versioned primary or secondary proposal. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |

### `decisions[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `collapsed` | bool or `()` | — | Default disclosure for a new item revision. |
| `detail` | string or `()` | — | Context revealed with the actions. |
| `id` | string | required | Unique request ID. |
| `notice` | string or `()` | — | Optional Host rejection or clarification. |
| `phase` | `"pending"` or `"busy"` or `"resolved"` or `()` | — | Accepted phase; pending by default. |
| `primary` | string | required | Primary quick-answer caption. |
| `receipt` | string or `()` | — | Accepted result shown while the resolved row is retained. |
| `revision` | integer ≥ 0 | required | Accepted revision echoed in a proposal. |
| `secondary` | string | required | Secondary quick-answer caption. |
| `title` | string | required | Short title. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `answer` | `on_answer` | object | Host-owned answer proposal. The Inbox never resolves optimistically. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `badge`, `card`, `closed`, `count`, `detail`, `empty`, `first`, `head`, `icon`, `notice`, `primary`, `receipt`, `root`, `row`, `secondary`, `title`.

## Theme

- Tokens: `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.success`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`patterns/approval_card`](approval_card.md)
