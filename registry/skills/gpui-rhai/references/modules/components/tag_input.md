# TagInput

`components/tag_input` · export `TagInput` · version 0.1.0. Generated from
[`registry/components/tag_input.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/tag_input.rhai); do not edit.

A single native editor for a controlled, ordered list of removable tags.

```rhai
import "components/tag_input" as tag_input;

tag_input::TagInput(#{ key:"labels", label:"Labels", values:["design"], on_change:Fn("labels_changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `avatar` | bool | `false` | Draws a decorative initial avatar for each tag. |
| `disabled` | bool | `false` | Displays the tags, but disables editor and removal. |
| `error` | string or `()` | — | External validation message; marks the editor invalid without erasing the draft. |
| `hint` | string or `()` | — | Quiet explanation below the field until an error takes its place. |
| `key` | string | required | Stable identity of the field and native editor. |
| `label` | string | required | Visible label and accessible name of the editor. |
| `max` | integer 1–128 or `()` | — | Maximum additions; a visible count appears when supplied. |
| `max_characters` | integer 1–256 | `96` | Maximum grapheme clusters in each newly entered tag. |
| `normalize` | `"trim"` or `"lowercase"` | `"trim"` | Trim incoming tags, optionally lowercasing before duplicate checks. |
| `on_change` | callback or `()` | — | Proposes one full tag list for a single edit, batch paste or removal. |
| `optional` | bool | `false` | Shows Optional by the label. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `placeholder` | string or `()` | — | Hint in an empty editor, including when chips already exist. |
| `split_on_space` | bool | `false` | Treat spaces and tabs as separators in addition to commas, semicolons and line breaks. |
| `style` | style | — | Style merged over the root part. |
| `validation` | `"text"` or `"email"` | `"text"` | Email requires a nonempty local part and a dotted domain, without whitespace. |
| `values` | array of string (at most 128) | required | Controlled ordered, distinct tags. Apply each accepted on_change list as the new value. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | array of string (at most 128) | A complete controlled list proposal. A batch paste never emits intermediate lists. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `chip`, `chip_armed`, `chip_avatar`, `chip_body`, `chip_flash`, `chip_remove`, `chip_remove_icon`, `count`, `count_full`, `editor`, `error`, `field`, `footer`, `header`, `hint`, `invalid`, `label`, `optional`, `placeholder`, `root`.

## Theme

- Tokens: `accent`, `border`, `danger`, `field.placeholder`, `focus_ring`, `radius.md`, `radius.sm`, `selection`, `space.spacing_unit`, `space.unit`, `spacing.xs`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.caption`, `typography.control`, `typography.initials`
- Environment: `density`, `size`

## Dependencies

[`components/tag`](tag.md)
