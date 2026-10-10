# Shimmer

`motion/shimmer` · export `Shimmer` · version 0.1.6. Generated from
[`registry/motion/shimmer.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/motion/shimmer.rhai); do not edit.

Shimmer is a loading placeholder that a beam sweeps across in an endless loop. State: stateless. Emits no events.

```rhai
import "motion/shimmer" as shimmer;

shimmer::Shimmer(#{ key: "loading", width: 220, height: 72 })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `height` | integer ≥ 1 | required | Height of the placeholder and the beam in logical pixels. |
| `key` | string | required | Motion identity of the shimmer; keep it stable so the sweep keeps its progress across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `width` | integer ≥ 1 | required | Width of the placeholder in logical pixels; the beam is a third as wide. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `beam`, `root`.
