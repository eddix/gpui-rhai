# GroupBox

`components/group_box` · export `GroupBox` · version 0.2.0. Generated from
[`registry/components/group_box.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/group_box.rhai); do not edit.

Labelled desktop group surface for related controls or settings.

```rhai
import "components/group_box" as group_box;

group_box::GroupBox(#{ label: "Sync", content: text("Settings") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `content` | node | required | The grouped controls or settings, `related` below the heading. |
| `description` | string or `()` | — | A muted caption under the label. |
| `disabled` | bool | `false` | Disables every control inside and dims the whole group. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Heading in the uppercase label voice under the top hairline; also the accessible name. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `content` | yes | no | The grouped controls or settings. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `description`, `label`, `root`.

## Theme

- Tokens: `border`, `space.related`, `spacing.xxs`, `text_muted`, `typography.caption`, `typography.label`
- Environment: `density`
