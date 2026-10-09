# Marquee

`motion/marquee` · export `Marquee` · version 0.1.6. Generated from
[`registry/motion/marquee.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/motion/marquee.rhai); do not edit.

Marquee scrolls its content horizontally in a loop. State: stateless. Emits no events.

```rhai
import "motion/marquee" as marquee;

marquee::Marquee(#{ key: "news", content: text("News"), distance: 320 })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `content` | node | required | Node that scrolls inside the clipped root. |
| `distance` | integer ≥ 1 | required | How far the content moves left in one loop, in logical pixels; a loop lasts six `ambient` motion durations. |
| `key` | string | required | Motion identity of the marquee; keep it stable so the loop keeps its progress across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `content` | yes | no | The scrolling content, passed as the `content` prop. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `root`.
