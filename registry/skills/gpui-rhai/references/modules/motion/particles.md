# Particles

`motion/particles` · export `Particles` · version 0.1.6. Generated from
[`registry/motion/particles.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/motion/particles.rhai); do not edit.

Bounded deterministic particle field. Geometry is built once; Canvas paints it in one batch. State: stateless. Emits no events.

```rhai
import "motion/particles" as particles;

particles::Particles(#{ key: "stars", width: 640, height: 240, count: 96 })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `count` | integer 1–512 | required | Number of particles; the Host motion quality caps it at 64 for low and 256 for medium. |
| `height` | integer ≥ 8 | required | Height of the field in logical pixels. |
| `key` | string | required | Motion identity of the field; keep it stable so the rotation keeps its progress across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `width` | integer ≥ 8 | required | Width of the field in logical pixels. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.
