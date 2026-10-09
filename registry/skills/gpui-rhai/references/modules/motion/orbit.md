# Orbit

`motion/orbit` · export `Orbit` · version 0.1.6. Generated from
[`registry/motion/orbit.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/motion/orbit.rhai); do not edit.

Orbit spins a satellite dot around a center dot in an endless loop. State: stateless. Emits no events.

```rhai
import "motion/orbit" as orbit;

orbit::Orbit(#{ key: "orbit", size: 96 })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `key` | string | required | Motion identity of the orbit; keep it stable so the loop keeps its progress across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `size` | integer ≥ 8 | required | Side of the square canvas in logical pixels; the dots scale with it. |
| `style` | style | — | Style merged over the root part. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.
