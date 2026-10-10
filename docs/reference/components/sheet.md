# Sheet

`components/sheet` · export `Sheet` · version 0.2.0. Generated from
[`registry/components/sheet.rhai`](../../../registry/components/sheet.rhai); do not edit.

Controlled temporary modal panel attached to a viewport edge. State: caller owns open state.

```rhai
import "components/sheet" as sheet;

sheet::Sheet(#{ key: "inspector", open: true, side: "end", title: "Inspector", content: panel, on_open_change: Fn("opened") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of node (at most 8) | `[]` | Buttons in a row at the end of the panel, after the content. |
| `content` | node | required | Body of the sheet; it fills the panel between the title and the actions. |
| `dismiss_on_escape` | bool | `true` | Whether Escape asks to close the sheet through `open_change`. |
| `dismiss_on_outside` | bool | `true` | Whether a press on the backdrop asks to close the sheet through `open_change`. |
| `initial_focus` | `"panel"` or `"first"` | `"first"` | What takes focus when the sheet opens: `first` its first focusable control, `panel` the panel itself. |
| `key` | string | required | Identity of the sheet and the id of its overlay; keep it unique among open overlays. |
| `on_open_change` | callback or `()` | — | Called with `false` when Escape or a backdrop press dismisses the sheet. |
| `open` | bool | required | Whether the sheet is open; the caller stores the `open_change` payload and passes it back. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `side` | `"start"` or `"end"` or `"top"` or `"bottom"` | `"end"` | Window edge the sheet attaches to; `start` and `end` follow the layout direction. |
| `size` | integer 160–960 | `360` | Width of a `start` or `end` sheet, or height of a `top` or `bottom` one, in logical pixels. |
| `style` | style | — | Style merged over the root part. |
| `title` | string | required | Heading of the panel and the sheet's accessible name. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `open_change` | `on_open_change` | bool | Emitted when Escape or a backdrop press dismisses the sheet; the payload is the requested open state. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `actions` | no | yes | Buttons in a row at the end of the panel. |
| `content` | yes | no | Body of the sheet. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `backdrop`, `content`, `focus_frame`, `panel`, `root`, `title`.

## Theme

- Tokens: `border`, `focus_ring`, `space.group`, `space.related`, `spacing.xl`, `surface_raised`, `text_primary`, `typography.title`
- Environment: `density`
