# TimeRangePicker

`components/time_range_picker` · export `TimeRangePicker` · version 0.2.2. Generated from
[`registry/components/time_range_picker.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/time_range_picker.rhai); do not edit.

Controlled relative windows and explicit date-time drafts, committed only on Apply.

```rhai
import "components/time_range_picker" as time_range_picker;

time_range_picker::TimeRangePicker(#{key:"period",label:"Query period",
    value:#{kind:"relative",duration_ms:3600000},on_change:Fn("range_changed")})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Disables the picker. |
| `key` | string | required | Stable picker and overlay identity. |
| `label` | string | required | Accessible picker name. |
| `max` | integer or `()` | — | Latest allowed absolute epoch instant, inclusive; unset permits future instants. |
| `max_span_ms` | integer ≥ 1 or `()` | — | Maximum duration for either range kind. |
| `min` | integer or `()` | — | Earliest allowed absolute epoch instant, inclusive. |
| `on_change` | callback or `()` | — | Called with an accepted relative preset or complete validated absolute range. |
| `parent_overlay` | string or `()` | — | Parent overlay identity when nested in a dialog. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `precision` | `"minute"` or `"second"` | `"minute"` | Native time field precision. |
| `presets` | array of object (at most 32) | `[]` | Quick relative presets; unset uses localized 15 minute, hour, day and week choices. |
| `revision` | integer | `0` | Caller revision token; change it to replace an open draft even when the range value is equal. |
| `shift_buttons` | bool | `false` | Offers previous/next windows shifted by their span, as absolute snapshots. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Explicit size or inherited size. |
| `style` | style | — | Style merged over the root part. |
| `time_zone` | string | `"local"` | local, UTC, offset:<minutes>, or IANA zone shared with charts. |
| `value` | object or object | required | Controlled relative duration or absolute epoch-millisecond range. |
| `zone_switch` | bool | `false` | Offers local and UTC draft presentation; does not mutate the controlled range. |

### `presets[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `duration_ms` | integer ≥ 1 | required | Positive trailing duration in milliseconds. |
| `label` | string | required | Localized preset text. |

### `value` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `duration_ms` | integer ≥ 1 | required | Trailing duration resolved by the caller at query time. |
| `kind` | `"relative"` | required | Relative range tag. |

### `value` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `end` | integer | required | End instant in epoch milliseconds, after start. |
| `kind` | `"absolute"` | required | Absolute range tag. |
| `start` | integer | required | Inclusive start instant in epoch milliseconds. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | any value | A validated controlled range proposal; no event fires for half-edited drafts or cancel. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `calendar`, `error`, `field`, `fields`, `panel`, `preset`, `presets`, `root`, `trigger`, `trigger_value`, `zone`.

## Theme

- Tokens: `border`, `focus_ring`, `metrics.control`, `metrics.field_pad`, `metrics.inset`, `radius.lg`, `radius.md`, `selection`, `spacing.lg`, `spacing.sm`, `spacing.xs`, `spacing.xxs`, `surface_hover`, `surface_raised`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.control`
- Environment: `density`, `size`

## Dependencies

[`components/button`](button.md), [`components/calendar`](calendar.md), [`components/input`](input.md), [`components/time_input`](time_input.md)
