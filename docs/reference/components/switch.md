# Switch

`components/switch` · export `Switch` · version 0.4.3. Generated from
[`registry/components/switch.rhai`](../../../registry/components/switch.rhai); do not edit.

Controlled binary switch with disabled and loading states. State: controlled by the checked prop.

```rhai
import "components/switch" as switch_component;

switch_component::Switch(#{ checked: true, label: "Updates", on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `checked` | bool | required | Whether the switch is on; the caller stores the `change` payload and passes it back. |
| `description` | string | `""` | Secondary text below the label and the switch's accessible description; clicking it proposes the same change. |
| `disabled` | bool | `false` | Blocks clicks and shows the disabled style. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Text beside the track; also the accessible name. |
| `loading` | bool | `false` | Marks a pending change: the thumb shows a dot and the switch blocks clicks like `disabled`. |
| `on_change` | callback or `()` | — | Called with the new checked value, the opposite of `checked`, when the switch is clicked. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placement` | `"start"` or `"end"` | `"start"` | Logical start puts the track before the text. End fills the available row with text first and the track at the opposite edge; respects RTL. |
| `size` | `"sm"` or `"md"` or `()` | — | Overrides the inherited size. Stock sm tracks are 26×16, md 32×20; geometry still follows theme and reading scale. |
| `style` | style | — | Style merged over the root part. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | bool | Emitted when the switch is clicked; the payload is the new checked value. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `control`, `description`, `label`, `root`, `root_end`, `text`, `thumb`, `track`.

## Theme

- Tokens: `accent`, `choice.description`, `choice.fill`, `disabled`, `focus_ring`, `metrics.choice_line`, `metrics.control`, `metrics.mark`, `metrics.switch_h`, `metrics.switch_inset`, `metrics.switch_thumb`, `metrics.switch_w`, `on_accent`, `radius.pill`, `spacing.sm`, `surface`, `surface_hover`, `switch.checked_thumb`, `switch.off`, `switch.thumb`, `switch.thumb_off`, `switch.thumb_off_ink`, `switch.thumb_on`, `switch.track_off`, `switch.track_on`, `text_primary`, `typography.caption`, `typography.control`, `typography.hint`
- Environment: `density`, `size`
