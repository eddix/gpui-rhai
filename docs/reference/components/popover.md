# Popover

`components/popover` · export `Popover` · version 0.2.0. Generated from
[`registry/components/popover.rhai`](../../../registry/components/popover.rhai); do not edit.

Popover renders controlled content through the native per-window overlay layer.

State: controlled by the caller.

```rhai
import "components/popover" as popover;

popover::Popover(#{ key: "profile", label: "Profile", trigger: text("Open"), content: text("Profile"), open: true })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `align` | `"start"` or `"center"` or `"end"` | `"start"` | Alignment of the panel along the trigger edge; `start` and `end` follow the writing direction. |
| `content` | node | required | Body of the panel, which is 240 to 400 px wide and padded by `metrics.inset`. |
| `dismiss_on_escape` | bool | `true` | Lets Escape close the popover through `on_open_change`. |
| `dismiss_on_outside` | bool | `true` | Lets a click outside the panel and its trigger close the popover through `on_open_change`. |
| `gap` | number ≥ 0 | `8.0` | Distance between the trigger and the panel, in logical pixels. |
| `key` | string | required | Stable identity of this instance; also the overlay id a nested overlay names in `parent_overlay`. |
| `label` | string | required | Accessible name of the popover panel. |
| `on_open_change` | callback or `()` | — | Called with `true` when the trigger opens the popover and `false` when it is dismissed; store it as `open`. |
| `open` | bool | required | Whether the panel is shown; the caller stores the `open_change` payload and passes it back. |
| `parent_overlay` | string or `()` | — | `key` of the open menu or popover this one nests in; closing that overlay closes this one too. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placement` | `"top"` or `"bottom"` or `"left"` or `"right"` | `"bottom"` | Preferred side of the trigger the panel opens on. |
| `style` | style | — | Style merged over the root part. |
| `trigger` | node | required | Node that anchors the panel; clicking it, or Enter and Space on it, toggles the popover. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `open_change` | `on_open_change` | bool | Emitted when the trigger or a dismissal asks to open or close; the payload is the requested `open`. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `content` | yes | no | Body of the panel. |
| `trigger` | yes | no | Node that anchors and toggles the panel. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `focus_frame`, `root`, `trigger`.

## Theme

- Tokens: `border`, `focus_ring`, `metrics.inset`, `radius.lg`, `space.related`, `surface_raised`, `typography.body`
- Environment: `corners`, `density`
