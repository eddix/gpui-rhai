# Card

`components/card` · export `Card` · version 0.2.0. Generated from
[`registry/components/card.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/components/card.rhai); do not edit.

General themed content surface with optional header and footer slots.

```rhai
import "components/card" as card;

card::Card(#{ header: text("Account"), content: text("Ada") })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `content` | node | required | The card body; header, content and footer sit `group` apart within `metrics.inset`. |
| `footer` | node or `()` | — | A node below the content. |
| `header` | node or `()` | — | A heading node above the content, in `subtitle` typography. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `variant` | `"block"` or `"outline"` | `"block"` | `block` fills with `surface_raised`; `outline` draws a hairline, for cards on a raised surface. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `content` | yes | no | The card body. |
| `footer` | no | no | The node below the content. |
| `header` | no | no | The heading above the content. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `content`, `footer`, `header`, `root`.

## Theme

- Tokens: `border`, `metrics.inset`, `radius.lg`, `space.group`, `surface_raised`, `text_primary`, `typography.subtitle`
- Environment: `corners`, `density`
