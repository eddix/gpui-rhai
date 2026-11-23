# BillingSchedule

`patterns/billing_schedule` · export `BillingSchedule` · version 0.1.0. Generated from
[`registry/patterns/billing_schedule.rhai`](../../../registry/patterns/billing_schedule.rhai); do not edit.

Build a phase and preview the first invoice and phase value without executing billing.

```rhai
import "patterns/billing_schedule" as billing_schedule;

billing_schedule::BillingSchedule(#{key:"phase",schedule:"build",draft:#{phase:"Build-out",start:"2026-01-01",duration:"milestone",lines:[]}})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `catalog` | array of object (at most 64) | `[]` | Products available to add in the order supplied. |
| `controlled` | bool | `false` | Only the caller's draft is authoritative; an edit merely emits a proposal. |
| `currency` | string or `()` | — | Display prefix, defaults to $. Amounts always use two decimal places. |
| `draft` | object | required | Starting local draft or current controlled snapshot. |
| `key` | string | required | Stable card instance. |
| `on_change` | callback or `()` | — | Receives the candidate draft only after an actual edit, not on mount. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `revision` | integer ≥ 0 | `0` | Accepted Host revision echoed in edits. |
| `schedule` | string | required | Phase identity. A new schedule clears the local draft. |
| `style` | style | — | Style merged over the root part. |
| `title` | string or `()` | — | Optional visible heading. |

### `catalog[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable catalog ID. |
| `name` | string | required | Product name. |
| `price_minor` | integer 0–1000000000 | required | Initial price in integer minor units. |
| `repeat` | `"once"` or `"monthly"` or `"quarterly"` or `"yearly"` | required | Initial cadence for a new line. |

### `draft` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `duration` | `"milestone"` or `"1m"` or `"3m"` or `"6m"` or `"12m"` | required | Phase length or milestone. |
| `lines` | array of object or object or object (at most 64) | required | Stable lines; at most one discount and one minimum. |
| `phase` | string | required | Editable phase name. |
| `start` | string | required | Start as YYYY-MM-DD. |

### `draft.lines[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable line ID. |
| `kind` | `"product"` | required | Product discriminant. |
| `name` | string | required | Visible product name. |
| `price_minor` | integer 0–1000000000 | required | Price per repeat, in integer minor units. |
| `repeat` | `"once"` or `"monthly"` or `"quarterly"` or `"yearly"` | required | Product cadence. |
| `timing` | `"advance"` or `"arrears"` | required | Only advance products enter the first invoice. |

### `draft.lines[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `discount_bps` | integer 0–10000 | required | Discount in basis points; 10,000 is 100%. |
| `id` | string | required | Stable line ID. |
| `kind` | `"discount"` | required | Discount discriminant. |
| `name` | string | required | Visible discount name. |

### `draft.lines[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `id` | string | required | Stable line ID. |
| `kind` | `"minimum"` | required | Minimum commitment discriminant. |
| `name` | string | required | Visible minimum name. |
| `price_minor` | integer 0–1000000000 | required | Phase total floor in integer minor units. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | object | The proposed phase, start, duration and lines, with the affected line ID when applicable. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `action`, `actions`, `caption`, `card`, `compact`, `date`, `date_trigger`, `duration`, `empty`, `end`, `field`, `field_caption`, `fields`, `fields_compact`, `head_row`, `heading`, `inline`, `line`, `line_kind`, `lines`, `menu`, `min_tag`, `name`, `option`, `phase`, `phase_dot`, `phase_field`, `phase_root`, `price`, `price_column`, `price_field`, `price_input`, `price_root`, `range`, `remove`, `repeat_column`, `root`, `summary`, `summary_amount`, `summary_label`, `timing`, `title`, `totals`, `trigger`, `trigger_icon`.

## Theme

- Tokens: `billing_schedule.card`, `billing_schedule.ink`, `billing_schedule.inset`, `billing_schedule.line`, `billing_schedule.muted`, `billing_schedule.text`, `billing_schedule.tint`, `billing_schedule.tray`, `border`, `focus_ring`, `radius.lg`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`

## Dependencies

[`components/combobox`](../components/combobox.md), [`components/date_picker`](../components/date_picker.md), [`components/input`](../components/input.md), [`components/number_input`](../components/number_input.md)
