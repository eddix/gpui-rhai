# StatusBar

`components/status_bar` · export `StatusBar` · version 0.2.0. Generated from
[`registry/components/status_bar.rhai`](../../../registry/components/status_bar.rhai); do not edit.

Compact application footer with stable logical start, center, and end regions.

```rhai
import "components/status_bar" as status_bar;

status_bar::StatusBar(#{ label: "Editor status", start: [text("Ready")], end: [text("UTF-8")] })
```

## Props

| Prop | Type | Required or default | Description |
|---|---|---|---|
| `center` | array of node (at most 16) | `[]` | Fields centered in the bar. |
| `end` | array of node (at most 16) | `[]` | Fields at the end edge; they keep their width. |
| `key` | string | — | Stable identity of this instance among its siblings; keeps its state across renders. |
| `label` | string | required | Accessible name of the bar, which is announced as a status bar. |
| `part_styles` | map of style | — | Styles merged over named parts, keyed by part name. |
| `start` | array of node (at most 16) | `[]` | Fields at the start edge; they truncate when space runs out. |
| `style` | style | — | Style merged over the root part. |

## Slots

| Slot | Required | Multiple | Description |
|---|---|---|---|
| `center` | no | yes | Fields centered in the bar. |
| `end` | no | yes | Fields at the end edge. |
| `start` | no | yes | Fields at the start edge. |

## Parts

Style a part with `part_styles` or in `ui/styles.rhai`: `center`, `end`, `root`, `start`.

## Theme

- Tokens: `border`, `metrics.inset`, `metrics.statusbar`, `spacing.md`, `surface_raised`, `text_muted`, `typography.caption`
- Environment: `density`
