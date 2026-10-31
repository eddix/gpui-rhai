# SelectableCardGroup

`components/selectable_card_group` · export `SelectableCardGroup` · version 0.2.2. Generated from
[`registry/components/selectable_card_group.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/selectable_card_group.rhai); do not edit.

Small controlled rich choices with one roving selection tab stop. `content` is presentational; independent interactive controls belong to `actions`, outside the radio/checkbox. Loading/error content does not disable selection.

```rhai
import "components/selectable_card_group" as selectable_card_group;

selectable_card_group::SelectableCardGroup(#{ key: "region", label: "Region", values: ["sg"],
    items: [#{ value: "sg", label: "Singapore", content: text("3 replicas") }], on_change: Fn("changed") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `allow_empty` | bool | `false` | Allows deselecting the final value; false rejects empty user proposals but never rewrites controlled values. |
| `columns` | integer 1–64 | `3` | Grid column count; Up/Down move by this display-index stride. |
| `disabled` | bool | `false` | Blocks selection and removes choice surfaces from tab order; independent actions keep their own policy. |
| `indicator` | `"radio"` or `"check"` or `"none"` or `()` | — | Additional visible selection mark; defaults to radio in single mode and check in multiple mode. With none, a Selected caption preserves a non-color cue. |
| `items` | array of object (at most 64) | required | Small choices in visual order. Use List or Select for larger collections. |
| `key` | string | required | Stable group identity and item-ref prefix. |
| `label` | string | required | Accessible group name. |
| `layout` | `"row"` or `"column"` or `"grid"` | `"row"` | Row, column or equal-column grid. Row cards stretch to equal heights. |
| `mode` | `"single"` or `"multiple"` | `"single"` | Radio choices or independent checkboxes. |
| `on_change` | callback or `()` | — | Full next values array from selection. Focus movement alone never emits. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `select_on_focus` | bool | `true` | Single mode arrows select their focused target. False moves focus only until Enter/Space. Multiple mode always moves focus only. |
| `size` | `"xs"` or `"sm"` or `"md"` or `"lg"` or `()` | — | Optional size environment for content and independent actions; card insets follow density. |
| `style` | style | — | Style merged over the root part. |
| `values` | array of string (at most 64) | required | Controlled selected values, at most one in single mode; no implicit initial selection. |

### `items[]` fields

| Field | Type | Required or default | Description |
|---|---|---|---|
| `actions` | node or `()` | — | Independent action area outside the choice. Clicking or typing here never changes selection; its controls retain their own disabled policy. |
| `content` | node | required | Presentational rich content, exposed as the choice description. Do not place links, inputs or buttons here. |
| `disabled` | bool | `false` | Blocks selection and skips this item during navigation. |
| `label` | string | required | Accessible name and visible title. |
| `value` | string | required | Unique nonempty value. |

## Events

| Event | Callback prop | Payload | Description |
|---|---|---|---|
| `change` | `on_change` | array of string (at most 64) | Full next selected values; app owns acceptance. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `card`, `card_selected`, `content`, `indicator`, `item`, `root`, `title`.

## Theme

- Tokens: `accent`, `border`, `disabled`, `focus_ring`, `metrics.inset`, `metrics.mark`, `on_accent`, `radius.lg`, `radius.xs`, `space.group`, `space.related`, `space.unit`, `surface_hover`, `surface_raised`, `text_muted`, `text_primary`, `typography.caption`, `typography.subtitle`
- Environment: `density`, `size`

## Dependencies

[`components/toggle_group`](toggle_group.md)
