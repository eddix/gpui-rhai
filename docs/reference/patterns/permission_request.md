# PermissionRequest

`patterns/permission_request` · export `PermissionRequest` · version 0.1.0. Generated from
[`registry/patterns/permission_request.rhai`](../../../registry/patterns/permission_request.rhai); do not edit.

PermissionRequest displays a capability request and an accepted grant, denial or expiry. State: preview expansion and optional local duration choice. Only the Host grants or revokes access.

```rhai
import "patterns/permission_request" as permission_request;

permission_request::PermissionRequest(#{key:"access",request:"review/1",revision:1,agent:"Reviewer",action:"read release notes"})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `action` | string | required | Plain display text describing the requested action, not a command. |
| `agent` | string | required | Who requests access. |
| `avatar` | node or `()` | — | Optional decorative avatar; otherwise initials are shown. |
| `busy` | bool | `false` | Blocks repeat proposals during a policy operation. |
| `can_renew` | bool | `false` | The Host allows a fresh request after denial or expiry. |
| `can_revoke` | bool | `false` | The Host allows a revoke proposal for this grant. |
| `duration_minutes` | integer 1–10080 | `60` | Duration offered for a timed request. The Host sets the actual deadline. |
| `grant` | object or `()` | — | Accepted grant record. Required for an allowed receipt. |
| `key` | string | required | Stable visual key. |
| `message` | string or `()` | — | Optional status or receipt subtitle. |
| `on_request` | callback or `()` | — | Receives versioned intents for allow, deny, scope selection, revoke or renew. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `preview` | array of object (at most 64) | `[]` | What will happen if granted. |
| `preview_limit` | integer 0–32 | `3` | Maximum rows before disclosure. |
| `preview_title` | string or `()` | — | Optional heading over preview entries. |
| `reason` | string or `()` | — | Plain explanation from the requester. |
| `request` | string | required | Identity of this permission request. |
| `revision` | integer ≥ 0 | required | Accepted policy revision echoed by every proposal. |
| `risk` | `"safe"` or `"risky"` or `"destructive"` | `"risky"` | Destructive requests never offer a perpetual grant. |
| `scope` | `"once"` or `"timed"` or `"always"` or `()` | — | Optional controlled scope. Otherwise the selection is local to this request revision. |
| `scopes` | array of `"once"` or `"timed"` or `"always"` (at most 3) | `["once", "timed", "always"]` | Offered grant scopes. Empty disables Allow. |
| `shortcuts` | bool | `true` | Allow deliberate Cmd/Ctrl+Enter and Escape when focus is inside the pending request. |
| `status` | `"pending"` or `"allowed"` or `"denied"` or `"expired"` or `"revoked"` | `"pending"` | Accepted outcome, never computed from a UI clock. |
| `style` | style | — | Style merged over the root part. |
| `target` | string or `()` | — | Optional target or model label. |

### `grant` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Accepted grant identity needed for revocation. |
| `remaining_ms` | integer ≥ 0 or `()` | — | Remaining time displayed by the Host. |
| `scope` | `"once"` or `"timed"` or `"always"` | required | Actually granted duration, independent of a proposal. |
| `total_ms` | integer ≥ 1 or `()` | — | Initial duration, used for the visual time track. |

### `preview[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable preview identity. |
| `info` | string or `()` | — | Optional annotation to its end. |
| `label` | string | required | One expected effect of the action. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `request` | `on_request` | object | Versioned capability proposal. The component does not grant, extend or revoke access. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `allow`, `allow_destructive`, `answer`, `buttons`, `deny`, `description`, `feedback`, `header`, `hint`, `icon`, `mark`, `preview`, `preview_count`, `preview_header`, `preview_info`, `preview_label`, `preview_more`, `preview_row`, `preview_title`, `reason`, `receipt`, `receipt_action`, `receipt_expired`, `receipt_mark`, `receipt_no`, `receipt_note`, `receipt_ok`, `receipt_title`, `risk`, `risk_high`, `risk_low`, `risk_medium`, `root`, `scope_option`, `scopes`, `surface`, `time_fill`, `time_track`, `title`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text.success`, `text.warning`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`
- Environment: `density`

## Dependencies

[`components/button`](../components/button.md), [`components/radio_group`](../components/radio_group.md), [`patterns/approval_card`](../patterns/approval_card.md)
