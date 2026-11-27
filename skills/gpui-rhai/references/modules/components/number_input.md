# NumberInput

`components/number_input` · export `NumberInput` · version 0.1.0. Generated from
[`registry/components/number_input.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/number_input.rhai); do not edit.

NumberInput is a controlled numeric field with a native editor, locale and steppers. State: a local draft and transient step feedback; the caller owns the committed number.

```rhai
import "components/number_input" as number_input;

number_input::NumberInput(#{key:"budget",label:"Token budget",value:4000,min:0,max:8000,step:100,unit:"tokens"})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Blocks focus and all edits. |
| `error` | string or `()` | — | Marks the field invalid and replaces the helper text. |
| `fraction_digits` | integer 0–12 | `6` | Maximum fractional digits presented after committing edits. |
| `grouping` | bool | `true` | Whether the formatted value includes grouping marks. |
| `hint` | string or `()` | — | Quiet helper text below the field. |
| `key` | string | required | Stable identity of the numeric field. |
| `label` | string | required | Visible and accessible field name. |
| `large_step` | number ≥ 0.000000000001 or `()` | — | Page Up and Page Down distance; defaults to ten steps. |
| `max` | number or `()` | — | Optional inclusive upper bound. |
| `min` | number or `()` | — | Optional inclusive lower bound and origin of the step grid. |
| `minimum_fraction_digits` | integer 0–12 | `0` | Minimum visible fractional digits after commit; must not exceed fraction_digits. |
| `on_change` | callback or `()` | — | Called when a committed edit or step proposes a different numeric value. |
| `optional` | bool | `false` | Shows a localized optional marker beside the label. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string or `()` | — | Hint drawn within an empty value. |
| `prefix` | node or `()` | — | Decorative prefix placed just before the number. |
| `read_only` | bool | `false` | Allows selection and copy without editing or steppers. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Explicit control size or inherited size. |
| `step` | number ≥ 0.000000000001 | `1.0` | Step size. Steps land on the grid based at min or zero. |
| `style` | style | — | Style merged over the root part. |
| `unit` | string or `()` | — | Visible unit following the value; not part of its text or numeric payload. |
| `value` | number or `()` | required | Controlled number. Unit means an empty field. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | number or `()` | A finite numeric proposal or unit if the field was cleared. Not emitted for partial drafts. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `error`, `field`, `field_invalid`, `field_read_only`, `input`, `label`, `message`, `optional`, `placeholder`, `prefix`, `root`, `step`, `steps`, `unit`.

## Theme

- Tokens: `border`, `control.border`, `danger`, `focus_ring`, `metrics.field_h`, `metrics.field_pad`, `metrics.field_radius`, `radius.sm`, `space.spacing_unit`, `space.unit`, `surface_raised`, `text_muted`, `text_primary`, `typography.caption`, `typography.control`, `typography.label`
- Environment: `density`, `size`
