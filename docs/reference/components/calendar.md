# Calendar

`components/calendar` · export `Calendar` · version 0.4.3. Generated from
[`registry/components/calendar.rhai`](../../../registry/components/calendar.rhai); do not edit.

The single calendar engine shared by DatePicker and TimeRangePicker.

```rhai
import "components/calendar" as calendar;

calendar::Calendar(#{key:"day",label:"Date",value:"2026-10-16",on_change:Fn("date_changed")})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `comparison_end` | string or `()` | — | Last date of the optional secondary range band, inclusive. |
| `comparison_start` | string or `()` | — | First date of the optional secondary range band. |
| `disabled` | bool | `false` | Disables calendar input. |
| `end` | string or `()` | — | Controlled range end, inclusive; unset while choosing the end. |
| `first_weekday` | `"sunday"` or `"monday"` or `"tuesday"` or `"wednesday"` or `"thursday"` or `"friday"` or `"saturday"` or `()` | — | Overrides the locale's first weekday for the grid and Home/End. |
| `key` | string | required | Stable calendar identity. |
| `label` | string | required | Accessible calendar name. |
| `max_date` | string or `()` | — | Inclusive latest ISO date. |
| `min_date` | string or `()` | — | Inclusive earliest ISO date. |
| `mode` | `"single"` or `"range"` | `"single"` | Single date or inclusive date range. |
| `month_anchor` | string or `()` | — | Initial visible month and cursor, independent of the selected range start. |
| `months` | integer 1–2 | `1` | One or two consecutive calendar months. |
| `on_change` | callback or `()` | — | Called with the chosen ISO date; range ownership stays with the caller. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `range_bands` | bool | `false` | Draw continuous range and preview bands, with only in-month dates presented. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Inherited control size unless explicitly set. |
| `style` | style | — | Style merged over the root part. |
| `today` | string or `()` | — | Presentation-zone ISO today; unset follows ctx.today and the Host clock. |
| `value` | string or `()` | — | Controlled selected ISO date, or range start. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string | Emitted when a day is activated by pointer or keyboard. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `banded_cell`, `banded_grid`, `banded_month_title`, `banded_nav`, `banded_range`, `banded_selected`, `banded_today`, `banded_weekday`, `day`, `day_comparison`, `day_disabled`, `day_focused`, `day_outside`, `day_preview`, `day_range`, `day_selected`, `day_today`, `grid`, `header`, `month_title`, `next`, `previous`, `root`, `week`, `weekday`, `weekdays`.

## Theme

- Tokens: `accent`, `accent_hover`, `disabled`, `focus_ring`, `metrics.control`, `metrics.marker`, `on_accent`, `radius.md`, `radius.sm`, `selection`, `space.spacing_unit`, `space.unit`, `spacing.sm`, `spacing.xs`, `surface_hover`, `text.accent`, `text_muted`, `text_primary`, `typography.body`, `typography.control`, `typography.label`
- Environment: `density`, `size`
