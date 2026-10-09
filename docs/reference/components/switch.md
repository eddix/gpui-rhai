# Switch

`components/switch` · export `Switch` · version 0.2.0. Generated from
[`registry/components/switch.rhai`](../../../registry/components/switch.rhai); do not edit.

Controlled binary switch with disabled and loading states. State: controlled by the checked prop.

```rhai
import "components/switch" as switch;

switch::Switch(#{ checked: true, label: "Updates", on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `checked` | bool | required | Whether the switch is on; the caller stores the `change` payload and passes it back. |
| `disabled` | bool | `false` | Blocks clicks and shows the disabled style. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Text beside the track; also the accessible name. |
| `loading` | bool | `false` | Marks a pending change: the thumb shows a dot and the switch blocks clicks like `disabled`. |
| `on_change` | callback or `()` | — | Called with the new checked value, the opposite of `checked`, when the switch is clicked. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | bool | Emitted when the switch is clicked; the payload is the new checked value. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `label`, `root`, `thumb`, `track`.

## Theme

- Tokens: `accent`, `disabled`, `focus_ring`, `metrics.control`, `metrics.mark`, `on_accent`, `radius.sm`, `spacing.sm`, `surface`, `surface_hover`, `text_primary`, `typography.caption`, `typography.control`
- Environment: `corners`, `density`, `size`
