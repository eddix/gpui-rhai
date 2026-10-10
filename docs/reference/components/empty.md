# Empty

`components/empty` · export `Empty` · version 0.2.0. Generated from
[`registry/components/empty.rhai`](../../../registry/components/empty.rhai); do not edit.

Consistent empty-state composition with optional icon, content, and actions.

```rhai
import "components/empty" as empty;

empty::Empty(#{ title: "No projects", description: "Create one to begin." })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of node (at most 4) | `[]` | Buttons centered under the text, `related` apart. |
| `content` | node or `()` | — | Extra content under the description. |
| `description` | string or `()` | — | Why it is empty and what to do next; centered, muted, at most 420px wide. |
| `icon` | node or `()` | — | A glyph above the title, in a raised square 1.5 times `metrics.control` on each side. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `title` | string | required | What is empty, as a subtitle; also the accessible name. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `actions` | no | yes | Buttons under the text. |
| `content` | no | no | Extra content under the description. |
| `icon` | no | no | A glyph above the title. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `content`, `description`, `icon`, `root`, `title`.

## Theme

- Tokens: `metrics.control`, `metrics.row`, `space.related`, `spacing.lg`, `spacing.sm`, `surface_raised`, `text_muted`, `text_primary`, `typography.body`, `typography.subtitle`
- Environment: `density`
