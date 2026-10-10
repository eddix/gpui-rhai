# ButtonGroup

`components/button_group` · export `ButtonGroup` · version 0.2.0. Generated from
[`registry/components/button_group.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/button_group.rhai); do not edit.

Visually joins related buttons into one horizontal or vertical control.

```rhai
import "components/button_group" as button_group;

button_group::ButtonGroup(#{ label: "View", buttons: [grid_button, list_button] })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `buttons` | array of node (at most 16) | required | Buttons to join, in order, at least one; each keeps its own action and only the group's outer corners round. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the group; it is not drawn. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | `horizontal` joins the buttons in a row, `vertical` in a column. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size for the buttons inside; unset, they inherit the `size` environment. |
| `style` | style | — | Style merged over the root part. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `buttons` | yes | yes | The joined buttons. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `first`, `item`, `last`, `root`.

## Theme

- Tokens: `radius.lg`, `radius.md`, `spacing.xxs`
- Environment: `corners`, `density`, `size`
