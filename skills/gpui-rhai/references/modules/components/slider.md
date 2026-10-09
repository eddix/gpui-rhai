# Slider

`components/slider` · export `Slider` · version 0.2.0. Generated from
[`registry/components/slider.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/slider.rhai); do not edit.

Controlled single-value range input with native drag preview and commit events. State: caller owns value; the native primitive retains only hot drag preview.

```rhai
import "components/slider" as slider;

slider::Slider(#{ key: "volume", value: 40, min: 0, max: 100, step: 5, label: "Volume", on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Blocks pointer and keyboard input and greys the thumb. |
| `key` | string | required | Names the native slider, which holds the drag preview across renders; it also keys the root. |
| `label` | string | required | Visible label at the start of the header row; also the accessible name. |
| `max` | number | `100.0` | Highest value, at the end of the track; must be above `min`. |
| `min` | number | `0.0` | Lowest value, at the start of the track; must be below `max`. |
| `on_change` | callback or `()` | — | Called with the new value on each arrow, Home or End key and when a drag ends. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | `horizontal` is 240 logical pixels wide (set a width on `root` to change it); `vertical` has a 180-pixel track. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `show_value` | bool | `true` | Shows the value (or `value_label`) at the end of the header row, over the end of the track. |
| `step` | number > 0 | `1.0` | Distance between accepted values, counted from `min`; one arrow key moves one step. At most `max - min`. |
| `style` | style | — | Style merged over the root part. |
| `value` | number | required | Current value, within `min`..`max`; the caller stores the `change` payload and passes it back. |
| `value_label` | string or `()` | — | Text shown for the value, such as `40%`; unset, the number itself is shown. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | number | Emitted on each arrow, Home or End key and when a drag ends, not during it; the payload is the new value, snapped to `step`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `control`, `fill`, `header`, `label`, `root`, `thumb`, `track`, `value`.

## Theme

- Tokens: `accent`, `disabled`, `focus_ring`, `metrics.control`, `metrics.mark`, `spacing.sm`, `spacing.xs`, `surface`, `surface_hover`, `text_muted`, `text_primary`, `typography.body`
- Environment: `density`, `size`
