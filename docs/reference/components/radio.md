# Radio

`components/radio` · export `Radio` · version 0.2.0. Generated from
[`registry/components/radio.rhai`](../../../registry/components/radio.rhai); do not edit.

Controlled radio option. RadioGroup owns roving keyboard behavior. State: controlled by the selected prop.

```rhai
import "components/radio" as radio;

radio::Radio(#{ value: "team", selected: true, label: "Team", on_select: Fn("selected") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Blocks clicks and shows the disabled style. |
| `focus_mark` | bool | `true` | Whether focus shows the ring on this option's circle; RadioGroup sets it only on its keyboard cursor. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Text beside the circle; also the accessible name. |
| `on_select` | callback or `()` | — | Called with `value` when the option is clicked, even if it is already selected. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `selected` | bool | required | Whether this is the chosen option; the caller derives it from the value it stores. |
| `style` | style | — | Style merged over the root part. |
| `value` | string | required | Value this option stands for; passed to `on_select` and used as the option's key. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `select` | `on_select` | string | Emitted when the option is clicked, even if it is already selected; the payload is `value`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `indicator`, `label`, `root`.

## Theme

- Tokens: `accent`, `border`, `disabled`, `focus_ring`, `metrics.control`, `metrics.mark`, `spacing.sm`, `surface_hover`, `surface_raised`, `text_primary`, `typography.control`
- Environment: `density`, `size`
