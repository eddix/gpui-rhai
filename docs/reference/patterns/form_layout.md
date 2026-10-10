# FormLayout

`patterns/form_layout` · export `FormLayout` · version 0.2.0. Generated from
[`registry/patterns/form_layout.rhai`](../../../registry/patterns/form_layout.rhai); do not edit.

FormLayout aligns labels on one column, groups fields, and lines the submit row up with the fields.

Horizontal labels sit in a metrics.control high cell so they share the control's text line. Fields are `related`, groups `group` apart (vertical forms one step larger).

```rhai
import "patterns/form_layout" as form_layout;

form_layout::FormLayout(#{ key: "service", label: "Service", fields: [
    #{ label: "Name", control: name_input, required: true }],
    submit: [save_button] })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `fields` | array of object (at most 64) | `[]` | Fields of a form without groups, in order; used only while `groups` is empty. |
| `groups` | array of object (at most 32) | `[]` | Field groups, each under an optional title; when not empty, `fields` is ignored. |
| `key` | string | required | Keys the root and prefixes the accessibility ids of the fields, such as `<key>-field-0-label`. |
| `label` | string | required | Accessible name of the form; it is not drawn. |
| `label_width` | length or `()` | — | Width of the label column of a horizontal form; unset, it is `metrics.label_column`. Ignored when vertical. |
| `orientation` | `"horizontal"` or `"vertical"` | `"horizontal"` | `horizontal` puts the labels in a column beside the controls; `vertical` stacks each label above its control. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `submit` | array of node (at most 4) | `[]` | Action buttons in a row under the fields; in a horizontal form the row starts past the label column. |

### `fields[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `control` | node | required | The control; it keeps its own width, so a control that should be wide sets one. |
| `description` | string or `()` | — | Helper text in caption type under the control; it describes the control while there is no `error`. |
| `error` | string or `()` | — | Message in `text.danger` under the control, announced as an alert; also set the control's own `error`. |
| `label` | string | required | Label text, in the label column or above the control in a vertical form. |
| `required` | bool | `false` | Adds a `text.danger` asterisk to the label and marks the control required. |

### `groups[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `fields` | array of object (at most 64) | required | Fields of the group, in order. |
| `title` | string or `()` | — | Heading above the group, drawn upper-case in `label` type and `text_muted`. |

### `groups[].fields[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `control` | node | required | The control; it keeps its own width, so a control that should be wide sets one. |
| `description` | string or `()` | — | Helper text in caption type under the control; it describes the control while there is no `error`. |
| `error` | string or `()` | — | Message in `text.danger` under the control, announced as an alert; also set the control's own `error`. |
| `label` | string | required | Label text, in the label column or above the control in a vertical form. |
| `required` | bool | `false` | Adds a `text.danger` asterisk to the label and marks the control required. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `submit` | no | yes | The action buttons under the fields. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `control`, `description`, `error`, `field`, `group`, `group_title`, `label`, `root`, `submit`.

## Theme

- Tokens: `border`, `metrics.control`, `metrics.label_column`, `space.group`, `space.related`, `space.section`, `space.unit`, `spacing.xxs`, `text.danger`, `text_muted`, `text_primary`, `typography.body`, `typography.caption`, `typography.label`
- Environment: `density`
