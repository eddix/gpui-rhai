# Section

`patterns/section` · export `Section` · version 0.2.0. Generated from
[`registry/patterns/section.rhai`](https://github.com/eddix/gpui-rhai/blob/main/registry/patterns/section.rhai); do not edit.

Section is a titled part of a region: a subtitle, an optional description and actions on the end side, then the content.

`voice: "label"` uses the label voice instead of a subtitle, for dense side panels. A region shows one title; sections inside it never repeat the title role.

```rhai
import "patterns/section" as section;

section::Section(#{ title: "Replicas", actions: [scale_button], content: table_node })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `actions` | array of node (at most 8) | `[]` | Controls on the end side of the header; they wrap below the heading when space runs out. |
| `content` | node | required | The section body, `related` below the header. |
| `description` | string or `()` | — | A muted caption under the title. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `style` | style | — | Style merged over the root part. |
| `title` | string | required | Section heading, drawn in the `voice`; also the section's accessible name. |
| `voice` | `"title"` or `"label"` | `"title"` | `title` draws a subtitle; `label` the uppercase muted label voice, for dense side panels. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `actions` | no | yes | Header controls on the end side. |
| `content` | yes | no | The section body. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `actions`, `content`, `description`, `header`, `root`, `title`.

## Theme

- Tokens: `metrics.control`, `metrics.label_column`, `space.related`, `spacing.xxs`, `text_muted`, `text_primary`, `typography.caption`, `typography.label`, `typography.subtitle`
