# Inline

`layouts/inline` · export `Inline` · version 0.2.0. Generated from
[`registry/layouts/inline.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/layouts/inline.rhai); do not edit.

Inline places children in a row with one spacing relationship and one control size.

`size` sets the environment for every control inside, so a row shares one height.

```rhai
import "layouts/inline" as inline;

inline::Inline(#{ size: "sm", children: [button_a, button_b] })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `align` | `"start"` or `"center"` or `"end"` | `"center"` | Vertical alignment of the children within the row. |
| `children` | array of node (at most 256) | required | Nodes placed in one row, in order. |
| `gap` | `"none"` or `"unit"` or `"related"` or `"group"` or `"section"` or `"xxs"` or `"xs"` or `"sm"` or `"md"` or `"lg"` or `"xl"` | `"related"` | Space between children: a relationship (`space.*`), a scale step (`spacing.*`) or `none`. |
| `justify` | `"start"` or `"between"` or `"end"` | `"start"` | Horizontal placement: packed at the start or end, or `between`, spread to both ends. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string or `()` | — | When set, exposes the row as an accessible group with this name. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size passed to every control inside, so the row shares one height. |
| `style` | style | — | Style merged over the root part. |
| `wrap` | bool | `false` | Lets children wrap onto further lines when the row is too narrow. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `children` | yes | yes | The nodes in the row. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.

## Theme

- Tokens: `space.group`, `space.related`, `space.section`, `space.unit`, `spacing.lg`, `spacing.md`, `spacing.sm`, `spacing.xl`, `spacing.xs`, `spacing.xxs`
- Environment: `size`
