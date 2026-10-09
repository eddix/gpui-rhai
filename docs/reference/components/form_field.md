# FormField

`components/form_field` · export `FormField` · version 0.2.0. Generated from
[`registry/components/form_field.rhai`](../../../registry/components/form_field.rhai); do not edit.

Associates label, control, description, required state, and error content.

State: stateless association wrapper.

```rhai
import "components/form_field" as form_field;

form_field::FormField(#{ id: "name", label: "Name", control: input_node, required: true })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `control` | node | required | The control the field describes; it is labelled by the label and described by the error or description. |
| `description` | string or `()` | — | Helper text in caption type under the control; it describes the control while there is no `error`. |
| `error` | string or `()` | — | Message in `text.danger` under the control, announced as an alert; also set the control's own `error` for its frame. |
| `id` | string | required | Stable id of the field; it keys the root and prefixes the accessibility ids `<id>-label`, `<id>-description` and `<id>-error`. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Text of the `Label` above the control; it names the control and the group. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `required` | bool | `false` | Adds a `text.danger` asterisk to the label and marks the control required for assistive tech. |
| `style` | style | — | Style merged over the root part. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `control` | yes | no | The control under the label. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `control`, `description`, `error`, `label`, `root`.

## Theme

- Tokens: `spacing.xs`, `text.danger`, `text_muted`, `typography.caption`

## Dependencies

[`components/label`](../components/label.md)
