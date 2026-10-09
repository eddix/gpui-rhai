# RangeSlider

`components/range_slider` · export `RangeSlider` · version 0.2.0. Generated from
[`registry/components/range_slider.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/range_slider.rhai); do not edit.

Controlled two-thumb range selection with native pointer preview and keyboard thumbs. State: caller owns accepted values.

```rhai
import "components/range_slider" as range_slider;

range_slider::RangeSlider(#{ key:"price", label:"Price", values:#{low:20,high:80}, on_change:Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Blocks pointer and keyboard input and greys the thumbs. |
| `high_label` | string | `"Maximum"` | Accessible name of the high thumb. |
| `key` | string | required | Names the native slider, which holds the drag preview across renders; it also keys the root. |
| `label` | string | required | Visible label at the start of the header row; also the accessible name of the slider. |
| `low_label` | string | `"Minimum"` | Accessible name of the low thumb. |
| `max` | number | `100.0` | Highest value, at the end of the track; must be above `min`. |
| `min` | number | `0.0` | Lowest value, at the start of the track; must be below `max`. |
| `minimum_gap` | number ≥ 0 | `0.0` | Smallest distance kept between `low` and `high`; a thumb stops there. At most `max - min`. |
| `on_change` | callback or `()` | — | Called with `#{ low, high }` when an arrow, Home or End key moves a thumb and when a drag ends. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | `horizontal` is 240 logical pixels wide (set a width on `root` to change it); `vertical` has a 180-pixel track. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `show_value` | bool | `true` | Shows `low – high` at the end of the header row. |
| `step` | number > 0 | `1.0` | Distance between accepted values, counted from `min`; one arrow key moves a thumb one step. At most `max - min`. |
| `style` | style | — | Style merged over the root part. |
| `values` | object | required | Current range; the caller stores the `change` payload and passes it back. |

### `values` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `high` | number | required | Value of the high thumb; at most `max` and at least `low + minimum_gap`. |
| `low` | number | required | Value of the low thumb; at least `min`. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | object | Emitted when a key moves a thumb and when a drag ends, not during it; the payload is the new range, snapped to `step`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `control`, `fill`, `header`, `label`, `root`, `thumb`, `track`, `value`.

## Theme

- Tokens: `accent`, `metrics.control`, `spacing.sm`, `spacing.xs`, `surface_hover`, `text_muted`, `text_primary`, `typography.body`
- Environment: `density`, `size`

## Dependencies

[`components/slider`](slider.md)
