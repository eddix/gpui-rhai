# DatePicker

`components/date_picker` · export `DatePicker` · version 0.2.0. Generated from
[`registry/components/date_picker.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/date_picker.rhai); do not edit.

Controlled single-date field composed from public date data helpers, atoms, and Overlay. State: keyed open/visible-month/focused-date transients.

```rhai
import "components/date_picker" as date_picker;

date_picker::DatePicker(#{ key: "appointment", label: "Appointment date", value: "2026-09-01", on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `clearable` | bool | `false` | Shows a clear button in the trigger while a date is set; it emits `change` with `()`. |
| `disabled` | bool | `false` | Disables the trigger so the calendar cannot open. |
| `display_style` | `"short"` or `"medium"` or `"long"` | `"short"` | Length of the trigger's date text, formatted for the current locale. |
| `error` | bool | `false` | Marks the value invalid: the trigger border turns `danger` and accessibility reports it invalid. |
| `key` | string | required | Identity of the picker; also its overlay id and the prefix of its inner keys (`<key>-trigger`, `<key>-panel`). |
| `label` | string | required | Accessible name of the trigger. |
| `max_date` | string or `()` | — | Latest selectable date, strict ISO `YYYY-MM-DD`, inclusive; not before `min_date`, and `value` must not be later. |
| `min_date` | string or `()` | — | Earliest selectable date, strict ISO `YYYY-MM-DD`, inclusive; earlier days are disabled and `value` must not be earlier. |
| `on_change` | callback or `()` | — | Called with the picked ISO date, or `()` when the date is cleared. |
| `parent_overlay` | string or `()` | — | Key of the open overlay this one nests in, such as a Dialog; the calendar stacks above it and closes with it. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string | `""` | Trigger text while no date is selected. |
| `placement` | `"top"` or `"bottom"` or `"left"` or `"right"` | `"bottom"` | Side of the trigger the calendar opens on. |
| `presets` | array of object (at most 32) | `[]` | Quick-pick buttons in the calendar footer; a preset outside `min_date` to `max_date` is disabled. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Control size; leave unset to inherit the environment size. The preset footer is always `xs`. |
| `style` | style | — | Style merged over the root part. |
| `value` | string or `()` | — | Selected date as strict ISO `YYYY-MM-DD`, or `()` for none; the caller stores the `change` payload and passes it back. |
| `width` | length or `()` | — | Width of the trigger; unset is 280 logical pixels. |

### `presets[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Shows the preset but blocks picking it. |
| `label` | string | required | Button text; must not be blank. |
| `value` | string | required | Strict ISO `YYYY-MM-DD` date the preset picks; must be unique among the presets. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string or `()` | Emitted when a day or preset other than `value` is picked, or the date is cleared; the payload is the ISO date or `()`. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `clear`, `day`, `day_disabled`, `day_focused`, `day_outside`, `day_selected`, `day_today`, `focus_frame`, `footer`, `grid`, `header`, `month_title`, `next`, `panel`, `preset`, `previous`, `root`, `trigger`, `trigger_icon`, `trigger_value`, `week`, `weekday`, `weekdays`.

## Theme

- Tokens: `accent`, `accent_hover`, `border`, `control.hover`, `danger`, `disabled`, `focus_ring`, `metrics.control`, `metrics.control_pad`, `metrics.field_pad`, `metrics.icon`, `metrics.inset`, `metrics.marker`, `on_accent`, `radius.lg`, `radius.md`, `spacing.sm`, `spacing.xs`, `surface_hover`, `surface_raised`, `text.accent`, `text_muted`, `text_primary`, `typography.body`, `typography.control`, `typography.label`
- Environment: `corners`, `density`, `size`
