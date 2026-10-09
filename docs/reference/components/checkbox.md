# Checkbox

`components/checkbox` · export `Checkbox` · version 0.2.0. Generated from
[`registry/components/checkbox.rhai`](../../../registry/components/checkbox.rhai); do not edit.

Controlled checkbox with checked, unchecked, and indeterminate states. State: controlled by checked/indeterminate props.

```rhai
import "components/checkbox" as checkbox;

checkbox::Checkbox(#{ checked: true, label: "Remember", on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `checked` | bool | required | Whether the box is checked; the caller stores `checked` from the `change` payload and passes it back. |
| `disabled` | bool | `false` | Blocks clicks and shows the disabled style. |
| `indeterminate` | bool | `false` | Shows the minus mark and a mixed accessible state over `checked`; the caller stores it from the `change` payload. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Text beside the box; also the accessible name. |
| `on_change` | callback or `()` | — | Called with the next `#{ checked, indeterminate }` state when the box is clicked. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | object | Emitted when the box is clicked; the payload is its next state, and a mixed box becomes checked. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `box`, `label`, `mark`, `root`.

## Theme

- Tokens: `accent`, `border`, `disabled`, `focus_ring`, `metrics.control`, `metrics.mark`, `on_accent`, `radius.xs`, `spacing.sm`, `surface_hover`, `surface_raised`, `text_primary`, `typography.control`
- Environment: `corners`, `density`, `size`
