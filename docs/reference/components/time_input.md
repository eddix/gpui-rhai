# TimeInput

`components/time_input` · export `TimeInput` · version 0.2.2. Generated from
[`registry/components/time_input.rhai`](../../../registry/components/time_input.rhai); do not edit.

Controlled locale-aware clock editing; partial text remains an editable draft.

```rhai
import "components/time_input" as time_input;

time_input::TimeInput(#{key:"clock",label:"Time",value:clock,on_change:Fn("clock_changed")})
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `disabled` | bool | `false` | Blocks editing and removes the field from traversal. |
| `error` | bool | `false` | Marks the field invalid. |
| `key` | string | required | Stable native editor identity. |
| `label` | string | required | Accessible time field name. |
| `on_change` | callback or `()` | — | Called with typed or stepped clock text. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `precision` | `"minute"` or `"second"` | `"minute"` | Minute or second precision; Up/Down steps this unit. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Explicit control size or inherited size. |
| `style` | style | — | Style merged over the root part. |
| `value` | string | required | Controlled localized clock text; incomplete drafts are allowed. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | string | Emitted for every native edit or Up/Down step; parse with ctx.parse_time. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `input`, `root`.

## Theme

- Environment: `size`

## Dependencies

[`components/input`](../components/input.md)
