# Skeleton

`components/skeleton` · export `Skeleton` · version 0.2.0. Generated from
[`registry/components/skeleton.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/skeleton.rhai); do not edit.

Content placeholder with optional Rust-side pulse animation.

```rhai
import "components/skeleton" as skeleton;

skeleton::Skeleton(#{ key: "avatar", width: 40, height: 40, animated: true })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `animated` | bool | `true` | Pulses the placeholder's opacity on the `ambient` motion duration. |
| `height` | integer ≥ 0 | required | Height in logical pixels. |
| `key` | string | required | Stable identity of the placeholder; keeps its pulse continuous across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `radius` | integer ≥ 0 | `0` | Corner radius in logical pixels. |
| `style` | style | — | Style merged over the root part. |
| `width` | integer ≥ 0 | required | Width in logical pixels. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.

## Theme

- Tokens: `surface_hover`
