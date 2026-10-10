# BorderBeam

`motion/border_beam` · export `BorderBeam` · version 0.1.6. Generated from
[`registry/motion/border_beam.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/motion/border_beam.rhai); do not edit.

BorderBeam traces an accent stroke around a rectangle in an endless loop. State: stateless. Emits no events.

```rhai
import "motion/border_beam" as border_beam;

border_beam::BorderBeam(#{ key: "beam", width: 220, height: 72 })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `height` | integer ≥ 1 | required | Height of the traced rectangle in logical pixels. |
| `key` | string | required | Motion identity of the beam; keep it stable so the loop keeps its progress across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `width` | integer ≥ 1 | required | Width of the traced rectangle in logical pixels. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.
