# OtpInput

`components/otp_input` · export `OtpInput` · version 0.1.0. Generated from
[`registry/components/otp_input.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/otp_input.rhai); do not edit.

OtpInput edits one controlled verification code using a single native text field. State: error-clear and resend timing only. The caller owns the verification attempt and code.

```rhai
import "components/otp_input" as otp_input;

otp_input::OtpInput(#{key:"verification",label:"Verification code",value:"",length:6,separator:true})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `align` | `"start"` or `"center"` | `"start"` | Align the editor, feedback and resend affordance. |
| `alphanumeric` | bool | `false` | Enables Latin letters alongside digits; committed letters are uppercased. |
| `autofocus` | bool | `false` | Focuses the native field on its first mount. |
| `clear_on_error` | bool | `true` | Propose an empty code after a short error interval. A replacement code or status withdraws the pending clear. |
| `disabled` | bool | `false` | Disables input and resend. |
| `error_message` | string or `()` | — | Message presented when verification fails. |
| `key` | string | required | Stable identity of this editor. |
| `label` | string | required | Accessible name of the one editor; its cells are decorative. |
| `length` | integer 1–24 | `6` | Number of code cells. |
| `on_change` | callback or `()` | — | Called with a new normalized code; store it and pass it back as value. |
| `on_complete` | callback or `()` | — | Called when an edit produces a full code, never merely because the Host supplied a complete value. |
| `on_resend` | callback or `()` | — | Called when the resend action is available and activated. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `resend_seconds` | integer 0–86400 | `30` | Cooldown while this view is active, initially and after an accepted Resend button activation. |
| `separator` | bool or integer 0–23 | `false` | True divides at the midpoint, or specify the cell index. The divider never enters the value. |
| `status` | `"idle"` or `"verifying"` or `"error"` or `"success"` | `"idle"` | Controlled presentation; verifying keeps focus and blocks edits, success shows the confirmed label. |
| `style` | style | — | Style merged over the root part. |
| `success_label` | string or `()` | — | Text rendered in the success pill; defaults to Verified. |
| `value` | string | required | Controlled code, default empty string at the caller. Leading zeroes are retained. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string | The full normalized code or the empty string on clear. |
| `complete` | `on_complete` | string | A newly entered full-length code. |
| `resend` | `on_resend` | none | A request to send another code. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `active`, `caret`, `cell`, `countdown`, `editor`, `error`, `error_icon`, `filled`, `invalid`, `invalid_active`, `resend`, `resend_hint`, `root`, `separator`, `success`, `success_icon`, `verifying`.

## Theme

- Tokens: `accent`, `border`, `danger`, `focus_ring`, `selection`, `space.spacing_unit`, `space.unit`, `success`, `surface`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.control`, `typography.otp_cell`, `typography.otp_success`
- Environment: `density`, `size`
