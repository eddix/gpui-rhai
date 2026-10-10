# Stack

`layouts/stack` · export `Stack` · version 0.2.0. Generated from
[`registry/layouts/stack.rhai`](../../../registry/layouts/stack.rhai); do not edit.

Stack places children in a column with one spacing relationship between them.

Layouts are visually neutral: no padding, color or frame. Prefer relationship names so the intent stays readable and the nesting audit can explain itself.

```rhai
import "layouts/stack" as stack;

stack::Stack(#{ gap: "group", children: [section_a, section_b] })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `align` | `"stretch"` or `"start"` or `"center"` or `"end"` | `"stretch"` | Horizontal alignment of the children; `stretch` makes each as wide as the stack. |
| `children` | array of node (at most 512) | required | Nodes placed in one column, in order. |
| `fill` | bool | `false` | Grows the stack to the remaining height of its parent. |
| `gap` | `"none"` or `"unit"` or `"related"` or `"group"` or `"section"` or `"xxs"` or `"xs"` or `"sm"` or `"md"` or `"lg"` or `"xl"` | `"related"` | Space between children: a relationship (`space.*`), a scale step (`spacing.*`) or `none`. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string or `()` | — | When set, exposes the column as an accessible group with this name. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `children` | yes | yes | The nodes in the column. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `root`.

## Theme

- Tokens: `space.group`, `space.related`, `space.section`, `space.unit`, `spacing.lg`, `spacing.md`, `spacing.sm`, `spacing.xl`, `spacing.xs`, `spacing.xxs`
