# Dialog

`components/dialog` · export `Dialog` · version 0.2.0. Generated from
[`registry/components/dialog.rhai`](../../../registry/components/dialog.rhai); do not edit.

Dialog presents controlled modal content through the native overlay layer.

State: controlled by the caller.

```rhai
import "components/dialog" as dialog;

dialog::Dialog(#{ key: "confirm", open: true, title: "Confirm", content: text("Continue?") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of node (at most 8) | `[]` | Buttons in a row at the end of the panel, after the content. |
| `content` | node | required | Body of the dialog, under the title. |
| `dismiss_on_escape` | bool | `true` | Whether Escape asks to close the dialog through `open_change`. |
| `dismiss_on_outside` | bool | `true` | Whether a press on the backdrop asks to close the dialog through `open_change`. |
| `initial_focus` | `"panel"` or `"first"` | `"panel"` | What takes focus when the dialog opens: `panel` the panel itself, `first` its first focusable control. |
| `key` | string | required | Identity of the dialog and the id of its overlay; keep it unique among open overlays. |
| `on_open_change` | callback or `()` | — | Called with `false` when Escape or a backdrop press dismisses the dialog. |
| `open` | bool | required | Whether the dialog is open; the caller stores the `open_change` payload and passes it back. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `title` | string | required | Heading of the panel and the dialog's accessible name. |
| `title_visible` | bool | `true` | Whether the title is drawn; a hidden title still names the dialog. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `open_change` | `on_open_change` | bool | Emitted when Escape or a backdrop press dismisses the dialog; the payload is the requested open state. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `actions` | no | yes | Buttons in a row at the end of the panel. |
| `content` | yes | no | Body of the dialog, under the title. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `backdrop`, `content`, `focus_frame`, `panel`, `root`, `title`.

## Theme

- Tokens: `border`, `focus_ring`, `radius.lg`, `space.group`, `space.related`, `spacing.xl`, `surface_raised`, `text_primary`, `typography.body`, `typography.title`
- Environment: `corners`, `density`
