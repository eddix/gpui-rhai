# AlertDialog

`components/alert_dialog` · export `AlertDialog` · version 0.2.0. Generated from
[`registry/components/alert_dialog.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/alert_dialog.rhai); do not edit.

Controlled confirmation dialog with explicit cancel and confirm semantics: every close reports `confirm` or `cancel` first, Escape and backdrop presses `cancel`. State: caller owns open state; the component retains no application value.

```rhai
import "components/alert_dialog" as alert_dialog;

alert_dialog::AlertDialog(#{ key: "delete", open: true, title: "Delete item?", on_confirm: Fn("remove") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `cancel_label` | string | `"Cancel"` | Text of the cancel button. |
| `confirm_label` | string | `"Continue"` | Text of the confirm button. |
| `confirm_variant` | `"primary"` or `"danger"` or `"warning"` | `"danger"` | Button variant of the confirm button. |
| `content` | node or `()` | — | Extra content after the description. |
| `description` | string or `()` | — | Muted text under the title. |
| `key` | string | required | Identity of the dialog and the id of its overlay; keep it unique among open overlays. |
| `on_cancel` | callback or `()` | — | Called with `()` when the cancel button is pressed or Escape or a backdrop press dismisses the dialog; `open_change` then asks to close. |
| `on_confirm` | callback or `()` | — | Called with `()` when the confirm button is pressed; `open_change` then asks to close. |
| `on_open_change` | callback or `()` | — | Called with `false` when either button is pressed or Escape or a backdrop press dismisses the dialog. |
| `open` | bool | required | Whether the dialog is open; the caller stores the `open_change` payload and passes it back. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `title` | string | required | Heading of the dialog and its accessible name. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `cancel` | `on_cancel` | none | Emitted when the cancel button is pressed or Escape or a backdrop press dismisses the dialog, before `open_change`; the payload is `()`. |
| `confirm` | `on_confirm` | none | Emitted when the confirm button is pressed, before `open_change`; the payload is `()`. |
| `open_change` | `on_open_change` | bool | Emitted when a button is pressed or the dialog is dismissed; the payload is the requested open state. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `content` | no | no | Extra content after the description. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `cancel`, `confirm`, `content`, `description`, `root`.

## Theme

- Tokens: `space.related`, `text_muted`, `typography.body`
- Environment: `density`

## Dependencies

[`components/button`](button.md), [`components/dialog`](dialog.md)
