# SlidingPanel

`components/sliding_panel` · export `SlidingPanel` · version 0.3.1. Generated from
[`registry/components/sliding_panel.rhai`](../../../registry/components/sliding_panel.rhai); do not edit.

Shows `content`, and when `value` changes slides the new content in from the side of the one it replaces as it fades in: a short way (`motion_distance("moderate")`) in the `fast` duration, mirrored in right-to-left text and settled by the Host's reduced or no-motion policy. It remembers the value it showed last, so the components around it can stay stateless: Tabs uses it for `panel_motion: "slide"`, Carousel for its items.

Only the new content moves; the one it replaces is gone at once. Nothing has to be painted after it leaves, so any content can slide, native fields and virtual lists included.

```rhai
import "components/sliding_panel" as sliding_panel;

sliding_panel::SlidingPanel(#{ key: "pages", value: current, order: ["one", "two", "three"],
    content: page(current) })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `content` | node | required | The shown content, owned by the caller. |
| `key` | string | required | Stable identity of the panel, which keeps the value it showed last. |
| `order` | array of string (at most 256) | required | Every value in display order. Content after the one it replaces comes in from the end side, content before it from the start side; when either value is not in `order` the content comes in without moving. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | Axis of the slide: `horizontal` from the start or end side, `vertical` from above or below. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `value` | string | required | Identity of the shown content; when it changes, the new content slides in. |
| `wrap` | bool | `false` | Whether `order` is a loop: the content comes in from the side of the shorter way round, so the first value after the last comes from the end side. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`, `slide`.
