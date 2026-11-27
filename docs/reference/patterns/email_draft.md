# EmailDraft

`patterns/email_draft` · export `EmailDraft` · version 0.1.0. Generated from
[`registry/patterns/email_draft.rhai`](../../../registry/patterns/email_draft.rhai); do not edit.

Review an authored email without giving this view transport or file access.

```rhai
import "patterns/email_draft" as email_draft;

email_draft::EmailDraft(#{key:"renewal",draft:"renewal",revision:0,contacts:[#{id:"finance",name:"Ada Lovelace",email:"ada@example.test"}],tones:[#{id:"brief",label:"Brief",body:"Hi {first}, your plan renews soon."}]})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `account` | string or `()` | — | Account caption. |
| `agent` | string or `()` | — | Draft author caption. |
| `amount` | string or `()` | — | Formatted amount. |
| `attachment` | object or `()` | — | Optional removable attachment. The component never resolves files. |
| `contacts` | array of object (at most 64) | required | Available recipients. An empty list disables sending. |
| `draft` | string | required | Business ID. Changing it resets local edits, recipient, attachment and tone. |
| `key` | string | required | Stable card instance. |
| `link` | object or `()` | — | Optional appended link. The Host owns both opening and composing its destination. |
| `notice` | string or `()` | — | Optional accepted or rejected status from the Host. |
| `on_discard` | callback or `()` | — | Requests discarding this draft and revision. |
| `on_link` | callback or `()` | — | Handles a request to open the appended link. |
| `on_send` | callback or `()` | — | Receives the proposed recipient, content and attachment. Does not send mail. |
| `on_undo` | callback or `()` | — | Requests restoring a draft; does not recall already delivered mail. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `phase` | `"draft"` or `"sending"` or `"sent"` or `"discarded"` | `"draft"` | Only the Host may enter a pending or receipt phase. |
| `receipt` | object or `()` | — | Accepted receipt in the sent phase. Without it the card shows only a generic sent status. |
| `reference` | string or `()` | — | Record reference. |
| `revision` | integer ≥ 0 | required | Accepted Host revision carried by requests. |
| `status` | string or `()` | — | Record status chip. |
| `style` | style | — | Style merged over the root part. |
| `tones` | array of object (at most 12) | required | Generated text variants in display order. A recipient change retains manual edits, updating the previous name; choosing a tone resets the edit. |

### `attachment` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Opaque Host-owned file ID. |
| `label` | string | required | Attachment name. |

### `contacts[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `email` | string | required | Displayed address. The Host resolves the recipient before sending. |
| `id` | string | required | Distinct recipient ID. |
| `name` | string | required | Recipient name. |

### `link` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Opaque Host link identity. |
| `label` | string | required | Visible link text. |

### `receipt` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `attachment_label` | string or `()` | — | Attachment label confirmed on delivery. |
| `recipient_name` | string | required | Name confirmed on delivery, independent of the current contact roster. |

### `tones[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `body` | string | required | Literal text with optional {first} and {name} tokens; not executable. |
| `id` | string | required | Distinct variant ID. |
| `label` | string | required | Tone button caption. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `discard` | `on_discard` | object | A discard proposal. |
| `link` | `on_link` | object | Open an opaque link through the Host. |
| `send` | `on_send` | object | A send proposal. Optional attachment and link IDs are empty when absent. |
| `undo` | `on_undo` | object | A restore proposal from a settled receipt. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `account`, `actions`, `agent`, `amount`, `attach`, `attach_label`, `attachment`, `attachment_icon`, `badge`, `badge_done`, `body`, `body_text`, `buttons`, `card`, `control_label`, `decision_label`, `discard`, `edit`, `editor`, `file_name`, `footer`, `footer_text`, `glyph`, `hint`, `hint_rail`, `link`, `menu`, `meta`, `notice`, `option`, `option_avatar`, `option_email`, `option_name`, `option_selected`, `receipt`, `receipt_icon`, `receipt_text`, `recipient`, `recipient_arrow`, `recipient_email`, `recipient_name`, `recipient_open`, `record`, `reference`, `remove`, `remove_icon`, `root`, `send`, `status`, `switcher`, `to_label`, `tone`, `tone_selected`, `tones`, `undo`.

## Theme

- Tokens: `accent`, `border`, `design.name`, `email_draft.account`, `email_draft.address`, `email_draft.card`, `email_draft.chip`, `email_draft.file`, `email_draft.hint`, `email_draft.link`, `email_draft.link_underline`, `email_draft.ok`, `email_draft.ok_bg`, `email_draft.on_send`, `email_draft.send`, `email_draft.tray`, `email_draft.warn`, `email_draft.warn_bg`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.key_legend`, `typography.label`, `typography.metadata`

## Dependencies

[`components/popover`](../components/popover.md), [`components/spinner`](../components/spinner.md), [`components/textarea`](../components/textarea.md)
