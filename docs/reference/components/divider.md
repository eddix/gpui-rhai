# Divider

`components/divider` · export `Divider` · version 0.2.0. Generated from
[`registry/components/divider.rhai`](../../../registry/components/divider.rhai); do not edit.

Divider separates adjacent content without owning layout spacing.

```rhai
import "components/divider" as divider;

divider::Divider(#{ orientation: "horizontal" })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | `horizontal` draws a full-width line; `vertical` a full-height one. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `thickness` | integer ≥ 1 | `1` | Line thickness in logical pixels; values above 8 are drawn as 8. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.

## Theme

- Tokens: `border`
