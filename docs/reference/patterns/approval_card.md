# ApprovalCard

`patterns/approval_card` · export `ApprovalCard` · version 0.1.0. Generated from
[`registry/patterns/approval_card.rhai`](../../../registry/patterns/approval_card.rhai); do not edit.

ApprovalCard presents a versioned request, its evidence and the accepted receipt. State: a revision-scoped checkbox; the Host supplies deadlines and all decisions.

```rhai
import "patterns/approval_card" as approval_card;

approval_card::ApprovalCard(#{key:"review",request:"review/1",revision:3,title:"Approve the release note"})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | `"tray"` or `"card"` | `"tray"` | Action placement. |
| `allow_always` | bool | `false` | Opt in to offering a durable choice; it is not an authorization by itself. |
| `busy` | bool | `false` | Suppress another proposal while the Host is processing one. |
| `description` | string or `()` | — | Optional context under the title. |
| `details` | array of object (at most 24) | `[]` | Accepted facts about the request. |
| `editable` | bool | `false` | Offer a separate Edit proposal. |
| `icon` | node or `()` | — | Decorative mark instead of the shield. |
| `key` | string | required | Visual identity of this card. |
| `label` | string or `()` | — | Optional tag above the card. |
| `message` | string or `()` | — | Host-owned feedback or receipt subtitle. |
| `on_activity` | callback or `()` | — | Receives distinct hover and focus-within edges. A Host may pause a soft reminder; do not use this to extend a hard deadline. |
| `on_decision` | callback or `()` | — | Receives a versioned proposal; never executes an action in the component. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `preview` | node or `()` | — | Application-owned preview node, not executable action metadata. |
| `remaining_ms` | integer ≥ 0 or `()` | — | Time left as seen by the Host; zero disables new approval, without changing the accepted status. |
| `request` | string | required | Accepted request ID. Use a new ID for a different request. |
| `revision` | integer ≥ 0 | required | Accepted request revision echoed in every proposal. |
| `risk` | `"low"` or `"medium"` or `"high"` or `()` | — | Optional risk badge. |
| `shortcuts` | bool | `true` | Show and enable card-scoped Cmd/Ctrl+Enter and Escape. |
| `status` | `"pending"` or `"approved"` or `"denied"` or `"expired"` | `"pending"` | Accepted status. Only the Host can publish a terminal receipt. |
| `style` | style | — | Style merged over the root part. |
| `title` | string | required | Action the person is deciding on. |

### `details[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `label` | string | required | Field label. |
| `value` | string | required | Displayed value. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `activity` | `on_activity` | object | Presentation hint for soft timers; no authority over expiration. |
| `decision` | `on_decision` | object | Proposes approval, denial or editing. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `always`, `approve`, `buttons`, `deny`, `description`, `detail`, `detail_label`, `detail_value`, `edit`, `feedback`, `footer`, `footer_inside`, `header`, `icon`, `label`, `mark`, `preview`, `receipt`, `receipt_expired`, `receipt_mark`, `receipt_no`, `receipt_note`, `receipt_ok`, `receipt_title`, `risk`, `risk_high`, `risk_low`, `risk_medium`, `root`, `surface`, `timer`, `timer_urgent`, `title`.

## Theme

- Tokens: `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.title`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/checkbox`](../components/checkbox.md)
