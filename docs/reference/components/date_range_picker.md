# DateRangePicker

`components/date_range_picker` · export `DateRangePicker` · version 0.1.0. Generated from
[`registry/components/date_range_picker.rhai`](../../../registry/components/date_range_picker.rhai); do not edit.

DateRangePicker proposes an inclusive civil-date interval and a comparison only on Apply. State: an open-only range draft and the roving preset position. The Host owns committed dates and comparison mode.

```rhai
import "components/date_range_picker" as date_range_picker;

date_range_picker::DateRangePicker(#{key:"period",label:"Reporting period",value:#{start:"2026-10-01",end:"2026-10-08"},today:"2026-10-16"})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `compare` | bool | `true` | Exposes the comparison selector and secondary calendar band. |
| `compare_mode` | `"off"` or `"previous"` or `"year"` | `"off"` | Controlled comparison mode. Store the returned mode together with the range. |
| `default_open` | bool | `false` | Opens on first mount only. |
| `disabled` | bool | `false` | Disables opening, editing and applying. |
| `first_weekday` | `"sunday"` or `"monday"` or `"tuesday"` or `"wednesday"` or `"thursday"` or `"friday"` or `"saturday"` or `()` | — | Overrides the locale's first weekday. |
| `key` | string | required | Stable picker and popover identity. |
| `label` | string | required | Accessible name of the trigger and panel. |
| `max_date` | string or `()` | — | Latest selectable day, inclusive. Defaults to today. |
| `min_date` | string or `()` | — | Earliest selectable day, inclusive. |
| `on_change` | callback or `()` | — | Receives the complete range, compare mode and derived optional comparison on Apply. |
| `parent_overlay` | string or `()` | — | Parent overlay when opened from a dialog. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `presets` | array of object (at most 32) or `()` | — | Overrides the default list. An empty list hides presets. |
| `revision` | integer | `0` | Change to invalidate a draft even when its controlled range is equal. |
| `style` | style | — | Style merged over the root part. |
| `today` | string or `()` | — | Anchor for Today and trailing presets. Defaults to the Host's local civil day. |
| `value` | object or `()` | — | Controlled ascending range. Unset presents the last 30 days inside the available bounds. |

### `presets[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `end` | string | required | Inclusive ISO end date. |
| `key` | string | required | Unique preset key. |
| `label` | string | required | Localized preset label. |
| `start` | string | required | Inclusive ISO start date. |

### `value` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `end` | string | required | Inclusive last ISO date. |
| `start` | string | required | Inclusive first ISO date. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | object | A complete inclusive range and optional derived comparison. Cancel and preset preview never emit. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `body`, `calendar`, `compare_swatch`, `comparison`, `count`, `footer`, `panel`, `preset`, `preset_compact`, `preset_custom`, `preset_selected`, `presets`, `presets_compact`, `root`, `summary`, `trigger`, `trigger_comparison`, `trigger_icon`, `trigger_value`.

## Theme

- Tokens: `accent`, `border`, `focus_ring`, `radius.lg`, `radius.sm`, `selection`, `space.spacing_unit`, `space.unit`, `spacing.md`, `spacing.sm`, `spacing.xs`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.control`
- Environment: `density`, `size`

## Dependencies

[`components/button`](../components/button.md), [`components/calendar`](../components/calendar.md), [`components/toggle_group`](../components/toggle_group.md)
