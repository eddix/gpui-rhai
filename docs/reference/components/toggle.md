# Toggle

`components/toggle` · export `Toggle` · version 0.2.0. Generated from
[`registry/components/toggle.rhai`](../../../registry/components/toggle.rhai); do not edit.

Controlled pressed-state tool button, distinct from Checkbox and Switch. State: caller owns pressed state.

```rhai
import "components/toggle" as toggle;

toggle::Toggle(#{ text: "Bold", pressed: true, on_pressed_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Blocks clicks and shows the disabled style. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `on_pressed_change` | callback or `()` | — | Called with the new pressed value, the opposite of `pressed`, when the toggle is clicked. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `prefix` | node or `()` | — | Node before the label, such as an icon. |
| `pressed` | bool | required | Whether the toggle is on; the caller stores the `pressed_change` payload and passes it back. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size; leave unset to inherit the environment size. |
| `style` | style | — | Style merged over the root part. |
| `suffix` | node or `()` | — | Node after the label. |
| `text` | string | required | Label text; also the accessible name. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `pressed_change` | `on_pressed_change` | bool | Emitted when the toggle is clicked; the payload is the new pressed value. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `prefix` | no | no | Node before the label, such as an icon. |
| `suffix` | no | no | Node after the label. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `label`, `prefix`, `root`, `suffix`.

## Theme

- Tokens: `selection`, `text_primary`
- Environment: `density`, `size`

## Dependencies

[`components/button`](../components/button.md)
